; =====================================================================================
; Sofa: desinstalador (ADR-0006 rev. 6, secao 6a; SEC-29, SEC-32; DR-03). Etapa BE-42: autostart de todos os perfis.
; (Os dados do usuario, purge e fallback, entram no BE-38 neste mesmo arquivo.)
;
; Dono do guard NSIS `System::Call` (lista NsisSystemCallOwners em scripts/ci-guards.ps1): so este
; arquivo chama `System::Call` de API que cria processo ou apaga. Nenhum `Delete` nem `RMDir` do
; NSIS toca a pasta de dados do usuario (guard SEC-31).
;
; O processo elevado SO DECIDE. Quem apaga os dados e o `sofa.exe --purge-user-data`, iniciado como
; o usuario interativo (filho do Explorer da sessao, token Medium). Se isso nao der certo, o fallback
; apaga NO MAXIMO `library.json` e `settings.json` na raiz da pasta de dados, por handle, sem curinga.
;
; As funcoes saem de uma macro para existirem com e sem o prefixo `un.`: o desinstalador usa as
; `un.*`; o harness de teste (windows/tests/harness.nsi, sem elevacao, em pasta temporaria) usa as
; mesmas, como instalador comum. Nada aqui e executado no instalador de producao.
;
; Convencao: as funcoes sujam $0-$9 e $R0-$R9 (o hook roda isolado, no inicio do Section Uninstall).
; Tamanhos de estrutura valem para o stub de 32 bits do NSIS (o sofa.exe e de 64, mas quem chama
; CreateProcessW e o stub).
; =====================================================================================

!include LogicLib.nsh

!define SOFA_PROFILELIST "SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList"
!define SOFA_TASK_PREFIX "Sofa Autostart ("
!define SOFA_PURGE_WAIT_SECONDS 60

; Estado (decisao do purge). Declarado uma vez para o desinstalador e o harness.
Var SofaExplorerH    ; handle do Explorer da sessao (0 = nao abriu)
Var SofaSidB         ; SID da sessao (2a fonte)
Var SofaSid          ; SID validado do usuario interativo ("" = nao resolvido)
Var SofaLocalApp     ; <perfil>\AppData\Local do usuario interativo
Var SofaRoot         ; <LocalApp>\app.sofa.launcher
Var SofaPurgeStarted ; 1 = o CreateProcessW do purge deu certo
Var SofaPurgeRead    ; 1 = GetExitCodeProcess devolveu um codigo valido
Var SofaPurgeCode    ; o codigo (so vale com SofaPurgeRead = 1)
Var SofaPurgeRunning ; 1 = passou o tempo e o purge ainda roda
Var SofaDeleted      ; quantos arquivos o fallback apagou
Var SofaOutcome      ; texto curto do resultado (log e testes)
!ifdef SOFA_HARNESS
  Var SofaTestLocalApp ; so no harness: LocalAppData de mentira (pasta temporaria)
!endif

!macro SOFA_UNINSTALL_FUNCS un

; ---------------------------------------------------------------------------------
; SID valido: ^S-1-5-21(-[0-9]{1,10}){4}$ (SEC-32). Entrada: SID na pilha. Saida: 1 ou 0.
; Caractere a caractere: so digitos e hifens, sem aspas, espaco nem barra.
; ---------------------------------------------------------------------------------
Function ${un}SofaIsValidSid
  Exch $R0            ; SID
  Push $R1            ; indice
  Push $R2            ; caractere
  Push $R3            ; digitos do grupo atual
  Push $R4            ; grupos fechados
  Push $R5            ; resultado
  StrCpy $R5 0

  StrCpy $R1 $R0 9
  StrCmpS $R1 "S-1-5-21-" 0 sid_end
  StrCpy $R1 9
  StrCpy $R3 0
  StrCpy $R4 0
  sid_loop:
    StrCpy $R2 $R0 1 $R1
    StrCmp $R2 "" sid_eos
    StrCmp $R2 "-" sid_dash
    StrCmp $R2 "0" sid_digit
    StrCmp $R2 "1" sid_digit
    StrCmp $R2 "2" sid_digit
    StrCmp $R2 "3" sid_digit
    StrCmp $R2 "4" sid_digit
    StrCmp $R2 "5" sid_digit
    StrCmp $R2 "6" sid_digit
    StrCmp $R2 "7" sid_digit
    StrCmp $R2 "8" sid_digit
    StrCmp $R2 "9" sid_digit
    Goto sid_end                 ; qualquer outro caractere reprova
  sid_digit:
    IntOp $R3 $R3 + 1
    ${If} $R3 > 10
      Goto sid_end
    ${EndIf}
    IntOp $R1 $R1 + 1
    Goto sid_loop
  sid_dash:
    ${If} $R3 < 1
      Goto sid_end
    ${EndIf}
    IntOp $R4 $R4 + 1
    ${If} $R4 > 3
      Goto sid_end
    ${EndIf}
    StrCpy $R3 0
    IntOp $R1 $R1 + 1
    Goto sid_loop
  sid_eos:
    ${If} $R4 = 3
    ${AndIf} $R3 >= 1
      StrCpy $R5 1
    ${EndIf}
  sid_end:
  StrCpy $R0 $R5
  Pop $R5
  Pop $R4
  Pop $R3
  Pop $R2
  Pop $R1
  Exch $R0
FunctionEnd

; ---------------------------------------------------------------------------------
; BE-42 / SEC-32: autostart de UM perfil. Entrada: SID na pilha (revalidado aqui).
; Tarefa `Sofa Autostart (<SID>)` por "$SYSDIR\schtasks.exe" (caminho absoluto, sem cmd, nome so
; de constante + SID validado) e os valores `Sofa` do Run e do StartupApproved em HKU\<SID>
; (so existe com o hive carregado; hive nao carregado deixa entrada orfa que o Windows ignora).
; ---------------------------------------------------------------------------------
Function ${un}SofaRemoveAutostartForSid
  Exch $R0
  Push $R1
  Push $R0
  Call ${un}SofaIsValidSid
  Pop $R1
  ${If} $R1 = 1
    nsExec::ExecToLog '"$SYSDIR\schtasks.exe" /Delete /TN "${SOFA_TASK_PREFIX}$R0)" /F'
    Pop $R1
    DeleteRegValue HKU "$R0\Software\Microsoft\Windows\CurrentVersion\Run" "Sofa"
    DeleteRegValue HKU "$R0\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Sofa"
  ${Else}
    DetailPrint "$(sofaSidIgnored)"
  ${EndIf}
  Pop $R1
  Pop $R0
FunctionEnd

; Todos os perfis da ProfileList (cobre tambem usuario sem hive carregado). DR-03: o autostart sai
; de todos os perfis porque o binario deixa de existir; os dados, nao.
Function ${un}SofaRemoveAutostartAllProfiles
  Push $R0
  Push $R1
  StrCpy $R0 0
  profiles_loop:
    EnumRegKey $R1 HKLM "${SOFA_PROFILELIST}" $R0
    StrCmp $R1 "" profiles_done
    IntOp $R0 $R0 + 1
    Push $R1
    Call ${un}SofaRemoveAutostartForSid
    Goto profiles_loop
  profiles_done:
  Pop $R1
  Pop $R0
FunctionEnd

!macroend

!ifndef SOFA_NO_AUTOINSERT
  !insertmacro SOFA_UNINSTALL_FUNCS "un."

  ; Hook de pre-desinstalacao (BE-42): o autostart de todos os perfis sai sempre; atualizar por cima
  ; nao passa por aqui (CA-53). Os dados do usuario entram no BE-38.
  Function un.SofaBeforeUninstall
    ${If} $UpdateMode = 1
      Return
    ${EndIf}
    Call un.SofaRemoveAutostartAllProfiles
  FunctionEnd
!endif
