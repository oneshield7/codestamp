# Asks Windows what each edge of the Codestamp window does (WM_NCHITTEST): resize borders must
# answer HTLEFT/HTRIGHT/HTTOP/HTBOTTOM, the inside HTCLIENT. Read-only.
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class HitTest {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr hWnd, int index);
}
"@
# Found by title within the test process: once minimized, MainWindowHandle can point at a helper
# window instead, and the user's own Codestamp has the same title.
. "$PSScriptRoot\test-window.ps1"
$hwnd = Get-TestWindow
$rect = New-Object HitTest+RECT
[void][HitTest]::GetWindowRect($hwnd, [ref]$rect)
$names = @{ 0 = 'NOWHERE'; 1 = 'CLIENT'; 2 = 'CAPTION'; 10 = 'LEFT'; 11 = 'RIGHT'; 12 = 'TOP'; 13 = 'TOPLEFT'; 14 = 'TOPRIGHT'; 15 = 'BOTTOM'; 16 = 'BOTTOMLEFT'; 17 = 'BOTTOMRIGHT' }
function Probe([int]$x, [int]$y) {
  $l = [IntPtr](($y -shl 16) -bor ($x -band 0xFFFF))
  $code = [int][HitTest]::SendMessage($hwnd, 0x84, [IntPtr]::Zero, $l)
  if ($names.ContainsKey($code)) { $names[$code] } else { "$code" }
}
$midX = [int](($rect.Left + $rect.Right) / 2)
$midY = [int](($rect.Top + $rect.Bottom) / 2)
"rect: $($rect.Left),$($rect.Top) - $($rect.Right),$($rect.Bottom)  zoomed: $([HitTest]::IsZoomed($hwnd))  minimized: $([HitTest]::IsIconic($hwnd))"
"style: 0x{0:X8}" -f [HitTest]::GetWindowLong($hwnd, -16)
"left edge:   $(Probe ($rect.Left + 2) $midY)"
"right edge:  $(Probe ($rect.Right - 3) $midY)"
"top edge:    $(Probe $midX ($rect.Top + 2))"
"bottom edge: $(Probe $midX ($rect.Bottom - 3))"
"bottom-right: $(Probe ($rect.Right - 3) ($rect.Bottom - 3))"
"center:      $(Probe $midX $midY)"
