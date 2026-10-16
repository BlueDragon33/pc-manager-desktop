; PC Manager managed offline launch: current-user Windows protocol only.
; Tauri's NSIS installMode is intentionally currentUser. Never write HKLM.
; Accept existing scheme only when it already points to this exact executable.
; A browser-supplied URL is an inert process argument, not an admin command.
;
; This hook is run in the disposable unsigned Preview too. It is NOT proof of
; signed Production installer acceptance (P9 manual gate remains pending).

!macro NSIS_HOOK_POSTINSTALL
  Push $R0
  Push $R1
  Push $R2

  StrCpy $R0 '"$INSTDIR\pc-manager-desktop.exe" "%1"'
  ReadRegStr $R1 HKCU "Software\Classes\pcmanager\shell\open\command" ""

  ; Existing handler with different command is owned by someone else.
  StrCmp $R1 $R0 pcmgr_register_owned pcmgr_check_missing
pcmgr_check_missing:
  StrCmp $R1 "" pcmgr_check_protocol pcmgr_install_skip
pcmgr_check_protocol:
  ClearErrors
  ReadRegStr $R2 HKCU "Software\Classes\pcmanager" "URL Protocol"
  ; A scheme with an existing URL Protocol value but no command is foreign.
  IfErrors pcmgr_register_owned pcmgr_install_skip
pcmgr_register_owned:
  WriteRegStr HKCU "Software\Classes\pcmanager" "" "URL:PC Manager Offline App"
  WriteRegStr HKCU "Software\Classes\pcmanager" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\pcmanager\shell\open\command" "" $R0
  DetailPrint "PC Manager offline launch registered for the current Windows account."
  Goto pcmgr_install_done
pcmgr_install_skip:
  DetailPrint "pcmanager:// already registered by another handler; preserving it."
pcmgr_install_done:
  Pop $R2
  Pop $R1
  Pop $R0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Push $R0
  Push $R1
  StrCpy $R0 '"$INSTDIR\pc-manager-desktop.exe" "%1"'
  ReadRegStr $R1 HKCU "Software\Classes\pcmanager\shell\open\command" ""

  ; Uninstaller must never remove a scheme owned by a different executable.
  StrCmp $R1 $R0 pcmgr_uninstall_owned pcmgr_uninstall_done
pcmgr_uninstall_owned:
  DeleteRegKey HKCU "Software\Classes\pcmanager"
  DetailPrint "Removed PC Manager's current-user offline launch association."
pcmgr_uninstall_done:
  Pop $R1
  Pop $R0
!macroend
