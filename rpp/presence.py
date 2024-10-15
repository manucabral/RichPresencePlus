from abc import ABC, abstractmethod


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
        self.name = None
        self.author = "Unknown"
        self.version = "1.0.0"
        self.web = False
        self.enabled = True
        self.running = False
        self.update_interval = 3
        self.metadata_file = metadata_file
        self.dev_mode = True
        self.client_id = None
        self.log = None
        self.path = None

        # Presence data
        self.title = None
        self.details = None
        self.state = None
        self.large_image = None
        self.small_image = None
        self.small_text = "Rich Presence Plus"
        self.buttons = None
        self.start = None
        self.end = None

    @abstractmethod
    def on_load(self) -> None:
        """
        Called when the presence is loaded.
        """
        pass

    @abstractmethod
    def on_update(self, **context) -> None:
        """
        Called when the presence is updated.

        Args:
            context: The context of the update.

        Context:
            runtime: The runtime instance.
        """
        pass

    @abstractmethod
    def on_close(self) -> None:
        """
        Called when the presence is closed.
        """
        pass

    @abstractmethod
    def force_update(self):
        """
        Force update the presence.
        """
        pass
