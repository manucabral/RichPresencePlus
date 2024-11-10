"""
Constants module for Rich Presence Plus.
"""


# pylint: disable=C0301, R0903
class Constants:
    """
    Constants class for Rich Presence Plus.
    """

    MAX_PRESENCES = 10
    PRESENCES_FOLDER = "presences"
    PRESENCES_ENPOINT = "https://api.github.com/repos/manucabral/RichPresencePlus/contents/presences/{presence_name}"
    PRESENCES_LIST_ENPOINT = (
        "https://api.github.com/repos/manucabral/RichPresencePlus/contents/presences"
    )
    RUNTIME_INTERVAL = 2
    PRESENCE_INTERVAL = 15
    DEV_MODE = False
    LOG_FILE = "rpp.log"

    USER_CHOICE = (
        r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\http\UserChoice"
    )
    FIND_BROWSER = 'tasklist /FI "IMAGENAME eq {process}" /FO CSV | findstr {process}'
    BROWSER_PATH = "{progId}\\shell\\open\\command"
    BROWSER_NAME = "{progId}\\Application"
    KILL_BROWSER = "taskkill /f /im {process} /t"

    STEAM_CONFIG_PATH = r"C:\Program Files (x86)\Steam\config\config.vdf"
    STEAM_BASE_ID4 = 76561197960265728
