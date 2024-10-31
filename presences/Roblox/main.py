"""
Roblox presence for Rich Presence Plus.
"""

import re
import os
import time
import json
import urllib.request
import rpp
from .icons import ICON, LOADING_ICON
from .states import STATES
from .endpoints import ENDPOINTS


@rpp.extension
class Roblox(rpp.Presence):
    """
    Roblox presence for Rich Presence Plus.
    """

    def __init__(self):
        """
        Initialize the presence.
        """
        super().__init__(metadata_file=True)
        self.large_image = ICON
        self.small_image = LOADING_ICON
        self.state = self.details = "Initializing"
        self.small_text = self.large_text = "Roblox"
        self.start = None
        self.last_state = "unknown"
        self.log_path = (
            os.getenv("ROBLOX_LOGS_PATH")
            if os.getenv("ROBLOX_LOGS_PATH")
            else os.path.expanduser("~") + "\\AppData\\Local\\Roblox\\logs"
        )

    def on_load(self):
        """
        Load the presence.
        """
        if not os.path.exists(self.log_path):
            self.log.error("Log path not found.")
            self.running = False
            return
        self.state = "No playing"
        self.details = "Initializing"
        self.start = int(time.time())
        self.log.info("Loaded")
        self.log.info("Using path: %s", self.log_path)

    def get_last_log(self) -> str:
        """
        Get the last log file.

        Returns:
            str: The last log file.
        """
        files = None
        with os.scandir(self.log_path) as entries:
            files = [
                entry.name
                for entry in entries
                if entry.is_file() and entry.name.endswith("_last.log")
            ]
        if not files:
            return None
        return files[-1]

    def get_request(self, url: str) -> dict:
        """
        Make a GET request to an API.

        Args:
            url (str): The URL of the API.

        Returns:
            dict: The parsed JSON response.
        """
        try:
            with urllib.request.urlopen(url) as response:
                data = response.read()
                parsed = json.loads(data)
                return parsed["data"][0]
        except urllib.error.HTTPError as exc:
            self.log.error(exc)
        except json.JSONDecodeError as exc:
            self.log.error(exc)
        return None

    def on_update(self, **context):
        """
        Update the presence.
        """
        last_log = self.get_last_log()
        if not last_log:
            self.log.warning("No log file found.")
            return
        states = []
        with open(
            os.path.join(self.log_path, last_log),
            "r",
            encoding="utf-8",
            errors="ignore",
        ) as file:
            for line in file:
                if STATES.MENU.value in line:
                    states.append("Menu")
                elif STATES.PLAYING.value in line:
                    match = re.compile(r"universeid:(\d+)").search(line)
                    if match:
                        universe_id = match.group(1)
                        states.append(universe_id)
                elif STATES.STOP.value in line:
                    states.append("No playing")

        if not states:
            self.log.warning("No states found.")
            return

        current_state = states[-1]
        if self.last_state == current_state:
            self.log.info("No changes.")
            return

        self.last_state = current_state
        self.start = int(time.time())

        if self.last_state == "Menu":
            self.state = "Browsing..."
            self.details = "In menu"
            self.large_image = ICON
            self.small_image = LOADING_ICON
            self.small_text = "Loading..."

        elif self.last_state == "No playing":
            self.state = "No playing"
            self.details = "No playing"
            self.large_image = ICON
            self.small_image = ICON

        else:
            self.small_image = ICON
            data = self.get_request(ENDPOINTS.GAMES.value.format(id=self.last_state))
            self.state = "By " + data["creator"]["name"]
            self.details = data["name"]
            self.small_text = str(data["playing"]) + " playing"
            data = self.get_request(
                ENDPOINTS.THUMBNAIL.value.format(id=self.last_state)
            )
            self.large_image = data["imageUrl"]

    def on_close(self):
        """
        Close the presence.
        """
        self.log.info("Closed")
