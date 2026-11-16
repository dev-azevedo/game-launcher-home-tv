; Harness de teste do desinstalador do Sofa (BE-38, BE-42): roda as MESMAS funcoes de
; windows/uninstall-hooks.nsh como um instalador comum, sem elevacao e em pastas temporarias.
; Quem compila e executa os cenarios e scripts/test-installer.ps1. Nada aqui entra no instalador de
; producao e a pasta `tests` fica fora do varredor de guards.
;
; Defines de compilacao (makensis /D...): SOFADIR, PLUGINDIR, OUT_EXE; opcional SOFA_TEST_FORCE_CREATE_FAIL.
;
; Uso do exe: harness.exe /S /SCENARIO=<nome> /OUT=<arquivo> [/ROOT= /LOCALAPP= /INSTDIR= /SIDFILE= /PREV=
;             /READ=0|1 /CODE=n /RUNNING=0|1]. O resultado e escrito em /OUT (uma linha por chave).
Unicode true
RequestExecutionLevel user
SilentInstall silent
OutFile "${OUT_EXE}"
!addplugindir "${PLUGINDIR}"

!include MUI2.nsh
!include LogicLib.nsh
!include FileFunc.nsh
; BE-64: os macros IsShortcutTarget e UnpinShortcut vem do utils.nsh que o bundler do Tauri gera em
; target/<triplo>/release/nsis/<arch>; so entra quando o script de teste o encontra (/DSOFA_TEST_UTILS=<pasta>).
!ifdef SOFA_TEST_UTILS
  !include x64.nsh
  !include "Win\COM.nsh"
  !include "Win\Propkey.nsh"
  !include "Win\RestartManager.nsh"
  !include "${SOFA_TEST_UTILS}\utils.nsh"
!endif

!define PRODUCTNAME "SofaLauncher"
!define VERSION "0.0.0"
!define BUNDLEID "app.sofa.launcher"
!define MAINBINARYNAME "sofa"
!define DISPLAYNAME "SofaLauncher"
; mesmas constantes do installer.nsi, para o install-dir.nsh (SEC-BUG-05) rodar aqui como la
!define INSTALLDIRNAME "Sofa"
!define PLACEHOLDER_INSTALL_DIR "placeholder\${INSTALLDIRNAME}"
InstallDir "${PLACEHOLDER_INSTALL_DIR}"
!define SOFA_HARNESS
!define SOFA_NO_AUTOINSERT

!insertmacro MUI_LANGUAGE "PortugueseBR"

Var UpdateMode
Var KeepDataCheckboxState

!include "${SOFADIR}\install-dir.nsh"
!include "${SOFADIR}\uninstall-hooks.nsh"
!ifdef SOFA_TEST_UTILS
  !include "${SOFADIR}\legacy-shortcut.nsh"
!endif
!include "${SOFADIR}\PortugueseBR.nsh"
!insertmacro SOFA_UNINSTALL_FUNCS ""

Var Scenario
Var OutFile_
Var Arg
Var Result

Function Emit
  ; $0 = linha
  FileOpen $9 "$OutFile_" a
  FileSeek $9 0 END
  FileWrite $9 "$0$\r$\n"
  FileClose $9
FunctionEnd

; O SID vem de um arquivo (/SIDFILE=), sem quebra de linha: assim o teste pode usar aspas, espacos e
; outros caracteres que a linha de comando nao carregaria. Saida em $Arg.
Function ReadSid
  ${GetOptions} $R9 "/SIDFILE=" $0
  StrCpy $Arg ""
  FileOpen $1 "$0" r
  FileRead $1 $Arg
  FileClose $1
FunctionEnd

Section
  ${GetParameters} $R9
  ${GetOptions} $R9 "/OUT=" $OutFile_
  ${GetOptions} $R9 "/SCENARIO=" $Scenario
  Delete "$OutFile_"

  ${If} $Scenario == "sid"
    Call ReadSid
    Push $Arg
    Call SofaIsValidSid
    Pop $0
    StrCpy $0 "valid=$0"
    Call Emit

  ${ElseIf} $Scenario == "profile"
    ; so le: nao apaga nada
    Call SofaOpenExplorer
    Call SofaResolveProfile
    StrCpy $0 "explorer=$SofaExplorerH"
    Call Emit
    StrCpy $0 "sid=$SofaSid"
    Call Emit
    StrCpy $0 "localapp=$SofaLocalApp"
    Call Emit
    ${If} $SofaExplorerH <> 0
      System::Call 'kernel32::CloseHandle(p $SofaExplorerH)'
    ${EndIf}

  ${ElseIf} $Scenario == "fallback"
    ${GetOptions} $R9 "/LOCALAPP=" $SofaLocalApp
    ${GetOptions} $R9 "/ROOT=" $SofaRoot
    Call SofaFallbackDelete
    StrCpy $0 "deleted=$SofaDeleted"
    Call Emit

  ${ElseIf} $Scenario == "decide"
    ${GetOptions} $R9 "/LOCALAPP=" $SofaLocalApp
    ${GetOptions} $R9 "/ROOT=" $SofaRoot
    ${GetOptions} $R9 "/READ=" $SofaPurgeRead
    ${GetOptions} $R9 "/CODE=" $SofaPurgeCode
    ${GetOptions} $R9 "/RUNNING=" $SofaPurgeRunning
    Call SofaPurgeDecide
    StrCpy $0 "outcome=$SofaOutcome"
    Call Emit
    ; ultimos 3 caracteres do texto da caixa: sem linhas em branco quando nao ha pasta
    StrCpy $0 $R5 3 -3
    StrCpy $0 "tail=$0"
    Call Emit
    StrCpy $0 "deleted=$SofaDeleted"
    Call Emit

  ${ElseIf} $Scenario == "purge"
    ; fluxo completo, com o stub no lugar do sofa.exe e o LocalAppData de mentira
    ${GetOptions} $R9 "/INSTDIR=" $INSTDIR
    ${GetOptions} $R9 "/LOCALAPP=" $SofaTestLocalApp
    Call SofaPurgeUserData
    StrCpy $0 "outcome=$SofaOutcome"
    Call Emit
    StrCpy $0 "deleted=$SofaDeleted"
    Call Emit
    StrCpy $0 "started=$SofaPurgeStarted"
    Call Emit
    StrCpy $0 "read=$SofaPurgeRead"
    Call Emit
    StrCpy $0 "code=$SofaPurgeCode"
    Call Emit

  ${ElseIf} $Scenario == "instdir"
    ; pasta fixa (SEC-BUG-05): $INSTDIR vem do `/D=` com que o harness foi aberto; /PREV= e a
    ; pasta da instalacao anterior que o instalador leria do HKLM
    StrCpy $SofaPrevDir ""
    ${GetOptions} $R9 "/PREV=" $SofaPrevDir
    Call SofaFixInstallDir
    StrCpy $0 "result=$SofaDirResult"
    Call Emit
    StrCpy $0 "instdir=$INSTDIR"
    Call Emit

  ${ElseIf} $Scenario == "legacylnk"
    ; BE-64: remove <pasta>\<nome legado>.lnk so se apontar para $INSTDIR\sofa.exe. /DIR= e a pasta de
    ; mentira (nunca $SMPROGRAMS nem $DESKTOP); /INSTDIR= e a pasta de instalacao de mentira.
!ifdef SOFA_TEST_UTILS
    ${GetOptions} $R9 "/INSTDIR=" $INSTDIR
    ${GetOptions} $R9 "/DIR=" $1
    Push $1
    Call SofaRemoveLegacyLnkIn
    Pop $0
    StrCpy $0 "removed=$0"
    Call Emit
!else
    StrCpy $0 "erro=sem-utils"
    Call Emit
!endif

  ${ElseIf} $Scenario == "autostart"
    Call ReadSid
    Push $Arg
    Call SofaRemoveAutostartForSid
    StrCpy $0 "done=1"
    Call Emit
  ${Else}
    StrCpy $0 "erro=cenario-desconhecido:[$Scenario] cmd=[$R9]"
    Call Emit
  ${EndIf}
SectionEnd
