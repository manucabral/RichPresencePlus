"""
Netflix presence for RPP.
"""
import time
from pathlib import Path
from typing import Optional, Any
from src.rpc import ActivityType, ClientRPC
from src.runtime import Runtime, Page
from src.logger import logger


def get_last_netflix_page(runtime: Runtime) -> Optional[Page]:
    """Get the last active Netflix page from runtime."""
    for page in runtime.pages:
        if page.url and "www.netflix.com" in page.url:
            return page
    return None


def calc_timestamps(
    duration: float, current: float, paused: bool
) -> tuple[Optional[int], Optional[int]]:
    """Calculate start and end timestamps for RPC."""
    if paused or duration <= 0:
        return None, None
    now = int(time.time())
    start = now - int(current)
    end = start + int(duration)
    return start, end


def main(
    rpc: ClientRPC,
    shared_state: dict,
    runtime: Optional[Runtime],
    interval: int,
    stop_event: Any,
) -> None:
    """Main Netflix presence worker."""

    if runtime is None:
        raise RuntimeError("Runtime is required.")

    JS_EXTRACTOR = (Path(__file__).parent / "extractor.js").read_text(encoding="utf-8")
    if not JS_EXTRACTOR:
        logger.error("Failed to load Netflix extractor script")
        return

    logger.info("Netflix presence started")
    last_snapshot = None

    try:
        while not stop_event.is_set():

            page = get_last_netflix_page(runtime)
            if page is None:
                logger.debug("No Netflix page found")
                stop_event.wait(interval)
                continue

            page.connect_if_needed()
            data = page.evaluate(JS_EXTRACTOR)

            if not data or "error" in data:
                snapshot = {
                    "state": "browsing",
                    "title": None,
                    "episode": None,
                    "season": None,
                    "paused": None,
                    "type": None,
                }

                if snapshot != last_snapshot:
                    logger.debug("Browsing Netflix")
                    payload = {
                        "state": "Looking for something to watch",
                        "details": "Browsing",
                        "activity_type": ActivityType.WATCHING,
                        "start_time": None,
                        "end_time": None,
                        "large_image": None,
                        "large_text": None,
                        "small_image": None,
                        "small_text": None,
                        "buttons": [],
                    }
                    rpc.update(**payload)
                    shared_state["last_rpc_update"] = payload
                    shared_state["last_update_time"] = time.time()
                    last_snapshot = snapshot

                stop_event.wait(interval)
                continue

            paused = data.get("paused", True)
            current_time = data.get("currentTime", 0)
            duration = data.get("duration", 0)
            media_type = data.get("type")
            media_title = data.get("title", "Netflix")
            episode = data.get("episode")
            season = data.get("season")
            episode_title = data.get("episodeTitle")
            artwork = data.get("artwork")

            if media_type == "show" and episode and season:
                state = (
                    f"S{season}:E{episode} - {episode_title}"
                    if episode_title
                    else f"S{season}:E{episode}"
                )
            elif media_type == "show" and episode_title:
                state = episode_title
            else:
                state = "Watching"

            snapshot = {
                "state": "watching",
                "title": media_title,
                "episode": episode,
                "season": season,
                "paused": paused,
                "type": media_type,
                "episode_title": episode_title,
            }

            if snapshot != last_snapshot:
                start_time, end_time = calc_timestamps(duration, current_time, paused)
                logger.debug(
                    f"Content changed: {media_title} - {state} (Paused: {paused})"
                )
                payload = {
                    "state": state,
                    "details": media_title,
                    "activity_type": ActivityType.WATCHING,
                    "start_time": start_time,
                    "end_time": end_time,
                    "large_image": artwork if artwork and len(artwork) < 256 else None,
                    "large_text": media_title,
                    "small_image": "pause" if paused else "play",
                    "small_text": "Paused" if paused else "Playing",
                    "buttons": (
                        [{"label": "Watch on Netflix", "url": page.url}]
                        if page.url
                        else []
                    ),
                }

                rpc.update(**payload)
                payload["large_image"] = artwork if artwork else None
                shared_state["last_rpc_update"] = payload
                shared_state["last_update_time"] = time.time()
                last_snapshot = snapshot
            else:
                logger.debug("No change detected, skipping RPC update")

            stop_event.wait(interval)

    except Exception as e:
        logger.error(f"Netflix presence error: {e}", exc_info=True)
    finally:
        logger.info("Netflix presence stopped")
