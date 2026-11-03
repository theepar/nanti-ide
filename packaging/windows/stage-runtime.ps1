param(
    [string]$Node = (Get-Command node).Source,
    [string]$Out = (Join-Path $PSScriptRoot '..\..\target\release\runtime')
)
# Collects what the installer bundles so a fresh machine needs nothing else: Node.js for extensions, MinGit for the
# Git panel and Microsoft's WebView2 bootstrapper. Downloads are checked (GitHub's published SHA-256, Microsoft's signature)
# and nothing downloaded is run here.
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = (Resolve-Path $Out).Path

Copy-Item $Node (Join-Path $Out 'node.exe') -Force

$git = Join-Path $Out 'git'
$release = Invoke-RestMethod 'https://api.github.com/repos/git-for-windows/git/releases/latest' -Headers @{ 'User-Agent' = 'nanti-build' }
$asset = $release.assets | Where-Object { $_.name -match '^MinGit-.*-64-bit\.zip$' } | Select-Object -First 1
$zip = Join-Path $Out $asset.name
Invoke-WebRequest $asset.browser_download_url -OutFile $zip
$want = $asset.digest -replace '^sha256:', ''
$have = (Get-FileHash $zip -Algorithm SHA256).Hash
if ($have -ne $want) { throw "MinGit checksum mismatch: $have, expected $want" }
if (Test-Path $git) { Remove-Item $git -Recurse -Force }
Expand-Archive $zip $git
Remove-Item $zip

$boot = Join-Path $Out 'MicrosoftEdgeWebview2Setup.exe'
Invoke-WebRequest 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $boot
$sig = Get-AuthenticodeSignature $boot
if ($sig.Status -ne 'Valid' -or $sig.SignerCertificate.Subject -notmatch 'Microsoft') { throw "WebView2 bootstrapper signature is $($sig.Status)" }

Write-Host "staged in ${Out}: node $((Get-Item (Join-Path $Out 'node.exe')).VersionInfo.ProductVersion), $($asset.name), WebView2 bootstrapper"
