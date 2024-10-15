"""
Constants module for Rich Presence Plus.
"""


class Constants:
    """
    Constants class for Rich Presence Plus.
    """

    MAX_PRESENCES = 10
    PRESENCES_FOLDER = "presences"
    PRESENCES_ENPOINT = "https://api.github.com/repos/manucabral/RichPresencePlus/contents/presences/{presence_name}"
    RUNTIME_INTERVAL = 1
    PRESENCE_INTERVAL = 15
    DEV_MODE = True
    LOG_FILE = "rpp.log"

    USER_CHOICE = (
        r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\http\UserChoice"
    )
    FIND_BROWSER = "wmic process where \"name='{process}'\" get ProcessId /format:value"
    BROWSER_PATH = "{progId}\\shell\\open\\command"
    BROWSER_NAME = "{progId}\\Application"
    KILL_BROWSER = "taskkill /f /im {process} /t"
