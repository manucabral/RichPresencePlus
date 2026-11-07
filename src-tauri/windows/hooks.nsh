; Start with Windows: asked once by the installer, removed by the uninstaller.
; The application reads and writes the same value: crates/rpp-host/src/startup.rs.

!define RPP_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define RPP_RUN_VALUE "RichPresencePlus"
!define RPP_MARKER_KEY "Software\manucabral\Rich Presence Plus"
; Native messaging host entries the application writes at every start:
; crates/rpp-host/src/native_host.rs and src-tauri/src/native.rs.
!define RPP_HOST_NAME "com.manucabral.richpresenceplus"

!macro NSIS_HOOK_POSTINSTALL
  ; Updates and unattended installs never ask. An update keeps the choice
  ; already made; an unattended first install leaves the question to the app.
  ${If} $UpdateMode <> 1
  ${AndIf} $PassiveMode <> 1
  ${AndIfNot} ${Silent}
    ReadRegDWORD $R0 HKCU "${RPP_MARKER_KEY}" "StartupAsked"
    ${If} $R0 <> 1
      ${If} $LANGUAGE = 1034
        StrCpy $R1 "¿Iniciar Rich Presence Plus con Windows?$\r$\n$\r$\nArranca escondida en la bandeja. Puedes cambiarlo después en Ajustes o desde el ícono de la bandeja."
      ${Else}
        StrCpy $R1 "Start Rich Presence Plus with Windows?$\r$\n$\r$\nIt starts hidden in the tray. You can change this later in Settings or from the tray icon."
      ${EndIf}
      MessageBox MB_YESNO|MB_ICONQUESTION "$R1" IDNO rpp_startup_declined
        WriteRegStr HKCU "${RPP_RUN_KEY}" "${RPP_RUN_VALUE}" '"$INSTDIR\${MAINBINARYNAME}.exe" --hidden'
      rpp_startup_declined:
      WriteRegDWORD HKCU "${RPP_MARKER_KEY}" "StartupAsked" 1
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; An update runs the old uninstaller first; the choice must survive it.
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "${RPP_RUN_KEY}" "${RPP_RUN_VALUE}"
    DeleteRegKey HKCU "${RPP_MARKER_KEY}"
    DeleteRegKey /ifempty HKCU "Software\manucabral"
    DeleteRegKey HKCU "Software\Google\Chrome\NativeMessagingHosts\${RPP_HOST_NAME}"
    DeleteRegKey HKCU "Software\Microsoft\Edge\NativeMessagingHosts\${RPP_HOST_NAME}"
    DeleteRegKey HKCU "Software\Mozilla\NativeMessagingHosts\${RPP_HOST_NAME}"
    DeleteRegKey HKCU "Software\Chromium\NativeMessagingHosts\${RPP_HOST_NAME}"
    DeleteRegKey HKCU "Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\${RPP_HOST_NAME}"
    DeleteRegKey HKCU "Software\Vivaldi\NativeMessagingHosts\${RPP_HOST_NAME}"
  ${EndIf}
!macroend
