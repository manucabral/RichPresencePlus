"""
Roblox presence for Rich Presence Plus.
"""

import re
import os
import time
from typing import Any
from src.rpc import ClientRPC, ActivityType
from src.logger import logger
from .icons import ICON, LOADING_ICON
from .states import STATES
from .endpoints import ENDPOINTS
from .utils import get_last_roblox_log, make_request


def main(rpc: ClientRPC, runtime: Any, stop_event: Any):
    """Main loop of the Roblox worker."""
    logger.info("Presence started")

    state = "Initializing"
    details = "Initializing"
    large_image = ICON
    large_text = "Roblox"
    small_image = LOADING_ICON
    small_text = "Roblox"
    start_time = int(time.time())

    log_path = (
        os.getenv("ROBLOX_LOGS_PATH")
        if os.getenv("ROBLOX_LOGS_PATH")
        else os.path.expanduser("~") + "\\AppData\\Local\\Roblox\\logs"
    )
    if not log_path:
        logger.error("No log path specified. Maybe not set?")
        return
    if not os.path.exists(log_path):
        logger.error("Log path not found: %s", log_path)
        return
    logger.info("Using log path: %s", log_path)

    last_state: str = "unknown"

    while not stop_event.is_set():
        try:
            last_log = get_last_roblox_log(log_path)
            if not last_log:
                logger.warning("No log file found.")
                stop_event.wait(5)
                continue

            states = []
            with open(last_log, "r", encoding="utf-8", errors="ignore") as file:
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
                logger.warning("No states found.")
                stop_event.wait(5)
                continue

            current_state = states[-1]
            if current_state == last_state:
                logger.debug("No changes.")
                stop_event.wait(5)
                continue

            last_state = current_state
            start_time = int(time.time())

            # prepare fields for rpc.update
            if last_state == "Menu":
                state = "Browsing..."
                details = "In menu"
                large_image = ICON
                small_image = LOADING_ICON
                small_text = "Loading..."

            elif last_state == "No playing":
                state = "No playing"
                details = "No playing"
                large_image = ICON
                small_image = ICON
                small_text = None

            else:
                small_image = ICON
                data = make_request(ENDPOINTS.GAMES.value.format(id=last_state))
                if data:
                    state = "By " + data.get("creator", {}).get("name", "Unknown")
                    details = data.get("name", "Unknown")
                    small_text = str(data.get("playing", 0)) + " playing"
                else:
                    state = "Playing"
                    details = None
                    small_text = None

                thumb = make_request(ENDPOINTS.THUMBNAIL.value.format(id=last_state))
                if thumb:
                    large_image = thumb.get("imageUrl", large_image)

            rpc.update(
                state=state,
                details=details,
                activity_type=ActivityType.PLAYING,
                start_time=start_time,
                end_time=None,
                large_image=large_image,
                large_text=large_text,
                small_image=small_image,
                small_text=small_text,
                buttons=[],
            )

            stop_event.wait(5)

        except Exception as e:
            logger.error("Error in Roblox presence: %s", e)
            stop_event.wait(5)

    logger.info("Presence stopped")
