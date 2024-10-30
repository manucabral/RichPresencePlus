"""
Each state of Roblox. (MENU, PLAYING, STOP or NO PLAYING)
"""

import enum


class STATES(enum.Enum):
    """
    Enum for Roblox states.
    """

    MENU = "returnToLuaAppInternal"
    PLAYING = "game_join_loadtime"
    STOP = "shutDown: (stage:UGCGame)"
