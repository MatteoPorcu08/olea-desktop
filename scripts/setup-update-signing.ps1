#Requires -Version 5.1
# OLEA: chiavi Tauri create SOLO su questo PC; trasferite nei GitHub Secrets via gh (cifratura lato client).
$ErrorActionPreference = 'Stop'
$repo = 'MatteoPorcu08/olea-desktop'

Write-Host "Configurazione aggiornamenti firmati OLEA" -ForegroundColor Green
if (-not (Get-Command npx -ErrorAction SilentlyContinue)) { throw 'Installa Node.js LTS da https://nodejs.org e riapri PowerShell.' }
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw 'Installa GitHub CLI da https://cli.github.com e riapri PowerShell.' }
& gh auth status *> $null
if ($LASTEXITCODE -ne 0) {
  Write-Host 'Accedi al tuo account GitHub usando il browser…'
  & gh auth login --hostname github.com --git-protocol https --web
  if ($LASTEXITCODE -ne 0) { throw 'Accesso GitHub non riuscito.' }
}
$dir = Join-Path $env:USERPROFILE 'OLEA-Firma-Aggiornamenti'
New-Item -Type Directory -Path $dir -Force | Out-Null
$key = Join-Path $dir 'olea-updater.key'
$public = "$key.pub"
if ((Test-Path $key) -xor (Test-Path $public)) { throw "File chiavi incompleti in $dir. Non sovrascrivere le chiavi esistenti." }
$existing = (Test-Path $key) -and (Test-Path $public)
if ($existing) {
  Write-Host 'Uso le chiavi gia esistenti (non le cambio).' -ForegroundColor Yellow
  $securePass = Read-Host 'Inserisci la password ORIGINALE della chiave' -AsSecureString
} else {
  Write-Host 'Creo una sola volta la chiave privata sul tuo PC.'
  $securePass = Read-Host 'Scegli una password FORTE (conservala nel tuo password manager)' -AsSecureString
}
$ptr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($securePass)
try {
  $pass = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($ptr)
  if ($pass.Length -lt 12) { throw 'La password deve avere almeno 12 caratteri.' }
  if (-not $existing) {
    & npx --yes '@tauri-apps/cli@2' signer generate --ci --write-keys $key --password $pass
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path $key) -or -not (Test-Path $public)) { throw 'Generazione chiavi fallita. Nessun secret modificato.' }
  }
  # gh cifra i segreti localmente prima di caricarli su GitHub.
  Get-Content -Raw -Path $key | & gh secret set TAURI_SIGNING_PRIVATE_KEY --repo $repo
  if ($LASTEXITCODE -ne 0) { throw 'Caricamento chiave privata non riuscito.' }
  $pass | & gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo $repo
  if ($LASTEXITCODE -ne 0) { throw 'Caricamento password non riuscito.' }
  Get-Content -Raw -Path $public | & gh variable set OLEA_UPDATER_PUBKEY --repo $repo
  if ($LASTEXITCODE -ne 0) { throw 'Caricamento chiave pubblica non riuscito.' }
} finally {
  if ($ptr -ne [IntPtr]::Zero) { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr) }
  $pass = $null
}
Write-Host "Chiave privata conservata localmente in: $dir" -ForegroundColor Green
Write-Host 'NON condividere la chiave privata o la password: crea una copia di sicurezza cifrata.' -ForegroundColor Yellow
Write-Host 'Avvio il workflow per la prima release stabile firmata…'
& gh workflow run release.yml --repo $repo --ref main
if ($LASTEXITCODE -ne 0) { throw 'Chiavi configurate, ma avvio workflow fallito: apri GitHub > Actions > OLEA - Aggiornamenti firmati Windows > Run workflow.' }
Write-Host 'Configurazione terminata. Controlla GitHub Actions e installa la prima release stabile.' -ForegroundColor Green
