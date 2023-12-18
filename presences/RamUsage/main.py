import psutil


def get_ram_usage():
    """Get the RAM usage of the system."""
    return psutil.virtual_memory().percent


percent = str(get_ram_usage()) + "%"

presence_update(
    details="RAM Usage: " + percent,
    large_image="ram",
    large_text="RAM Usage: " + percent,
    start=time.time(),
)

# update each 2 seconds
time.sleep(2)
