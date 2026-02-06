"""
API module for RichPresencePlus webview interaction.
Provides the RPPApi class for JavaScript calls.
"""

import os
from typing import Optional, Dict

from src.constants import config
from src.browser_manager import BrowserManager
from src.presence_manager import PresenceManager
from src.runtime import Runtime
from src.user import get_user_settings
from src.logger import logger, set_log_level
from src.github_sync import get_remote_presences_list, install, uninstall
from src.process_manager import get_processes_by_port, close_pids, is_discord_running
from src.steam import Steam


# pylint: disable=too-many-public-methods
class RPPApi:
    """
    API class for webview JavaScript interaction.
    """

    def __init__(
        self,
        browser_manager: BrowserManager,
        presence_manager: PresenceManager,
        runtime: Runtime,
        user_settings=None,
    ):
        """
        Initializes the API with browser and presence managers.
        """
        self.bm = browser_manager
        self.pm = presence_manager
        self.rt = runtime
        self.us = user_settings if user_settings is not None else get_user_settings()
        self.steam = Steam()

        if self.steam.enabled and self.steam.accounts:
            steam_account = self.steam.accounts[0]
            logger.info("Setting default Steam account: %s", steam_account)
            self.pm.steam_account = steam_account
        self.force_cache = False

    def update_connected_browser(self) -> None:
        """Update the currently connected browser."""
        logger.info("Updating connected browser...")
        self.bm.update_launched_browser()
        # self.rt.set_bidi_connection(self.bm.bidi_session)

    def close_browser(self, url: str) -> bool:
        """Close the browser associated with the given URL."""
        logger.info("Closing browser for URL: %s", url)
        try:
            self.rt.close()
            self.rt.protocol = None

            # CDP browser only
            if not url:
                url = self.bm.current_cdp() or ""
            if not url:
                if self.bm.launched_browser:
                    logger.info("Closing browser without CDP URL")
                    try:
                        self.close_network_processes(self.bm.target_port)
                    except Exception:
                        pass
                    self.bm.launched_browser = None
                    return True
                raise ValueError("Cannot find a connected browser to close")
            browser = self.bm.identify_cdp(url)
            if not browser:
                raise ValueError("No connected browser found")
            result = self.bm.close_cdp_browser(url)
            try:
                self.close_network_processes(self.bm.target_port)
            except Exception:
                logger.warning(
                    "Failed to close network processes for port %d", self.bm.target_port
                )
            return result
        except Exception as exc:
            raise exc
        finally:
            self.rt.refresh_state()
            self.bm.update_launched_browser()

    def get_current_cdp_url(self) -> Optional[str]:
        """Get the current CDP WebSocket URL."""
        cdp = self.bm.current_cdp()
        logger.info("Getting current CDP URL")
        logger.info("URL %s", cdp)
        return cdp

    def get_connected_browser(self) -> Optional[Dict]:
        """Get the currently connected browser as a dict."""
        browser = self.bm.launched_browser
        logger.info("Getting connected browser: %s", browser)
        if browser:
            return browser.to_dict()
        return None

    def get_installed_browsers(self) -> list:
        """Get a list of installed browsers."""
        logger.info("Getting installed browsers...")
        browsers = self.bm.all_in_obj()
        return browsers

    def launch_browser(self, browser_name: str, safe_profile: bool = True) -> bool:
        """Launch a browser by name."""
        logger.info("Launching browser: %s", browser_name)
        if self.bm.launched_browser:
            raise RuntimeError("A browser is already launched")
        browser = self.bm.get_browser_by_name(browser_name)
        if not browser:
            raise ValueError(f"Browser '{browser_name}' not found")

        # reset runtime state before launching new browser
        self.rt.close()
        self.rt.protocol = None

        def on_port_ready(ws_url):
            if ws_url is None:
                logger.error("Browser launched but CDP port never became available")
                return

            logger.info("CDP port ready, initializing Runtime")
            self.rt.protocol = "cdp"
            try:
                loaded = self.rt.load(start_background=True)
                if loaded and self.rt.is_connected():
                    logger.info("Runtime CDP connection established")
                else:
                    logger.error("Runtime failed to connect to CDP")
            except Exception as exc:
                logger.error("Failed to establish CDP connection: %s", exc)

        result = self.bm.launch(browser, safe_profile=safe_profile, callback=on_port_ready)
        logger.info("Launching %s with result: %s", browser_name, result)
        return result

    def get_presence_shared_state(self, presence_name: str) -> Dict:
        """Get the shared state for a specific presence."""
        logger.info("Getting shared state for %s", presence_name)
        spec = self.pm.get_worker(presence_name)
        if not spec:
            raise ValueError(f"Presence '{presence_name}' not found")
        if not spec.shared_state:
            logger.info("No shared state for %s", presence_name)
            return {}
        
        state_dict = dict(spec.shared_state)
        last_rpc = state_dict.get("last_rpc_update", {})
        
        return {
            "state": last_rpc.get("state"),
            "details": last_rpc.get("details"),
            "start_time": last_rpc.get("start_time"),
            "end_time": last_rpc.get("end_time"),
            "large_image": last_rpc.get("large_image"),
            "large_text": last_rpc.get("large_text"),
            "small_image": last_rpc.get("small_image"),
            "small_text": last_rpc.get("small_text"),
        } 

    def get_installed_presences(self) -> list:
        """Get a list of installed presences."""
        presences = self.pm.list_workers()
        logger.info("Getting installed presences (%d total)", len(presences))
        return [
            {
                "name": spec.name,
                "path": spec.path,
                "verified": spec.verified,
                "image": spec.image,
                "client_id": spec.client_id,
                "entrypoint": spec.entrypoint,
                "description": spec.description,
                "callable_name": spec.callable_name,
                "running": spec.running,
                "interval": spec.interval,
                "enabled": spec.enabled,
                "backoff_time": spec.backoff_time,
                "web": spec.web,
                "on_exit": spec.on_exit,
                "runs": spec.runs,
            }
            for spec in presences.values()
        ]

    def start_presence(self, presence_name: str):
        """Start a presence worker by name."""
        logger.info("Starting presence: %s", presence_name)
        spec = self.pm.get_worker(presence_name)
        if not spec:
            raise ValueError(f"Presence '{presence_name}' not found")
        if spec.enabled is False:
            raise RuntimeError(f"Presence '{presence_name}' is disabled")
        self.pm.start(spec)

    def stop_presence(self, presence_name: str):
        """Stop a presence worker by name."""
        logger.info("Stopping presence: %s", presence_name)
        spec = self.pm.get_worker(presence_name)
        if not spec:
            raise ValueError(f"Presence '{presence_name}' not found")
        self.pm.stop(spec)

    def get_remote_presences(self):
        """Get a list of remote presences available."""
        logger.info("Fetching remote presences list...")
        presences = get_remote_presences_list(force_refresh=self.force_cache)
        if self.force_cache:
            self.force_cache = False
        return presences

    def install_remote_presence(self, presence_name: str):
        """Install a remote presence by name."""
        logger.info("Installing remote presence: %s", presence_name)
        if not presence_name:
            raise ValueError("Presence name is required for installation")
        path = os.path.join(config.presences_dir, presence_name)
        if os.path.exists(path):
            raise RuntimeError(f"Presence '{presence_name}' is already installed")
        success, msg = install(presence_name, path)
        logger.info("Install result: %s, %s", success, msg)
        self.force_cache = True
        self.pm.discover(force=True, dev=config.development_mode)
        return {"success": success, "message": msg}

    def remove_installed_presence(self, presence_name: str):
        """Uninstall an installed presence by name."""
        logger.info("Uninstalling presence: %s", presence_name)
        if not presence_name:
            raise ValueError("Presence name is required for uninstallation")
        path = os.path.join(config.presences_dir, presence_name)
        if not os.path.exists(path):
            raise RuntimeError(f"Presence '{presence_name}' is not installed")
        worker = self.pm.get_worker(presence_name)
        if worker and worker.running:
            return False, "Cannot uninstall a running presence"
        success, msg = uninstall(path)
        logger.info("Uninstall result: %s, %s", success, msg)
        self.force_cache = True
        self.pm.discover(force=True, dev=config.development_mode)
        return {"success": success, "message": msg}

    def get_network_processes(self):
        """Get processes using specific ports."""
        ports = [9222, self.bm.target_port]
        result = {}
        for port in ports:
            processes = get_processes_by_port(port)
            result[port] = [process.to_dict() for process in processes]
        return result

    def close_network_processes(self, port: int):
        """Close processes using a specific port."""
        try:
            processes = get_processes_by_port(port)
            close_pids(processes)
            self.bm.launched_browser = None
            self.rt.refresh_state()
            return True
        except Exception as exc:
            logger.error("Failed to close processes on port %d: %s", port, exc)
            return False

    def is_discord_running(self) -> bool:
        """Check if Discord is running."""
        return is_discord_running()

    def get_user_setting_key(self, key: str):
        """Get a user setting value by key."""
        return self.us.get_option(key)

    def set_user_setting_key(self, key: str, value):
        """
        Set a user setting value by key.
        """
        current = self.us.get_option(key)
        if str(current) == str(value):
            logger.info("No change for %s: value is already %r", key, value)
            return False
        if key == "runtime_interval":
            self.rt.interval = int(value)
            logger.info("Updated runtime interval to %d seconds", self.rt.interval)
        if key in ("browser_target_port", "profile_name"):
            logger.info("Updated %s to %s", key, value)
            try:
                cdp = self.bm.current_cdp()
                if cdp:
                    logger.info("Closing launched browser due to %s change", key)
                    self.close_browser(cdp or "")
            except Exception as exc:
                logger.error("Failed to close browser on port change: %s", exc)
            finally:
                if key == "browser_target_port":
                    self.bm.target_port = int(value)
                    self.rt.port = int(value)
                if key == "profile_name":
                    self.bm.profile_name = str(value)
        if key == "logs_level":
            set_log_level(value)
        if key == "profile_name":
            self.bm.profile_name = str(value)
        return self.us.set_option(key, value)

    def get_steam_accounts(self):
        """Get the list of Steam accounts from user settings."""
        if not self.steam.enabled:
            raise RuntimeError("Steam is disabled or not configured properly")
        accounts = self.steam.accounts
        logger.info("Retrieving %d Steam accounts", len(accounts))
        return [
            {
                "name": account.name,
                "steam_id64": account.steam_id64,
                "steam_id3": account.steam_id3,
            }
            for account in accounts
        ]

    def get_current_steam_path(self) -> str:
        """Get the current Steam configuration file path."""
        if not self.steam.enabled:
            raise RuntimeError("Steam is disabled or not configured properly")
        logger.info("Current Steam config path: %s", self.steam.config_path)
        return self.steam.config_path

    def get_current_steam_account(self):
        """Get the currently active Steam account."""
        if not self.steam.enabled:
            raise RuntimeError("Steam is disabled or not configured properly")
        account = self.pm.steam_account
        if not account:
            return None
        logger.info("Current Steam account: %s", account)
        return {
            "name": account.name,
            "steam_id64": account.steam_id64,
            "steam_id3": account.steam_id3,
        }

    def set_steam_path(self, path: str) -> bool:
        """Set the Steam configuration file path."""
        logger.info("Setting Steam config path to: %s", path)
        if not os.path.exists(path):
            raise ValueError("The specified Steam config path does not exist")
        self.steam.config_path = path
        self.steam.accounts = []
        self.steam.load_accounts()
        return True

    def set_steam_account(self, steam_id3: int) -> bool:
        """Set the active Steam account by Steam ID3."""
        logger.info("Setting active Steam account to Steam ID3: %d", steam_id3)
        if not self.steam.enabled:
            raise RuntimeError("Steam is disabled or not configured properly")
        account = next(
            (acc for acc in self.steam.accounts if acc.steam_id3 == steam_id3), None
        )
        if not account:
            raise ValueError("The specified Steam account was not found")
        self.pm.steam_account = account
        logger.info("Active Steam account set to: %s", account)
        logger.info("Stopping all presences to apply Steam account change")
        self.pm.stop_all(only_web=False)
        return True
