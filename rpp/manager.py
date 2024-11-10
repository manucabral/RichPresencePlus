"""
Module responsible for managing all Rich Presence Plus system.
"""

import os
import sys
import typing
import importlib.util
import threading
import concurrent.futures
import tempfile
import filecmp
import time
from .version import __version__
from .constants import Constants
from .logger import get_logger, RPPLogger
from .presence import Presence
from .runtime import Runtime
from .steam import Steam
from .utils import (
    load_env,
    download_github_folder,
    exist_github_folder,
    get_available_presences,
    check_version_compatibility,
)


# pylint: disable=R0903, R0902, R0913, W0703, R0917
class Manager:
    """
    Manager class for Rich Presence Plus.
    """

    def __init__(
        self,
        presences_folder: str = Constants.PRESENCES_FOLDER,
        runtime: Runtime = None,
        runtime_interval=Constants.RUNTIME_INTERVAL,
        steam: Steam = None,
        dev_mode=Constants.DEV_MODE,
    ):
        self.log: RPPLogger = get_logger("Manager")
        self.dev_mode: bool = dev_mode
        self.web_enabled: bool = False
        self.folder: str = presences_folder
        self.steam: Steam = steam
        self.runtime: Runtime = runtime
        self.runtime_interval: int = runtime_interval
        self.presence_interval: int = Constants.PRESENCE_INTERVAL
        self.executor: concurrent.futures.ThreadPoolExecutor = (
            concurrent.futures.ThreadPoolExecutor(max_workers=Constants.MAX_PRESENCES)
        )
        self.stop_event: threading.Event = threading.Event()
        self.presences: typing.List[Presence] = []
        self.presence_to_stop: typing.Optional[Presence] = None

        self.check_folder(presences_folder)

    def check_folder(self, folder: str) -> None:
        """
        Check if the folder exists, if not, create it.
        """
        if not os.path.exists(folder):
            os.makedirs(folder)

    def load(self) -> None:
        """
        Load all presences from the presences folder.
        """
        root_path = os.path.abspath(self.folder)
        if root_path not in sys.path:
            sys.path.append(root_path)
        for root, _, files in os.walk(self.folder):
            files.sort(key=lambda x: x != "main.py")
            for file in files:
                if file.startswith("__"):
                    continue
                if file.endswith(".py"):
                    self.load_presence(root, file)
                if file == ".env":
                    load_env(path=os.path.join(root, file), origin=root)

    def load_presence_entry_module(self, module: typing.Any, root: str) -> None:
        """
        Load entry presence module.
        """
        for attr in dir(module):
            obj = getattr(module, attr)
            if (
                isinstance(obj, type)
                and obj is not Presence
                and issubclass(obj, Presence)
            ):
                instance = obj()
                instance.path = root
                instance.set_dev_mode(self.dev_mode)
                instance.prepare(log=False)
                if instance.enabled:
                    self.presences.append(instance)

    def load_presence(self, root: str, file: str) -> None:
        """
        Load presence modules.
        """
        module_name = file[:-3]
        relative_path = os.path.relpath(root, self.folder)
        module_path = os.path.join(relative_path, module_name).replace(os.sep, ".")
        folder_name = os.path.basename(root)
        if not self.dev_mode:
            if not exist_github_folder(
                Constants.PRESENCES_ENPOINT.format(presence_name=folder_name),
                os.getenv("GITHUB_API_TOKEN"),
            ):
                self.log.warning(
                    "Presence %s not found in remote repository.", folder_name
                )
                return
        try:
            spec = importlib.util.find_spec(module_path)
            if spec is None:
                raise ImportError(f"Module {module_path} not found")
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            self.log.info("Loaded module for {%s} (%s)", relative_path, module_name)
            if file == "main.py":
                self.load_presence_entry_module(module, root)
        except Exception as exc:
            self.log.error("Error loading %s: %s", module_path, exc)
            return

    def stop_presences(self) -> None:
        """
        Stop all presences.
        """
        for presence in self.presences:
            if not presence.running:
                continue
            if not presence.enabled:
                continue
            presence.on_close()

    def __presence_thread(self, presence: Presence) -> None:
        """
        Run the presence in a thread.
        """
        try:
            presence.on_load()
            if presence.web:
                if not self.web_enabled:
                    self.web_enabled = True
                if not self.runtime.connected:
                    self.log.warning(
                        "%s uses web features but runtime is not connected",
                        presence.name,
                    )
            while not self.stop_event.is_set() and presence.running:
                presence.on_update(runtime=self.runtime, steam=self.steam)
                time.sleep(presence.update_interval)
        except Exception as exc:
            self.log.error("Error running %s: %s", presence.name, exc)

    def __runtime_thread(self, callback: typing.Callable) -> None:
        """
        Run the runtime in a thread.

        Args:
            callback (typing.Callable): The callback function.
        """
        self.log.info("Runtime thread started (interval: %ds)", self.runtime_interval)
        self.runtime.running = True
        while (
            not self.stop_event.is_set()
            and self.runtime.connected
            and self.runtime.running
        ):
            self.runtime.update()
            time.sleep(self.runtime_interval)
        self.runtime.running = False
        self.log.info("Runtime thread stopped. Maybe the browser was closed.")
        if callback == self.runtime.shutdown_callback and callback is not None:
            self.log.info("Calling shutdown callback...")
            for presence in self.presences:
                if presence.web and presence.running:
                    presence.on_close()
                    presence.running = False
                    self.log.warning(
                        "%s stopped because the runtime was closed.", presence.name
                    )
            return callback()

    def download_presence(self, presence_name: str) -> None:
        """
        Download a presence from the remote repository.

        Args:
            presence_name (str): The presence name.
        """
        if self.dev_mode:
            self.log.info("Skipping download in dev mode.")
            return
        for presence in self.presences:
            if presence.name == presence_name:
                self.log.warning("Presence %s already downloaded.", presence_name)
                return
        try:
            download_github_folder(
                Constants.PRESENCES_ENPOINT.format(presence_name=presence_name),
                os.path.join(self.folder, presence_name),
                os.getenv("GITHUB_API_TOKEN"),
            )
            self.log.info("Presence %s downloaded.", presence_name)
        except Exception as exc:
            self.log.error("On download %s: %s", presence_name, exc)

    def remove_presence(self, presence_name: str) -> None:
        """
        Remove a presence.

        Args:
            presence_name (str): The presence name.
        """
        presence_path = os.path.join(self.folder, presence_name)
        if not os.path.exists(presence_path):
            self.log.error("Presence %s not found.", presence_name)
            return

        for presence in self.presences:
            if presence.name == presence_name and presence.running:
                self.log.warning("Cannot remove running presence %s.", presence_name)
                return
        try:
            for root, _, files in os.walk(presence_path):
                for file in files:
                    os.remove(os.path.join(root, file))
            os.rmdir(presence_path)
            self.log.info("Presence %s removed.", presence_name)
        except Exception as exc:
            self.log.error("On removing %s: %s", presence_name, exc)

    def compare(self) -> None:
        """
        Compare the presences with the remote repository.
        If a difference is found, the presence is disabled.
        """
        if not self.presences:
            self.log.error("No presences loaded.")
            return
        if self.dev_mode:
            self.log.info("Skipping comparison in dev mode.")
            return
        with tempfile.TemporaryDirectory(prefix="rpp_") as tempdir:
            self.log.info("Using temp dir %s", tempdir)
            for presence in self.presences:
                if not presence.enabled:
                    continue
                if not check_version_compatibility(presence.package):
                    self.log.warning(
                        "%s is incompatible with the current RPP version (%s)",
                        presence.name,
                        __version__,
                    )
                    presence.enabled = False
                    continue
                try:
                    presence.prepare(log=False)
                    self.log.info("Comparing %s...", presence.name)
                    download_github_folder(
                        Constants.PRESENCES_ENPOINT.format(presence_name=presence.name),
                        os.path.join(tempdir, presence.name),
                        os.getenv("GITHUB_API_TOKEN"),
                    )
                    if filecmp.dircmp(
                        presence.path,
                        os.path.join(tempdir, presence.name),
                        ignore=["__pycache__", ".env"],
                    ).diff_files:
                        self.log.info("Inconsistency found in %s.", presence.name)
                        self.log.info("Please update %s.", presence.name)
                        presence.enabled = False
                        break
                except Exception as exc:
                    self.log.error("On compare %s: %s", presence.name, exc)
                    continue

    def sync_presences(self) -> typing.List[dict]:
        """
        Sync the presences with the remote repository.

        Returns:
            list[dict]: A list of synced presences.
        """
        remote_presences = get_available_presences(os.getenv("GITHUB_API_TOKEN"))
        synced_presences = []
        for presence in remote_presences:
            installed = False
            for local_presence in self.presences:
                if presence == local_presence.name:
                    installed = True
                    break
            synced_presences.append(
                {
                    "name": presence,
                    "installed": installed,
                }
            )
        return synced_presences

    def run_presences(self) -> None:
        """
        Run all presences.
        """
        for presence in self.presences:
            if not presence.enabled:
                continue
            if not self.web_enabled:
                self.web_enabled = True
            presence.running = True
            presence.prepare()
            self.executor.submit(self.__presence_thread, presence)
        self.log.info("Presences started.")

    def run_presence(self, presence: Presence) -> None:
        """
        Run a presence.
        """
        if not presence.enabled:
            self.log.error("%s is disabled.", presence.name)
            return
        if presence.running:
            self.log.warning("%s already running.", presence.name)
            return
        self.executor.submit(self.__presence_thread, presence)

    def run_runtime(self) -> None:
        """
        Run the runtime.
        """
        if not self.runtime:
            self.log.error("No runtime loaded.")
            return
        if self.runtime.running:
            self.log.warning("Runtime already running.")
            return
        if not self.runtime.connected:
            self.log.warning("Skipping runtime thread, not connected.")
            return
        self.executor.submit(self.__runtime_thread, self.runtime.shutdown_callback)

    def start(self) -> None:
        """
        Start all presences.
        """
        if not self.presences:
            self.log.error("No presences loaded.")
            return

        try:
            self.run_presences()
            if self.web_enabled:
                self.run_runtime(None)
            while not self.stop_event.is_set():
                time.sleep(0.1)
        except KeyboardInterrupt:
            self.log.info("Stopping presences...")
            self.stop_event.set()
        finally:
            self.executor.shutdown(wait=True)
            self.stop_presences()
            self.log.info("Presences stopped.")
