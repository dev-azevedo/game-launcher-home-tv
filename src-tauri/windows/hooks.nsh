; installerHooks do Tauri (`bundle.windows.nsis.installerHooks`). O template `installer.nsi` inclui
; este arquivo no topo e, depois das variaveis, inclui os dois irmaos (SOFADIR).
;
;   install-hooks.nsh    pagina "Iniciar com o Windows" e gravacao de HKLM\Software\app.sofa.launcher
;   uninstall-hooks.nsh  autostart de todos os perfis, purge dos dados e fallback restrito (dono do
;                        guard de System::Call, scripts/ci-guards.ps1)

!define SOFADIR "${__FILEDIR__}"

!macro NSIS_HOOK_POSTINSTALL
  Call SofaWriteInstallInfo
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Call un.SofaBeforeUninstall
!macroend
