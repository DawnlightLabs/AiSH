param(
  [switch]$Headless,
  [switch]$SkipModel,
  [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"

$Repo = "DawnlightLabs/AiSH"
$InstallRoot = Join-Path $env:LOCALAPPDATA "AiSH"
$BinDir = Join-Path $InstallRoot "bin"
$ExePath = Join-Path $BinDir "aish.exe"

function Get-Arch {
  switch ($env:PROCESSOR_ARCHITECTURE) {
    "ARM64" { "arm64"; break }
    default { "x64" }
  }
}

function Get-DownloadUrl {
  param([string]$Arch)
  $asset = "aish-windows-$Arch.zip"
  if ($Version -eq "latest") {
    return "https://github.com/$Repo/releases/latest/download/$asset"
  }
  return "https://github.com/$Repo/releases/download/$Version/$asset"
}

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$arch = Get-Arch
$url = Get-DownloadUrl -Arch $arch
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "aish-$arch.zip"
$extract = Join-Path ([System.IO.Path]::GetTempPath()) "aish-install-$arch"

Write-Host "Downloading AiSH from $url"
Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing

$checksumUrl = "$url.sha256"
$checksumFile = "$tmp.sha256"
try {
  Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumFile -UseBasicParsing
  $expected = ((Get-Content $checksumFile -Raw).Trim() -split "\s+")[0].ToLower()
  $actual = (Get-FileHash $tmp -Algorithm SHA256).Hash.ToLower()
  if ($expected -ne $actual) {
    throw "checksum mismatch for $asset"
  }
  Write-Host "Verified SHA256: $actual"
} catch {
  Write-Warning "Could not verify release checksum: $_"
}

if (Test-Path $extract) {
  Remove-Item $extract -Recurse -Force
}
Expand-Archive -Path $tmp -DestinationPath $extract -Force

$downloaded = Get-ChildItem -Path $extract -Filter "aish.exe" -Recurse | Select-Object -First 1
if (-not $downloaded) {
  throw "Downloaded archive did not contain aish.exe"
}

Copy-Item $downloaded.FullName $ExePath -Force

if ($Headless) {
  $setupArgs = @("--install-headless", "--add-path", "--set-model-path", "--windows-terminal", "--editor-profiles")
  if ($SkipModel -or $env:AISH_SKIP_MODEL -eq "1") {
    $setupArgs += "--skip-model"
  } else {
    $setupArgs += "--model-check"
  }
  & $ExePath @setupArgs
} else {
  & $ExePath --install
}

Write-Host ""
Write-Host "AiSH installed at $ExePath"
Write-Host "Open a new terminal and run: aish"
