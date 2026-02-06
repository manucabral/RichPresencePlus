"""
YouTube Presence for Rich Presence Plus.
"""

import time
from typing import Optional, Any, Dict
from src.rpc import ActivityType, ClientRPC
from src.runtime import Runtime, Page
from src.logger import logger
from .state import YouTubeState
from .utils import (
    extract_video_id,
    extract_title,
    extract_author,
    extract_shorts_title,
    extract_shorts_author,
    extract_author_url,
    extract_thumbnail,
    calc_time_from_now,
    extract_video_times,
    extract_playback_state,
    eval_page,
)


def get_youtube_pages(runtime: Runtime) -> Dict[str, Page]:
    pages: Dict[str, Page] = {}
    for page in runtime.pages:
        try:
            url = page.url or ""
            if "music.youtube.com" in url:
                continue
            if "youtube.com" in url or "youtu.be" in url:
                page_id = page.id or url
                pages[page_id] = page
        except Exception:
            continue
    return pages


def build_rpc_payload(snapshot: Dict[str, Any]) -> Dict[str, Any]:
    title = snapshot.get("title") or "Watching YouTube"
    author = snapshot.get("author")
    video_id = snapshot.get("video_id")
    playback = snapshot.get("playback")
    duration = snapshot.get("duration") or 0
    current = snapshot.get("current") or 0

    details = title
    state = f"By {author}" if author else None
    large_image = extract_thumbnail(video_id) or "youtube_logo"
    small_image = (
        "play" if playback == "playing" else "pause" if playback == "paused" else None
    )
    small_text = (
        "Playing"
        if playback == "playing"
        else "Paused" if playback == "paused" else None
    )
    start_time, end_time = calc_time_from_now(duration, current)

    buttons = []
    page_url = snapshot.get("url")
    if page_url:
        buttons.append({"label": "Watch on YouTube", "url": page_url})
    author_url = snapshot.get("author_url")
    if author_url:
        buttons.append({"label": "Author", "url": author_url})

    payload: Dict[str, Any] = {
        "state": state,
        "details": details,
        "activity_type": ActivityType.WATCHING,
        "start_time": start_time if start_time else None,
        "end_time": end_time if end_time else None,
        "large_image": large_image,
        "large_text": "YouTube",
        "small_image": small_image,
        "small_text": small_text,
        "buttons": buttons,
    }
    return payload


def main(
    rpc: ClientRPC,
    shared_state: dict,
    runtime: Optional[Runtime] = None,
    interval: int = 5,
    stop_event: Any = None,
) -> None:
    if runtime is None:
        raise RuntimeError("Runtime is required for YouTube presence")

    logger.info("Presence started.")
    payload = {
        "state": "Idle",
        "details": None,
        "activity_type": ActivityType.WATCHING,
        "start_time": None,
        "end_time": None,
        "large_image": "youtube_logo",
        "large_text": "YouTube",
        "small_image": None,
        "small_text": None,
        "buttons": [],
    }
    rpc.update(**payload)
    shared_state["last_rpc_update"] = payload
    shared_state["last_update_time"] = time.time()
    state = YouTubeState()
    was_idle = False

    try:
        while not stop_event.is_set():
            pages = get_youtube_pages(runtime)

            if not pages:
                if not was_idle:
                    logger.info("No YouTube tabs - setting idle")
                    try:
                        payload = {
                            "state": "No tabs open",
                            "details": "Waiting...",
                            "activity_type": ActivityType.WATCHING,
                            "start_time": None,
                            "end_time": None,
                            "large_image": "youtube_logo",
                            "large_text": "YouTube",
                            "small_image": None,
                            "small_text": None,
                            "buttons": [],
                        }
                        rpc.update(**payload)
                        shared_state["last_rpc_update"] = payload
                        shared_state["last_update_time"] = time.time()
                        logger.info("Set idle!")
                    except Exception:
                        logger.debug("Idle RPC update failed", exc_info=True)
                    state.cleanup()
                    was_idle = True
                stop_event.wait(interval)
                continue

            was_idle = False

            recent = state.select_best_page(pages)
            if state.connected_page is not None:
                try:
                    current_id = state.connected_page.id or None
                except Exception:
                    current_id = None
            else:
                current_id = None

            recent_id = recent.id or None if recent is not None else None

            if recent is None:
                logger.info("No valid YouTube page found")
                page = None
            elif current_id == recent_id:
                logger.info("Continuing with connected page id=%s", current_id)
                page = state.connected_page
            else:
                logger.info("Switching to new page id=%s", recent_id)
                if state.connected_page is not None and current_id != recent_id:
                    state.cleanup()
                page = recent

            if page is None:
                stop_event.wait(interval)
                continue

            try:
                page.connect_if_needed(timeout=3.0)
                if state.connected_page != page:
                    state.connected_page = page
                    state.last_page_id = page.id
            except Exception as exc:
                logger.warning("Error connecting to page %s: %s", page.id, str(exc))
                state.cleanup()
                stop_event.wait(interval)
                continue

            snapshot: Dict[str, Any] = {}
            js_location = "(function(){try{return window.location.href;}catch(e){return null;}})()"
            real_url = eval_page(page, js_location) or page.url or ""
            logger.debug(
                "Page URL: page.url=%s, js_location=%s, real_url=%s",
                page.url,
                eval_page(page, js_location),
                real_url,
            )
            snapshot["url"] = real_url
            snapshot["video_id"] = extract_video_id(page, real_url) or None

            if real_url.rstrip("/") == "https://www.youtube.com":
                snapshot["title"] = "Browsing YouTube"
                snapshot["author"] = None
                snapshot["author_url"] = None
            elif "/shorts/" in real_url:
                snapshot["title"] = extract_shorts_title(page) or "Watching Shorts"
                snapshot["author"] = extract_shorts_author(page)
                snapshot["author_url"] = None
            else:
                snapshot["title"] = extract_title(page) or None
                snapshot["author"] = extract_author(page) or None

            snapshot["author_url"] = extract_author_url(page) or None
            duration, current = extract_video_times(page)
            snapshot["duration"] = duration
            snapshot["current"] = current
            snapshot["playback"] = extract_playback_state(page) or None

            if state.should_update(snapshot):
                logger.info("Snapshot changed, updating RPC: %s", snapshot)
                payload = build_rpc_payload(snapshot)
                try:
                    shared_state["last_rpc_update"] = payload
                    shared_state["last_update_time"] = time.time()
                    rpc.update(**payload)
                except Exception:
                    logger.debug("RPC update failed", exc_info=True)
                state.mark(snapshot)
            else:
                logger.debug("No RPC update needed")

            stop_event.wait(interval)
    finally:
        try:
            state.cleanup()
        except Exception:
            logger.debug("Error during cleanup", exc_info=True)
        logger.info("YouTube Presence stopping")
