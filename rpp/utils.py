"""
Utility functions for the RPP package.
"""

import os
import json
import urllib.request
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
    log.info(token)
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
                "https://api.github.com/repos/manucabral/RichPresencePlus/contents/presences",
                token,
            ),
            timeout=10,
        )
        data = json.loads(response.read())
        return [item["name"] for item in data if item["type"] == "dir"]
    except urllib.error.HTTPError as exc:
        log.error(f"Failed to get available presences: {exc}")
        return []


def load_env() -> None:
    """
    Load the environment variables from the .env file.
    """
    try:
        with open(".env", "r") as file:
            for line in file:
                key, value = line.strip().split("=")
                os.environ[key] = value
        log.info("Loaded .env file.")
    except FileNotFoundError:
        log.warning("No .env file found.")
    except Exception as exc:
        log.error(f"Failed to load .env file: {exc}")
