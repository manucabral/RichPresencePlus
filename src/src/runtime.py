"""
Runtime management for browser debugging protocol (CDP) endpoints.
"""

import os
import multiprocessing
import threading
from typing import List, Optional, Any, Dict, Callable, Tuple
import requests
from .utils import safe_close_page
from .constants import config
from .logger import logger
from .page import Page


class Runtime:
    """
    Represents a runtime environment exposing a CDP endpoint.
    """

    def __init__(
        self,
        port: int = config.browser_target_port,
        host: str = "localhost",
        interval: float = 2.0,
        origin: Optional[str] = None,
    ):
        """
        Initialize the Runtime instance.
        """
        try:
            pid = os.getpid()
            proc_name = multiprocessing.current_process().name
            logger.debug(
                "Creating Runtime in pid=%s process=%s (port=%s) origin=%s",
                pid,
                proc_name,
                port,
                origin,
            )
        except Exception:
            pass
        self.origin: Optional[str] = origin
        self.host = host
        self.port = int(port)
        self.interval = float(interval or 0.0)
        self._pages: List[Page] = []
        self.connected: bool = False
        self._connected_callbacks: List[Callable[[bool], None]] = []
        self._pages_dict: Dict[str, Page] = {}
        self._lock = threading.Lock()
        self._stop_event = threading.Event()
        self._thread: Optional[threading.Thread] = None

    def __json_url(self) -> str:
        """
        Constructs the JSON URL for the runtime.
        """
        return f"http://{self.host}:{self.port}/json"

    def _fetch_pages(self) -> Tuple[bool, List[Dict[str, Any]]]:
        """
        Fetches the list of pages from the runtime's /json endpoint.
        Returns (ok, pages_list). ok=True if the endpoint responded,
        False on connection failure.
        """
        try:
            resp = requests.get(self.__json_url(), timeout=0.5)
        except requests.RequestException:
            logger.warning("Cannot connect to runtime at %s", self.__json_url())
            return False, []

        if resp.status_code != 200:
            logger.error(
                "Failed to fetch pages from %s (status=%s)",
                self.__json_url(),
                resp.status_code,
            )
            return False, []
        try:
            data = resp.json()
        except Exception:
            logger.exception("Error parsing JSON response from %s", self.__json_url())
            return False, []

        if isinstance(data, dict) and "webSocketDebuggerUrl" in data:
            return True, [data]
        if isinstance(data, list):
            return True, data
        return True, []

    def _update_from_raw(self, raw_pages: List[Dict[str, Any]]) -> None:
        """Updates the list of pages reusing existing Page objects."""
        pages: List[Page] = []
        for entry in raw_pages:
            try:
                if entry.get("type") and entry.get("type") != "page":
                    continue

                page_id = entry.get("id") or entry.get("id_")
                page_url = entry.get("url")
                page_ws_url = entry.get("webSocketDebuggerUrl") or entry.get("ws_url")

                if not page_id:
                    # attempt to derive an id
                    page_id = page_ws_url or page_url or None
                if not page_id:
                    continue

                if page_id in self._pages_dict:
                    existing = self._pages_dict[page_id]
                    if (
                        getattr(existing, "url", None) == page_url
                        and getattr(existing, "ws_url", None) == page_ws_url
                    ):
                        pages.append(existing)
                        continue
                page = Page(
                    id_=page_id,
                    url=page_url,
                    title=entry.get("title"),
                    ws_url=page_ws_url,
                    chromium=True,
                )
                self._pages_dict[page_id] = page
                pages.append(page)
            except Exception:
                logger.warning("Malformed page entry detected and skipped: %s", entry)
                continue

        current_ids = {
            getattr(p, "id", getattr(p, "id_", None))
            for p in pages
            if getattr(p, "id", None) or getattr(p, "id_", None)
        }
        old_ids = set(self._pages_dict.keys())
        for removed_id in old_ids - current_ids:
            removed_page = self._pages_dict.pop(removed_id, None)
            safe_close_page(removed_page)

        with self._lock:
            # swap atomically
            self._pages = pages

    def _probe_once(self) -> bool:
        """Probes the CDP endpoint and updates the list of pages."""
        logger.debug("Probing at %s", self.__json_url())
        ok, raw = self._fetch_pages()
        try:
            self._update_from_raw(raw)
        except Exception:
            logger.exception("Failed to update pages from raw fetch")
        callbacks: List[Callable[[bool], None]] = []
        with self._lock:
            prev = bool(self.connected)
            self.connected = bool(ok)
            if prev != self.connected:
                callbacks = list(self._connected_callbacks)
        if callbacks:
            for cb in callbacks:
                try:
                    t = threading.Thread(target=cb, args=(self.connected,), daemon=True)
                    t.start()
                except Exception:
                    logger.exception("Connected-callback raised when scheduling")
        return bool(ok)

    def add_connected_callback(self, cb: Callable[[bool], None]) -> None:
        """
        Register a callback to be called when connection status changes.
        """
        with self._lock:
            self._connected_callbacks.append(cb)

    def remove_connected_callback(self, cb: Callable[[bool], None]) -> None:
        """
        Remove a previously-registered connected callback.
        """
        with self._lock:
            try:
                while cb in self._connected_callbacks:
                    self._connected_callbacks.remove(cb)
            except Exception:
                pass

    def _background_loop(self) -> None:
        """Background loop that periodically probes the runtime."""
        logger.debug("Background probe started (interval=%s)", self.interval)
        while not self._stop_event.is_set():
            try:
                self._probe_once()
            except Exception:
                logger.exception("Error in runtime background probe")
            self._stop_event.wait(self.interval)
        logger.debug("Loop stopped")

    def load(self, start_background: bool = True) -> bool:
        """Load the runtime and optionally start background probing."""
        ok = self._probe_once()
        if (
            start_background
            and self.interval > 0
            and (self._thread is None or not self._thread.is_alive())
        ):
            logger.debug("Starting background probe thread")
            self._stop_event.clear()
            self._thread = threading.Thread(target=self._background_loop, daemon=True)
            self._thread.start()
        return ok

    def stop(self) -> None:
        """Stop background polling (if running) and clean up pages."""
        self._stop_event.set()
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=1.0)

        with self._lock:
            for p in getattr(self, "_pages", []) or []:
                try:
                    safe_close_page(p)
                except Exception:
                    pass
            self._pages = []
            self.connected = False
        logger.debug("Stopped successfully")

    def close(self) -> None:
        """
        Close the runtime and clean up resources.
        """
        return self.stop()

    def is_connected(self) -> bool:
        """Returns whether the runtime is currently connected."""
        with self._lock:
            return bool(self.connected)

    def refresh_state(self) -> None:
        """Force refresh of the runtime state (no BiDi support)."""
        try:
            self._probe_once()
        except Exception:
            logger.debug("Failed to refresh runtime state", exc_info=True)

    @property
    def pages(self) -> List[Page]:
        """Return a thread-safe copy of the current pages."""
        with self._lock:
            return list(self._pages)


class SimpleRuntimeShim:
    """
    Lightweight runtime-like shim that reads a multiprocessing.Manager
    proxy list of page dicts and exposes a pages property returning
    Page instances. Pages created here are read-only: they will NOT
    open WebSocket connections (safe for worker processes).
    """

    def __init__(self, proxy: Any):
        self._proxy = proxy
        self._pages_cache: Dict[str, Page] = {}

    @property
    def pages(self) -> list:
        """
        Return a list of Page objects based on the current proxy data.
        """
        out = []
        try:
            items = list(self._proxy)
        except Exception:
            items = []

        current_ids = set()
        for item in items:
            try:
                if not isinstance(item, dict):
                    continue

                page_id = item.get("id")
                page_url = item.get("url")
                page_ws_url = item.get("ws_url")
                page_chromium = bool(item.get("chromium", True))

                if not page_id:
                    continue

                current_ids.add(page_id)

                # reuse existing Page object if present and unchanged
                if page_id in self._pages_cache:
                    existing = self._pages_cache[page_id]
                    if existing.url == page_url and existing.ws_url == page_ws_url:
                        out.append(existing)
                        continue

                page = Page(
                    id_=page_id,
                    url=page_url,
                    title=item.get("title"),
                    ws_url=page_ws_url,
                    chromium=page_chromium,
                )

                # For clarity, mark as readonly (optional attribute)
                try:
                    setattr(page, "readonly", True)
                except Exception:
                    pass

                self._pages_cache[page_id] = page
                out.append(page)
            except Exception:
                continue

        # cleanup removed pages
        old_ids = set(self._pages_cache.keys())
        for removed_id in old_ids - current_ids:
            removed_page = self._pages_cache.pop(removed_id, None)
            safe_close_page(removed_page)

        return out
