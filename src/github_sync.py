"""
GitHub synchronization functions.
"""

import json
import time
import base64
import shutil
from pathlib import Path
from typing import Dict, List, Tuple
import requests
from .logger import logger
from .constants import config


def github_headers() -> dict:
    """
    Prepare GitHub API headers.
    """
    headers = {"Accept": "application/vnd.github+json"}
    if config.github_token:
        logger.debug("Using GitHub token for authentication")
        headers["Authorization"] = f"Bearer {config.github_token}"
    return headers


def load_cache() -> List[dict] | None:
    """
    Load presences list from cache if not expired.
    """
    cache_path = Path(config.cache_filename)
    if not cache_path.exists():
        logger.debug("Cache file does not exist")
        return None

    try:
        with open(cache_path, "r", encoding="utf-8") as f:
            cache = json.load(f)

        age = time.time() - cache.get("timestamp", 0)
        if age > config.cache_ttl_seconds:
            logger.debug("Cache expired")
            return None

        logger.info("Loaded presences from cache")
        return cache.get("data")

    except Exception as exc:
        logger.warning("Cache load failed: %s", exc)
        return None


def save_cache(data: List[dict]):
    """
    Save presences list to cache.
    """
    try:
        logger.debug("Saving presences to cache")
        with open(
            config.cache_filename, "w", encoding="utf-8", errors="ignore"
        ) as file_:
            json.dump(
                {
                    "timestamp": int(time.time()),
                    "data": data,
                },
                file_,
                indent=2,
            )
        logger.debug("Cache saved")
    except Exception as exc:
        logger.warning("Cache save failed: %s", exc)


def get_remote_presences_list(force_refresh: bool = False) -> List[dict]:
    """
    Returns list of:
    { name, manifest }
    Cached in rootfolder for instant response.
    """

    if not force_refresh:
        cached = load_cache()
        if cached is not None:
            return cached

    logger.info("Fetching remote presences from GitHub")

    url = "https://api.github.com/repos/"
    url += f"{config.github_owner}/{config.github_repo}/contents/"
    url += Path(config.presences_dir).name
    resp = requests.get(url, headers=github_headers(), timeout=10)
    resp.raise_for_status()

    presences = []

    for item in resp.json():
        if item["type"] != "dir":
            continue

        name = item["name"]
        manifest_url = (
            f"https://api.github.com/repos/"
            f"{config.github_owner}/{config.github_repo}/contents/"
            f"{config.presences_dir.name}/{name}/manifest.json"
        )

        manifest = None

        try:
            mr = requests.get(manifest_url, headers=github_headers(), timeout=5)
            if mr.status_code == 200:
                data = mr.json()
                if data.get("encoding") == "base64":
                    content = base64.b64decode(data["content"]).decode("utf-8")
                    manifest = json.loads(content)
        except Exception as exc:
            logger.debug("Manifest missing for %s: %s", name, exc)

        local_path = Path(config.presences_dir) / name
        installed = local_path.exists() and local_path.is_dir()
        if manifest:
            manifest["installed"] = installed
        presences.append(
            {
                "name": name,
                "manifest": manifest,
            }
        )

    save_cache(presences)
    return presences


def get_remote_tree(path: str) -> Dict[str, dict]:
    """
    Recursive listing of remote files.
    return: dict[relative_path] = { sha, download_url }
    """
    url = f"https://api.github.com/repos/{config.github_owner}/{config.github_repo}/contents/{path}"
    resp = requests.get(url, headers=github_headers(), timeout=15)
    resp.raise_for_status()

    files = {}

    for item in resp.json():
        if item["type"] == "file":
            rel = Path(item["path"]).relative_to(path)
            files[str(rel)] = {
                "sha": item["sha"],
                "download_url": item["download_url"],
            }
        elif item["type"] == "dir":
            files.update(get_remote_tree(item["path"]))

    return files


def download_file(url: str, dest: Path):
    """
    Download a file from URL to destination path.
    """
    try:
        dest.parent.mkdir(parents=True, exist_ok=True)
        resp = requests.get(url, timeout=20)
        resp.raise_for_status()
        with open(dest, "wb") as file_:
            file_.write(resp.content)
    except Exception as exc:
        logger.error("Failed to download %s: %s", url, exc)
        raise exc


def save_meta(local_dir: Path, meta: dict):
    """
    Save meta information to local .meta.json file.
    """
    logger.debug("Saving meta information")
    try:
        with open(local_dir / config.meta_filename, "w", encoding="utf-8") as file_:
            json.dump(meta, file_, indent=2)
    except Exception as exc:
        logger.error("Failed to save meta information: %s", exc)


def load_meta(local_dir: Path) -> dict:
    """
    Load meta information from local .meta.json file.
    """
    path = local_dir / config.meta_filename
    if path.exists():
        try:
            with open(path, "r", encoding="utf-8") as file_:
                return json.load(file_)
        except Exception as exc:
            logger.error("Failed to load meta information: %s", exc)
    return {}


def install(remote_folder: str, local_folder: str) -> Tuple[bool, str]:
    """
    Install presence from remote to local folder.
    """
    logger.info("Installing %s", remote_folder)
    remote_dir = "presences/" + remote_folder
    local_dir = Path(local_folder)

    if local_dir.exists():
        return False, "Local folder already exists"

    try:
        files = get_remote_tree(remote_dir)
        meta = {}

        for rel, info in files.items():
            download_file(info["download_url"], local_dir / rel)
            meta[rel] = info["sha"]

        save_meta(local_dir, meta)
        return True, "Installed successfully"

    except Exception as exc:
        return False, f"Install failed: {exc}"


def uninstall(local_folder: str) -> Tuple[bool, str]:
    """
    Uninstall presence by removing local folder.
    """
    local_dir = Path(local_folder)

    if not local_dir.exists():
        return False, "Nothing to uninstall"

    try:
        shutil.rmtree(local_dir)
        return True, "Uninstalled"

    except Exception as exc:
        return False, f"Uninstall failed: {exc}"


def sync(remote_folder: str, local_folder: str) -> Tuple[bool, str]:
    """
    Sync presence between remote and local folder.
    """
    local_dir = Path(local_folder)
    local_dir.mkdir(parents=True, exist_ok=True)
    logger.info("Syncing %s", remote_folder)
    logger.debug("For local: %s", local_dir.resolve())
    try:
        remote_files = get_remote_tree(remote_folder)
        local_meta = load_meta(local_dir)
        new_meta = {}

        # update / download
        for rel, info in remote_files.items():
            if local_meta.get(rel) != info["sha"]:
                download_file(info["download_url"], local_dir / rel)
            new_meta[rel] = info["sha"]

        # remove obsolete
        for rel in local_meta:
            if rel not in new_meta:
                f = local_dir / rel
                if f.exists():
                    f.unlink()

        save_meta(local_dir, new_meta)
        return True, "Sync complete"

    except Exception as exc:
        return False, f"Sync failed: {exc}"
