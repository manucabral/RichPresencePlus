import time
from typing import Optional, Any, Dict
from src.rpc import ActivityType, ClientRPC
from src.runtime import Runtime, Page
from src.logger import logger
from .state import YouTubeMusicState


def on_load(rpc: ClientRPC):
    """
    Actions to perform when the presence is loaded.
    """
    rpc.update(
        state="Listening to music",
        details=None,
        activity_type=ActivityType.LISTENING,
        start_time=int(time.time()),
        end_time=None,
        large_image="youtube_music_logo",
        large_text="YouTube Music",
        small_image=None,
        small_text=None,
        buttons=[],
    )


def get_ytm_pages(runtime: Runtime) -> Dict[str, Page]:
    """Get all YouTube Music pages from the runtime."""
    ytm_pages: Dict[str, Page] = {}
    for page in runtime.pages:
        if not page.url or not page.id:
            continue
        if "music.youtube.com" in page.url:
            ytm_pages[page.id] = page
    return ytm_pages


def format_rpc_data(media_session: Dict[str, Any]) -> Dict[str, Any]:
    """Format media session data for RPC update."""
    title = media_session.get("title")
    artist = media_session.get("artist")
    album = media_session.get("album")
    artwork = media_session.get("artwork")
    playbackState = media_session.get("playbackState")

    details = title if title else "Listening to music"
    state = artist if artist else None
    if album:
        state = f"{state} | {album}" if state else f"{album}"

    return {
        "state": state,
        "details": details,
        "activity_type": ActivityType.LISTENING,
        "start_time": int(time.time()),
        "end_time": None,
        "large_image": "youtube_music_logo" if artwork is None else artwork,
        "large_text": "YouTube Music",
        "small_image": "pause" if playbackState == "paused" else "playing",
        "small_text": playbackState.capitalize() if playbackState else None,
        "buttons": [],
    }


def main(rpc: ClientRPC, runtime: Optional[Runtime], interval: int, stop_event: Any):
    """Main loop of the YouTube Music worker."""
    if runtime is None:
        raise RuntimeError("Runtime is required for YouTube Music presence")

    logger.info("Presence started")
    on_load(rpc)

    state = YouTubeMusicState()
    was_idle = False  # Track if we are in idle state

    try:
        while not stop_event.is_set():
            # Get available YouTube Music pages
            pages = get_ytm_pages(runtime)

            if len(pages) == 0:
                # No YouTube Music tabs - update to idle
                if not was_idle:
                    logger.info("No YouTube Music tabs - updating to idle")
                    payload = {
                        "state": "No tabs open",
                        "details": "Waiting...",
                        "activity_type": ActivityType.LISTENING,
                        "start_time": None,
                        "end_time": None,
                        "large_image": "youtube_music_logo",
                        "large_text": "YouTube Music",
                        "small_image": None,
                        "small_text": None,
                        "buttons": [],
                    }
                    logger.debug("RPC update (idle) payload: %s", payload)
                    rpc.update(**payload)
                    state.cleanup()
                    was_idle = True

                logger.debug("No YouTube Music pages detected. Waiting 5 seconds.")
                stop_event.wait(5)
                continue

            # There are pages, reset idle flag
            was_idle = False

            # If we already have a connected page, check if it still exists
            if state.connected_page and state.last_page_id:
                # If the page is still in the runtime list
                if state.last_page_id in pages:
                    # Use the connected page we already have (maintains connection)
                    page = state.connected_page
                    logger.debug("Reusing connected page: %s", page.id)
                else:
                    # Previous page no longer exists, clean up and select new
                    logger.debug("Previous page no longer exists, selecting new")
                    state.cleanup()
                    page = state.select_best_page(pages)
            else:
                # No connected page, select one
                page = state.select_best_page(pages)

            if page is None:
                logger.debug("No valid page found. Waiting 5 seconds.")
                stop_event.wait(5)
                continue

            logger.debug("Target: %s (id=%s)", page.title, page.id)

            # Connect if needed (reuse existing connection if alive)
            try:
                page.connect_if_needed(timeout=3.0)
                # Only update if it's a new page
                if state.connected_page != page:
                    state.update_connected_page(page)
            except Exception as exc:
                logger.warning("Error connecting to page %s: %s", page.id, exc)
                state.cleanup()
                stop_event.wait(5)
                continue

            # Read media session
            media_session = None
            try:
                media_session = page.get_media_session(timeout=3.0)
            except Exception as exc:
                logger.warning("Error reading media session from %s: %s", page.url, exc)
                stop_event.wait(5)
                continue

            if media_session is None:
                logger.debug("No media session available. Waiting 5 seconds.")
                stop_event.wait(5)
                continue

            # Update RPC only if necessary (media changed)
            if state.should_update_rpc(media_session):
                logger.info("Updating RPC: %s", media_session)
                rpc_data = format_rpc_data(media_session)
                logger.debug("RPC update (media) payload: %s", rpc_data)
                rpc.update(**rpc_data)
                state.mark_update(media_session)
            else:
                logger.debug("No RPC update needed, media unchanged")

            stop_event.wait(interval)
    finally:
        state.cleanup()
        logger.info("Stopping")
