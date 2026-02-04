"""
CDP Adapter - Chrome DevTools Protocol implementation.
"""

import json
import time
import threading
from typing import List, Dict, Any

import requests
from websockets.sync.client import connect

from .protocol_adapter import ProtocolAdapter
from .context import Context
from ..logger import logger


class CDPAdapter(ProtocolAdapter):
    """Chrome DevTools Protocol adapter for Chromium-based browsers."""

    def __init__(self, host: str, port: int):
        self.host = host
        self.port = port
        self.connected = False
        self._contexts_cache: List[Context] = []
        self._lock = threading.Lock()

    def _json_url(self) -> str:
        return f"http://{self.host}:{self.port}/json"

    def connect(self) -> bool:
        """Test CDP endpoint availability."""
        try:
            resp = requests.get(self._json_url(), timeout=1.0)
            self.connected = resp.status_code == 200
            logger.debug("CDP connection: %s", self.connected)
            return self.connected
        except requests.RequestException as exc:
            logger.debug("CDP connection failed: %s", exc)
            self.connected = False
            return False

    def get_contexts(self) -> List[Context]:
        """Fetch pages from CDP /json endpoint."""
        try:
            resp = requests.get(self._json_url(), timeout=1.0)
            if resp.status_code != 200:
                return []

            data = resp.json()

            if isinstance(data, dict):
                data = [data]
            elif not isinstance(data, list):
                return []

            contexts = []
            for item in data:
                if item.get("type") != "page":
                    continue

                context = Context(
                    id=item.get("id", ""),
                    url=item.get("url", ""),
                    title=item.get("title", ""),
                    protocol="cdp",
                    ws_url=item.get("webSocketDebuggerUrl"),
                )
                contexts.append(context)

            with self._lock:
                self._contexts_cache = contexts

            logger.debug("Retrieved %d CDP contexts", len(contexts))
            return contexts

        except Exception as exc:
            logger.error("Failed to get CDP contexts: %s", exc)
            return []

    def evaluate_script(
        self, context_id: str, expression: str, await_promise: bool = False
    ) -> Dict[str, Any]:
        """Execute JavaScript using CDP Runtime.evaluate."""
        context = next((c for c in self._contexts_cache if c.id == context_id), None)

        if not context or not context.ws_url:
            raise ValueError(f"Context {context_id} not found or has no WebSocket URL")

        try:
            ws = connect(context.ws_url, open_timeout=2.0)

            cmd = {
                "id": int(time.time() * 1000),
                "method": "Runtime.evaluate",
                "params": {
                    "expression": expression,
                    "awaitPromise": await_promise,
                    "returnByValue": True,
                },
            }

            ws.send(json.dumps(cmd))
            response = json.loads(ws.recv())
            ws.close()

            if "result" in response:
                result = response["result"].get("result", {})
                return {
                    "type": result.get("type", "undefined"),
                    "value": result.get("value"),
                    "error": None,
                }

            error = response.get("error", {})
            return {
                "type": "error",
                "value": None,
                "error": error.get("message", "Unknown error"),
            }

        except Exception as exc:
            logger.error("CDP evaluate failed: %s", exc)
            return {"type": "error", "value": None, "error": str(exc)}

    def close(self):
        """Cleanup CDP adapter."""
        with self._lock:
            self._contexts_cache = []
            self.connected = False

    def is_connected(self) -> bool:
        return self.connected
