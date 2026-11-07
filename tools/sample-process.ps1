# One line of JSON: memory by process type and processor seconds, for the app in APP_PID.
#
# Used by tools/try-memory.mjs. Memory is the private bytes of each process, and the web
# view's processes are told apart by the --type of their command line.
$id = [int]$env:APP_PID
$all = Get-CimInstance Win32_Process
function Kids($parent) {
    foreach ($child in $all | Where-Object { $_.ParentProcessId -eq $parent }) {
        $child
        Kids $child.ProcessId
    }
}
$app = Get-Process -Id $id -ErrorAction SilentlyContinue
if (-not $app) { '{"gone":true}'; exit }
$r = [ordered]@{
    app = [math]::Round($app.PrivateMemorySize64 / 1MB, 1)
    app_ws = [math]::Round($app.WorkingSet64 / 1MB, 1)
    handles = $app.HandleCount
    threads = $app.Threads.Count
    browser = 0.0
    renderer = 0.0
    'gpu-process' = 0.0
    utility = 0.0
    other = 0.0
    cpu = [math]::Round($app.TotalProcessorTime.TotalSeconds, 2)
    cpu_app = [math]::Round($app.TotalProcessorTime.TotalSeconds, 2)
    cpu_renderer = 0.0
    cpu_other = 0.0
}
foreach ($item in @(Kids $id)) {
    $p = Get-Process -Id $item.ProcessId -ErrorAction SilentlyContinue
    if (-not $p) { continue }
    $type = if ($item.CommandLine -match '--type=([\w-]+)') { $Matches[1] } else { 'browser' }
    if (-not $r.Contains($type)) { $type = 'other' }
    $r[$type] = [math]::Round($r[$type] + $p.PrivateMemorySize64 / 1MB, 1)
    $seconds = $p.TotalProcessorTime.TotalSeconds
    $r.cpu = [math]::Round($r.cpu + $seconds, 2)
    $slot = if ($type -eq 'renderer') { 'cpu_renderer' } else { 'cpu_other' }
    $r[$slot] = [math]::Round($r[$slot] + $seconds, 2)
}
$r | ConvertTo-Json -Compress
