; =====================================================================================
; Sofa: pasta de instalacao FIXA (SEC-BUG-05, ARQ-BUG-16, SEC-20, SEC-32, ADR-0006).
;
; O instalador e por maquina e roda elevado. Se a pasta pudesse ser outra (pagina de pasta, `/D=`
; ou a instalacao anterior gravada no registro), uma pasta fora de Program Files herdaria a ACL
; da raiz da unidade (Usuarios autenticados podem modificar) e qualquer processo do usuario
; trocaria o `uninstall.exe` ou o `sofa.exe` que o instalador e o desinstalador executam elevados.
; Por isso a pasta e sempre `$PROGRAMFILES64\Sofa`: sem pagina de pasta, `/D=` diferente recusado,
; instalacao anterior em outra pasta recusada (antes de qualquer ExecWait do desinstalador).
;
; Entrada: $INSTDIR (o que o NSIS pos por `/D=`, ou o valor padrao) e $SofaPrevDir (pasta gravada em
; HKLM\Software\app.sofa.launcher pela instalacao anterior; vazio se nao ha).
; Saida: $SofaDirResult = 0 (ok; $INSTDIR e a pasta fixa), 1 (`/D=` diferente da fixa) ou
; 2 (instalacao anterior fora da pasta fixa). Quem chama mostra a mensagem e aborta.
; Sem ${INSTALLDIRNAME} e ${PLACEHOLDER_INSTALL_DIR} definidos antes deste arquivo, nao compila.
; =====================================================================================

!include LogicLib.nsh

Var SofaPrevDir
Var SofaDirResult

!macro SOFA_DIR_FUNCS un
Function ${un}SofaFixInstallDir
  Push $R8
  StrCpy $SofaDirResult 0
  StrCpy $R8 "$PROGRAMFILES64\${INSTALLDIRNAME}"
  ; sem `/D=` o $INSTDIR e o valor de espaco reservado do template: conta como vazio
  ${If} $INSTDIR == "${PLACEHOLDER_INSTALL_DIR}"
    StrCpy $INSTDIR ""
  ${EndIf}
  ${If} $INSTDIR != ""
  ${AndIf} $INSTDIR != $R8
    StrCpy $SofaDirResult 1
    Goto sofa_dir_done
  ${EndIf}
  ${If} $SofaPrevDir != ""
  ${AndIf} $SofaPrevDir != $R8
    StrCpy $SofaDirResult 2
    Goto sofa_dir_done
  ${EndIf}
  StrCpy $INSTDIR $R8
  sofa_dir_done:
  Pop $R8
FunctionEnd
!macroend

!insertmacro SOFA_DIR_FUNCS ""
!ifndef SOFA_NO_AUTOINSERT
  !insertmacro SOFA_DIR_FUNCS "un."
!endif
