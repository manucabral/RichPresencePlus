"""
Utilities for the project
"""

import re
from typing import Any, Optional

URL_PATTERN = re.compile(
    r"^https?://"
    r"(?:(?:[A-Z0-9](?:[A-Z0-9-]{0,61}[A-Z0-9])?\.)+[A-Z]{2,6}\.?|"  # domain
    r"localhost|"
    r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})"  # ip
    r"(?::\d+)?"  # optional port
    r"(?:/?|[/?]\S+)$",
    re.IGNORECASE,
)


def safe_close_page(page: Any) -> None:
    """
    Safely close a page if it is not None.
    """
    if page is not None:
        try:
            page.close()
        except Exception:
            pass


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


def resolve_callable(module, requested_name: Optional[str]):
    """
    Try to resolve callable in module. Try requested_name (if provided),
    then common fallbacks. Returns (callable_obj, resolved_name) or (None, None).
    """
    # prefer explicit requested_name if given, otherwise try defaults including "execute"
    candidates = []
    if requested_name:
        candidates.append(requested_name)
    candidates += ["execute", "main"]
    for name in candidates:
        if hasattr(module, name):
            obj = getattr(module, name)
            if callable(obj):
                return obj, name
    return None, None


def is_valid_url(url: str) -> bool:
    """Check if a URL is valid."""
    if not url or not isinstance(url, str):
        return False
    return URL_PATTERN.match(url.strip()) is not None
