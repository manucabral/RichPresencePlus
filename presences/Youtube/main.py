def main():
    # Checks if the presence is starting using the last_video global variable.
    # Note: last_video is defined in the presence metadata.
    global last_video
    if last_video == "unknown":
        presence_update(state="Starting...")
        last_video = None
        return

    # Get the current tab
    tab = runtime.current_tab

    # if the tab is not in YouTube, then the user is not watching a video.
    if not "www.youtube.com" in tab.url:
        presence_update(state="Not watching.")
        return

    # Get the media session
    media_session = tab.media_session()

    # Check if the media session is active
    active = media_session and media_session.state != "none"

    # If the media session is not active that means the user is browsing in YouTube.
    if not active:
        presence_update(state="Browsing...")
        return

    # Get the media session data
    paused = media_session.state == "paused"
    channel = tab.exec("document.querySelector('#text > a').href")
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
        start = time.time()
        last_video = tab.url

    # Update the presence.
    presence_update(
        state=state,
        details=details,
        large_image=large_image,
        small_image=small_image,
        buttons=buttons,
        start=start,
    )


# Calls the main function.
main()

# Updates the presence every 5 seconds.
time.sleep(5)
