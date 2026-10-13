# Setup local code-signing certificate for Windows Authenticode signing
$certName = "Windows Desktop Host"

$existing = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Where-Object { $_.Subject -like "*$certName*" }

if (-not $existing) {
    Write-Host "[1/3] Generating code signing certificate ($certName)..." -ForegroundColor Cyan
    $cert = New-SelfSignedCertificate -Type CodeSigningCert `
        -Subject "CN=$certName" `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -NotAfter (Get-Date).AddYears(5) `
        -KeyExportPolicy Exportable `
        -KeyUsage DigitalSignature `
        -FriendlyName "Desktop Host Code Signing"

    Write-Host "[2/3] Adding certificate to Trusted Root store..." -ForegroundColor Cyan
    $certPath = Join-Path $PSScriptRoot "DesktopHost_CodeSign.cer"
    Export-Certificate -Cert $cert -FilePath $certPath | Out-Null
    Import-Certificate -FilePath $certPath -CertStoreLocation "Cert:\CurrentUser\Root" | Out-Null
    Remove-Item $certPath -Force -ErrorAction SilentlyContinue

    Write-Host "[3/3] Exporting PFX for portable signing..." -ForegroundColor Cyan
    $pfxPath = Join-Path $PSScriptRoot "cert.pfx"
    $secPass = ConvertTo-SecureString -String "LocalSign2026!" -Force -AsPlainText
    Export-PfxCertificate -Cert $cert -FilePath $pfxPath -Password $secPass | Out-Null
    Write-Host "[SUCCESS] Code signing certificate created and trusted." -ForegroundColor Green
} else {
    Write-Host "[OK] Certificate '$certName' is already installed and ready." -ForegroundColor Green
}
