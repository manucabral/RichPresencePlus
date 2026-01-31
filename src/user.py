"""
User settings management — thread-safe, atomic persistence to disk.
"""

import os
import json
import threading
from dataclasses import dataclass, asdict
from typing import Dict, Any, Optional
from src.logger import logger
from src.constants import config


class UserSettings:
    """User settings for the application — thread-safe and saved atomically on disk."""

    def __init__(
        self,
        profile_name: str,
        runtime_interval: int,
        browser_target_port: int,
        logs_level: str,
        filepath: Optional[str] = None,
    ) -> None:
        self.profile_name: str = profile_name
        self.runtime_interval: int = int(runtime_interval)
        self.browser_target_port: int = int(browser_target_port)
        self.logs_level: str = logs_level
        self.filepath: str = filepath or config.user_settings_filename
        self._lock = threading.Lock()

        logger.info(
            "UserSettings initialized: "
            "profile_name=%s, runtime_interval=%d, "
            "browser_target_port=%d, logs_level=%s",
            self.profile_name,
            self.runtime_interval,
            self.browser_target_port,
            self.logs_level,
        )

    @property
    def _allowed_keys(self):
        return (
            "profile_name",
            "runtime_interval",
            "browser_target_port",
            "logs_level",
        )

    def set_option(self, key: str, value: Any) -> None:
        """Set an option and persist to disk.

        Raises AttributeError if key is not allowed.
        """
        if key.startswith("_"):
            raise AttributeError("Cannot set private attribute")

        with self._lock:
            if key in self._allowed_keys:
                setattr(self, key, value)
                logger.info("Set user setting %s to %r", key, value)
                saved = save_us_to_file(self, self.filepath)
                if not saved:
                    logger.error(
                        "Failed to persist user settings after setting %s", key
                    )
            else:
                raise AttributeError(f"No such option: {key}")

    def get_option(self, key: str) -> Any:
        """Get an option value.

        Raises AttributeError if key is not allowed.
        """
        if key.startswith("_"):
            raise AttributeError("Cannot read private attribute")

        with self._lock:
            if key in self._allowed_keys:
                logger.info("Getting user setting %s", key)
                return getattr(self, key)
            raise AttributeError(f"No such option: {key}")

    def save(self) -> bool:
        """Persist current settings to disk (thread-safe)."""
        with self._lock:
            logger.info("Saving user settings to file %s", self.filepath)
            return save_us_to_file(self, self.filepath)


def exist_us_file(filepath: Optional[str] = None) -> bool:
    """Check if the user settings file exists."""
    path = filepath or config.user_settings_filename
    exists = os.path.isfile(path)
    logger.info("Checking if user settings file exists at %s: %s", path, exists)
    return exists


@dataclass
class UserSettingsData:
    """Data class for user settings serialization."""

    profile_name: str
    runtime_interval: int
    browser_target_port: int
    logs_level: str

    def to_dict(self) -> Dict[str, object]:
        """Return a dict representation of the UserSettingsData."""
        return asdict(self)

    @classmethod
    def from_user_settings(cls, us: UserSettings) -> "UserSettingsData":
        """Create UserSettingsData from UserSettings instance."""
        return cls(
            profile_name=str(us.profile_name),
            runtime_interval=int(us.runtime_interval),
            browser_target_port=int(us.browser_target_port),
            logs_level=str(us.logs_level),
        )

    @classmethod
    def to_user_settings(cls, data: "UserSettingsData") -> UserSettings:
        """Create UserSettings instance from UserSettingsData."""
        us = UserSettings(
            profile_name=data.profile_name,
            runtime_interval=data.runtime_interval,
            browser_target_port=data.browser_target_port,
            logs_level=data.logs_level,
        )
        return us


def load_us_from_file(filepath: Optional[str] = None) -> UserSettings:
    """Load user settings from a JSON file and return a UserSettings instance.

    Raises FileNotFoundError if path doesn't exist, ValueError on invalid contents.
    """
    path = filepath or config.user_settings_filename

    if not os.path.isfile(path):
        logger.error("User settings file not found: %s", path)
        raise FileNotFoundError(path)

    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)

        required_keys = {
            "profile_name",
            "runtime_interval",
            "browser_target_port",
            "logs_level",
        }
        if not required_keys.issubset(set(data.keys())):
            missing = required_keys - set(data.keys())
            logger.error("User settings file %s is missing keys: %s", path, missing)
            raise ValueError(f"Missing keys in user settings file: {missing}")

        usd = UserSettingsData(**data)
        us = UserSettingsData.to_user_settings(usd)
        us.filepath = path
        logger.info("Loaded user settings from file %s", path)
        return us

    except json.JSONDecodeError as exc:
        logger.error("Failed to parse user settings JSON from %s: %s", path, exc)
        raise
    except Exception:
        logger.exception("Unexpected error while loading user settings from %s", path)
        raise


def _atomic_write(path: str, data: str) -> None:
    """Write data to path atomically using a temporary file and os.replace."""
    dirpath = os.path.dirname(path) or "."
    tmp_path = os.path.join(dirpath, f".{os.path.basename(path)}.tmp")
    with open(tmp_path, "w", encoding="utf-8") as f:
        f.write(data)
    os.replace(tmp_path, path)


def save_us_to_file(us: UserSettings, filepath: Optional[str] = None) -> bool:
    """Save user settings to a JSON file atomically.

    Returns True on success, False on failure (and logs the exception).
    """
    path = filepath or getattr(us, "_filepath", None) or config.user_settings_filename
    try:
        usd = UserSettingsData.from_user_settings(us)
        raw = json.dumps(usd.to_dict(), indent=4, ensure_ascii=False)
        # Ensure directory exists
        os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
        _atomic_write(path, raw)
        logger.info("Saved user settings to file %s", path)
        return True
    except Exception as exc:
        logger.exception("Failed to save user settings to file %s: %s", path, exc)
        return False
