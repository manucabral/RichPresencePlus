import rpp
from cx_Freeze import setup, Executable

# for now Windows only
base = 'Win32GUI'

build_exe_options = {
    "excludes": [
        "unittest",
        "rpp.Manager",
        "rpp.Runtime",
        "rpp.Presence",
        "rpp.Browser",
    ],
    "packages": ["websocket", "presences", "pypresence"],
}

executables = [
    Executable('app.py', base=base, icon='logo.ico', target_name=rpp.__title__)
]

setup(name=rpp.__title__,
        version=rpp.__version__,
        description=rpp.__description__,
        options={"build_exe": build_exe_options},
        executables=executables)