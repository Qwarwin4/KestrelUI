!macro NSIS_HOOK_PREINSTALL
  StrCpy $1 "taskkill.exe /F /T /IM winuirev-service.exe"
  DetailPrint 'Exec: $1'
  nsExec::Exec $1
  Pop $0

  StrCpy $1 "taskkill.exe /F /T /IM winuirev.exe"
  DetailPrint 'Exec: $1'
  nsExec::Exec $1
  Pop $0

  ; Clean static folder to remove assets from previous versions
  DetailPrint 'Cleaning static folder from previous installation...'
  RMDir /r "$INSTDIR\static"

  DetailPrint 'Cleaning webview2 runtime from previous installation...'
  RMDir /r "$INSTDIR\runtime"

  File /a "${__FILEDIR__}\..\..\sluhk.dll"
  File /a "${__FILEDIR__}\..\..\SHA256SUMS"
  File /a "${__FILEDIR__}\..\..\SHA256SUMS.sig"

  ; Include PDB file only for nightly builds
  ${StrLoc} $0 "${VERSION}" "nightly" ">"
  ${If} $0 != ""
    File /a "${__FILEDIR__}\..\..\seelen_ui.pdb"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Install the service
  DetailPrint 'Exec: winuirev-service.exe install'
  nsExec::Exec '"$INSTDIR\winuirev-service.exe" install'
  Pop $0
  ; Refresh file associations icons
  !insertmacro UPDATEFILEASSOC
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Gracefully stop the service
  DetailPrint 'Exec: winuirev-service.exe stop'
  nsExec::Exec '"$INSTDIR\winuirev-service.exe" stop'
  Pop $0
  ; Remove the service
  DetailPrint 'Exec: winuirev-service.exe uninstall'
  nsExec::Exec '"$INSTDIR\winuirev-service.exe" uninstall'
  Pop $0
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\sluhk.dll"
  Delete "$INSTDIR\SHA256SUMS"
  Delete "$INSTDIR\SHA256SUMS.sig"
  Delete "$INSTDIR\seelen_ui.pdb"

  ; Refresh file associations icons
  !insertmacro UPDATEFILEASSOC
!macroend