"""
Constants used throughout the project.
"""

import enum

PRESENCES_URL = "https://github.com/manucabral/RichPresencePlus/tree/4497a5a7b97d04395f0cf242952d8fd22edd8d70/presences/{name}"
LOG_FILENAME = "presenceplus.log"
REMOTE_URL = "http://localhost:{port}/json"

RESTRICTED_GLOBALS = [
    "input",
    "open",
    "eval",
    "exec",
    "compile",
    "exit",
    "quit",
]


class TimeLimit(enum.Enum):
    """
    Times limits used throughout the project.
    """

    DISCORD = 15
    RPP = 0.1


class LogLevel(enum.Enum):
    """
    The level of logging to use.
    """

    INFO = 0
    WARNING = 1
    ERROR = 2
    CRITICAL = 3
    DEBUG = 4


class RunMode(enum.Enum):
    """
    The mode in which the application is running.
    """

    BOTH = 0
    WEB = 1
    DESKTOP = 2
