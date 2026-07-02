!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Running AiSH provider setup"

  IfFileExists "$INSTDIR\aish-provider-shell-x86_64-pc-windows-msvc.exe" 0 +3
    ExecWait '"$INSTDIR\aish-provider-shell-x86_64-pc-windows-msvc.exe" --setup-non-interactive --add-path --set-model-path --windows-terminal --editor-profiles --model-check'
    Goto done

  IfFileExists "$INSTDIR\resources\aish-provider-shell-x86_64-pc-windows-msvc.exe" 0 +3
    ExecWait '"$INSTDIR\resources\aish-provider-shell-x86_64-pc-windows-msvc.exe" --setup-non-interactive --add-path --set-model-path --windows-terminal --editor-profiles --model-check'
    Goto done

  IfFileExists "$INSTDIR\aish-provider-shell-aarch64-pc-windows-msvc.exe" 0 +3
    ExecWait '"$INSTDIR\aish-provider-shell-aarch64-pc-windows-msvc.exe" --setup-non-interactive --add-path --set-model-path --windows-terminal --editor-profiles --model-check'
    Goto done

  IfFileExists "$INSTDIR\resources\aish-provider-shell-aarch64-pc-windows-msvc.exe" 0 +3
    ExecWait '"$INSTDIR\resources\aish-provider-shell-aarch64-pc-windows-msvc.exe" --setup-non-interactive --add-path --set-model-path --windows-terminal --editor-profiles --model-check'
    Goto done

  DetailPrint "AiSH provider setup skipped: provider sidecar not found"

done:
!macroend
