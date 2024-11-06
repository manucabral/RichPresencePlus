# Custom Browser Path Configuration

If the application is unable to find the browser path or you want to change it to a different directory, you can set a custom path by creating a `.env` file in the application's directory.

### Steps:

1. Create a `.env` file in the same directory as the application.
2. Inside the `.env` file, add the following line:
```
BROWSER_PATH=<path_to_your_browser_executable>
```
3. Replace `<path_to_your_browser_executable>` with the full path to your browser's `.exe` file.

Example:
```
BROWSER_PATH=C:\Program Files\WindowsApps\Arc.exe
```
This will allow the application to use the browser located in the specified path.
