"""
Utility functions for the RPP package.
"""

import os
import json
import urllib.request
import urllib.parse
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
        response = urllib.request.urlopen(generate_request(url, token), timeout=10)
        data = json.loads(response.read())
        return bool(data)
    except urllib.error.HTTPError as exc:
        log.error(f"on check {exc}")
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
    log.info(f"Downloading {name}...")
    try:
        url = urllib.parse.quote(url, safe=":/")
        response = urllib.request.urlopen(generate_request(url, token), timeout=10)
        data = json.loads(response.read())
    except urllib.error.HTTPError as e:
        log.error(f"Failed to download {name}: {e}")
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
        response = urllib.request.urlopen(
            generate_request(
                Constants.PRESENCES_LIST_ENPOINT,
                token,
            ),
            timeout=10,
        )
        data = json.loads(response.read())
        return [item["name"] for item in data if item["type"] == "dir"]
    except urllib.error.HTTPError as exc:
        log.error(f"Failed to get available presences: {exc}")
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
        log.info(f"Loaded env variables from {origin}.")
    except FileNotFoundError:
        log.warning(f"No .env file found in {origin}.")
    except Exception as exc:
        log.error(f"Failed to load .env file: {exc}")


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
