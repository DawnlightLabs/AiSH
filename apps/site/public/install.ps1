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
$UninstallScript = Join-Path $InstallRoot "uninstall.ps1"
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$ShortcutPath = Join-Path $StartMenuDir "AiSH.lnk"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\AiSH"
$AppPathKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\aish.exe"

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

function Get-InstalledVersion {
  $fallback = $Version.TrimStart("v")
  if ($fallback -eq "latest") {
    $fallback = "Unknown"
  }

  try {
    $versionLine = (& $ExePath --version 2>$null | Select-Object -First 1)
    if ($versionLine -match '(\d+\.\d+\.\d+(?:[-+][^\s]+)?)') {
      return $Matches[1]
    }
  } catch {
    Write-Verbose "Could not read installed version: $_"
  }

  return $fallback
}

function Write-Uninstaller {
  $content = @'
param([switch]$Quiet)

$ErrorActionPreference = "SilentlyContinue"
$InstallRoot = Join-Path $env:LOCALAPPDATA "AiSH"
$BinDir = Join-Path $InstallRoot "bin"
$ShortcutPath = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\AiSH.lnk"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\AiSH"
$AppPathKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\aish.exe"

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

Remove-Item -LiteralPath $ShortcutPath -Force
Remove-Item -LiteralPath $UninstallKey -Recurse -Force
Remove-Item -LiteralPath $AppPathKey -Recurse -Force

if (-not $Quiet) {
  Write-Host "AiSH has been removed from PATH, the Start menu, and Installed apps."
  Write-Host "Any open AiSH terminal must be closed before its files can be deleted."
}

$cleanup = "timeout /t 2 /nobreak > nul & rmdir /s /q `"$InstallRoot`""
Start-Process -FilePath $env:ComSpec -ArgumentList @('/d', '/c', $cleanup) -WindowStyle Hidden
'@

  Set-Content -LiteralPath $UninstallScript -Value $content -Encoding UTF8
}

function Register-WindowsApp {
  param([string]$DisplayVersion)

  New-Item -ItemType Directory -Force -Path $StartMenuDir | Out-Null

  $shell = New-Object -ComObject WScript.Shell
  try {
    $shortcut = $shell.CreateShortcut($ShortcutPath)
    $shortcut.TargetPath = $ExePath
    $shortcut.WorkingDirectory = $env:USERPROFILE
    $shortcut.IconLocation = "$ExePath,0"
    $shortcut.Description = "AiSH — Artificially Intelligent Shell"
    $shortcut.Save()
  } finally {
    if ($shell) {
      [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell)
    }
  }

  Write-Uninstaller

  New-Item -Path $AppPathKey -Force | Out-Null
  Set-Item -Path $AppPathKey -Value $ExePath
  New-ItemProperty -Path $AppPathKey -Name "Path" -Value $BinDir -PropertyType String -Force | Out-Null

  $uninstallCommand = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$UninstallScript`""
  $quietUninstallCommand = "$uninstallCommand -Quiet"
  $sizeBytes = (Get-ChildItem -LiteralPath $InstallRoot -File -Recurse -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
  $estimatedSize = if ($sizeBytes) { [int][Math]::Ceiling($sizeBytes / 1KB) } else { 0 }

  New-Item -Path $UninstallKey -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "DisplayName" -Value "AiSH" -PropertyType String -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "DisplayVersion" -Value $DisplayVersion -PropertyType String -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "Publisher" -Value "Dawnlight Labs" -PropertyType String -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "DisplayIcon" -Value "$ExePath,0" -PropertyType String -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "InstallLocation" -Value $InstallRoot -PropertyType String -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "InstallDate" -Value (Get-Date -Format "yyyyMMdd") -PropertyType String -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "UninstallString" -Value $uninstallCommand -PropertyType String -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "QuietUninstallString" -Value $quietUninstallCommand -PropertyType String -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "URLInfoAbout" -Value "https://aish.dawnlightlabs.com" -PropertyType String -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "EstimatedSize" -Value $estimatedSize -PropertyType DWord -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "NoModify" -Value 1 -PropertyType DWord -Force | Out-Null
  New-ItemProperty -Path $UninstallKey -Name "NoRepair" -Value 1 -PropertyType DWord -Force | Out-Null

  Write-Host "[✓] Registered AiSH in the Start menu and Installed apps"
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
    throw "checksum mismatch for aish-windows-$arch.zip"
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

if ($LASTEXITCODE -ne 0) {
  throw "AiSH setup exited with code $LASTEXITCODE"
}

Register-WindowsApp -DisplayVersion (Get-InstalledVersion)

Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $checksumFile -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $extract -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "AiSH installed at $ExePath"
Write-Host "Open AiSH from the Start menu or run: aish"