# bundle.windows.signCommand: signs one file with the Codestamp certificate from the current user's
# store, CODESTAMP_SIGNING_THUMBPRINT if set, otherwise the newest valid "CN=Codestamp".
param([Parameter(Mandatory = $true)][string]$File)
$ErrorActionPreference = 'Stop'
$store = 'Cert:\CurrentUser\My'

if ($env:CODESTAMP_SIGNING_THUMBPRINT) {
  if ($env:CODESTAMP_SIGNING_THUMBPRINT -notmatch '^[0-9A-Fa-f]{40}$') {
    throw 'CODESTAMP_SIGNING_THUMBPRINT must be a 40-character certificate thumbprint.'
  }
  $cert = Get-Item (Join-Path $store $env:CODESTAMP_SIGNING_THUMBPRINT)
} else {
  $cert = Get-ChildItem $store -CodeSigningCert |
    Where-Object { $_.Subject -eq 'CN=Codestamp' -and $_.NotAfter -gt (Get-Date) } |
    Sort-Object NotAfter -Descending |
    Select-Object -First 1
}
if (-not $cert) {
  throw 'No Codestamp code-signing certificate found. Run src-tauri\windows\new-signing-certificate.ps1 once.'
}

$signtool = if ($env:SIGNTOOL) { $env:SIGNTOOL } else {
  Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter signtool.exe |
    Where-Object { $_.FullName -match '\\x64\\' } |
    Sort-Object FullName |
    Select-Object -Last 1 -ExpandProperty FullName
}
if (-not $signtool) {
  throw 'signtool.exe was not found. Install the Windows SDK or set SIGNTOOL to its path.'
}

# A timestamp keeps the signature valid after the certificate expires; "none" skips it offline.
$timestamp = if ($env:CODESTAMP_TIMESTAMP_URL) { $env:CODESTAMP_TIMESTAMP_URL } else { 'http://timestamp.digicert.com' }
$arguments = @('sign', '/sha1', $cert.Thumbprint, '/s', 'My', '/fd', 'SHA256')
if ($timestamp -ne 'none') {
  $arguments += @('/tr', $timestamp, '/td', 'SHA256')
}
& $signtool @arguments $File
if ($LASTEXITCODE -ne 0) {
  throw "signtool could not sign $File"
}
