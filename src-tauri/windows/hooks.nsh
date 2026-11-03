; Make the current-user NSIS installation available to new shell sessions.
; Tauri calls these hooks after `$INSTDIR` has been established.
!include "WinMessages.nsh"
!include "StrFunc.nsh"
${UnStrRep}

!macro NotifyEnvironmentChanged
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ReadRegStr $0 HKCU "Environment" "Path"
  ${If} $0 == ""
    WriteRegExpandStr HKCU "Environment" "Path" "$INSTDIR"
    !insertmacro NotifyEnvironmentChanged
  ${Else}
    ; Delimit the value to match a complete PATH entry, not a prefix.
    StrCpy $1 ";$0;"
    ${StrLoc} $2 "$1" ";$INSTDIR;" ">"
    ${If} $2 == ""
      WriteRegExpandStr HKCU "Environment" "Path" "$0;$INSTDIR"
      !insertmacro NotifyEnvironmentChanged
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ReadRegStr $0 HKCU "Environment" "Path"
  ${If} $0 == "$INSTDIR"
    DeleteRegValue HKCU "Environment" "Path"
    !insertmacro NotifyEnvironmentChanged
  ${ElseIf} $0 != ""
    ; The installer appends its own entry. Remove it without affecting other entries.
    ${UnStrRep} $1 "$0" ";$INSTDIR" ""
    ${UnStrRep} $1 "$1" "$INSTDIR;" ""
    ${If} $1 != "$0"
      WriteRegExpandStr HKCU "Environment" "Path" "$1"
      !insertmacro NotifyEnvironmentChanged
    ${EndIf}
  ${EndIf}
!macroend
