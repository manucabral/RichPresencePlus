from typing import Optional, Dict, Any
from src.runtime import Page
from src.logger import logger


class YouTubeMusicState:
    """State management for YouTube Music presence."""

    def __init__(self):
        self.last_media_session: Optional[Dict[str, Any]] = None

        self.connected_page: Optional[Page] = None
        self.last_page_id: Optional[str] = None

    def should_update_rpc(self, new_media: Optional[Dict[str, Any]]) -> bool:
        """
        If the RPC should be updated based on new media session data.
        """
        if self.last_media_session is None:
            return True
        if new_media != self.last_media_session:
            return True
        return False

    def mark_update(self, media_session: Optional[Dict[str, Any]]) -> None:
        """Marks that an RPC update was sent."""
        self.last_media_session = media_session

    def select_best_page(self, pages: Dict[str, Page]) -> Optional[Page]:
        """
        Selects the best page to connect to, preferring the previous one.

        Returns:
            Best page to connect to, or None if no pages are available
        """
        if not pages:
            return None

        if self.last_page_id and self.last_page_id in pages:
            page = pages[self.last_page_id]
            logger.debug("Reusing previously connected page %s", self.last_page_id)
            return page

        #  take the last page (most recently opened)
        pages_list = list(pages.values())
        selected = pages_list[-1]
        logger.debug("Selecting new page %s", selected.id)
        return selected

    def update_connected_page(self, page: Page) -> None:
        """Updates the connected page."""
        self.connected_page = page
        self.last_page_id = page.id

    def cleanup(self) -> None:
        """Cleans up resources."""
        if self.connected_page:
            try:
                self.connected_page.close()
            except Exception as exc:
                logger.debug("Error closing page during cleanup: %s", exc)
        self.connected_page = None
        self.last_page_id = None
