"""
Configuration constants.
"""

# pylint: disable=invalid-name

import os
import sys
import pathlib
from dataclasses import dataclass


def _get_base_paths() -> tuple[pathlib.Path, pathlib.Path, pathlib.Path]:
    """
    Determine base paths depending on frozen/development mode.

    Returns:
        Tuple of (BASE_DIR, PRESENCES_DIR, FRONTEND_DIR)
    """
    frozen_mode = getattr(sys, "frozen", False)

    if frozen_mode:
        base_dir = pathlib.Path(sys.executable).parent
        presences_dir = base_dir / "presences"
        frontend_dir = base_dir / "dist"
    else:
        base_dir = pathlib.Path(__file__).resolve().parent
        presences_dir = base_dir.parent / "presences"
        frontend_dir = base_dir.parent / "dist"

    return base_dir, presences_dir, frontend_dir


FROZEN_MODE = getattr(sys, "frozen", False)
BASE_DIR, PRESENCES_DIR, FRONTEND_DIR = _get_base_paths()


@dataclass(frozen=True)
class Config:
    """
    Configuration settings.
    """

    title: str = "Rich Presence Plus"
    version: str = "0.1.1-beta"
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
    browser_target_port: int = int(os.getenv("RPP_BROWSER_PORT", "4969"))
    geckodriver_path: str = "geckodriver"
    runtime_interval: int = 2  # seconds

    development_mode: bool = True
    frontend_dev_server_url: str = os.getenv(
        "RPP_DEV_SERVER_URL", "http://localhost:5173"
    )
    window_width: int = 950
    window_height: int = 650

    github_owner: str = "manucabral"
    github_repo: str = "RichPresencePlus"
    github_token: str | None = os.getenv("RPP_GITHUB_TOKEN")
    meta_filename: str = ".meta.json"
    cache_filename: str = "presences.cache.json"
    cache_ttl_seconds: int = 300  # 5 minutes

    steam_config_path: str = os.getenv(
        "RPP_STEAM_CONFIG", r"C:\Program Files (x86)\Steam\config\config.vdf"
    )
    steam_base_id4: int = 76561197960265728


config = Config()
