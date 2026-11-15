; =====================================================================================
; Sofa: instalacao (ADR-0006 rev. 6, secao 2; CA-51, RN-19, SEC-21, SEC-29).
;
; O instalador elevado NAO cria autostart em perfil nenhum: so grava a escolha da caixa em
; HKLM\Software\app.sofa.launcher\InstallDefaults (DWORD 0 ou 1, o que o app le) e o
; InstallLocation. Cada usuario aplica o padrao na 1a abertura, com o proprio token.
; Nada daqui escreve em HKCU, nem em %LOCALAPPDATA%, nem em ACL.
; =====================================================================================

!include LogicLib.nsh
!include nsDialogs.nsh

; Pagina "Iniciar com o Windows": caixa marcada por padrao (CA-51) e a orientacao de entrar no
; Windows sem teclado (RN-19). Ao reinstalar, parte do valor ja gravado.
Function SofaOptionsShow
  ${If} $PassiveMode = 1
    Abort
  ${EndIf}
  ${If} $SofaStartupState == ""
    Call SofaInitialStartupState
  ${EndIf}

  !insertmacro MUI_HEADER_TEXT "$(sofaOptionsTitle)" "$(sofaOptionsSubtitle)"
  nsDialogs::Create 1018
  Pop $0
  ${If} $0 == error
    Abort
  ${EndIf}
  ${If} $(^RTL) = 1
    nsDialogs::SetRTL $(^RTL)
  ${EndIf}

  ${NSD_CreateCheckBox} 0 0 100% 12u "$(sofaStartupCheckbox)"
  Pop $SofaStartupCheckbox
  ${If} $SofaStartupState = 1
    ${NSD_Check} $SofaStartupCheckbox
  ${EndIf}
  ${NSD_CreateLabel} 12u 14u -12u 24u "$(sofaStartupHint)"
  Pop $0
  ${NSD_CreateLabel} 0 50u 100% 48u "$(sofaNoKeyboardNote)"
  Pop $0
  ${NSD_SetFocus} $SofaStartupCheckbox
  nsDialogs::Show
FunctionEnd

Function SofaOptionsLeave
  ${NSD_GetState} $SofaStartupCheckbox $0
  ${If} $0 = ${BST_CHECKED}
    StrCpy $SofaStartupState 1
  ${Else}
    StrCpy $SofaStartupState 0
  ${EndIf}
FunctionEnd

; Estado inicial da caixa e valor de instalacoes sem tela (/S, /P, /UPDATE): o que ja esta gravado
; (reinstalar ou atualizar mantem a escolha, CA-53); sem nada gravado, ligado (CA-51).
; So o DWORD 0 desliga; qualquer outro valor ou tipo conta como o padrao (ligado).
Function SofaInitialStartupState
  StrCpy $SofaStartupState 1
  ClearErrors
  ReadRegDWORD $0 SHCTX "${MANUPRODUCTKEY}" "InstallDefaults"
  ${IfNot} ${Errors}
    ${If} $0 = 0
      StrCpy $SofaStartupState 0
    ${EndIf}
  ${EndIf}
FunctionEnd

; Hook de pos-instalacao (SEC-29: a unica chave de HKLM escrita pelo instalador, alem da entrada
; de "Aplicativos instalados" e dos atalhos).
Function SofaWriteInstallInfo
  ${If} $SofaStartupState == ""
    Call SofaInitialStartupState
  ${EndIf}
  WriteRegStr SHCTX "${MANUPRODUCTKEY}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD SHCTX "${MANUPRODUCTKEY}" "InstallDefaults" $SofaStartupState
FunctionEnd
