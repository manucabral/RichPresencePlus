import urllib.request
import json
from functools import wraps
from .tab import Tab
from .logger import get_logger, RPPLogger


def check_connection(func):
    """
    Decorator to check if the runtime is connected before calling a method.
    """

    @wraps(func)
    def wrapper(self, *args, **kwargs):
        if not self.connected:
            self.log.warning(
                "Make sure the runtime is connected to the browser before calling this method"
            )
            return None
        return func(self, *args, **kwargs)

    return wrapper


class Runtime:
    """
    Class to interact with the browser runtime.
    """

    def __init__(self, port: int):
        """
        Initialize the runtime.
        """
        self.port: int = port
        self.data: list[Tab] = []
        self.connected: bool = False
        self.log: RPPLogger = get_logger("Runtime")
        self.update()

    def update(self):
        """
        Update the data.
        """
        try:
            with urllib.request.urlopen(
                f"http://localhost:{self.port}/json", timeout=1
            ) as url:
                data = json.loads(url.read().decode())
                if not data:
                    return []
                self.data = [Tab(**d) for d in data]
                if not self.connected:
                    self.connected = True
                    self.log.info("Connected to browser successfully")
                self.log.debug("Updated data")
        except Exception as exc:
            self.log.warning("Could not connect to browser")
            self.connected = False

    @check_connection
    def tabs(self) -> list[Tab]:
        """
        Get the tabs.

        Returns:
            list[Tab]: A list of tabs
        """
        return self.data

    @check_connection
    def current_tab(self) -> Tab:
        """
        Get the current tab.

        Returns:
            Tab: The current tab
        """
        return self.data[0]

    @check_connection
    def filter_tabs(self, url: str) -> list[Tab]:
        """
        Filter tabs by url

        Args:
            url (str): The url to filter by

        Returns:
            list[Tab]: A list of tabs that match the url
        """
        return [tab for tab in self.data if url in tab.url]
