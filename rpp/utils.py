"""
Utility functions for the RPP package.
"""

import os
import re
import json
import urllib.request
import urllib.parse
from packaging import version as pversion
from .version import __version__
from .constants import Constants
from .logger import get_logger

log = get_logger("Utils")


def generate_request(url: str, token: str = None) -> urllib.request.Request:
    """
    Generate a request object with the necessary headers.

    Args:
        url (str): URL to the request.
        token (str): GitHub token for the request.

    Returns:
        urllib.request.Request: The request object.
    """
    headers = {}
    if token:
        headers["Authorization"] = f"token {token}"
    return urllib.request.Request(url, headers=headers)


def download_github_file(url: str, file: str) -> None:
    """
    Download a file from GitHub.

    Args:
        url (str): URL to the file.
        file (str): File to save the downloaded content.
    """
    urllib.request.urlretrieve(url, file)


def exist_github_folder(url: str, token: str = None) -> bool:
    """
    Check if a GitHub repository directory exists.

    Args:
        url (str): URL to the GitHub repository directory.

    Returns:
        bool: Whether the directory exists.
    """
    try:
        url = urllib.parse.quote(url, safe=":/")
        with urllib.request.urlopen(
            generate_request(url, token), timeout=10
        ) as response:
            data = json.loads(response.read())
        return bool(data)
    except urllib.error.HTTPError as exc:
        log.error("Check %s", exc)
        return False


def download_github_folder(url: str, folder: str, token: str = None) -> None:
    """
    Download the contents of a GitHub repository directory.

    Args:
        url (str): URL to the GitHub repository directory.
        folder (str): Folder to save the downloaded files.
    """
    if not os.path.exists(folder):
        os.makedirs(folder)
    name = url.split("/")[-1]
    log.info("Downloading %s", name)
    try:
        url = urllib.parse.quote(url, safe=":/")
        with urllib.request.urlopen(
            generate_request(url, token), timeout=10
        ) as response:
            data = json.loads(response.read())
    except urllib.error.HTTPError as e:
        log.error("Failed to download %s: %s", name, e)
    for item in data:
        if item["type"] == "file":
            download_github_file(
                item["download_url"], os.path.join(folder, item["name"])
            )


def get_available_presences(token: str = None) -> list[str]:
    """
    Get the available presences from the remote repository.

    Args:
        token (str): GitHub token for the request.

    Returns:
        list[str]: A list of available presences.
    """
    try:
        with urllib.request.urlopen(
            generate_request(
                Constants.PRESENCES_LIST_ENPOINT,
                token,
            ),
            timeout=10,
        ) as response:
            data = json.loads(response.read())
        return [item["name"] for item in data if item["type"] == "dir"]
    except urllib.error.HTTPError as exc:
        log.error("On get available presences: %s", exc)
        return []


def load_env(path: str = ".env", origin: str = "main") -> None:
    """
    Load the environment variables from the .env file.
    """
    try:
        with open(path, "r", encoding="utf-8", errors="ignore") as file:
            for line in file:
                key, value = line.strip().split("=")
                os.environ[key] = value
        log.info("Loaded env variables from %s", origin)
    except FileNotFoundError:
        log.warning("No .env file found in %s.", origin)
    except ValueError:
        log.error("Invalid .env file format.")
    # pylint: disable=broad-except
    except Exception as exc:
        log.error("Unexpected error: %s", exc)


def remove_none(d: dict) -> dict:
    """
    Remove None values from a dictionary
    """

    def clean_dict(d: dict) -> dict:
        """
        Clean the dictionary from None values
        """
        return {
            k: clean_dict(v) if isinstance(v, dict) else v
            for k, v in d.items()
            if v is not None and (not isinstance(v, dict) or len(v) > 0)
        }

    return clean_dict(d)


def check_version_compatibility(version: str) -> bool:
    """
    Check if the current version is compatible with the required version.

    Args:
        version (str): Version to check.

    Returns:
        bool: Whether the version is compatible.

    E.g.:
        check_version_compatibility(">=1.0.0,<2.0.0")
        check_version_compatibility("==1.0.0")
        check_version_compatibility("<2.0.0")
        check_version_compatibility(">=1.0.0")
    """
    try:
        actual_version = pversion.parse(__version__)
        conditions = [req.strip() for req in version.split(",")]
        for condition in conditions:
            if condition.startswith("=="):
                required_version = pversion.parse(condition[2:])
                if actual_version != required_version:
                    log.error("Version mismatch: %s", version)
                    return False
            elif condition.startswith(">="):
                required_version = pversion.parse(condition[2:])
                if actual_version < required_version:
                    log.error("Version too old: %s", version)
                    return False
            elif condition.startswith("<"):
                required_version = pversion.parse(condition[1:])
                if actual_version >= required_version:
                    log.error("Version too new: %s", version)
                    return False
            else:
                raise ValueError(f"Unknown version condition: {condition}")
        return True
    except pversion.InvalidVersion as exc:
        log.error("Invalid version: %s", exc)
        return False
    except ValueError as exc:
        log.error("Invalid version condition: %s", exc)
        return False


def get_steam_presence(steam_id3: int) -> dict:
    """
    Get the presence information for a Steam account.

    Args:
        steam_id3 (int): Steam ID3 of the account.

    Returns:
        dict: The presence information.
    """
    state = {"name": None, "state": None}
    try:
        with urllib.request.urlopen(
            f"https://steamcommunity.com/miniprofile/{steam_id3}",
            timeout=10,
        ) as response:
            response = response.read().decode("utf-8")
        name_pattern = re.compile(r'<span class="miniprofile_game_name">([^<]+)</span>')
        state_pattern = re.compile(r'<span class="rich_presence">(.*?)</span>')
        game_name_match = name_pattern.search(response)
        game_name = game_name_match.group(1) if game_name_match else None
        game_state_match = state_pattern.search(response)
        game_state = game_state_match.group(1) if game_state_match else None
        state = {"name": game_name, "state": game_state}
        return state
    except urllib.error.HTTPError as exc:
        log.error("Failed to get Steam presence: %s", exc)
    except AttributeError as exc:
        log.error("Failed to parse Steam presence: %s", exc)
    # pylint: disable=broad-except
    except Exception as exc:
        log.error("Unexpected error: %s", exc)
    return state
