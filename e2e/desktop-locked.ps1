# Prints "locked" while the Windows lock screen has the input desktop (menus and key presses
# cannot reach apps then), else "unlocked". LogonUI alone is not proof: it can outlive an unlock.
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class InputDesktop {
  [DllImport("user32.dll", SetLastError = true)] static extern IntPtr OpenInputDesktop(uint flags, bool inherit, uint access);
  [DllImport("user32.dll", SetLastError = true)] static extern bool GetUserObjectInformation(IntPtr obj, int index, StringBuilder info, int length, out int needed);
  [DllImport("user32.dll")] static extern bool CloseDesktop(IntPtr desktop);
  public static string Name() {
    IntPtr desktop = OpenInputDesktop(0, false, 0x0100);
    if (desktop == IntPtr.Zero) { return "none"; }
    var name = new StringBuilder(256);
    int needed;
    GetUserObjectInformation(desktop, 2, name, name.Capacity, out needed);
    CloseDesktop(desktop);
    return name.ToString();
  }
}
"@
if ([InputDesktop]::Name() -eq 'Default') { 'unlocked' } else { 'locked' }
