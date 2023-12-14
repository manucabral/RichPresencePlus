"""
Constants used throughout the project.
"""
import enum

LOG_FILENAME = "presenceplus.log"


class LogLevel(enum.Enum):
    INFO = 0
    WARNING = 1
    ERROR = 2
    CRITICAL = 3
    DEBUG = 4


class RunMode(enum.Enum):
    BOTH = 0
    WEB = 1
    DESKTOP = 2
