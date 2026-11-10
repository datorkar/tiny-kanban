; Tiny Kanban: NSIS installer hooks (included by Tauri's installer template).
;
; For IT: install with   tiny-kanban_<version>_x64-setup.exe /S /DISABLEUPDATES
; to turn off self-updates. This writes DisableUpdates=1 under
; Software\TinyKanban (HKCU for a per-user install, HKLM for per-machine),
; which the app checks before looking for updates. Pass it again on upgrades.
!include "FileFunc.nsh"

!macro NSIS_HOOK_POSTINSTALL
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "/DISABLEUPDATES" $R1
  ${IfNot} ${Errors}
    WriteRegDWORD SHCTX "Software\TinyKanban" "DisableUpdates" 1
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue SHCTX "Software\TinyKanban" "DisableUpdates"
  DeleteRegKey /ifempty SHCTX "Software\TinyKanban"
!macroend
