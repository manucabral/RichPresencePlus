import os
import winreg
import subprocess
from .logger import RPPLogger, get_logger
from .constants import Constants


class Browser:
    """
    Browser class for managing the browser instances.
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
        self._microsoft_store: bool = self.check_microsoft_store_app()
        self.log.info("Initialized.")
        self.executor = subprocess.run

    @property
    def microsoft_store(self) -> bool:
        return self._microsoft_store

    @microsoft_store.setter
    def microsoft_store(self, value: bool) -> None:
        self._microsoft_store = value

    def get_progid(self) -> str:
        """
        Get the ProgID for the browser.
        """
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, Constants.USER_CHOICE) as key:
            progid = winreg.QueryValueEx(key, "ProgId")[0]
            if not progid:
                raise RuntimeError("ProgID not found. Internal registry error.")
            return progid

    def get_path(self) -> str:
        """
        Get the browser path from the ProgID.

        Returns:
            str: The browser path.
        """
        custom_path = os.getenv("BROWSER_PATH")
        if custom_path:
            self.log.info("Using custom path: %s", custom_path)
            return custom_path
        with winreg.OpenKey(
            winreg.HKEY_CLASSES_ROOT,
            Constants.BROWSER_PATH.format(progId=self.progid),
        ) as key:
            path = winreg.QueryValue(key, None).split('"')[1]
            if not os.path.exists(path):
                self.log.warning("Browser path not found at %s", path)
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

    def check_microsoft_store_app(self) -> bool:
        """
        Check if the browser is a Microsoft Store app.

        Returns:
            bool: True if the browser is a Microsoft Store app, False otherwise.
        """
        command = [
            "powershell",
            "-Command",
            "Get-StartApps | Where-Object { $_.Name -like '{}*' }".format(
                self.process.replace(".exe", "")
            ),
        ]
        try:
            result = subprocess.run(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                shell=True,
                text=True,
                check=True,
            )
            return bool(result.stdout.strip())
        except subprocess.CalledProcessError:
            return False

    def get_app_id(self) -> str:
        """
        Get the application ID for the Microsoft Store app.

        Returns:
            str: The application ID.
        """
        command = [
            "powershell",
            "-Command",
            "Get-StartApps | Where-Object { $_.Name -like '{}*' }".format(
                self.process.replace(".exe", "")
            ),
        ]
        try:
            result = subprocess.run(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                shell=True,
                text=True,
                check=True,
            )
            app_info = result.stdout.strip()
            if app_info:
                # Extract the AppID from the output
                app_id = app_info.split()[-1]
                return app_id
            else:
                raise RuntimeError("AppID not found.")
        except subprocess.CalledProcessError:
            raise RuntimeError("Failed to get AppID.")

    def kill(self, admin: bool = False) -> None:
        """
        Kill the browser process using PowerShell with admin privileges if needed.

        Args:
            admin (bool, optional): Use admin privileges. Defaults to False.
        """
        command = (
            [
                "powershell",
                "-Command",
                "Start-Process",
                "powershell",
                "-ArgumentList",
                f"\"Stop-Process -Name '{self.process.replace('.exe', '')}' -Force\"",
                "-Verb",
                "RunAs",
            ]
            if admin
            else Constants.KILL_BROWSER.format(process=self.process)
        )

        def as_admin():
            self.log.warning(
                "No permission to kill browser normally. Trying to kill as admin."
            )
            if not admin:
                self.kill(admin=True)

        try:
            # pylint: disable=consider-using-with
            process = subprocess.Popen(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                shell=True,
            )
            self.log.info("Killed with PID: %s (%s)", process.pid, self.process)
            if self.running() and self.process == "Arc.exe":
                # Arc is a special case
                as_admin()
        except subprocess.CalledProcessError as exc:
            if not self.running():
                self.log.warning("Skipping kill, browser not running.")
                return
            self.log.error(exc)
            as_admin()
        except subprocess.TimeoutExpired:
            self.log.error("Timeout killing browser process.")
        except PermissionError:
            self.log.error("No permission to kill browser process.")
        except FileNotFoundError:
            self.log.error("Browser path not found.")

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
                check=True,
            )
            out = result.stdout.strip()
            return bool(out)
        except subprocess.CalledProcessError:
            return False

    def start(self, remote_port: int = 9222, admin: bool = False) -> None:
        """
        Start the browser with remote debugging.

        Args:
            remote_port (int, optional): The remote debugging port. Defaults to 9222.
        """
        if self.running():
            self.log.warning("Refusing to start browser, already running.")
            return

        if self.microsoft_store:
            app_id = self.get_app_id()
            command = (
                [
                    "powershell",
                    "-Command",
                    "Start-Process",
                    "powershell",
                    "-ArgumentList",
                    f'"explorer.exe shell:appsFolder\\{app_id} --remote-debugging-port={remote_port} '
                    f"--remote-allow-origins=http://127.0.0.1:{remote_port} "
                    f'--remote-allow-origins=http://localhost:{remote_port}"',
                    "-Verb",
                    "RunAs",
                ]
                if admin
                else [
                    "explorer.exe",
                    f"shell:appsFolder\\{app_id}",
                    f"--remote-debugging-port={remote_port}",
                    f"--remote-allow-origins=http://127.0.0.1:{remote_port}",
                    f"--remote-allow-origins=http://localhost:{remote_port}",
                ]
            )
        else:
            command = (
                [
                    "powershell",
                    "-Command",
                    "Start-Process",
                    f"'{self.path}'",
                    "-ArgumentList",
                    f"'--remote-debugging-port={remote_port}',"
                    f"'--remote-allow-origins=http://127.0.0.1:{remote_port}',"
                    f"'--remote-allow-origins=http://localhost:{remote_port}'",
                    "-Verb",
                    "RunAs",
                ]
                if admin
                else [
                    self.path,
                    f"--remote-debugging-port={remote_port}",
                    f"--remote-allow-origins=http://127.0.0.1:{remote_port}",
                    f"--remote-allow-origins=http://localhost:{remote_port}",
                ]
            )

        def as_admin():
            self.log.warning(
                "No permission to start browser normally. Trying to start as admin."
            )
            if not admin:
                self.start(remote_port, True)

        try:
            # pylint: disable=consider-using-with
            process = subprocess.Popen(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                shell=True,
                text=True,
            )
            self.log.info("Started with PID: %s (%s)", process.pid, self.process)
            special_cases = ["Arc.exe"]  # This is an special case
            if self.process in special_cases:
                _, stderr = process.communicate()
                result = stderr.strip()
                if "Access is denied" in result:
                    as_admin()
        except PermissionError:
            as_admin()
        except FileNotFoundError:
            self.log.error("Browser path not found.")
        except subprocess.TimeoutExpired:
            self.log.error("Timeout starting browser.")
        except subprocess.CalledProcessError as exc:
            self.log.error("Error starting browser.")
            self.log.error(exc)
            as_admin()
