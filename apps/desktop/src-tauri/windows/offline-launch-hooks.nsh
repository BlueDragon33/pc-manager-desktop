; PC Manager's HKCU-only launch scheme. The browser URL is never forwarded.
; No HKLM changes, elevation, shell/PowerShell dispatch or native commands.
; Tauri installMode must remain currentUser.
!macro NSIS_HOOK_POSTINSTALL
  Push $R0
  Push $R1
  Push $R2
  Push $R3

  StrCpy $R0 '"$INSTDIR\pc-manager-desktop.exe"'
  StrCpy $R3 '"$INSTDIR\pc-manager-desktop.exe" "%1"'
  ReadRegStr $R1 HKCU "Software\Classes\pcmanager\shell\open\command" ""

  StrCmp $R1 $R0 pcmgr_register_owned
  StrCmp $R1 $R3 pcmgr_register_owned
  StrCmp $R1 "" pcmgr_check_missing pcmgr_install_skip

pcmgr_check_missing:
  ; Do not claim an existing bare/partial protocol from another installer.
  ; RegOpenKeyExW returns 0 for any existing key even with no default value.
  System::Call 'advapi32::RegOpenKeyExW(p 0x80000001, w "Software\Classes\pcmanager", i 0, i 0x20019, *p .r2) i.r1'
  IntCmp $R1 0 pcmgr_existing_key pcmgr_register_owned pcmgr_register_owned
pcmgr_existing_key:
  System::Call 'advapi32::RegCloseKey(p r2)'
  Goto pcmgr_install_skip

pcmgr_register_owned:
  WriteRegStr HKCU "Software\Classes\pcmanager" "" "URL:PC Manager Offline App"
  WriteRegStr HKCU "Software\Classes\pcmanager" "URL Protocol" ""
  WriteRegStr HKCU "Software\Classes\pcmanager\shell\open\command" "" $R0
  DetailPrint "PC Manager offline launch registered for this Windows user."
  Goto pcmgr_install_done
pcmgr_install_skip:
  DetailPrint "pcmanager:// owned by another handler; leaving it untouched."
pcmgr_install_done:
  Pop $R3
  Pop $R2
  Pop $R1
  Pop $R0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Push $R0
  Push $R1
  Push $R2
  StrCpy $R0 '"$INSTDIR\pc-manager-desktop.exe"'
  StrCpy $R2 '"$INSTDIR\pc-manager-desktop.exe" "%1"'
  ReadRegStr $R1 HKCU "Software\Classes\pcmanager\shell\open\command" ""

  ; Ownership is established only by the exact installed executable command.
  StrCmp $R1 $R0 pcmgr_uninstall_owned
  StrCmp $R1 $R2 pcmgr_uninstall_owned pcmgr_uninstall_done
pcmgr_uninstall_owned:
  DeleteRegKey HKCU "Software\Classes\pcmanager"
  DetailPrint "Removed owned PC Manager offline launch handler."
pcmgr_uninstall_done:
  Pop $R2
  Pop $R1
  Pop $R0
!macroend
