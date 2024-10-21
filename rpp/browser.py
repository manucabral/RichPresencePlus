"""
Browser module for manage the browser instances. 
Based on pybrinf (https://github.com/manucabral/pybrinf)
"""

import os
import winreg
import subprocess
from .logger import RPPLogger, get_logger
from .constants import Constants


class Browser:
    """
    Browser class for manage the browser instances.
    """

    def __init__(self):
        """
        Initialize the browser.
        """
        self.log: RPPLogger = get_logger(self.__class__.__name__)
        self.progid: str = self.get_progid()
        self.path = self.get_path()
        self.name: str = self.get_name()
        self.process: str = self.path.split("\\")[-1]
        self.log.info("Initialized.")

    def get_progid(self) -> str:
        """
        Get the ProgID for the browser.
        """
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, Constants.USER_CHOICE) as key:
            progid = winreg.QueryValueEx(key, "ProgId")[0]
            if not progid:
                raise Exception("Can't find browser progid")
            return progid

    def get_path(self) -> str:
        """
        Get the browser path from the ProgID.

        Returns:
            str: The browser path.
        """
        with winreg.OpenKey(
            winreg.HKEY_CLASSES_ROOT,
            Constants.BROWSER_PATH.format(progId=self.progid),
        ) as key:
            path = winreg.QueryValue(key, None).split('"')[1]
            if not os.path.exists(path):
                self.log.warning(f"Browser path not found at {path}")
        return path

    def get_name(self) -> str:
        """
        Get the browser name.

        Returns:
            str: The browser name. E.g. "Google Chrome"
        """
        try:
            with winreg.OpenKeyEx(
                winreg.HKEY_CLASSES_ROOT,
                Constants.BROWSER_NAME.format(progId=self.progid),
            ) as key:
                name = winreg.QueryValueEx(key, "ApplicationName")[0]
            return name or "Browser"
        except FileNotFoundError:
            self.log.warning("Browser name not found. Using progid.")
            return self.progid

    def kill(self) -> None:
        """
        Kill the browser process.
        """
        try:
            subprocess.run(
                Constants.KILL_BROWSER.format(process=self.process),
                shell=True,
                check=True,
            )
            self.log.info("Killed browser process.")
        except subprocess.CalledProcessError:
            self.log.error("Error killing browser process.")

    def running(self) -> bool:
        """
        Check if the browser is running.

        Returns:
            bool: True if the browser is running, False otherwise.
        """
        try:
            result = subprocess.run(
                Constants.FIND_BROWSER.format(process=self.process),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                shell=True,
                text=True,
            )
            out = result.stdout.strip()
            return bool(out)
        except subprocess.CalledProcessError:
            return False

    def start(self, remote_port: int = 9222) -> None:
        """
        Start the browser with remote debugging.

        Args:
            remote_port (int, optional): The remote debugging port. Defaults to 9222.
        """
        if self.running():
            self.log.warning("Refusing to start browser, already running.")
            return
        try:
            subprocess.Popen(
                [
                    self.path,
                    f"--remote-debugging-port={remote_port}",
                    f"--remote-allow-origins=http://127.0.0.1:{remote_port}",
                    f"--remote-allow-origins=http://localhost:{remote_port}",
                ]
            )
            self.log.info("Started.")
        except Exception as exc:
            self.log.error(f"Error starting browser: {exc}")
