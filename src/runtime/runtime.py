"""
Runtime - Main browser automation runtime.
"""

import threading
from typing import Optional, List, Dict, Any, Callable

import requests

from .protocol_adapter import ProtocolAdapter
from .cdp_adapter import CDPAdapter
from .context import Context
from ..constants import config
from ..logger import logger


class Runtime:
    """
    Main runtime class for browser automation.
    """

    def __init__(
        self,
        port: int = config.browser_target_port,
        host: str = "localhost",
        interval: float = 2.0,
        protocol: Optional[str] = None,  # "cdp", "bidi", or None for auto-detect
        origin: Optional[str] = None,  # optional for logging/debugging
    ):
        self.host = host
        self.port = port
        self.interval = interval
        self.protocol = protocol
        self.origin = origin

        self._adapter: Optional[ProtocolAdapter] = None
        self._stop_event = threading.Event()
        self._thread: Optional[threading.Thread] = None
        self._connected_callbacks: List[Callable[[bool], None]] = []
        self._lock = threading.Lock()

    def _detect_protocol(self) -> Optional[str]:
        """Auto-detect which protocol is available."""
        logger.debug("Auto-detecting protocol on port %d", self.port)

        # try CDP first
        try:
            resp = requests.get(f"http://{self.host}:{self.port}/json", timeout=1.0)
            if resp.status_code == 200:
                data = resp.json()
                if isinstance(data, list) and len(data) > 0:
                    if "webSocketDebuggerUrl" in data[0]:
                        logger.info("Detected CDP protocol")
                        return "cdp"
        except requests.RequestException:
            pass

        # BiDi (Firefox) is disabled for now
        logger.warning("No protocol detected on port %d (BiDi is disabled)", self.port)
        return None

    def _create_adapter(self, protocol: str) -> ProtocolAdapter:
        """Factory method to create appropriate adapter."""
        if protocol == "cdp":
            return CDPAdapter(self.host, self.port)
        if protocol == "bidi":  # is disabled
            raise ValueError("BiDi protocol is disabled - use a Chromium-based browser")
        raise ValueError(f"Unsupported protocol: {protocol}")

    def load(self, start_background: bool = True) -> bool:
        """Initialize runtime and optionally start background polling."""
        with self._lock:
            if (
                self._adapter
                and hasattr(self._adapter, "is_connected")
                and self._adapter.is_connected()
            ):
                logger.debug("Runtime already connected, skipping load()")
                return True

            detected = self.protocol
            if not detected:
                detected = self._detect_protocol()

            logger.info("Runtime protocol: %s", detected)
            if not detected:
                logger.error("Failed to detect browser protocol")
                return False

            self.protocol = detected
            if not self._adapter:
                self._adapter = self._create_adapter(detected)

            if (
                hasattr(self._adapter, "is_connected")
                and not self._adapter.is_connected()
            ):
                connected = self._adapter.connect()

                if not connected:
                    logger.error("Failed to connect using %s protocol", detected)
                    return False
            elif not hasattr(self._adapter, "is_connected"):
                connected = self._adapter.connect()

                if not connected:
                    logger.error("Failed to connect using %s protocol", detected)
                    return False

            if start_background and detected == "cdp" and self.interval > 0:
                self._start_background_polling()
            elif detected == "bidi":  # is disabled
                logger.debug("BiDi protocol loaded, fetching initial contexts")
                self._adapter.get_contexts()

            logger.info("Runtime loaded with %s protocol", detected)
            return True

    def _start_background_polling(self):
        """Start background thread for periodic updates (CDP only)."""
        if self._thread and self._thread.is_alive():
            return

        def poll_loop():
            logger.debug("Background polling started")
            while not self._stop_event.is_set():
                try:
                    if self._adapter:
                        self._adapter.get_contexts()
                except Exception as exc:
                    logger.error("Polling error: %s", exc)
                self._stop_event.wait(self.interval)
            logger.debug("Background polling stopped")

        self._stop_event.clear()
        self._thread = threading.Thread(target=poll_loop, daemon=True)
        self._thread.start()

    def stop(self):
        """Stop runtime and cleanup."""
        self._stop_event.set()

        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=1.0)

        if self._adapter:
            self._adapter.close()
            self._adapter = None

        logger.debug("Runtime stopped")

    def close(self):
        """Alias for stop()."""
        self.stop()

    @property
    def pages(self) -> List[Context]:
        """Get current contexts (compatible with Runtime.pages)."""
        if not self._adapter:
            return []
        return self._adapter.get_contexts()

    def evaluate_script(
        self, context: Context, expression: str, await_promise: bool = False
    ) -> Dict[str, Any]:
        """
        Execute JavaScript in a context.

        Args:
            context: Target context
            expression: JavaScript code
            await_promise: Wait for promise resolution

        Returns:
            Dict with 'type', 'value', and 'error' keys
        """
        if not self._adapter:
            raise RuntimeError("Runtime not loaded")

        return self._adapter.evaluate_script(context.id, expression, await_promise)

    def is_connected(self) -> bool:
        """Check if runtime is connected."""
        if not self._adapter:
            return False
        return self._adapter.is_connected()

    def refresh_state(self):
        """Force refresh of contexts."""
        if self._adapter:
            self._adapter.get_contexts()
