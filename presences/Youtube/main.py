def on_studio(url: str):
    """
    Handles the YouTube Studio sections.
    """
    details = "Dashboard"

    if "analytics" in url:
        details = "Analytics"
    elif "comments" in url:
        details = "Viewing comments"

    return presence_update(state="Studio", details=details)


def main():
    # Checks if the presence is starting using the last_video global variable.
    # Note: last_video and start_time are global variables defined in the metadata.yml file.
    global last_video, start_time
    if last_video == "unknown":
        last_video = None
        start_time = time.time()
        return presence_update(state="Starting...", start=start_time)

    # Get the current tab
    tab = runtime.current_tab

    # if the tab is not in YouTube, then the user is not watching a video.
    if not "youtube.com" in tab.url:
        return presence_update(state="Not watching.")

    # if user is in YouTube Studio, then call the on_studio function which handles the different sections.
    if "studio.youtube.com" in tab.url:
        return on_studio(tab.url)

    if "shorts" in tab.url:
        return presence_update(state="Watching Shorts...")

    # Get the media session
    media_session = tab.media_session()

    # Check if the media session is active
    active = media_session and media_session.state != "none"

    # If the media session is not active that means the user is browsing in YouTube.
    if not active:
        return presence_update(state="Browsing...")

    # Get the media session data
    paused = media_session.state == "paused"
    channel = tab.exec(
        "document.querySelector('#top-row ytd-video-owner-renderer > a').href"
    )
    state = media_session.title
    details = media_session.artist
    large_image = media_session.artwork
    small_image = "pause" if paused else "play"
    buttons = [
        {"label": "Watch", "url": tab.url},
        {"label": "Channel", "url": channel},
    ]

    # if the last video is not the current video, then the user is watching a new video.
    if tab.url != last_video:
        log("New video detected.")
        start_time = time.time()
        last_video = tab.url

    # Update the presence.
    return presence_update(
        state=state,
        details=details,
        large_image=large_image,
        small_image=small_image,
        buttons=buttons,
        start=start_time,
    )


# Calls the main function.
main()

# Updates the presence every 5 seconds.
time.sleep(5)
