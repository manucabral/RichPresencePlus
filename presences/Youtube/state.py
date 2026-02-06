from typing import Any, Dict, Optional
from src.runtime import Page
from src.logger import logger


class YouTubeState:
    """
    YouTube presence state manager.
    """

    def __init__(self) -> None:
        self.connected_page: Optional[Page] = None
        self.last_page_id: Optional[str] = None
        self._last_snapshot: Optional[Dict[str, Any]] = None

    def cleanup(self) -> None:
        """
        Clean up the connected page and reset state.
        """
        try:
            if self.connected_page:
                try:
                    self.connected_page.close()
                except Exception:
                    logger.debug("Error closing connected page", exc_info=True)
        finally:
            self.connected_page = None
            self.last_page_id = None
            self._last_snapshot = None

    def select_best_page(self, pages: Dict[str, Page]) -> Optional[Page]:
        """
        Always return the most recently-seen page (the last entry in pages).
        """
        if not pages:
            return None
        vals = list(pages.values())
        logger.info("Selecting best page id=%s url=%s", vals[0].id, vals[0].url)
        return vals[0]

    def should_update(self, snapshot: Dict[str, Any]) -> bool:
        """
        Determine if the presence should be updated based on the snapshot.
        """
        keys = ("title", "author", "video_id", "playback", "duration", "current")
        reduced = {k: snapshot.get(k) for k in keys}
        return reduced != self._last_snapshot

    def mark(self, snapshot: Dict[str, Any]) -> None:
        """
        Mark the current snapshot as the last known state.
        """
        keys = ("title", "author", "video_id", "playback", "duration", "current")
        self._last_snapshot = {k: snapshot.get(k) for k in keys}
