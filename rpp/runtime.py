"""
Runtime module used for send commands to the browser.
"""
import httpx
from .tab import Tab
from .logger import log
from .constants import REMOTE_URL

# pylint: disable=broad-except


class Runtime:
    """
    Used for send commands to the chrome browser.
    """

    def __init__(self):
        self.__tabs = []
        self.__port = 9228
        self.__connected = False
        self.__current_tab = None

    def __repr__(self):
        """
        Return the representation of the object.
        """
        return f"<Runtime connected={self.__connected}>"

    def __req(self) -> None:
        """
        Connect to the browser.
        """
        try:
            # set timeout to 5 seconds
            return httpx.get(REMOTE_URL.format(port=self.__port), timeout=5).json()
        except Exception as exc:
            raise exc

    def __update(self) -> None:
        """
        Update the tabs list.
        """
        if not self.__connected:
            return
        try:
            tabs = self.__req()
            self.__tabs = [tab for tab in tabs if tab["type"] == "page"]
            if len(tabs) == 0:
                log("No tabs found.", level="WARNING")
                return
            self.__tabs = [Tab(**tab) for tab in tabs]
            self.__current_tab = self.__tabs[0]
        except Exception as exc:
            self.__connected = False
            self.__tabs = []
            log(exc, level="ERROR")

    def tabs(self, force_update: bool = True) -> list:
        """
        Return the tabs.
        """
        if force_update:
            self.__update()
        return self.__tabs

    @property
    def current_tab(self) -> Tab:
        """
        Return the current tab.
        """
        self.__update()
        return self.__current_tab

    @property
    def connected(self) -> bool:
        """
        Return if the runtime is connected.
        """
        return self.__connected

    def connect(self) -> bool:
        """
        Connect to the browser.
        """
        if self.__connected:
            return True
        try:
            self.__req()
            log("Connected to the browser.")
            self.__connected = True
            return self.__connected
        except Exception as exc:
            self.__connected = False
            log(
                "Failed to connect to the browser because " + str(exc),
                level="ERROR",
            )
            return False
