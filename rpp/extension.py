"""
This module provides a decorator to extend the base class of the presence for create new presences.
"""

import rpp
import json
import time

try:
    import pypresence
except ImportError:
    # that means dev mode is enabled
    pypresence = None

from .constants import Constants
from .presence import Presence
from .logger import get_logger


def extension(cls: Presence) -> Presence:
    """
    Decorator to extend the base class of the presence for create new presences.
    """

    class Wrapp(cls):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)

            if not self.name:
                self.name = cls.__name__

            self.log = get_logger(self.name, filename=f"{self.name}.log")
            if self.metadata_file:
                self.log.info("Using metadata file.")
                return

            if not hasattr(self, "client_id") or not self.client_id:
                raise ValueError("No client_id provided.")

        def info(self) -> None:
            """
            Show the presence information.
            """
            self.log.info(f"Using v{self.version} ({self.client_id}) by {self.author}")

        def data(self) -> dict:
            """
            Get the presence data.
            """
            exclude_keys = ["log", "__rpc", "path"]
            return {
                key: value
                for key, value in self.__dict__.items()
                if key not in exclude_keys
            }

        def set_dev_mode(self, mode: bool) -> None:
            """
            Set the dev mode.
            """
            self.dev_mode = mode
            self.log.info(f"Dev mode {'enabled' if mode else 'disabled'}")

        def __load_metadata(self):
            """
            Load the metadata from the metadata file.
            """
            if not self.path:
                raise ValueError("No path provided for metadata file")
            try:
                with open(self.path + "/metadata.json", "r", encoding="utf-8") as file:
                    metadata = json.load(file)
                    self.client_id = metadata.get("clientId")
                    self.name = metadata.get("name", self.name)
                    self.author = metadata.get("author", "Unknown")
                    self.web = metadata.get("web", False)
                    self.update_interval = metadata.get("updateInterval", 3)
                    self.info()
            except Exception as exc:
                self.log.error(f"Failed to load metadata: {exc}")

        def prepare(self) -> None:
            if self.metadata_file:
                self.__load_metadata()
            self.log.info("Loaded.")

            if self.dev_mode:
                return

        def on_load(self) -> None:
            """
            Called when the presence is loaded.
            """
            if self.dev_mode:
                return
            try:
                self.__rpc = pypresence.Presence(self.client_id)
                self.__rpc.connect()
            except Exception as exc:
                self.log.error(f"Failed to connect to Discord: {exc}")
                return
            super().on_load()
            self.update()

        def on_update(self, **context) -> None:
            """
            Called when the presence is updated.
            """
            super().on_update(**context)

        def on_close(self) -> None:
            """
            Called when the presence is closed.
            """
            super().on_close()
            if self.dev_mode:
                return
            try:
                if self.__rpc:
                    self.__rpc.close()
            except Exception as exc:
                self.log.error(f"Failed to close connection to Discord: {exc}")

        def force_update(self) -> None:
            """
            Force the presence to update.
            """
            if self.dev_mode:
                return
            if not self.last_update:
                self.last_update = time.time()
            if time.time() - self.last_update > Constants.PRESENCE_INTERVAL:
                self.update()
                self.log.debug(f"Forced update for {self.name}")
            else:
                self.log.debug(f"Skipping update for {self.name}")

        def update(self) -> None:
            """
            Update the presence.
            """
            if self.dev_mode:
                self.log.debug(self.data())
                return
            self.log.debug(f"Updating")
            try:
                self.large_text = f"{rpp.__title__} v{rpp.__version__}"
                if len(self.details) > 128:
                    self.details = self.details[:125] + "..."
                if len(self.state) > 128:
                    self.state = self.state[:125] + "..."
                self.__rpc.update(
                    state=self.state,
                    details=self.details,
                    large_image=self.large_image,
                    large_text=self.large_text,
                    small_image=self.small_image,
                    small_text=self.small_text,
                    start=self.start,
                    end=self.end,
                    buttons=[
                        {
                            "label": "Download App",
                            "url": "https://github.com/manucabral/RichPresencePlus",
                        },
                    ],
                )
                self.last_update = time.time()
            except Exception as exc:
                self.log.error(f"Failed to update on Discord: {exc}")

    return Wrapp
