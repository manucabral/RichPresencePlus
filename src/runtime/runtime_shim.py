"""
Runtime Shim - Lightweight runtime for worker processes.
"""

from typing import List, Dict, Any

from .context import Context
from ..logger import logger


class SimpleRuntimeShim:
    """
    Lightweight shim that wraps a shared_pages proxy (list of dicts)
    to expose a contexts/pages property compatible with the Runtime interface.
    Used by worker processes to access browser pages via shared memory.
    """

    def __init__(self, shared_pages: List[Dict[str, Any]]):
        """
        Initialize shim with a multiprocessing Manager list proxy.

        Args:
            shared_pages: A Manager().list() proxy containing page dicts.
        """
        self._shared_pages = shared_pages
        self.interval = 1.0  # Default interval for compatibility

    @property
    def pages(self) -> List[Context]:
        """
        Return pages as Context objects from the shared proxy.
        For BiDi pages, workers use pre-fetched data (shim protocol) since
        BiDi sessions cannot be shared across WebSocket connections.
        """
        result = []
        try:
            snapshot = list(self._shared_pages)
            for item in snapshot:
                if isinstance(item, dict):
                    ws_url = item.get("ws_url")
                    protocol = item.get("protocol", "shim")
                    bidi_info = item.get("bidi_info")
                    media_session = item.get("media_session")

                    if protocol == "bidi" or bidi_info:
                        protocol = "shim"  # workers always use shim for BiDi pages
                    elif ws_url:
                        protocol = "cdp"
                    else:
                        protocol = "shim"

                    result.append(
                        Context(
                            id=item.get("id", ""),
                            url=item.get("url", ""),
                            title=item.get("title", ""),
                            protocol=protocol,
                            ws_url=ws_url,
                            _bidi_adapter=None,
                            _prefetched_media_session=media_session,
                        )
                    )
        except Exception as exc:
            logger.debug("SimpleRuntimeShim.pages failed: %s", exc)
        return result

    def load(self) -> bool:
        """
        No-op for shim; pages are managed externally.
        """
        return True

    def close(self):
        """
        No-op for shim; resources are managed by parent process.
        """

    def is_connected(self) -> bool:
        """
        Always returns True for shim.
        """
        return True
