"""
Rich Presence Plus
------------------
A simple Discord Rich Presence manager for custom desktop or web status.
"""

from .logger import get_logger
from .manager import Manager
from .presence import Presence
from .extension import extension
from .browser import Browser
from .runtime import Runtime
from .tab import Tab

__title__ = "Rich Presence Plus"
__version__ = "0.0.1"
__author__ = "Manuel Cabral"
__license__ = "GPL-3.0"


__all__ = ["get_logger", "Manager", "Presence", "extension", "Runtime", "Tab"]
