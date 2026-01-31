"""
Configuration constants.
"""

# pylint: disable=invalid-name

import sys
import pathlib
import multiprocessing
from dataclasses import dataclass

FROZEN_MODE = getattr(sys, "frozen", False)

if FROZEN_MODE:
    BASE_DIR = pathlib.Path(sys.executable).parent
else:
    BASE_DIR = pathlib.Path(__file__).resolve().parent

try:
    if multiprocessing.current_process().name == "MainProcess":
        if FROZEN_MODE:
            print(f"Running in frozen mode. BASE_DIR set to: {BASE_DIR}")
        else:
            print(f"Running in normal mode. BASE_DIR set to: {BASE_DIR}")
except Exception:
    if FROZEN_MODE:
        print(f"Running in frozen mode. BASE_DIR set to: {BASE_DIR}")
    else:
        print(f"Running in normal mode. BASE_DIR set to: {BASE_DIR}")

if FROZEN_MODE:
    PRESENCES_DIR = BASE_DIR / "presences"
    FRONTEND_DIR = BASE_DIR / "dist"
else:
    PRESENCES_DIR = BASE_DIR.parent / "presences"
    FRONTEND_DIR = BASE_DIR.parent / "dist"


try:
    if multiprocessing.current_process().name == "MainProcess":
        print(f"PRESENCES_DIR set to: {PRESENCES_DIR}")
        print(f"FRONTEND_DIR set to: {FRONTEND_DIR}")
except Exception:
    print(f"PRESENCES_DIR set to: {PRESENCES_DIR}")
    print(f"FRONTEND_DIR set to: {FRONTEND_DIR}")


@dataclass(frozen=True)
class Config:
    """
    Configuration settings.
    """

    title: str = "Rich Presence Plus"
    version: str = "0.1.1"
    description: str = "An advanced and simple Rich Presence application."
    author: str = "Manuel Cabral"

    base_dir: pathlib.Path = BASE_DIR
    presences_dir: pathlib.Path = PRESENCES_DIR
    frontend_dir: pathlib.Path = FRONTEND_DIR
    presences_entrypoint: str = "main.py"
    presences_callable: str = "execute"
    user_settings_filename: str = "settings.json"

    logs_dir: pathlib.Path = (
        BASE_DIR.parent / "logs" if not FROZEN_MODE else BASE_DIR / "logs"
    )
    logs_level: str = "INFO"
    logs_default_name: str = "app"

    browser_profile_name: str = "RPP"
    browser_target_port: int = 4969
    geckodriver_path: str = "geckodriver"
    runtime_interval: int = 2  # seconds

    development_mode: bool = False
    frontend_dev_server_url: str = "http://localhost:5173"
    window_width: int = 950
    window_height: int = 650

    github_owner: str = "manucabral"
    github_repo: str = "RichPresencePlus"
    github_token: str | None = ""
    meta_filename: str = ".meta.json"
    cache_filename: str = "presences.cache.json"
    cache_ttl_seconds: int = 300  # 5 minutes


config = Config()
