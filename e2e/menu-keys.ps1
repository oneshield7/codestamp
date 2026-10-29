# While the native menu is open: optionally saves a screenshot of the Codestamp window area, then
# presses the given keys (SendKeys syntax) to pick an item.
param([string]$Keys, [string]$Shot = '')
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Area {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
}
"@
. "$PSScriptRoot\test-window.ps1"
if ($Shot -ne '') {
  $rect = New-Object Area+RECT
  [void][Area]::GetWindowRect((Get-TestWindow), [ref]$rect)
  $width = [Math]::Min(900, $rect.Right - $rect.Left)
  $height = [Math]::Min(520, $rect.Bottom - $rect.Top)
  $bitmap = New-Object System.Drawing.Bitmap $width, $height
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
  $bitmap.Save($Shot, [System.Drawing.Imaging.ImageFormat]::Png)
  $graphics.Dispose()
  $bitmap.Dispose()
}
[System.Windows.Forms.SendKeys]::SendWait($Keys)
