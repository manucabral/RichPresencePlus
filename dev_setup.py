"""
Used to build the development script.
"""
from cx_Freeze import setup, Executable


setup(
    name="Rich Presence Plus Development Script",
    version="0.0.1",
    description="Used to test Rich Presence Plus presences.",
    executables=[Executable("dev/main.py", target_name="rpp_dev.exe", base="Console")],
    options={
        "build_exe": {
            "excludes": [
                "tkinter",
                "unittest",
                "sqlite3",
                "ctypes",
            ]
        }
    },
)
