param(
  [string]$ProviderPath = "",
  [switch]$InstallApp,
  [switch]$SkipModel,
  [switch]$AddPath = $true,
  [switch]$SetModelPath = $true,
  [switch]$WindowsTerminal = $true,
  [switch]$DefaultTerminal,
  [switch]$EditorProfiles = $true
)

$ErrorActionPreference = "Stop"

function Resolve-AiSHProvider {
  param([string]$Path)

  if ($Path -and (Test-Path $Path)) {
    return (Resolve-Path $Path).Path
  }

  $candidates = @(
    "$PSScriptRoot\aish.exe",
    "$PSScriptRoot\..\aish.exe",
    "$PSScriptRoot\..\resources\aish.exe",
    "$PSScriptRoot\..\resources\aish-provider-shell.exe",
    "$env:LOCALAPPDATA\AiSH\bin\aish.exe"
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return (Resolve-Path $candidate).Path
    }
  }

  $cmd = Get-Command aish -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }

  throw "Could not locate aish.exe provider shell."
}

$provider = Resolve-AiSHProvider $ProviderPath
$args = @("--setup-non-interactive")

if ($AddPath) { $args += "--add-path" }
if ($SetModelPath) { $args += "--set-model-path" }
if ($WindowsTerminal) { $args += "--windows-terminal" }
if ($DefaultTerminal) { $args += "--default-terminal" }
if ($EditorProfiles) { $args += "--editor-profiles" }
if (-not $SkipModel) { $args += "--model-check" }

Write-Host "Running AiSH provider setup: $provider $($args -join ' ')"
& $provider @args

if ($LASTEXITCODE -ne 0) {
  throw "AiSH provider setup failed with exit code $LASTEXITCODE"
}

Write-Host "AiSH installer setup complete."
