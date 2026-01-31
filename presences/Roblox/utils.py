import os
import json
from typing import Optional
import requests
from src.logger import logger


def get_last_roblox_log(log_path: Optional[str]) -> Optional[str]:
    """
    Get the last Roblox log file from the specified log path.
    """
    if log_path is None:
        return None
    if not os.path.exists(log_path):
        logger.error("Log path not found: %s", log_path)
        return None
    with os.scandir(log_path) as entries:
        files = [
            entry.name
            for entry in entries
            if entry.is_file() and entry.name.endswith("_last.log")
        ]
    if not files:
        return None
    return os.path.join(log_path, files[-1])


def make_request(url: str) -> Optional[dict]:
    """
    Make a GET request
    """
    try:
        response = requests.get(url)
        response.raise_for_status()
        return response.json().get("data", [])[0]
    except requests.RequestException as e:
        logger.error("Request error: %s", e)
    except (json.JSONDecodeError, IndexError) as e:
        logger.error("JSON parsing error: %s", e)
    return None
