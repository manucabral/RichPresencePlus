from main import __title__, __version__, __description__
from cx_Freeze import setup, Executable

build_exe_options = {
    "excludes": [
        "tkinter",
        "unittest",
        "pypresence",
        "rpp.Manager",
        "rpp.Runtime",
        "rpp.Presence",
        "rpp.Browser",
        "rpp.ClientRPC",
        "rpp.load_env",
        "rpp.get_available_presences",
    ],
    "packages": ["websocket", "presences"],
}

setup(
    name=__title__,
    version=__version__,
    description=__description__,
    options={"build_exe": build_exe_options},
    executables=[Executable("main.py", base="Console", target_name="rpp_dev")],
)
