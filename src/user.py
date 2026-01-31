"""
User settings management — simple persistence to disk.
No locks, no atomic replace. Guaranteed overwrite.
"""

import os
import json
from typing import Dict, Any, Optional

from src.logger import logger
from src.constants import config

SETTINGS_PATH = str(config.base_dir.parent / config.user_settings_filename)


def exist_us_file(filepath: Optional[str] = None) -> bool:
    """
    Check if the user settings file exists.
    """
    path = filepath or SETTINGS_PATH
    exists = os.path.isfile(path)
    logger.info("Checking if user settings file exists at %s: %s", path, exists)
    return exists


def _write_file(path: str, data: str) -> None:
    """
    Direct write to file (reliable on Windows).
    """
    directory = os.path.dirname(path) or str(config.base_dir.parent)
    os.makedirs(directory, exist_ok=True)
    try:
        with open(path, "w+", encoding="utf-8") as f:
            f.write(data)
    except Exception as exc:
        logger.exception("Failed to write file: %s", path)
        raise exc
    logger.info("File written successfully: %s", path)


class UserSettings:
    """User settings — simple disk persistence (not thread-safe)."""

    _allowed_keys = (
        "profile_name",
        "runtime_interval",
        "browser_target_port",
        "logs_level",
    )

    def __init__(
        self,
        profile_name: str,
        runtime_interval: int,
        browser_target_port: int,
        logs_level: str,
        filepath: Optional[str] = None,
    ) -> None:
        self.profile_name = str(profile_name)
        self.runtime_interval = int(runtime_interval)
        self.browser_target_port = int(browser_target_port)
        self.logs_level = str(logs_level)
        self.filepath = filepath if filepath is not None else SETTINGS_PATH

        logger.info(
            "UserSettings initialized: profile_name=%s, runtime_interval=%d, "
            "browser_target_port=%d, logs_level=%s",
            self.profile_name,
            self.runtime_interval,
            self.browser_target_port,
            self.logs_level,
        )

    def to_dict(self) -> Dict[str, Any]:
        """
        Convert settings to a dictionary.
        """
        return {
            "profile_name": self.profile_name,
            "runtime_interval": self.runtime_interval,
            "browser_target_port": self.browser_target_port,
            "logs_level": self.logs_level,
        }

    @classmethod
    def load_from_file(cls) -> "UserSettings":
        """
        Load user settings from the JSON file.
        """
        path = SETTINGS_PATH

        if not os.path.isfile(path):
            raise FileNotFoundError(path)

        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)

        missing = set(cls._allowed_keys) - set(data.keys())
        if missing:
            raise ValueError(f"Missing keys in settings file: {missing}")

        logger.info("Loaded user settings from %s", path)

        return cls(
            profile_name=data["profile_name"],
            runtime_interval=data["runtime_interval"],
            browser_target_port=data["browser_target_port"],
            logs_level=data["logs_level"],
        )

    def save(self) -> bool:
        """
        Save user settings to the JSON file.
        """
        try:
            raw = json.dumps(self.to_dict(), indent=4, ensure_ascii=False)
            _write_file(self.filepath, raw)
            logger.info("User settings saved to %s", self.filepath)
            return True
        except Exception:
            logger.exception("Failed to save user settings to %s", self.filepath)
            return False

    def set_option(self, key: str, value: Any) -> None:
        """
        Set a user setting value by key.
        """
        if key not in self._allowed_keys:
            raise AttributeError(f"No such option: {key}")

        if key in ("runtime_interval", "browser_target_port"):
            value = int(value)
        else:
            value = str(value)

        if getattr(self, key) == value:
            logger.info("No change for %s: value is already %r", key, value)
            return

        setattr(self, key, value)
        logger.info("Set user setting %s to %r", key, value)
        self.save()

    def get_option(self, key: str) -> Any:
        """
        Get a user setting value by key.
        """
        if key not in self._allowed_keys:
            raise AttributeError(f"No such option: {key}")
        return getattr(self, key)


_user_settings: UserSettings | None = None


def get_user_settings() -> UserSettings:
    """
    Get the singleton UserSettings instance.
    """
    if exist_us_file():
        _user_settings = UserSettings.load_from_file()
    else:
        _user_settings = UserSettings(
            profile_name=config.browser_profile_name,
            runtime_interval=config.runtime_interval,
            browser_target_port=config.browser_target_port,
            logs_level=config.logs_level,
        )
        _user_settings.save()

    return _user_settings
