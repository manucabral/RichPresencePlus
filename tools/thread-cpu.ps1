# Which threads of the running application use the processor, by the name Rust gave them.
#
#   powershell -File tools/thread-cpu.ps1 [-Seconds 30]
#
# It only reads counters. Close the window first to see what the application costs in the
# tray; with it open the web view's work is a different process and does not show here.
param([int]$Seconds = 30)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class ThreadNames {
    [DllImport("kernel32.dll")] static extern IntPtr OpenThread(uint access, bool inherit, uint id);
    [DllImport("kernel32.dll")] static extern int GetThreadDescription(IntPtr thread, out IntPtr name);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr memory);
    public static string Of(uint id) {
        IntPtr thread = OpenThread(0x0800, false, id); // THREAD_QUERY_LIMITED_INFORMATION
        if (thread == IntPtr.Zero) return "?";
        string name = "(unnamed)";
        IntPtr text;
        if (GetThreadDescription(thread, out text) == 0 && text != IntPtr.Zero) {
            string found = Marshal.PtrToStringUni(text);
            if (!String.IsNullOrEmpty(found)) name = found;
            LocalFree(text);
        }
        CloseHandle(thread);
        return name;
    }
}
"@

$app = Get-Process rpp-desktop -ErrorAction Stop | Select-Object -First 1
function Read-Threads($process) {
    $times = @{}
    foreach ($thread in (Get-Process -Id $process.Id).Threads) {
        $times[$thread.Id] = $thread.TotalProcessorTime.TotalMilliseconds
    }
    $times
}
$first = Read-Threads $app
Start-Sleep -Seconds $Seconds
$second = Read-Threads $app
$total = 0.0
$rows = foreach ($id in $second.Keys) {
    $used = $second[$id] - $(if ($first.ContainsKey($id)) { $first[$id] } else { 0 })
    $total += $used
    [pscustomobject]@{ Thread = [ThreadNames]::Of([uint32]$id); Id = $id; Percent = [math]::Round($used / ($Seconds * 1000) * 100, 3) }
}
"threads: $($second.Count), total $([math]::Round($total / ($Seconds * 1000) * 100, 3))% of a core over $Seconds s"
$rows | Where-Object { $_.Percent -gt 0 } | Sort-Object Percent -Descending | Select-Object -First 10 | Format-Table -AutoSize
