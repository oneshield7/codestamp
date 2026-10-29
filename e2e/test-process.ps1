# Dot-sourced by the helpers. The test copy runs from .work\target\release. A Codestamp started
# from anywhere else carries the same process name, so only the path tells the two apart.
$TestExe = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '.work\target\release\codestamp-desktop.exe'))
function Get-TestProcess {
  Get-Process -Name 'codestamp-desktop' -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $TestExe }
}
