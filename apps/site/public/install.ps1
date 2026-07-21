param(
  [switch]$Headless,
  [switch]$SkipModel,
  [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"

$Repo = "DawnlightLabs/AiSH"
$InstallRoot = Join-Path $env:LOCALAPPDATA "AiSH"
$BinDir = Join-Path $InstallRoot "bin"
$RuntimeDir = Join-Path $InstallRoot "runtime"
$ModelDir = Join-Path $InstallRoot "models"
$ExePath = Join-Path $BinDir "aish.exe"
$RuntimePath = Join-Path $RuntimeDir "llama-cli.exe"
$ModelPath = Join-Path $ModelDir "Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf"

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

New-Item -ItemType Directory -Force -Path $BinDir, $RuntimeDir, $ModelDir | Out-Null

$arch = Get-Arch
$asset = "aish-windows-$arch.zip"
$url = Get-DownloadUrl -Arch $arch
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) $asset
$extract = Join-Path ([System.IO.Path]::GetTempPath()) "aish-install-$arch"
$checksumFile = "$tmp.sha256"

Write-Host "Downloading AiSH from $url"
Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing

try {
  Invoke-WebRequest -Uri "$url.sha256" -OutFile $checksumFile -UseBasicParsing
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

$runtime = Get-ChildItem -Path $extract -Filter "llama-cli.exe" -Recurse | Select-Object -First 1
if (-not $runtime) {
  throw "Downloaded archive did not contain the bundled llama-cli runtime"
}

if (Test-Path $RuntimeDir) {
  Remove-Item $RuntimeDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $RuntimeDir | Out-Null
Copy-Item -Path (Join-Path $runtime.Directory.FullName "*") -Destination $RuntimeDir -Recurse -Force
if (-not (Test-Path -LiteralPath $RuntimePath)) {
  throw "AiSH runtime installation did not create $RuntimePath"
}

$env:AISH_MODEL_PATH = $ModelPath
$env:AISH_LLAMA_CLI = $RuntimePath
[Environment]::SetEnvironmentVariable("AISH_MODEL_PATH", $ModelPath, "User")
[Environment]::SetEnvironmentVariable("AISH_LLAMA_CLI", $RuntimePath, "User")

# Run setup from the extracted staging directory. The provider shell then copies
# itself into the managed install path. Running the final aish.exe directly here
# would make setup try to overwrite its own locked executable.
$SetupExe = $downloaded.FullName

if ($Headless) {
  $setupArgs = @("--install-headless", "--add-path", "--set-model-path", "--windows-terminal", "--editor-profiles")
  if ($SkipModel -or $env:AISH_SKIP_MODEL -eq "1") {
    $setupArgs += "--skip-model"
  } else {
    $setupArgs += "--model-check"
  }
  & $SetupExe @setupArgs
} else {
  & $SetupExe --install
}

if ($LASTEXITCODE -ne 0) {
  throw "AiSH setup exited with code $LASTEXITCODE"
}

if (-not (Test-Path -LiteralPath $ExePath)) {
  throw "AiSH setup did not create $ExePath"
}

& $ExePath --repair-install --quiet
if ($LASTEXITCODE -ne 0) {
  throw "AiSH Windows app registration failed with code $LASTEXITCODE"
}

Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $checksumFile -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $extract -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "AiSH installed at $ExePath"
Write-Host "Local runtime installed at $RuntimePath"
Write-Host "Model path: $ModelPath"
Write-Host "Open AiSH from the Start menu or run: aish"
Write-Host "Uninstall later with: aish --uninstall"
