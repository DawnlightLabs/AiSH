param(
  [string]$InstallDir = "$env:LOCALAPPDATA\AiSH",
  [switch]$SkipModel
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$BinDir = Join-Path $InstallDir "bin"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

Push-Location $Root
cargo build --release -p aish-provider-shell
Pop-Location

$Source = Join-Path $Root "target\release\aish.exe"
$Target = Join-Path $BinDir "aish.exe"
Copy-Item $Source $Target -Force

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (($UserPath -split ";") -notcontains $BinDir) {
  [Environment]::SetEnvironmentVariable("Path", "$UserPath;$BinDir", "User")
  Write-Host "Added AiSH to user PATH. Open a new terminal after this script."
}

if (-not $SkipModel) {
  & $Target --setup
}

Write-Host "AiSH provider shell installed at $Target"
