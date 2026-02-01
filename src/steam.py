"""
Steam-related utilities.
"""

import re
import requests
from .constants import config
from .logger import logger


class SteamAccount:
    """
    Steam account class.
    """

    def __init__(self, name: str, steam_id64: int):
        """
        Initialize a Steam account.
        """
        self.name: str = name
        self.steam_id64: int = steam_id64
        self.steam_id3: int = steam_id64 - config.steam_base_id4

    def __str__(self) -> str:
        """
        Return the string representation of the Steam account.
        """
        return f"{self.__class__.__name__}({self.name}, {self.steam_id64})"

    def __repr__(self) -> str:
        """
        Return the string representation of the Steam account.
        """
        return str(self)


# pylint: disable=R0903
class Steam:
    """
    Steam class.
    """

    def __init__(self, config_path: str = config.steam_config_path):
        """
        Initialize the Steam class.
        """
        self.config_path: str = config_path
        self.enabled: bool = True
        self.accounts: list[SteamAccount] = []
        self.load_accounts()
        logger.info("Init: %s", self.config_path)

    def load_accounts(self) -> list[SteamAccount]:
        """
        Load the steam accounts.

        Returns:
            list[SteamAccount]: The steam accounts.
        """
        if self.accounts:
            return self.accounts
        try:
            with open(self.config_path, "r", encoding="utf-8", errors="ignore") as file:
                content = file.read()
                pattern = r'"(\w+)"\s*\{\s*"SteamID"\s*"(\d+)"\s*\}'
                matches = re.findall(pattern, content)
                if not matches:
                    return self.accounts
                for name, steam_id64 in matches:
                    self.accounts.append(SteamAccount(name, int(steam_id64)))
            logger.info("Found %d steam accounts", len(self.accounts))
            return self.accounts
        except FileNotFoundError:
            logger.error(
                "Steam configuration file not found,"
                "maybe steam is installed in a different directory"
            )
            self.enabled = False
            return []


def get_steam_presence(steam_id3: int) -> dict:
    """
    Get the presence information for a Steam account.

    Args:
        steam_id3 (int): Steam ID3 of the account.

    Returns:
        dict: The presence information.
    """
    state = {"name": None, "state": None}
    try:
        response = requests.get(
            f"https://steamcommunity.com/miniprofile/{steam_id3}",
            timeout=10,
        )
        response = response.text
        name_pattern = re.compile(r'<span class="miniprofile_game_name">([^<]+)</span>')
        state_pattern = re.compile(r'<span class="rich_presence">(.*?)</span>')
        game_name_match = name_pattern.search(response)
        game_name = game_name_match.group(1) if game_name_match else None
        game_state_match = state_pattern.search(response)
        game_state = game_state_match.group(1) if game_state_match else None
        state = {"name": game_name, "state": game_state}
        return state
    except requests.RequestException as exc:
        logger.error("Error fetching Steam presence: %s", exc)
        return state
