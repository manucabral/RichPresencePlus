# Unblock all files and run Rich Presence Plus

Write-Host "Unblocking files..." -ForegroundColor Cyan
Get-ChildItem .\ -Recurse | Unblock-File

Write-Host "Starting Rich Presence Plus..." -ForegroundColor Green
& ".\Rich Presence Plus.exe"
