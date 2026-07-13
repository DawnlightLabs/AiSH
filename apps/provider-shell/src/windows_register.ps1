param(
  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$ExePath,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$InstallRoot,

  [Parameter(Mandatory = $true)]
  [ValidateNotNullOrEmpty()]
  [string]$DisplayVersion
)

$ErrorActionPreference = "Stop"
$BinDir = Join-Path $InstallRoot "bin"
$UninstallScript = Join-Path $InstallRoot "uninstall.ps1"
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$ShortcutPath = Join-Path $StartMenuDir "AiSH.lnk"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\AiSH"
$AppPathKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths\aish.exe"

New-Item -ItemType Directory -Force -Path $StartMenuDir | Out-Null
$shell = New-Object -ComObject WScript.Shell
try {
  $shortcut = $shell.CreateShortcut($ShortcutPath)
  $shortcut.TargetPath = $ExePath
  $shortcut.WorkingDirectory = $env:USERPROFILE
  $shortcut.IconLocation = "$ExePath,0"
  $shortcut.Description = "AiSH - Artificially Intelligent Shell"
  $shortcut.Save()
} finally {
  if ($shell) {
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell)
  }
}

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
