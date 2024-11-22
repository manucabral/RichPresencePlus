# Arc Browser Compatibility with Microsoft Store

If you have Arc browser installed from the Microsoft Store and the application cannot connect or is not working directly, you can try the following:

1. Test it in the prerelease v0.1.0.
2. Locate the installation path for Arc, for example:
   `C:\Users\alesi\AppData\Local\Microsoft\WindowsApps\TheBrowserCompany.Arc_ttt1ap7aakyb4\Arc.exe`
3. Open Notepad and write the following (replace the path with the one you copied):
   
   `BROWSER_PATH=your_copied_path_here`
5. Save the file as **“.env”** and place it in the application folder.
6. Open Rich Presence Plus and press the "open browser" button.
7. Open a tab at `http://localhost:9222/json`.
8. Press the "connect" button.

If this doesn't work, don't hesitate to open an issue, or for better communication, join the Discord.



https://github.com/user-attachments/assets/b402a77e-6fac-4837-bc7a-32428c0c96ae

