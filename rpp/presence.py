"""
Presence base class for creating new presences.
"""

from abc import ABC, abstractmethod
from .rpc import ActivityType


# pylint: disable=R0902
class Presence(ABC):
    """
    Abstract class for creating a presence for Rich Presence Plus.
    """

    def __init__(self, metadata_file: bool = False):
        """
        Initialize the presence.

        Args:
            metadata_file (bool): Whether the presence is being loaded from a metadata (json) file.

        """
        self.client_id: str = None
        self.name: str = None
        self.author: str = None
        self.version: str = None
        self.web: bool = False
        self.automatic: bool = True
        self.enabled: bool = True
        self.running: bool = False
        self.update_interval: int = 3

        # Internal variables
        self.metadata_file: bool = metadata_file
        self.dev_mode: bool = False
        self.path: str = None
        self.log = None

        # Presence data
        self.title: str = None
        self.details: str = None
        self.state: str = None
        self.large_image: str = None
        self.large_text: str = None
        self.small_image: str = None
        self.small_text: str = None
        self.activity_type: ActivityType = ActivityType.PLAYING
        self.buttons: list = None
        self.start: int = None
        self.end: int = None

    @abstractmethod
    def on_load(self) -> None:
        """
        Called when the presence is loaded.
        """

    @abstractmethod
    def on_update(self, **context) -> None:
        """
        Called when the presence is updated.

        Args:
            context: The context of the update.

        Context:
            runtime: The runtime instance.
        """

    @abstractmethod
    def on_close(self) -> None:
        """
        Called when the presence is closed.
        """

    def force_update(self):
        """
        Force update the presence.
        """
