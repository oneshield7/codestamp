# Creates the self-signed "CN=Codestamp" code-signing certificate in the current user's Personal
# store (trust settings are left alone) and exports its public part to codestamp.cer next to this file.
$ErrorActionPreference = 'Stop'
$store = 'Cert:\CurrentUser\My'
$cert = Get-ChildItem $store -CodeSigningCert |
  Where-Object { $_.Subject -eq 'CN=Codestamp' -and $_.NotAfter -gt (Get-Date).AddDays(30) } |
  Sort-Object NotAfter -Descending |
  Select-Object -First 1
if ($cert) {
  Write-Output "Using the existing certificate $($cert.Thumbprint), valid until $($cert.NotAfter.ToString('yyyy-MM-dd'))."
} else {
  $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=Codestamp' `
    -FriendlyName 'Codestamp code signing (self-signed)' `
    -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
    -KeyExportPolicy NonExportable -NotAfter (Get-Date).AddYears(3) `
    -CertStoreLocation $store
  Write-Output "Created certificate $($cert.Thumbprint), valid until $($cert.NotAfter.ToString('yyyy-MM-dd'))."
}
$public = Join-Path $PSScriptRoot 'codestamp.cer'
Export-Certificate -Cert $cert -FilePath $public -Type CERT | Out-Null
Write-Output "Public certificate for other PCs: $public"
