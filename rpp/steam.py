"""
Steam-related utilities.
"""

import os
import re
from .constants import Constants
from .logger import get_logger, RPPLogger


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
        self.steam_id32: int = steam_id64 - Constants.STEAM_BASE_ID4
        self.log: RPPLogger = get_logger(self.name or "SteamAccount")

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

    def __init__(self, config_path: str = Constants.STEAM_CONFIG_PATH):
        """
        Initialize the Steam class.
        """
        if os.getenv("STEAM_CONFIG_PATH"):
            config_path = os.getenv("STEAM_CONFIG_PATH")
        self.config_path: str = config_path
        self.log: RPPLogger = get_logger("Steam")
        self.accounts: list[SteamAccount] = []
        self.log.info("Init: %s", self.config_path)
        self.enabled: bool = True
        self.load_accounts()

    def load_accounts(self) -> list[SteamAccount]:
        """
        Load the steam accounts.

        Returns:
            list[SteamAccount]: The steam accounts.
        """
        if self.accounts:
            return self.accounts
        try:
            with open(self.config_path, "r", encoding="utf-8") as file:
                content = file.read()
                pattern = r'"(\w+)"\s*\{\s*"SteamID"\s*"(\d+)"\s*\}'
                matches = re.findall(pattern, content)
                if not matches:
                    return self.accounts
                for name, steam_id64 in matches:
                    self.accounts.append(SteamAccount(name, int(steam_id64)))
            self.log.info("Found %d steam accounts", len(self.accounts))
            return self.accounts
        except FileNotFoundError:
            self.log.error(
                "Steam configuration file not found,"
                "maybe steam is installed in a different directory"
            )
            self.log.info("Please specify the real path into the .env file with key STEAM_CONFIG_PATH")
            self.enabled = False
            return []
