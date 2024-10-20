import rpp
import re
from .reader import PhasmoReader
from .maps import MAPS


@rpp.extension
class Phasmophobia(rpp.Presence):
    def __init__(self):
        super().__init__(metadata_file=True)
        self.reader = PhasmoReader(self.log)

    def format_data(self, state: str, data: dict) -> dict:
        if state == "In-game":
            if data["Loaded Level"] == "Main Menu":
                return {"state": "In main menu", "large_image": "logo"}
            mapname = re.sub(r"(\w)([A-Z])", r"\1 \2", data["Loaded Level"])
            details = (
                int(data["Players"]) > 1
                and f'Playing with {data["Players"]} players'
                or "Playing Solo"
            )
            return {
                "state": details,
                "details": mapname,
                "large_image": MAPS.get(data["Loaded Level"].lower(), "logo"),
                "large_text": mapname,
                "small_image": "logo",
                "small_text": data["Difficulty"],
            }
        return {"state": state, "large_image": "logo"}

    def on_load(self):
        self.log.info("Loaded")

    def on_update(self, **context):
        self.reader.load()
        temp = self.reader.update()
        formated_data = self.format_data(temp["state"], temp["data"])
        self.state = formated_data["state"]
        self.details = formated_data.get("details")
        self.large_image = formated_data.get("large_image")
        self.large_text = formated_data.get("large_text")
        self.small_image = formated_data.get("small_image")
        self.small_text = formated_data.get("small_text")
        self.log.info(f"Updated: {self.state}")

    def on_close(self):
        self.reader.clear()
        self.log.info("Closed")
