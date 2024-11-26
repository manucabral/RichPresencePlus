"""
This module provides a decorator to extend the base class of the presence for create new presences.
"""

import json
import time
from .version import __title__, __version__
from .rpc import ClientRPC
from .presence import Presence
from .logger import get_logger


# pylint: disable=W0201, R0903, E0203, R0915
def extension(cls: Presence) -> Presence:
    """
    Decorator to extend the base class of the presence for create new presences.
    """

    # pylint: disable=R0902
    class Wrapp(cls):
        """
        Wrapper class for the presence
        """

        last_update = None

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
            exclude_tokens = ["log", "__rpc", "path", "activity_type"]
            exclude_keys = {key for key in self.__dict__ if key not in exclude_tokens}
            return {key for key, _ in self.__dict__.items() if key not in exclude_keys}

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
                    self.automatic = metadata.get("automatic", True)
                    self.version = metadata.get("version", None)
                    self.update_interval = metadata.get("updateInterval", 3)
                    self.package = metadata.get("package", None)
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
            # pylint: disable=W0703
            except Exception as exc:
                self.log.error("Failed to connect to Discord: %s", exc)
                return
            super().on_load()
            self.update()
            self.last_update = time.time()

        def on_update(self, **context) -> None:
            """
            Called when the presence is updated.
            """
            super().on_update(**context)
            if self.automatic:
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
            # pylint: disable=W0703
            except Exception as exc:
                self.log.error("Failed to close connection to Discord: %s", exc)

        def force_update(self) -> None:
            """
            Force the presence to update.
            """
            self.log.debug("Forcing update.")
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

        def __convert(self, value: any) -> any:
            """
            Convert the value to a string.

            Args:
                value (any): The value to convert.

            Returns:
                any: The converted value.
            """
            try:
                return json.dumps(value)
            # pylint: disable=W0703
            except Exception:
                return str(value)

        def deserialize(self) -> dict:
            """
            Deserialize the presence.

            Returns:
                dict: The deserialized presence.
            """
            exclude_tokens = ["log", "__rpc", "path"]
            return {
                key: self.__convert(value)
                for key, value in self.__dict__.items()
                if key not in exclude_tokens
            }

        def update(self) -> None:
            """
            Update the presence.
            """
            if self.automatic:
                ok = self.__interval_time()
                if not ok:
                    return
            if self.dev_mode:
                self.log.debug(json.dumps(self.deserialize(), indent=4))
                return
            self.log.debug("Sending update to Discord.")
            try:
                if self.details and len(self.details) > 128:
                    self.details = self.details[:125] + "..."
                if self.state and len(self.state) > 128:
                    self.state = self.state[:125] + "..."
                self.__rpc.update(
                    state=self.state,
                    details=self.details,
                    large_image=self.large_image,
                    large_text=self.large_text,
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
            # pylint: disable=W0703
            except Exception as exc:
                self.log.error("Failed to update on Discord: %s", exc)

    return Wrapp
