; installerHooks do Tauri (`bundle.windows.nsis.installerHooks`). O template `installer.nsi` inclui
; este arquivo no topo e, depois das variaveis, inclui os dois irmaos (SOFADIR).
;
;   install-hooks.nsh    pagina "Iniciar com o Windows" e gravacao de HKLM\Software\app.sofa.launcher
;   legacy-shortcut.nsh  limpeza do atalho "Sofa" com acento da 0.1.0 (D-57, BE-64)
;   uninstall-hooks.nsh  autostart de todos os perfis, purge dos dados e fallback restrito (dono do
;                        guard de System::Call, scripts/ci-guards.ps1)

!define SOFADIR "${__FILEDIR__}"

!macro NSIS_HOOK_POSTINSTALL
  Call SofaWriteInstallInfo
  ; D-57: tira o atalho legado da 0.1.0 (Sofa com acento) (so se apontar para o sofa.exe desta pasta); tambem em /UPDATE
  Call SofaRemoveLegacyShortcuts
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Call un.SofaBeforeUninstall
!macroend
