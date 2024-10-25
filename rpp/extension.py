"""
This module provides a decorator to extend the base class of the presence for create new presences.
"""

import rpp
import json
import time
from .rpc import ClientRPC
from .presence import Presence
from .logger import get_logger


def extension(cls: Presence) -> Presence:
    """
    Decorator to extend the base class of the presence for create new presences.
    """

    # pylint: disable=R0902
    class Wrapp(cls):
        """
        Wrapper class for the presence
        """

        def __init__(self, *args, **kwargs):
            """
            Initialize the presence.
            """
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
            self.log.info(
                "Using v%s (%s) by %s",
                self.version,
                self.client_id,
                self.author,
            )

        def data(self) -> dict:
            """
            Get the presence data.
            """
            exclude_keys = ["log", "__rpc", "path"]
            return {
                key: value if hasattr(value, "__dict__") else value
                for key, value in self.__dict__.items()
                if key not in exclude_keys
            }

        def set_dev_mode(self, mode: bool) -> None:
            """
            Set the dev mode.
            """
            self.dev_mode = mode
            self.log.info("Dev mode %s", "enabled" if mode else "disabled")

        def __load_metadata(self, log: bool = True) -> None:
            """
            Load the metadata from the metadata file.

            Args:
                log (bool): Whether to log the metadata.
            """
            if not self.path:
                raise ValueError("No path provided for metadata file")
            try:
                with open(self.path + "/metadata.json", "r", encoding="utf-8") as file:
                    metadata = json.load(file)
                    self.client_id = str(metadata.get("clientId"))
                    self.name = metadata.get("name", self.name)
                    self.author = metadata.get("author", "Unknown")
                    self.web = metadata.get("web", False)
                    self.version = metadata.get("version", None)
                    self.update_interval = metadata.get("updateInterval", 3)
                    if log:
                        self.info()
            # pylint: disable=W0703
            except FileNotFoundError:
                self.log.error("Metadata file not found.")
            except json.JSONDecodeError:
                self.log.error("Failed to decode metadata.")
            except Exception as exc:
                self.log.error("Failed to load metadata: %s", exc)

        def prepare(self, log: bool = False) -> None:
            """
            Prepare the presence.
            Load the metadata if necessary.
            """
            if self.metadata_file:
                self.__load_metadata(log)
            self.log.info("Loaded.")

        def on_load(self) -> None:
            """
            Called when the presence is loaded.
            """
            if self.dev_mode:
                return
            try:
                self.__rpc = ClientRPC(client_id=self.client_id)
                self.__rpc.connect()
            except Exception as exc:
                self.log.error(f"Failed to connect to Discord: {exc}")
                return
            super().on_load()
            self.update()
            self.last_update = time.time()

        def on_update(self, **context) -> None:
            """
            Called when the presence is updated.
            """
            super().on_update(**context)
            self.update()

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
            self.update()

        def __interval_time(self) -> bool:
            """
            Check the update interval (15 seconds).
            """
            now = time.time()
            if self.last_update is None:
                self.last_update = now
                return True
            max_time = self.last_update + 15
            if max_time > now:
                diff = now - (self.last_update + 15)
                remaining = int(abs(diff))
                if self.dev_mode:
                    self.log.debug("Skipping update %d seconds remaining", remaining)
                return False
            self.last_update = now
            return True

        def update(self) -> None:
            """
            Update the presence.
            """
            ok = self.__interval_time()
            if not ok:
                return
            if self.dev_mode:
                # pretty print the data
                self.log.debug(json.dumps(self.data(), indent=4))

                return
            self.log.debug("Sending update to Discord.")
            try:
                self.large_text = f"{rpp.__title__} v{rpp.__version__}"
                if self.details and len(self.details) > 128:
                    self.details = self.details[:125] + "..."
                if self.state and len(self.state) > 128:
                    self.state = self.state[:125] + "..."
                self.__rpc.update(
                    state=self.state,
                    details=self.details,
                    large_image=self.large_image,
                    large_text=(
                        self.large_text
                        if self.large_text
                        else f"{rpp.__title__} v{rpp.__version__}"
                    ),
                    small_image=self.small_image,
                    small_text=self.small_text,
                    activity_type=self.activity_type,
                    start_time=self.start,
                    end_time=self.end,
                    buttons=(
                        [
                            {
                                "label": "Download App",
                                "url": "https://github.com/manucabral/RichPresencePlus/releases",
                            },
                        ]
                        if not self.buttons
                        else self.buttons
                    ),
                )
            except Exception as exc:
                self.log.error(f"Failed to update on Discord: {exc}")

    return Wrapp
