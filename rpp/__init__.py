"""
Rich Presence Plus
----------------
A simple Discord Rich Presence manager for desktop and web apps.
"""

from .logger import log
from .presence import Presence

__all__ = [
    "log",
    "Presence",
]
