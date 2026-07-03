# Troubleshooting

This guide covers common AiSH setup and runtime problems.

## AiSH command is not found

Check that the install directory is on `PATH`.

Windows PowerShell:

```powershell
$env:Path -split ';'
```

macOS / Linux:

```bash
echo "$PATH" | tr ':' '\n'
```

Run the installer or setup command again only after checking whether it would edit your profile.

```bash
aish --install
```

## Shell profile was not added

Run setup again with profile flags when supported:

```bash
aish --install-headless --add-path --editor-profiles --model-check
```

Check the relevant profile manually:

- PowerShell profile: `$PROFILE`
- zsh: `~/.zshrc`
- bash: `~/.bashrc` or `~/.bash_profile`

Do not paste profile contents into public issues if they include private paths or tokens.

## Windows Terminal profile does not show AiSH

- Restart Windows Terminal.
- Check whether the generated profile exists in Windows Terminal settings.
- Confirm the AiSH executable path still exists.
- Confirm the executable is not blocked by antivirus or SmartScreen.

## VS Code terminal profile does not show AiSH

- Restart VS Code.
- Check user settings for terminal profiles.
- Confirm the configured executable path exists.
- Re-run setup with editor profile support if needed.

## Model path check fails

- Confirm the model file exists.
- Confirm the user account has read access.
- Avoid paths inside temporary download directories.
- Avoid committing model files to the repository.

## Release artifact fails to run

- Confirm you downloaded the artifact for your OS and architecture.
- On macOS, check whether the artifact was quarantined by the browser.
- On Linux, confirm executable permissions when using tarballs or AppImages.
- On Windows, check SmartScreen or antivirus logs.

## Opening a bug report

Use the bug report template and include:

- OS and shell.
- AiSH version or commit SHA.
- Install method.
- Exact command run.
- Error output with secrets removed.
