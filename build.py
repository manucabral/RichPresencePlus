"""
Build script for RPP using Nuitka.
Compatible with pythonnet on Python 3.12+
"""

import sys
import subprocess
from src.logger import logger


def build():
    """Build the application with Nuitka."""

    nuitka_args = [
        sys.executable,
        "-m",
        "nuitka",
        "--standalone",
        "--windows-console-mode=disable",
        "--output-dir=dist_build",
        "--output-filename=Rich Presence Plus",
        # metadata
        "--windows-icon-from-ico=assets/logo.ico",
        "--windows-product-name=Rich Presence Plus",
        "--windows-file-description=Discord Rich Presence Manager for Desktop and Web presences",
        "--windows-company-name=Manuel Cabral",
        "--windows-file-version=0.1.1",
        "--windows-product-version=0.1.1",
        # data directories
        "--include-data-dir=presences=presences",
        "--include-data-dir=frontend/dist=dist",
        # modules
        "--include-module=webview",
        "--include-module=requests",
        "--include-module=websockets",
        "--include-module=pythonnet",
        # exclude testing frameworks
        "--nofollow-import-to=unittest",
        "--nofollow-import-to=unittest.mock",
        "--nofollow-import-to=doctest",
        "--nofollow-import-to=pytest",
        "--nofollow-import-to=test",
        # exclude webview platforms
        "--nofollow-import-to=webview.platforms.cocoa",
        "--nofollow-import-to=webview.platforms.gtk",
        "--nofollow-import-to=webview.platforms.qt",
        "--nofollow-import-to=webview.platforms.android",
        "--nofollow-import-to=webview.platforms.linux",
        # exclude GUI frameworks
        "--nofollow-import-to=tkinter",
        "--nofollow-import-to=_tkinter",
        "--nofollow-import-to=PyQt5",
        "--nofollow-import-to=PyQt6",
        "--nofollow-import-to=PySide2",
        "--nofollow-import-to=PySide6",
        "--nofollow-import-to=gi",
        # exclude build/documentation tools
        "--nofollow-import-to=pydoc",
        "--nofollow-import-to=distutils",
        "--nofollow-import-to=setuptools",
        # optimization flags
        "--remove-output",
        "--python-flag=no_docstrings",
        "--python-flag=no_asserts",
        "--msvc=latest",
        "main.py",
    ]

    logger.info("Building with Nuitka...")
    logger.info(" ".join(nuitka_args))

    try:
        subprocess.run(nuitka_args, check=True)
        logger.info("Build completed successfully!")
        logger.info("Output: dist_build/main.dist/")
    except subprocess.CalledProcessError as e:
        logger.error(f"Build failed with error code {e.returncode}")
        sys.exit(1)


if __name__ == "__main__":
    build()
