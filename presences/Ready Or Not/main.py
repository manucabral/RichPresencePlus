"""
Ready Or Not Presence for Rich Presence Plus
"""

from typing import Any
from src.rpc import ActivityType, ClientRPC
from src.logger import logger
from src.steam import SteamAccount, get_steam_presence

LOGO = "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1144200/header.jpg"


def main(
    rpc: ClientRPC, interval: int, steam_account: SteamAccount, stop_event: Any
) -> None:
    logger.info("Ready Or Not presence started")

    if not steam_account:
        logger.warning("No steam account provided, cannot fetch presence")
        return

    logger.info("Using %s", steam_account)
    try:
        while not stop_event.is_set():
            payload = get_steam_presence(steam_account.steam_id3)
            if payload is None:
                logger.warning(
                    "No presence data retrieved for Steam ID3: %d",
                    steam_account.steam_id3,
                )
                continue
            if payload.get("name") == "Ready Or Not":
                rpc.update(
                    details=payload.get("name") or "Ready Or Not",
                    state=payload.get("state") or "Playing",
                    activity_type=ActivityType.PLAYING,
                    large_image=LOGO,
                    large_text="Ready Or Not",
                    small_image=None,
                    small_text=None,
                    start_time=None,
                    end_time=None,
                    buttons=None,
                )
            else:
                rpc.update(
                    details="Not in Game",
                    state="Idle",
                    activity_type=ActivityType.PLAYING,
                    large_image=LOGO,
                    large_text="Ready Or Not",
                    small_image=None,
                    small_text=None,
                    start_time=None,
                    end_time=None,
                    buttons=None,
                )

            stop_event.wait(interval)
    finally:
        logger.info("Ready Or Not presence stopped")
