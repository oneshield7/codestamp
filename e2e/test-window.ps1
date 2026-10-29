# Dot-sourced by the window helpers. Get-TestWindow returns the main window of the test copy only
# (found by its path), never the user's own Codestamp, which has the same class and title.
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class TestWindow {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  public static IntPtr Find(uint pid) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => {
      uint owner;
      GetWindowThreadProcessId(h, out owner);
      if (owner != pid) return true;
      var cls = new StringBuilder(64); GetClassName(h, cls, 64);
      var title = new StringBuilder(64); GetWindowText(h, title, 64);
      if (cls.ToString() == "Tauri Window" && title.ToString() == "Codestamp") { found = h; return false; }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@
. "$PSScriptRoot\test-process.ps1"
function Get-TestWindow {
  $process = Get-TestProcess | Select-Object -First 1
  if (-not $process) { throw 'The test copy is not running' }
  $handle = [TestWindow]::Find([uint32]$process.Id)
  if ($handle -eq [IntPtr]::Zero) { throw 'No window of the test copy found' }
  $handle
}
