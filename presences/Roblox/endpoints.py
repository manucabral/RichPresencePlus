"""
Simple enum endpoint.
"""

import enum


class ENDPOINTS(enum.Enum):
    """
    Enum for Roblox API endpoints.
    """

    GAMES = "https://games.roblox.com/v1/games?universeIds={id}"
    # pylint: disable=line-too-long
    THUMBNAIL = "https://thumbnails.roblox.com/v1/games/icons?universeIds={id}&size=512x512&format=Png&isCircular=false"
