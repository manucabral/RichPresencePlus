"""
Protocol Adapter - Abstract base for browser protocol implementations.
"""

from abc import ABC, abstractmethod
from typing import List, Dict, Any

from .context import Context


class ProtocolAdapter(ABC):
    """Abstract base for protocol-specific implementations."""

    @abstractmethod
    def connect(self) -> bool:
        """Establish connection to browser."""

    @abstractmethod
    def get_contexts(self) -> List[Context]:
        """Get available execution contexts."""

    @abstractmethod
    def evaluate_script(
        self, context_id: str, expression: str, await_promise: bool = False
    ) -> Dict[str, Any]:
        """Execute JavaScript in a context."""

    @abstractmethod
    def close(self):
        """Close connection and cleanup."""

    @abstractmethod
    def is_connected(self) -> bool:
        """Check if adapter is connected."""
