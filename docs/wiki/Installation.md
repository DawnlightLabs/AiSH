# Installation

AiSH can be installed through the published install scripts or GitHub Releases.

## Windows PowerShell

```powershell
irm https://aish.dawnlightlabs.com/install.ps1 | iex
```

Then run setup:

```powershell
aish --install
```

## macOS / Linux

```bash
curl -fsSL https://aish.dawnlightlabs.com/install | bash
```

Then run setup:

```bash
aish --install
```

## Headless setup

For automated setup where supported:

```bash
aish --install-headless --add-path --set-model-path --editor-profiles --model-check
```

## Backup downloads

Release artifacts are published through GitHub Releases.

Check the release notes before installing. Artifact support may vary by platform and CPU architecture.

## Uninstall / repair

Uninstall behavior should remove binaries and profile entries that AiSH created. If a profile was edited manually, review the relevant shell profile before deleting anything.
