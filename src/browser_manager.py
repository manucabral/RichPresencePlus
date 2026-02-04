"""
Browser Manager for Rich Presence Plus.
"""

import os
import re
import json
import time
import winreg
import subprocess
import socket
import threading
from contextlib import closing
from typing import Optional, List, Dict, Set, Callable, Any
import requests
from websockets.sync.client import connect as ws_connect
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry
from .constants import config
from .logger import logger


def _open_key(hive: int, path: str, access=winreg.KEY_READ):
    """
    Open a registry key.
    """
    try:
        return winreg.OpenKey(hive, path, 0, access)
    except OSError:
        return None


def _enum_subkeys(key) -> List[str]:
    """
    Enumerate subkeys of a registry key.
    """
    i = 0
    names: List[str] = []
    while True:
        try:
            names.append(winreg.EnumKey(key, i))
            i += 1
        except OSError:
            break
    return names


def _get_value(key, name: str = "") -> Optional[str]:
    """
    Get a string value from a registry key.
    """
    try:
        value, _ = winreg.QueryValueEx(key, name)
        return value
    except OSError:
        return None


def _clean_path(path: Optional[str]) -> Optional[str]:
    """
    Clean a registry path string by stripping quotes and extra data.
    """
    if not path or not isinstance(path, str):
        return None
    s = str(path).strip()
    s = s.strip('"').strip("'")
    if "," in s:
        s = s.split(",", maxsplit=1)[0].strip()
    if not s.lower().endswith(".exe"):
        m = re.search(r'"([^"]+?\.exe)"', path, flags=re.IGNORECASE)
        if m:
            s = m.group(1)
        else:
            m2 = re.search(r"([A-Za-z]:\\[^\s\"]+?\.exe)", path, flags=re.IGNORECASE)
            if m2:
                s = m2.group(1)
            else:
                s = s.split()[0]
    s = s.rstrip('"').rstrip("'")
    return s or None


def _is_executable(path: str) -> bool:
    """
    Check if the provided path points to an executable file.
    """
    try:
        return os.path.isfile(path) and path.lower().endswith(".exe")
    except OSError:
        return False


def _wait_for_port(
    port: int,
    host: str = "127.0.0.1",
    timeout: float = 10.0,
    process: Optional[subprocess.Popen] = None,
) -> bool:
    """
    Wait for a TCP port to be open on host:port until timeout seconds.
    """
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with closing(socket.create_connection((host, port), timeout=0.5)):
                return True
        except OSError:
            if process is not None and process.poll() is not None:
                return False
            time.sleep(0.2)
    return False


def read_smi() -> List[Dict[str, Optional[str]]]:
    """
    Reads the Start Menu Internet registry keys to find installed browsers.
    """
    results: List[Dict[str, Optional[str]]] = []
    roots = [
        (winreg.HKEY_CURRENT_USER, r"SOFTWARE\Clients\StartMenuInternet"),
        (winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\Clients\StartMenuInternet"),
        (winreg.HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Clients\StartMenuInternet"),
    ]
    for hive, base in roots:
        key = _open_key(hive, base)
        if not key:
            continue
        for sub in _enum_subkeys(key):
            subkey = _open_key(hive, base + "\\" + sub)
            if not subkey:
                continue
            caps = _open_key(hive, base + "\\" + sub + r"\Capabilities")
            appname = _get_value(caps, "ApplicationName") if caps else None
            if not appname:
                appname = _get_value(subkey, "") or sub

            icon = _get_value(caps, "ApplicationIcon") if caps else None
            path: Optional[str] = None
            if icon and isinstance(icon, str):
                path = _clean_path(icon)

            cmdkey = _open_key(hive, base + "\\" + sub + r"\shell\open\command")
            cmd = _get_value(cmdkey, "") if cmdkey else None
            if not path and cmd and isinstance(cmd, str):
                if '"' in cmd:
                    parts = cmd.split('"')
                    if len(parts) >= 2:
                        path = parts[1]
                else:
                    path = cmd.split()[0]
                path = _clean_path(path)

            results.append(
                {"name": appname, "path": os.path.normpath(path) if path else None}
            )
    return results


class Browser:
    """
    Represents a browser instance targeting a specific port.
    """

    def __init__(
        self, id_: str, name: str, path: str, port: int, chromium: bool = True
    ):
        """
        Initializes a Browser instance.
        """
        self.id = id_
        self.name = name
        self.path = path
        self.port = port
        self.chromium = chromium
        self.bidi_ws_url: Optional[str] = None # disabled for now

    def to_dict(self) -> Dict[str, Any]:
        """
        Convert the Browser instance to a dictionary.
        """
        return {
            "id": self.id,
            "name": self.name,
            "path": self.path,
            "port": self.port,
            "chromium": self.chromium,
            "bidi_ws_url": self.bidi_ws_url,
        }


class BrowserManager:
    """
    Manages browser instances targeting a specific port.
    """

    def __init__(
        self,
        target_port: int = config.browser_target_port,
        profile_name: str = config.browser_profile_name,
    ):
        self.loaded: bool = False
        self.profile_name: str = profile_name
        self.target_port: int = target_port
        self.browsers: List[Browser] = []
        self.processes: Dict[str, subprocess.Popen] = {}
        self.launched_browser: Optional[Browser] = None
        self.current_ws_url: Optional[str] = None
        self._last_load_time: float = 0.0
        self._load_ttl: int = getattr(config, "browser_load_ttl", 60)
        self._last_cdp_check: float = 0.0
        self._cdp_ttl: int = getattr(config, "browser_cdp_ttl", 2)
        self.session = requests.Session()
        retries = Retry(
            total=1, backoff_factor=0.3, status_forcelist=(429, 500, 502, 503, 504)
        )
        adapter = HTTPAdapter(max_retries=retries)
        self.session.mount("http://", adapter)
        self.session.mount("https://", adapter)
        self.bidi_ws_url: Optional[str] = None # disabled for now

    def _wait_for_ws(self, ws_url: str, timeout: float = 5.0) -> bool:
        """
        Wait until a WebSocket URL accepts a connection within `timeout` seconds.
        Open & close immediately to test reachability (no BiDi usage).
        """
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                conn = ws_connect(ws_url, open_timeout=0.5)
                conn.close()
                logger.debug("WebSocket handshake succeeded to %s", ws_url)
                return True
            except Exception:
                time.sleep(0.2)
        return False

    def load(self, callback: Optional[Callable] = None) -> List[Browser]:
        """
        Load installed browsers from the system registry.
        """
        now = time.time()
        if self.loaded and (now - self._last_load_time) < self._load_ttl:
            logger.debug(
                "Using cached browser list (age=%.1fs)", now - self._last_load_time
            )
            return self.browsers
        raw_result = read_smi()
        seen: Set[str] = set()
        items: List[Browser] = []
        for entry in raw_result:
            name = entry.get("name")
            raw_path = entry.get("path")
            path = _clean_path(raw_path) if raw_path else None
            if not name:
                continue
            if not path or not _is_executable(path):
                continue
            if name in seen:
                continue
            seen.add(name)
            browser_id = name.lower().replace(" ", "_")
            logger.info("Discovered: %s (%s)", name, browser_id)
            browser_instance = Browser(
                id_=browser_id,
                name=name,
                path=os.path.normpath(path),
                port=self.target_port,
                chromium="firefox" not in browser_id.lower(),
            )
            items.append(browser_instance)
        self.browsers = items
        self.loaded = True
        self._last_load_time = time.time()
        try:
            logger.info("Invoking update_launched_browser")
            self.update_launched_browser(callback=callback)
        except Exception as exc:
            logger.debug("update_launched_browser failed during load : %s", exc)
        return items

    def update_launched_browser(self, callback: Optional[Callable] = None) -> None:
        """
        Update the currently launched browser based on CDP connection.
        BiDi is disabled.
        """
        logger.info("Updating launched browser...")

        if self.launched_browser:
            logger.info("Browser already set: %s", self.launched_browser.name)
            if callback:
                callback(self.current_ws_url)
            return

        # try CDP
        url = self.current_cdp()
        if callback:
            logger.info("Calling callback with URL: %s", url)
            callback(url)
        if url:
            self.launched_browser = self.identify_cdp(url)
            if self.launched_browser:
                logger.info("Current connected browser: %s", self.launched_browser.name)
                return

        logger.info("No CDP browser currently connected.")
        self.launched_browser = None

    def launch(self, browser: Browser, callback: Optional[Callable] = None) -> bool:
        """
        Launch the specified browser with the configured profile and CDP port.
        """
        if self.launched_browser:
            logger.error(
                "Cannot launch %s, another browser is already launched: %s",
                browser.name,
                self.launched_browser.name,
            )
            return False
        if not _is_executable(browser.path):
            logger.error("Cannot launch, invalid executable path: %s", browser.path)
            return False
        if not self.loaded:
            logger.error("Cannot launch, browsers not loaded")
            return False
        if browser.id in self.processes and self.launched_browser is not None:
            logger.error("%s is already launched", browser.id)
            return False
        profile_dir = (
            config.base_dir.parent / "profiles" / browser.name / self.profile_name
        )
        logger.info("Using profile: %s", profile_dir)
        try:
            is_firefox = (
                "firefox" in os.path.basename(browser.path).lower()
                or "firefox" in browser.id.lower()
            )

            if is_firefox: # BiDi is disabled, not supported
                logger.error(
                    "Firefox detected but BiDi is disabled - Firefox is not supported"
                )
                logger.error(
                    "Please use a Chromium-based browser (Chrome, Edge, Brave, etc.)"
                )
                return False

            # Chromium-like browsers
            command = [
                browser.path,
                "--remote-allow-origins=*",
                f"--remote-debugging-port={browser.port}",
                f"--user-data-dir={profile_dir}",
            ]
            process = subprocess.Popen(
                command,
                shell=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )
            self.processes[browser.id] = process
            logger.info(
                "Launching browser %s with command: %s",
                browser.name,
                " ".join(command),
            )
            logger.info("Launched %s (PID: %d)", browser.name, process.pid)
            self.launched_browser = browser

            if callback:

                def wait_and_callback():
                    logger.info("Waiting for CDP port %d to be ready...", browser.port)
                    if _wait_for_port(browser.port, timeout=10.0, process=process):
                        logger.info("CDP port %d is ready", browser.port)
                        callback(f"ws://127.0.0.1:{browser.port}/json/version")
                    else:
                        logger.error(
                            "CDP port %d did not become available", browser.port
                        )
                        try:
                            process.terminate()
                            process.wait(timeout=2)
                        except Exception:
                            try:
                                process.kill()
                            except Exception:
                                pass
                        self.launched_browser = None
                        callback(None)

                wait_thread = threading.Thread(target=wait_and_callback, daemon=True)
                wait_thread.start()
            else:
                logger.info("Waiting for CDP port %d to be ready...", browser.port)
                if not _wait_for_port(browser.port, timeout=10.0, process=process):
                    logger.error("CDP port %d did not become available", browser.port)
                    try:
                        process.terminate()
                        process.wait(timeout=2)
                    except Exception:
                        try:
                            process.kill()
                        except Exception:
                            pass
                    self.launched_browser = None
                    return False

            return True
        except Exception as exc:
            logger.error("Error launching browser %s: %s", browser.name, exc)
            return False

    def get_browser_by_name(self, name: str) -> Optional[Browser]:
        """
        Get a browser instance by its name.
        """
        if not self.loaded:
            logger.warning("Not loaded")
            return None
        for browser in self.browsers:
            if browser.name.lower() == name.lower():
                return browser
        logger.warning("Browser not found: %s", name)
        return None

    def identify_cdp(self, ws_url: str, timeout: int = 5) -> Optional[Browser]:
        """
        Identify the browser connected via the given CDP WebSocket URL.
        """
        if not self.loaded:
            logger.warning("Not loaded")
            return None
        if self.launched_browser is not None:
            return self.launched_browser
        connection = None
        try:
            logger.info("Identifying CDP browser at %s", ws_url)
            connection = ws_connect(ws_url, open_timeout=timeout)
            get_version_payload = {"id": 1, "method": "SystemInfo.getInfo"}
            connection.send(json.dumps(get_version_payload))
            version_response = json.loads(connection.recv())

            command_line = None
            if isinstance(version_response, dict) and "result" in version_response:
                command_line = (
                    version_response["result"].get("commandLine")
                    or version_response["result"].get("CommandLine")
                    or None
                )
            if command_line:
                match = re.search(r'"([^"]+)"', command_line)
                if match:
                    exe_path = match.group(1)
                else:
                    exe_path = None
                for browser in self.browsers:
                    if (
                        exe_path
                        and os.path.normpath(browser.path).lower()
                        == os.path.normpath(exe_path).lower()
                    ):
                        return browser
            return None

        except Exception as exc:
            logger.error("Error identifying CDP browser: %s", exc)
            self.launched_browser = None
            return None
        finally:
            if connection:
                try:
                    connection.close()
                except Exception:
                    pass

    def close_port(self, port: int) -> bool:
        """
        netstat -ano | findstr :9222
        """
        cmd = f"netstat -ano | findstr :{port}"
        logger.info("Closing processes using port %d", port)
        try:
            output = subprocess.check_output(
                cmd, shell=True, text=True, creationflags=subprocess.CREATE_NO_WINDOW
            )
            lines = output.strip().splitlines()
            pids: Set[int] = set()
            for line in lines:
                parts = line.split()
                if len(parts) >= 5:
                    pid_str = parts[4]
                    try:
                        pid = int(pid_str)
                        pids.add(pid)
                    except ValueError:
                        continue
            for pid in pids:
                logger.info("Terminating process with PID %d", pid)
                proc = subprocess.Popen(
                    f"taskkill /PID {pid} /F",
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    creationflags=subprocess.CREATE_NO_WINDOW,
                    shell=False,
                )
                try:
                    proc.wait(timeout=5)
                except Exception:
                    pass
            return True
        except subprocess.CalledProcessError as exc:
            logger.error("Error closing port %d: %s", port, exc)
            return False

    def close_cdp_browser(self, ws_url: str, timeout: int = 5) -> bool:
        """
        Close the browser connected via the given CDP WebSocket URL.
        """
        if not self.loaded:
            logger.warning("Not loaded")
            return False
        ws_closed = False
        connection = None
        try:
            logger.info("Closing CDP browser at %s", ws_url)

            # attempt CDP graceful close
            try:
                connection = ws_connect(ws_url, open_timeout=timeout)
                payload = {"id": 1, "method": "Browser.close", "params": {}}
                connection.send(json.dumps(payload))
                response = connection.recv()
                data = json.loads(response)
                logger.info("CDP closed successfully %s", data)
                self.launched_browser = None
                ws_closed = True
            except Exception as exc:
                logger.debug("Error closing CDP websocket: %s", exc)

        except Exception as exc:
            logger.error("Error closing CDP browser: %s", exc)
            return False
        finally:
            if connection:
                try:
                    connection.close()
                except Exception:
                    pass

        if not ws_closed:
            try:
                browser_id = ws_url.rstrip("/").split("/")[-1]
                close_url = (
                    f"http://localhost:{self.target_port}/json/close/{browser_id}"
                )
                response = requests.get(close_url, timeout=timeout)
                if response.status_code != 200:
                    logger.warning(
                        "HTTP close returned status %s", response.status_code
                    )
                else:
                    logger.info("HTTP Closed CDP browser successfully")
            except Exception as exc:
                logger.warning("Cannot close via HTTP: %s", exc)

        for browser_id, process in list(self.processes.items()):
            if not process:
                continue
            if process.poll() is not None:
                logger.info("Process for browser %s already exited", browser_id)
                del self.processes[browser_id]
                continue
            logger.info("Terminating process for browser %s", browser_id)
            try:
                process.terminate()
                process.wait(timeout=timeout)
            except Exception:
                try:
                    process.kill()
                except Exception:
                    pass
            del self.processes[browser_id]
        return True

    def _check_cdp_connection(self, target_url: str) -> Optional[str]:
        logger.info("Checking CDP connection at %s", target_url)
        resp = self.session.get(target_url, timeout=2)
        logger.debug("CDP connection response status: %s", resp.status_code)
        if resp.status_code != 200:
            logger.debug("CDP status %s, not connected", resp.status_code)
            self.current_ws_url = None
            self._last_cdp_check = time.time()
            return None
        data = resp.json()
        ws = data.get("webSocketDebuggerUrl")
        self.current_ws_url = ws
        self._last_cdp_check = time.time()
        logger.debug("Found websocket url: %s", ws)
        return ws

    def current_cdp(self) -> Optional[str]:
        """
        Get the current CDP WebSocket URL if a browser is connected.
        Only for Chromium-like browsers.
        """
        now = time.time()
        if (now - self._last_cdp_check) < self._cdp_ttl:
            logger.debug(
                "Returning cached CDP url (age=%.2fs)", now - self._last_cdp_check
            )
            return self.current_ws_url

        try:
            logger.info("Checking Ws (chromium) connection...")
            status = self._check_cdp_connection(
                f"http://127.0.0.1:{self.target_port}/json/version"
            )
            return status
        except requests.RequestException as exc:
            logger.info("No CDP (ws) connection found: %s", exc)
            self.current_ws_url = None
            self._last_cdp_check = time.time()
            return None

    def all_in_obj(self) -> list:
        """
        Get all discovered browsers as a list of dictionaries.
        """
        return [
            {
                "id": browser.id,
                "name": browser.name,
                "path": browser.path,
                "port": browser.port,
            }
            for browser in self.browsers
        ]
