# Prints the test window's rectangle and the bounds of the monitor it is on, as JSON.
# Only the window of the test copy (found by its path) is read.
. "$PSScriptRoot\test-window.ps1"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class WindowPlace {
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct MonitorInfo { public int Size; public Rect Monitor; public Rect Work; public uint Flags; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect rect);
  [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr h, uint flags);
  [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr m, ref MonitorInfo info);
}
"@
$handle = Get-TestWindow
$rect = New-Object WindowPlace+Rect
[WindowPlace]::GetWindowRect($handle, [ref]$rect) | Out-Null
$info = New-Object WindowPlace+MonitorInfo
$info.Size = [System.Runtime.InteropServices.Marshal]::SizeOf($info)
[WindowPlace]::GetMonitorInfo([WindowPlace]::MonitorFromWindow($handle, 2), [ref]$info) | Out-Null
@{
  window = @($rect.Left, $rect.Top, $rect.Right, $rect.Bottom)
  monitor = @($info.Monitor.Left, $info.Monitor.Top, $info.Monitor.Right, $info.Monitor.Bottom)
  work = @($info.Work.Left, $info.Work.Top, $info.Work.Right, $info.Work.Bottom)
} | ConvertTo-Json -Compress
