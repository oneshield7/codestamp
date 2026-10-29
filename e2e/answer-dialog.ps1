# Fills the file name of a native open/save dialog and confirms it (or cancels with -Cancel).
# Uses window messages aimed at the dialog itself, so no keystrokes can land in another app.
param([string]$Title, [string]$Path, [switch]$Cancel)

$source = @'
using System; using System.Runtime.InteropServices; using System.Text;
public static class FileDialog {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);
  [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr parent, EnumProc proc, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern IntPtr SendMessage(IntPtr h, int msg, IntPtr w, string l);
  [DllImport("user32.dll")] static extern IntPtr SendMessage(IntPtr h, int msg, IntPtr w, IntPtr l);
  const int WM_SETTEXT = 0x000C, WM_CHAR = 0x0102, EM_SETSEL = 0x00B1, BM_CLICK = 0x00F5;

  static IntPtr Child(IntPtr dialog, string cls, int[] ids) {
    IntPtr found = IntPtr.Zero;
    EnumChildWindows(dialog, (h, l) => {
      var name = new StringBuilder(64); GetClassName(h, name, 64);
      if (name.ToString() == cls && Array.IndexOf(ids, GetDlgCtrlID(h)) >= 0) { found = h; return false; }
      return true;
    }, IntPtr.Zero);
    return found;
  }

  public static string Answer(IntPtr dialog, string path) {
    IntPtr box = Child(dialog, "Edit", new[] { 1148, 1001 });
    if (box == IntPtr.Zero) return "no file name box";
    // The save dialog ignores WM_SETTEXT, so the name is typed into this one box as characters.
    SendMessage(box, WM_SETTEXT, IntPtr.Zero, "");
    SendMessage(box, EM_SETSEL, IntPtr.Zero, new IntPtr(-1));
    foreach (char c in path) SendMessage(box, WM_CHAR, new IntPtr(c), IntPtr.Zero);
    return Press(dialog, 1, "answered");
  }

  public static string Press(IntPtr dialog, int id, string result) {
    IntPtr button = Child(dialog, "Button", new[] { id });
    if (button == IntPtr.Zero) return "no button " + id;
    SendMessage(button, BM_CLICK, IntPtr.Zero, IntPtr.Zero);
    return result;
  }
}
'@
Add-Type -TypeDefinition $source

$deadline = (Get-Date).AddSeconds(20)
while ((Get-Date) -lt $deadline) {
  $dialog = [FileDialog]::FindWindow('#32770', $Title)
  if ($dialog -ne [IntPtr]::Zero) {
    Start-Sleep -Milliseconds 500
    if ($Cancel) { Write-Output ([FileDialog]::Press($dialog, 2, 'cancelled')) } else { Write-Output ([FileDialog]::Answer($dialog, $Path)) }
    exit 0
  }
  Start-Sleep -Milliseconds 300
}
Write-Output 'dialog not found'
exit 1
