"""
Utility functions for the RPP package.
"""

import os
import json
import urllib.request
from .logger import get_logger

log = get_logger("Utils")


def download_github_file(url: str, file: str) -> None:
    """
    Download a file from GitHub.

    Args:
        url (str): URL to the file.
        file (str): File to save the downloaded content.
    """
    urllib.request.urlretrieve(url, file)


def exist_github_folder(url: str) -> bool:
    """
    Check if a GitHub repository directory exists.

    Args:
        url (str): URL to the GitHub repository directory.

    Returns:
        bool: Whether the directory exists.
    """
    try:
        print(url)
        response = urllib.request.urlopen(url, timeout=10)
        data = json.loads(response.read())
        return bool(data)
    except urllib.error.HTTPError as exc:
        log.error(f"on check {exc}")
        return False


def download_github_folder(url: str, folder: str) -> None:
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
        response = urllib.request.urlopen(url, timeout=10)
        data = json.loads(response.read())
    except urllib.error.HTTPError as e:
        log.error(f"Failed to download {name}: {e}")
    for item in data:
        if item["type"] == "file":
            download_github_file(
                item["download_url"], os.path.join(folder, item["name"])
            )
