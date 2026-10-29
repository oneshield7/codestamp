; Puts the install folder (with codestamp-cli.exe) on the user's PATH and takes it off at uninstall.
; The folder reaches PowerShell through an environment variable, so no path can change the command.
!macro CODESTAMP_EDIT_PATH ACTION
  System::Call 'Kernel32::SetEnvironmentVariable(t "CODESTAMP_BIN", t "$INSTDIR")'
  System::Call 'Kernel32::SetEnvironmentVariable(t "CODESTAMP_PATH_ACTION", t "${ACTION}")'
  nsExec::Exec `powershell.exe -NoProfile -NonInteractive -Command "$$d = $$env:CODESTAMP_BIN; $$k = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment'); $$old = [string]$$k.GetValue('Path', '', 'DoNotExpandEnvironmentNames'); $$parts = @($$old -split ';' | Where-Object { $$_ -ne '' -and $$_ -ne $$d }); if ($$env:CODESTAMP_PATH_ACTION -eq 'add') { $$parts += $$d }; $$new = $$parts -join ';'; if ($$new -ne $$old) { $$k.SetValue('Path', $$new, 'ExpandString') }; $$k.Close()"`
  Pop $0
  ; New terminals pick up the change after this broadcast (HWND_BROADCAST, WM_SETTINGCHANGE).
  SendMessage 0xFFFF 0x1A 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro CODESTAMP_EDIT_PATH add
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro CODESTAMP_EDIT_PATH remove
!macroend
