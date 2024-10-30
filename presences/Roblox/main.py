import re
import os
import time
import enum
import json
import urllib.request
import rpp

ICON = "https://i.pinimg.com/564x/1a/c6/c5/1ac6c5d7cbf6c64b13923d7e258f34a3.jpg"
LOADING_ICON = "https://media1.tenor.com/m/tga0EoNOH-8AAAAC/loading-load.gif"


class States(enum.Enum):
    Menu = "returnToLuaAppInternal"
    Playing = "game_join_loadtime"
    Stop = "shutDown: (stage:UGCGame)"


class Endpoints(enum.Enum):
    Games = "https://games.roblox.com/v1/games?universeIds={id}"
    Thumbnail = "https://thumbnails.roblox.com/v1/games/icons?universeIds={id}&size=512x512&format=Png&isCircular=false"


@rpp.extension
class Roblox(rpp.Presence):
    def __init__(self):
        super().__init__(metadata_file=True)
        self.running = True
        self.last_state = "unknown"
        self.log_path = (
            os.getenv("ROBLOX_LOGS_PATH")
            if os.getenv("ROBLOX_LOGS_PATH")
            else os.path.expanduser("~") + "\\AppData\\Local\\Roblox\\logs"
        )

    def on_load(self):
        if not os.path.exists(self.log_path):
            self.log.error("Log path not found.")
            self.running = False
            return
        self.large_image = ICON
        self.small_image = LOADING_ICON
        self.state = "No playing"
        self.details = "Initializing"
        self.start = int(time.time())
        self.log.info("Loaded")
        self.log.info("Using path: %s", self.log_path)

    def get_last_log(self) -> str:
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
                if States.Menu.value in line:
                    states.append("Menu")
                elif States.Playing.value in line:
                    match = re.compile(r"universeid:(\d+)").search(line)
                    if match:
                        universe_id = match.group(1)
                        states.append(universe_id)
                elif States.Stop.value in line:
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
            data = self.get_request(Endpoints.Games.value.format(id=self.last_state))
            self.state = "By " + data["creator"]["name"]
            self.details = data["name"]
            self.small_text = str(data["playing"]) + " playing"
            data = self.get_request(
                Endpoints.Thumbnail.value.format(id=self.last_state)
            )
            self.large_image = data["imageUrl"]

    def on_close(self):
        self.log.info("Closed")
