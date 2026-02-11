"""
GitHub synchronization functions.
"""

import json
import time
import base64
import shutil
import hashlib
import re
from pathlib import Path
from typing import Dict, List, Tuple, Optional
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


def calculate_file_sha(file_path: Path) -> str:
    """
    Calculate SHA-1 hash of a file (Git blob format).
    Matches the SHA format used by GitHub API.
    """
    try:
        if not file_path.exists():
            return ""
        with open(file_path, "rb") as f:
            content = f.read()
        blob = b"blob " + str(len(content)).encode() + b"\0" + content
        return hashlib.sha1(blob).hexdigest()
    except Exception as exc:
        logger.debug("Failed to calculate SHA for %s: %s", file_path, exc)
        return ""


def parse_version(version: str) -> Optional[Tuple[int, int, int, str, int]]:
    """
    Parse semantic version with prerelease suffixes.

    Examples:
        "0.1.1-beta" -> (0, 1, 1, "beta", 0)
        "1.2.3-rc2" -> (1, 2, 3, "rc", 2)
        "2.0.0" -> (2, 0, 0, "", 0)

    Returns:
        Tuple of (major, minor, patch, suffix, suffix_number) or None if invalid
    """
    pattern = r"^(\d+)\.(\d+)\.(\d+)(?:-([a-z]+)(\d*))?$"
    match = re.match(pattern, version.strip())

    if not match:
        return None

    major = int(match.group(1))
    minor = int(match.group(2))
    patch = int(match.group(3))
    suffix = match.group(4) or ""
    suffix_num = int(match.group(5)) if match.group(5) else 0

    return (major, minor, patch, suffix, suffix_num)


def compare_versions(current: str, remote: str) -> Optional[int]:
    """
    Compare two semantic versions.
    """
    current_parsed = parse_version(current)
    remote_parsed = parse_version(remote)

    if not current_parsed or not remote_parsed:
        logger.warning("Invalid version format: current=%s, remote=%s", current, remote)
        return None

    curr_major, curr_minor, curr_patch, curr_suffix, curr_num = current_parsed
    rem_major, rem_minor, rem_patch, rem_suffix, rem_num = remote_parsed

    # major.minor.patch
    if (curr_major, curr_minor, curr_patch) < (rem_major, rem_minor, rem_patch):
        return -1
    if (curr_major, curr_minor, curr_patch) > (rem_major, rem_minor, rem_patch):
        return 1

    # compare suffixes
    suffix_priority = {"alpha": 1, "beta": 2, "rc": 3, "": 4}

    curr_priority = suffix_priority.get(curr_suffix, 0)
    rem_priority = suffix_priority.get(rem_suffix, 0)

    if curr_priority < rem_priority:
        return -1
    if curr_priority > rem_priority:
        return 1

    # compare suffix numbers
    if curr_num < rem_num:
        return -1
    if curr_num > rem_num:
        return 1

    return 0


def get_remote_version() -> Optional[str]:
    """
    Fetch the remote version from GitHub repository.
    """
    try:
        url = (
            f"https://api.github.com/repos/{config.github_owner}/"
            f"{config.github_repo}/contents/src/constants.py"
        )

        resp = requests.get(url, headers=github_headers(), timeout=10)
        resp.raise_for_status()

        data = resp.json()
        if data.get("encoding") == "base64":
            content = base64.b64decode(data["content"]).decode("utf-8")
            match = re.search(r'version:\s*str\s*=\s*["\']([^"\']+)["\']', content)
            if match:
                version = match.group(1)
                logger.debug("Remote version found: %s", version)
                return version

        logger.warning("Could not extract version from remote constants.py")
        return None

    except Exception as exc:
        logger.error("Failed to fetch remote version: %s", exc)
        return None


def check_version_update() -> Dict:
    """
    Check if current version is outdated compared to remote.
    """
    current_version = config.version
    remote_version = get_remote_version()

    if not remote_version:
        return {
            "current": current_version,
            "remote": None,
            "outdated": False,
            "comparison": None,
            "message": "Could not fetch remote version",
        }

    comparison = compare_versions(current_version, remote_version)

    if comparison is None:
        return {
            "current": current_version,
            "remote": remote_version,
            "outdated": False,
            "comparison": None,
            "message": "Invalid version format",
        }

    if comparison < 0:
        return {
            "current": current_version,
            "remote": remote_version,
            "outdated": True,
            "comparison": comparison,
            "message": f"Update available: {remote_version}",
        }
    if comparison > 0:
        return {
            "current": current_version,
            "remote": remote_version,
            "outdated": False,
            "comparison": comparison,
            "message": "Ahead of remote version",
        }

    return {
        "current": current_version,
        "remote": remote_version,
        "outdated": False,
        "comparison": comparison,
        "message": "Up to date",
    }


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
    Check if local presence is synchronized with remote.
    """
    local_dir = Path(local_folder)
    local_dir.mkdir(parents=True, exist_ok=True)
    logger.info("Checking sync status for %s", remote_folder)
    logger.debug("Local path: %s", local_dir.resolve())

    try:
        remote_files = get_remote_tree(remote_folder)
        local_meta = load_meta(local_dir)

        remote_modified = []
        missing_files = []
        obsolete_files = []
        locally_modified = []

        # Check for remote changes (modified or new files)
        for rel, info in remote_files.items():
            if rel not in local_meta:
                missing_files.append(rel)
            elif local_meta.get(rel) != info["sha"]:
                remote_modified.append(rel)

        # check for obsolete local files (not in remote)
        for rel in local_meta:
            if rel not in remote_files:
                obsolete_files.append(rel)

        # check for local modifications
        for rel, expected_sha in local_meta.items():
            local_file = local_dir / rel
            if local_file.exists():
                actual_sha = calculate_file_sha(local_file)
                if actual_sha and actual_sha != expected_sha:
                    locally_modified.append(rel)
                    logger.debug(
                        "Local modification detected in %s: expected %s, got %s",
                        rel,
                        expected_sha[:8],
                        actual_sha[:8],
                    )

        if remote_modified or missing_files or obsolete_files or locally_modified:
            parts = []
            if remote_modified:
                parts.append(f"{len(remote_modified)} remote change/s")
            if missing_files:
                parts.append(f"{len(missing_files)} missing")
            if obsolete_files:
                parts.append(f"{len(obsolete_files)} obsolete")
            if locally_modified:
                parts.append(f"{len(locally_modified)} local modification/s")

            message = "Out of sync: " + ", ".join(parts)
            logger.warning("Presence %s is out of sync: %s", remote_folder, message)
            return False, message

        logger.debug("Presence %s is up to date", remote_folder)
        return True, "Already up to date"

    except Exception as exc:
        logger.error("Failed to check sync status for %s: %s", remote_folder, exc)
        return False, f"Sync check failed: {exc}"


def force_sync(remote_folder: str, local_folder: str) -> Tuple[bool, str]:
    """
    Force synchronization of local presence with remote repository.
    Downloads updated files and removes obsolete ones.
    """
    local_dir = Path(local_folder)
    local_dir.mkdir(parents=True, exist_ok=True)
    logger.info("Force syncing %s", remote_folder)
    logger.debug("Local path: %s", local_dir.resolve())

    try:
        remote_files = get_remote_tree(remote_folder)
        local_meta = load_meta(local_dir)
        new_meta = {}

        updated_files = []
        removed_files = []

        for rel, info in remote_files.items():
            if local_meta.get(rel) != info["sha"]:
                logger.debug("Downloading updated file: %s", rel)
                download_file(info["download_url"], local_dir / rel)
                updated_files.append(rel)
            new_meta[rel] = info["sha"]

        # Remove obsolete files
        for rel in local_meta:
            if rel not in new_meta:
                f = local_dir / rel
                if f.exists():
                    logger.debug("Removing obsolete file: %s", rel)
                    f.unlink()
                    removed_files.append(rel)

        save_meta(local_dir, new_meta)

        if updated_files or removed_files:
            parts = []
            if updated_files:
                parts.append(f"{len(updated_files)} file(s) updated")
            if removed_files:
                parts.append(f"{len(removed_files)} file(s) removed")
            message = "Sync complete: " + ", ".join(parts)
            logger.info("Force sync completed for %s: %s", remote_folder, message)
        else:
            message = "Already up to date"
            logger.debug("No changes needed for %s", remote_folder)

        return True, message

    except Exception as exc:
        logger.error("Force sync failed for %s: %s", remote_folder, exc)
        return False, f"Sync failed: {exc}"
