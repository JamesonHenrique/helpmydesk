; SPDX-License-Identifier: GPL-3.0-or-later
; Uninstaller hook (F2-UNINST, ADR-029): offers to erase the local history too. The AI key in
; the Windows Credential Manager is removed by "Apagar tudo" in the app; the uninstaller does not
; touch the vault. Silent uninstalls keep the data (/SD IDNO).
; Spec 020 (SEC-020): the installer is per machine (`installMode: perMachine`), so the app, the
; helper and this uninstaller live in $INSTDIR under Program Files. The NSIS template removes that
; whole folder on uninstall, so the helper and the installation folder go with it (FR-012); the
; helper writes no registry key of its own, so there is nothing under HKLM to clean up here.
!macro NSIS_HOOK_POSTUNINSTALL
  MessageBox MB_YESNO|MB_ICONQUESTION "Apagar também o histórico do HelpMyDesk neste computador?$\r$\nIsso não dá para desfazer." /SD IDNO IDNO hmd_keep_history
    RMDir /r "$APPDATA\io.github.jamesonhenrique.helpmydesk"
    RMDir /r "$LOCALAPPDATA\io.github.jamesonhenrique.helpmydesk"
  hmd_keep_history:
!macroend
