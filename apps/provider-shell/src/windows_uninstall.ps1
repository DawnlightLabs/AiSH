param([switch]$Quiet)

$ErrorActionPreference = "SilentlyContinue"
$InstallRoot = Join-Path $env:LOCALAPPDATA "AiSH"
$BinDir = Join-Path $InstallRoot "bin"
$ShortcutPath = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\AiSH.lnk"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\AiSH"
$AppPathKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\aish.exe"
$ProfileGuid = "{8f6d930e-7f49-4bd8-9d29-a15000000001}"

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath) {
  $nextPath = ($userPath -split ';' | Where-Object {
    $_ -and -not $_.Trim().Trim('"').Equals($BinDir, [StringComparison]::OrdinalIgnoreCase)
  }) -join ';'
  [Environment]::SetEnvironmentVariable("Path", $nextPath, "User")
}

$modelPath = [Environment]::GetEnvironmentVariable("AISH_MODEL_PATH", "User")
if ($modelPath -and $modelPath.StartsWith($InstallRoot, [StringComparison]::OrdinalIgnoreCase)) {
  [Environment]::SetEnvironmentVariable("AISH_MODEL_PATH", $null, "User")
}

$terminalPaths = @(
  (Join-Path $env:LOCALAPPDATA "Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json"),
  (Join-Path $env:LOCALAPPDATA "Packages\Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe\LocalState\settings.json"),
  (Join-Path $env:LOCALAPPDATA "Microsoft\Windows Terminal\settings.json")
)

foreach ($settingsPath in $terminalPaths) {
  if (-not (Test-Path -LiteralPath $settingsPath)) { continue }
  try {
    $json = Get-Content -LiteralPath $settingsPath -Raw | ConvertFrom-Json
    if ($json.profiles -and $json.profiles.list) {
      $json.profiles.list = @($json.profiles.list | Where-Object {
        $_.guid -ne $ProfileGuid -and $_.name -ne "AiSH"
      })
      if ($json.defaultProfile -eq $ProfileGuid) {
        $json.PSObject.Properties.Remove("defaultProfile")
      }
      $json | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $settingsPath -Encoding UTF8
    }
  } catch {}
}

$editorPaths = @(
  (Join-Path $env:APPDATA "Code\User\settings.json"),
  (Join-Path $env:APPDATA "Cursor\User\settings.json"),
  (Join-Path $env:APPDATA "Windsurf\User\settings.json"),
  (Join-Path $env:APPDATA "VSCodium\User\settings.json")
)

foreach ($settingsPath in $editorPaths) {
  if (-not (Test-Path -LiteralPath $settingsPath)) { continue }
  try {
    $json = Get-Content -LiteralPath $settingsPath -Raw | ConvertFrom-Json
    $profiles = $json.'terminal.integrated.profiles.windows'
    if ($profiles) {
      $profiles.PSObject.Properties.Remove("AiSH")
      $json | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $settingsPath -Encoding UTF8
    }
  } catch {}
}

Remove-Item -LiteralPath $ShortcutPath -Force
Remove-Item -LiteralPath $UninstallKey -Recurse -Force
Remove-Item -LiteralPath $AppPathKey -Recurse -Force

if (-not $Quiet) {
  Write-Host "AiSH has been removed from PATH, the Start menu, Installed apps, Windows Terminal, and supported editor profiles."
  Write-Host "Any open AiSH terminal must close before the remaining files are deleted."
}

$cleanup = "timeout /t 2 /nobreak > nul & rmdir /s /q `"$InstallRoot`""
Start-Process -FilePath $env:ComSpec -ArgumentList @('/d', '/c', $cleanup) -WindowStyle Hidden
