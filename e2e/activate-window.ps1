# Puts the test window in front, so key presses go to it (never the user's own Codestamp).
. "$PSScriptRoot\test-process.ps1"
$process = Get-TestProcess | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
[void](New-Object -ComObject WScript.Shell).AppActivate($process.Id)
