"""
Rich Presence Plus
------------------
A simple Discord Rich Presence manager for desktop and web apps.
"""

from .logger import log
from .runtime import Runtime
from .presence import Presence

__title__ = "Rich Presence Plus"
__author__ = "Manuel Cabral"
__version__ = "0.0.1"
__license__ = "GPLv3"

__all__ = [
    "log",
    "Runtime",
    "Presence",
]
