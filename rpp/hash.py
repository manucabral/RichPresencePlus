import hashlib
import os


def dir_md5(directory: str) -> str:
    """
    Get the MD5 hash of a directory.

    Args:
        directory (str): Directory to get the MD5 hash.

    Returns:
        str: MD5 hash of the directory.
    """
    hash_md5 = hashlib.md5()
    for root, _, files in os.walk(directory):
        for file in files:
            if file.startswith("__") or file.endswith(".pyc"):
                continue
            if file == "__pycache__":
                continue
            with open(os.path.join(root, file), "rb") as f:
                for chunk in iter(lambda: f.read(4096), b""):
                    hash_md5.update(chunk)
    return hash_md5.hexdigest()
