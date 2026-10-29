# Resizes the Codestamp test window so its content area is -Width x -Height, or -Maximize.
# Only the window of the test copy (found by its path) is touched.
param([int]$Width = 0, [int]$Height = 0, [switch]$Maximize)

$source = @'
using System; using System.Runtime.InteropServices;
public static class AppWindow {
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int command);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int width, int height, bool repaint);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect rect);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out Rect rect);
}
'@
Add-Type -TypeDefinition $source

. "$PSScriptRoot\test-process.ps1"
$process = Get-TestProcess | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $process) { throw 'No Codestamp window found' }
$handle = $process.MainWindowHandle
$SW_RESTORE = 9
$SW_MAXIMIZE = 3
if ($Maximize) {
  [AppWindow]::ShowWindow($handle, $SW_MAXIMIZE) | Out-Null
  'maximized'
  return
}
if ($Width -le 0 -or $Height -le 0) { throw 'Give -Width and -Height, or -Maximize' }
[AppWindow]::ShowWindow($handle, $SW_RESTORE) | Out-Null
Start-Sleep -Milliseconds 300
$window = New-Object AppWindow+Rect
$client = New-Object AppWindow+Rect
[AppWindow]::GetWindowRect($handle, [ref]$window) | Out-Null
[AppWindow]::GetClientRect($handle, [ref]$client) | Out-Null
# The frame and title bar sit outside the content area.
$frameWidth = ($window.Right - $window.Left) - ($client.Right - $client.Left)
$frameHeight = ($window.Bottom - $window.Top) - ($client.Bottom - $client.Top)
[AppWindow]::MoveWindow($handle, 40, 40, $Width + $frameWidth, $Height + $frameHeight, $true) | Out-Null
"content $Width x $Height"
