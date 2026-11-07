# Writes the memory of the running application once a minute, to find leaks.
#
#   powershell -File tools/soak.ps1 -Out soak.csv -Hours 3
#
# It only reads process counters. Each line is one sample; `pid` changes when the
# application was started again, and `webview_mb` is 0 while the window is closed.
param(
    [Parameter(Mandatory = $true)][string]$Out,
    [double]$Hours = 3,
    [int]$EverySeconds = 60
)

function Descendants($all, $id) {
    foreach ($child in $all | Where-Object { $_.ParentProcessId -eq $id }) {
        $child
        Descendants $all $child.ProcessId
    }
}

"time,pid,app_private_mb,app_working_mb,handles,threads,webview_mb" | Set-Content -Path $Out -Encoding ascii
$end = (Get-Date).AddHours($Hours)
while ((Get-Date) -lt $end) {
    $app = Get-Process rpp-desktop -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($app) {
        $all = Get-CimInstance Win32_Process
        $webview = 0
        foreach ($item in @(Descendants $all $app.Id)) {
            $process = Get-Process -Id $item.ProcessId -ErrorAction SilentlyContinue
            if ($process) { $webview += $process.PrivateMemorySize64 }
        }
        # Whatever the language of Windows, a decimal point: the commas separate columns.
        $point = [Globalization.CultureInfo]::InvariantCulture
        $line = [string]::Format($point, "{0},{1},{2:F1},{3:F1},{4},{5},{6:F1}", (Get-Date -Format "s"), $app.Id,
            ($app.PrivateMemorySize64 / 1MB), ($app.WorkingSet64 / 1MB), $app.HandleCount, $app.Threads.Count, ($webview / 1MB))
        $line | Add-Content -Path $Out -Encoding ascii
    }
    Start-Sleep -Seconds $EverySeconds
}
