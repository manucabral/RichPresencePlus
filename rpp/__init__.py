"""
Rich Presence Plus
------------------
A simple Discord Rich Presence manager for custom desktop or web status.
"""

from .logger import get_logger
from .manager import Manager
from .presence import Presence
from .extension import extension
from .rpc import ClientRPC, ActivityType
from .steam import Steam, SteamAccount
from .constants import Constants
from .browser import Browser
from .runtime import Runtime
from .utils import load_env, get_available_presences, get_steam_presence
from .tab import Tab
from .version import __version__, __title__

__description__ = (
    "A simple Discord Rich Presence manager for custom desktop or web status."
)
__author__ = "Manuel Cabral"
__license__ = "GPL-3.0"


__all__ = [
    "get_logger",
    "Manager",
    "Presence",
    "ClientRPC",
    "ActivityType",
    "extension",
    "Constants",
    "Browser",
    "Runtime",
    "load_env",
    "get_available_presences",
    "get_steam_presence",
    "Tab",
    "Steam",
    "SteamAccount",
]
