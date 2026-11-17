<#
Builds a signed Wallshaders installer and the latest.json that the app's updater reads.

  .\scripts\release.ps1                          # build only; files land in target\release\bundle\nsis\
  .\scripts\release.ps1 -Notes "What changed"    # same, with release notes shown in the update dialog
  .\scripts\release.ps1 -Publish                 # also create the GitHub release (needs gh, signed in)

The version comes from src-tauri\tauri.conf.json, so bump it there (and in the
Cargo.toml files and package.json) first. The private key defaults to
%USERPROFILE%\.tauri\wallshaders.key. Without that key no update can be signed,
and installed copies only accept updates signed with it.
#>
param(
  [string]$Notes = "",
  [string]$KeyPath = "$env:USERPROFILE\.tauri\wallshaders.key",
  [string]$Repo = "DorFinkelstein/wallshaders",
  [switch]$Publish
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not (Test-Path $KeyPath)) { throw "Signing key not found at $KeyPath" }
$conf = Get-Content "src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$version = $conf.version
$name = $conf.productName
Write-Host "Building $name $version"

$env:TAURI_SIGNING_PRIVATE_KEY = $KeyPath
if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "" }
# Native tools print progress on stderr, which Windows PowerShell 5 would treat as errors.
$ErrorActionPreference = "Continue"
npm run app:build
$ErrorActionPreference = "Stop"
if ($LASTEXITCODE -ne 0) { throw "Build failed" }

$dir = "target\release\bundle\nsis"
$installer = "${name}_${version}_x64-setup.exe"
$sigPath = Join-Path $dir "$installer.sig"
if (-not (Test-Path $sigPath)) { throw "No signature at $sigPath. Is createUpdaterArtifacts on?" }

$latest = [ordered]@{
  version   = $version
  notes     = $Notes
  pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{
      signature = (Get-Content $sigPath -Raw).Trim()
      url       = "https://github.com/$Repo/releases/download/v$version/$installer"
    }
  }
}
$latestPath = Join-Path $dir "latest.json"
# UTF-8 without a byte order mark.
[IO.File]::WriteAllText((Join-Path $root $latestPath), ($latest | ConvertTo-Json -Depth 5))
Write-Host "Wrote $latestPath"

if ($Publish) {
  $files = @((Join-Path $dir $installer), $latestPath)
  # Notes go through a file: multi-line text does not survive Windows PowerShell's argument passing.
  $notesFile = Join-Path $env:TEMP "wallshaders-release-notes.md"
  [IO.File]::WriteAllText($notesFile, $(if ($Notes) { $Notes } else { "$name $version" }))
  $ErrorActionPreference = "Continue"
  gh release create "v$version" @files --repo $Repo --title "$name $version" --notes-file $notesFile
  $ErrorActionPreference = "Stop"
  if ($LASTEXITCODE -ne 0) { throw "gh release create failed" }
  Write-Host "Published v$version"
} else {
  Write-Host "Not published. Upload these two files to a GitHub release tagged v${version}:"
  Write-Host "  $dir\$installer"
  Write-Host "  $latestPath"
}
