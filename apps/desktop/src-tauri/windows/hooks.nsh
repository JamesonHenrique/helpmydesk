; SPDX-License-Identifier: GPL-3.0-or-later
; Uninstaller hook (F2-UNINST, ADR-029): offers to erase the local history too. The AI key in
; the Windows Credential Manager is removed by "Apagar tudo" in the app; the uninstaller does not
; touch the vault. Silent uninstalls keep the data (/SD IDNO).
; Spec 020 (SEC-020): the installer is per machine (`installMode: perMachine`), so the app, the
; helper and this uninstaller live in $INSTDIR under Program Files. The NSIS template of Tauri
; 2.12.1 deletes each file it installed (main binary, sidecars, resources) and then the empty
; folder (FR-012); the helper writes no registry key of its own, so there is nothing under HKLM to
; clean up here.
; Per machine, the template runs with `SetShellVarContext all`, which points $APPDATA at
; ProgramData. The history lives in the profile of the person, so the context is switched to
; `current` for the delete and back to `all` after. Limit: when a standard user types an
; administrator's password in the UAC, "current" is that administrator; "Apagar tudo" in the app
; is the path that always reaches the right profile.
!macro NSIS_HOOK_POSTUNINSTALL
  MessageBox MB_YESNO|MB_ICONQUESTION "Apagar também o histórico do HelpMyDesk neste computador?$\r$\nIsso não dá para desfazer." /SD IDNO IDNO hmd_keep_history
    SetShellVarContext current
    RMDir /r "$APPDATA\io.github.jamesonhenrique.helpmydesk"
    RMDir /r "$LOCALAPPDATA\io.github.jamesonhenrique.helpmydesk"
    SetShellVarContext all
  hmd_keep_history:
!macroend
