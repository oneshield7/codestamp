# Brings the minimized test window back; the user's own Codestamp is never touched.
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Restore {
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
}
"@
. "$PSScriptRoot\test-window.ps1"
[void][Restore]::ShowWindow((Get-TestWindow), 9)
