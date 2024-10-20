import os
import subprocess


class PhasmoReader:

    def __init__(self, log=None):
        self.log = log
        self.data = None
        self.path = os.path.join(
            os.environ["LOCALAPPDATA"] + "Low",
            "Kinetic Games",
            "Phasmophobia",
            "Player.log",
        )

    def _format(self, data: str) -> dict:
        data_dict = {}
        for item in data.split("|"):
            key, value = item.split(":", 1)
            key = key.strip()
            value = value.strip()
            if key == "Difficulty" and "(Difficulty)" in value:
                value = "Custom"
            data_dict[key] = value
        return data_dict

    def _filter(self, word: str) -> list:
        return [line for line in self.data.splitlines() if word in line]

    def clear(self) -> None:
        self.data = None

    def running(self) -> bool:
        try:
            result = subprocess.run(
                [
                    "powershell",
                    "-Command",
                    f"Get-Process -Name phasmophobia -ErrorAction SilentlyContinue",
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            return "phasmophobia" in result.stdout.lower()
        except Exception as exc:
            self.log.error(f"Failed to check if game is running: {exc}")
            return False

    def send_state(self, state="", data=None) -> dict:
        return {"state": state, "data": data}

    def load(self):
        try:
            with open(self.path, "r", errors="ignore", encoding="utf-8") as log_file:
                self.data = log_file.read()
        except Exception as exc:
            self.log.error(f"Failed to read file: {exc}")

    def update(self):
        if not self.data:
            self.load()
        if not self.running():
            return self.send_state("Game not running")
        level = self._filter("Level: ")
        if not level:
            return self.send_state("Game started")
        data = self._format(level[-1])
        return self.send_state("In-game", data)
