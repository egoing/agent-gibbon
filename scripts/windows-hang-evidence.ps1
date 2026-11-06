# Read-only, content-free evidence. Does not suspend, terminate or activate windows.
[CmdletBinding()]
param([string]$OutputDirectory = (Join-Path $env:TEMP ('gibbon-hang-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))))
$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$info = (& node (Join-Path $PSScriptRoot 'dev.mjs') info | ConvertFrom-Json)
if ($LASTEXITCODE -ne 0 -or $info.root -ne $taskRoot) { throw 'Cannot verify worktree identity' }
$targets = @(Get-CimInstance Win32_Process -Filter "Name = 'agent-gibbon.exe'" | Where-Object { $_.ExecutablePath -eq $info.binary })
if ($targets.Count -ne 1) { throw "Expected one process at the worktree binary; found $($targets.Count)" }
$targetProcessId = [int]$targets[0].ProcessId
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class GibbonHangProbe {
    public delegate bool EnumCallback(IntPtr hwnd, IntPtr data);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumCallback cb, IntPtr data);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", SetLastError=true)] static extern IntPtr SendMessageTimeout(IntPtr hwnd, uint msg, IntPtr w, IntPtr l, uint flags, uint timeout, out IntPtr result);
    public static object[] Probe(uint pid) {
        var rows = new List<object>();
        EnumWindows((hwnd, unused) => {
            uint owner; uint thread = GetWindowThreadProcessId(hwnd, out owner);
            if (owner != pid) return true;
            IntPtr result;
            // WM_NULL, SMTO_BLOCK | SMTO_ABORTIFHUNG. Never send keys or activate.
            bool responds = SendMessageTimeout(hwnd, 0, IntPtr.Zero, IntPtr.Zero, 3, 500, out result) != IntPtr.Zero;
            rows.Add(new { hwnd = hwnd.ToInt64(), thread_id = thread, visible = IsWindowVisible(hwnd), responds });
            return true;
        }, IntPtr.Zero);
        return rows.ToArray();
    }
}
'@
$before = Get-Process -Id $targetProcessId
$cpuBefore = $before.TotalProcessorTime.TotalMilliseconds
$sampleStart = [DateTime]::UtcNow
$windows = [GibbonHangProbe]::Probe($targetProcessId)
Start-Sleep -Milliseconds 1000
$after = Get-Process -Id $targetProcessId
if ($after.Path -ne $info.binary -or $after.StartTime -ne $before.StartTime) { throw 'Process identity changed during sampling' }
$threads = @($after.Threads | ForEach-Object {
    $state = $_.ThreadState.ToString()
    $reason = $null
    if ($state -eq 'Wait') { try { $reason = $_.WaitReason.ToString() } catch { $reason = 'Unavailable' } }
    [pscustomobject]@{ thread_id = $_.Id; state = $state; wait_reason = $reason }
})
$evidence = [ordered]@{
    captured_at_utc = [DateTime]::UtcNow.ToString('o')
    root = $info.root; instance = $info.id; binary = $info.binary
    pid = $targetProcessId; process_started_at_utc = $after.StartTime.ToUniversalTime().ToString('o')
    responding = $after.Responding
    sample_wall_ms = ([DateTime]::UtcNow - $sampleStart).TotalMilliseconds
    sample_cpu_ms = $after.TotalProcessorTime.TotalMilliseconds - $cpuBefore
    windows = $windows; threads = $threads
}
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$outputPath = Join-Path (Resolve-Path $OutputDirectory).Path 'process.json'
$evidence | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 -LiteralPath $outputPath
Write-Output $outputPath
