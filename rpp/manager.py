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
from .constants import Constants
from .logger import get_logger, RPPLogger
from .presence import Presence
from .runtime import Runtime
from .utils import download_github_folder, exist_github_folder, get_available_presences


class Manager:
    """
    Manager class for Rich Presence Plus.
    """

    def __init__(
        self,
        presences_folder: str = Constants.PRESENCES_FOLDER,
        runtime=None,
        runtime_interval=Constants.RUNTIME_INTERVAL,
        dev_mode=Constants.DEV_MODE,
    ):
        self.log: RPPLogger = get_logger("Manager")
        self.dev_mode: bool = dev_mode
        self.web_enabled: bool = False
        self.folder: str = presences_folder
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
            for file in files:
                if file.startswith("__"):
                    continue
                if file == "main.py":
                    self.load_presence(root, file)

    def load_presence(self, root: str, file: str) -> None:
        """
        Load a presence from a file.
        """
        self.log.info(f"Loading presence from {root}")
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
                    f"Presence {folder_name} not found in remote repository."
                )
                return
        try:
            spec = importlib.util.find_spec(module_path)
            if spec is None:
                raise ImportError(f"Module {module_path} not found")
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            """
            if self.checkRestrictedModules(module):
                return
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
                    if instance.enabled:
                        self.presences.append(instance)
        except Exception as exc:
            self.log.error(f"Error loading {module_path}: {exc}")
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
                        f"{presence.name} uses web features but runtime is not connected"
                    )
            while not self.stop_event.is_set() and presence.running:
                presence.on_update(runtime=self.runtime)
                time.sleep(presence.update_interval)
        except Exception as exc:
            self.log.error(f"Error running {presence.name}: {exc}")

    def __runtime_thread(self) -> None:
        """
        Run the runtime in a thread.
        """
        self.log.info(f"Runtime thread started (interval: {self.runtime_interval}s)")
        self.runtime.running = True
        while (
            not self.stop_event.is_set()
            and self.runtime.connected
            and self.runtime.running
        ):
            self.runtime.update()
            time.sleep(self.runtime_interval)
        self.runtime.running = False

    def __main_thread(self) -> None:
        """
        Run the main thread.
        """
        while not self.stop_event.is_set():
            time.sleep(self.presence_interval)
            for presence in self.presences:
                if presence.running:
                    presence.update()
        self.stop_presences()

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
                self.log.warning(f"Presence {presence_name} already downloaded.")
                return
        try:
            download_github_folder(
                Constants.PRESENCES_ENPOINT.format(presence_name=presence_name),
                os.path.join(self.folder, presence_name),
                os.getenv("GITHUB_API_TOKEN"),
            )
            self.log.info(f"Presence {presence_name} downloaded.")
        except Exception as exc:
            self.log.error(f"Downloading {presence_name}: {exc}")

    def remove_presence(self, presence_name: str) -> None:
        """
        Remove a presence.

        Args:
            presence_name (str): The presence name.
        """
        presence_path = os.path.join(self.folder, presence_name)
        if not os.path.exists(presence_path):
            self.log.error(f"Presence {presence_name} not found.")
            return

        for presence in self.presences:
            if presence.name == presence_name and presence.running:
                self.log.warning(f"Cannot remove running presence {presence_name}.")
                return
        try:
            for root, _, files in os.walk(presence_path):
                for file in files:
                    os.remove(os.path.join(root, file))
            os.rmdir(presence_path)
            self.log.info(f"Presence {presence_name} removed.")
        except Exception as exc:
            self.log.error(f"Removing {presence_name}: {exc}")

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
            self.log.info(f"Temp directory: {tempdir}")
            for presence in self.presences:
                if not presence.enabled:
                    continue
                try:
                    presence.prepare(log=False)
                    self.log.info(f"Comparing {presence.name}...")
                    download_github_folder(
                        Constants.PRESENCES_ENPOINT.format(presence_name=presence.name),
                        os.path.join(tempdir, presence.name),
                        os.getenv("GITHUB_API_TOKEN"),
                    )
                    if filecmp.dircmp(
                        presence.path, os.path.join(tempdir, presence.name)
                    ).diff_files:
                        self.log.info(f"Inconsistency found in {presence.name}.")
                        self.log.info(f"Please update {presence.name}.")
                        presence.enabled = False
                        break
                except Exception as exc:
                    self.log.error(f"Comparing {presence.name}: {exc}")
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
        self.executor.submit(self.__main_thread)
        self.log.info("Presences started.")

    def run_main(self) -> None:
        """
        Run the main thread.
        """
        self.executor.submit(self.__main_thread)

    def run_presence(self, presence: Presence) -> None:
        """
        Run a presence.
        """
        if not presence.enabled:
            self.log.error(f"{presence.name} is not enabled.")
            return
        if presence.running:
            self.log.warning(f"{presence.name} already running.")
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
        self.executor.submit(self.__runtime_thread)

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
                self.run_runtime()
            while not self.stop_event.is_set():
                time.sleep(0.1)
        except KeyboardInterrupt:
            self.log.info("Stopping presences...")
            self.stop_event.set()
        finally:
            self.executor.shutdown(wait=True)
            self.stop_presences()
            self.log.info("Presences stopped.")
