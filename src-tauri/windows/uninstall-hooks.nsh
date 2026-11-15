; =====================================================================================
; Sofa: desinstalador (ADR-0006 rev. 6, secoes 6a e 6b; SEC-25, SEC-29, SEC-31, SEC-32; DR-03, DR-06; D-43).
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

; ---------------------------------------------------------------------------------
; Usuario interativo (DR-03, E-19): o Explorer da sessao. Define SofaExplorerH: handle com
; PROCESS_CREATE_PROCESS | PROCESS_QUERY_LIMITED_INFORMATION, ou 0. Sem SeDebugPrivilege: se o
; DACL do Explorer nao deixar (UAC aprovado por OUTRA conta), fica 0 e o fluxo segue para o
; caminho alternativo.
; ---------------------------------------------------------------------------------
Function ${un}SofaOpenExplorer
  StrCpy $SofaExplorerH 0
  System::Call 'user32::GetShellWindow()p .r0'
  ${If} $0 <> 0
    System::Call 'user32::GetWindowThreadProcessId(p r0, *i .r1)i .r0'
    ${If} $1 <> 0
      System::Call 'kernel32::OpenProcess(i 0x1080, i 0, i r1)p .r2'
      StrCpy $SofaExplorerH $2
    ${EndIf}
  ${EndIf}
FunctionEnd

; SID do dono do token do processo. Entrada: handle. Saida: SID em texto ou "".
Function ${un}SofaSidOfProcess
  Exch $R0
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  StrCpy $4 ""
  ${If} $R0 <> 0
    System::Call 'advapi32::OpenProcessToken(p R0, i 8, *p .r0)i .r1'
    ${If} $1 <> 0
      ; TokenUser = 1. A 1a chamada so mede o buffer.
      System::Call 'advapi32::GetTokenInformation(p r0, i 1, p 0, i 0, *i .r2)i .r1'
      ${If} $2 > 0
        System::Alloc $2
        Pop $3
        System::Call 'advapi32::GetTokenInformation(p r0, i 1, p r3, i r2, *i .r2)i .r1'
        ${If} $1 <> 0
          System::Call '*$3(p .r1)'
          System::Call 'advapi32::ConvertSidToStringSidW(p r1, *p .r2)i .r1'
          ${If} $1 <> 0
            System::Call '*$2(&w128 .r4)'
            System::Call 'kernel32::LocalFree(p r2)'
          ${EndIf}
        ${EndIf}
        System::Free $3
      ${EndIf}
      System::Call 'kernel32::CloseHandle(p r0)'
    ${EndIf}
  ${EndIf}
  StrCpy $R0 $4
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
  Exch $R0
FunctionEnd

; SID do usuario da sessao deste processo (WTSUserName + WTSDomainName -> LookupAccountNameW).
; Segunda fonte, que nao precisa abrir o Explorer. Define SofaSidB: SID em texto ou "".
Function ${un}SofaSidFromSession
  StrCpy $SofaSidB ""
  System::Call 'kernel32::GetCurrentProcessId()i .r0'
  System::Call 'kernel32::ProcessIdToSessionId(i r0, *i .r1)i .r2'
  ${If} $2 = 0
    Return
  ${EndIf}
  System::Call 'wtsapi32::WTSQuerySessionInformationW(p 0, i r1, i 5, *p .r3, *i .r4)i .r2'
  ${If} $2 = 0
    Return
  ${EndIf}
  System::Call '*$3(&w256 .r5)'
  System::Call 'wtsapi32::WTSFreeMemory(p r3)'
  System::Call 'wtsapi32::WTSQuerySessionInformationW(p 0, i r1, i 7, *p .r3, *i .r4)i .r2'
  ${If} $2 = 0
    Return
  ${EndIf}
  System::Call '*$3(&w256 .r6)'
  System::Call 'wtsapi32::WTSFreeMemory(p r3)'
  ${If} $5 == ""
    Return
  ${EndIf}
  ${If} $6 != ""
    StrCpy $7 "$6\$5"
  ${Else}
    StrCpy $7 "$5"
  ${EndIf}
  System::Alloc 68
  Pop $0
  System::Alloc 1024
  Pop $1
  StrCpy $2 68
  StrCpy $3 512
  System::Call 'advapi32::LookupAccountNameW(p 0, w r7, p r0, *i r2, p r1, *i r3, *i .r4)i .r5'
  ${If} $5 <> 0
    System::Call 'advapi32::ConvertSidToStringSidW(p r0, *p .r2)i .r5'
    ${If} $5 <> 0
      System::Call '*$2(&w128 .r8)'
      System::Call 'kernel32::LocalFree(p r2)'
      StrCpy $SofaSidB $8
    ${EndIf}
  ${EndIf}
  System::Free $0
  System::Free $1
FunctionEnd

; Define SofaSid, SofaLocalApp e SofaRoot a partir do usuario INTERATIVO, nunca do %LOCALAPPDATA%
; do processo elevado (que pode ser de outra conta, E-19). Duas fontes (token do Explorer e sessao
; do processo); se as duas responderem e divergirem, nao resolve (falha fechada, SEC-25).
Function ${un}SofaResolveProfile
  StrCpy $SofaSid ""
  StrCpy $SofaLocalApp ""
  StrCpy $SofaRoot ""

  Push $SofaExplorerH
  Call ${un}SofaSidOfProcess
  Pop $R0
  Push $R0
  Call ${un}SofaIsValidSid
  Pop $R1
  ${If} $R1 <> 1
    StrCpy $R0 ""
  ${EndIf}

  Call ${un}SofaSidFromSession
  StrCpy $R2 $SofaSidB
  Push $R2
  Call ${un}SofaIsValidSid
  Pop $R1
  ${If} $R1 <> 1
    StrCpy $R2 ""
  ${EndIf}

  ${If} $R0 != ""
  ${AndIf} $R2 != ""
  ${AndIf} $R0 != $R2
    Return                       ; as fontes divergem: nada e apagado
  ${EndIf}
  ${If} $R0 == ""
    StrCpy $R0 $R2
  ${EndIf}
  ${If} $R0 == ""
    Return
  ${EndIf}

  ReadRegStr $R3 HKLM "${SOFA_PROFILELIST}\$R0" "ProfileImagePath"
  ${If} $R3 == ""
    Return
  ${EndIf}
  ExpandEnvStrings $R3 "$R3"
  StrCpy $SofaSid $R0
  StrCpy $SofaLocalApp "$R3\AppData\Local"
  StrCpy $SofaRoot "$SofaLocalApp\${BUNDLEID}"
FunctionEnd

; ---------------------------------------------------------------------------------
; Purge como o usuario interativo, com codigo de saida (ADR-0006 6b, caminho preferido).
; Entrada: SofaExplorerH. O filho nasce com o Explorer como processo pai (PARENT_PROCESS) e herda
; o token dele (Medium, usuario interativo); bInheritHandles = FALSE; linha de comando constante.
; ---------------------------------------------------------------------------------
Function ${un}SofaRunPurgeViaSystem
  StrCpy $SofaPurgeStarted 0
  StrCpy $SofaPurgeRead 0
  StrCpy $SofaPurgeCode ""
  StrCpy $SofaPurgeRunning 0
  !ifdef SOFA_TEST_FORCE_CREATE_FAIL
    ; so no harness: simula o CreateProcessW que falha (codigo nao lido)
    Return
  !endif
  ${If} $SofaExplorerH = 0
    Return
  ${EndIf}

  ; lista de atributos com 1 entrada
  System::Call 'kernel32::InitializeProcThreadAttributeList(p 0, i 1, i 0, *i .r1)i .r0'
  ${If} $1 <= 0
    Return
  ${EndIf}
  System::Alloc $1
  Pop $2                                   ; $2 = lista
  System::Call 'kernel32::InitializeProcThreadAttributeList(p r2, i 1, i 0, *i r1)i .r0'
  ${If} $0 = 0
    System::Free $2
    Return
  ${EndIf}
  System::Call '*(p $SofaExplorerH)p .r3'  ; $3 = ponteiro para o handle do pai
  ; PROC_THREAD_ATTRIBUTE_PARENT_PROCESS = 0x00020000
  System::Call 'kernel32::UpdateProcThreadAttribute(p r2, i 0, p 0x20000, p r3, p 4, p 0, p 0)i .r0'
  ${If} $0 <> 0
    ; STARTUPINFOEXW de 32 bits: cb=72 e lpAttributeList no fim
    System::Call '*(i 72, p 0, p 0, p 0, i 0, i 0, i 0, i 0, i 0, i 0, i 0, i 0, &i2 0, &i2 0, p 0, p 0, p 0, p 0, p r2)p .r4'
    System::Call '*(p 0, p 0, i 0, i 0)p .r5'   ; PROCESS_INFORMATION
    StrCpy $6 '"$INSTDIR\${MAINBINARYNAME}.exe" --purge-user-data'
    StrCpy $7 "$INSTDIR\${MAINBINARYNAME}.exe"
    ; EXTENDED_STARTUPINFO_PRESENT = 0x00080000; sem heranca de handles
    System::Call 'kernel32::CreateProcessW(w r7, w r6, p 0, p 0, i 0, i 0x80000, p 0, w "$INSTDIR", p r4, p r5)i .r0'
    ${If} $0 <> 0
      StrCpy $SofaPurgeStarted 1
      System::Call '*$5(p .r8, p .r9)'      ; hProcess, hThread
      System::Call 'kernel32::CloseHandle(p r9)'
      StrCpy $R8 0
      purge_wait:
        System::Call 'kernel32::WaitForSingleObject(p r8, i 1000)i .r0'
        ${If} $0 = 0                          ; WAIT_OBJECT_0: o purge terminou
          System::Call 'kernel32::GetExitCodeProcess(p r8, *i .r1)i .r0'
          ${If} $0 <> 0
            StrCpy $SofaPurgeRead 1
            StrCpy $SofaPurgeCode $1
          ${EndIf}
          Goto purge_waited
        ${EndIf}
        IntOp $R8 $R8 + 1
        ${If} $R8 < ${SOFA_PURGE_WAIT_SECONDS}
          Goto purge_wait
        ${EndIf}
        StrCpy $SofaPurgeRunning 1            ; passou o tempo e ainda roda: nada de fallback
      purge_waited:
      System::Call 'kernel32::CloseHandle(p r8)'
    ${EndIf}
    System::Free $5
    System::Free $4
  ${EndIf}
  System::Free $3
  System::Call 'kernel32::DeleteProcThreadAttributeList(p r2)'
  System::Free $2
FunctionEnd

; Caminho alternativo (RunAsUser do plugin + FindProcess em laco): o codigo nao e lido.
Function ${un}SofaRunPurgeViaRunAsUser
  StrCpy $SofaPurgeStarted 1
  StrCpy $SofaPurgeRead 0
  StrCpy $SofaPurgeCode ""
  StrCpy $SofaPurgeRunning 0
  nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" "--purge-user-data"
  Pop $0
  Sleep 1500
  StrCpy $R8 0
  alt_wait:
    nsis_tauri_utils::FindProcess "${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 <> 0                             ; nao achou: o purge terminou (ou nunca subiu)
      Return
    ${EndIf}
    Sleep 1000
    IntOp $R8 $R8 + 1
    ${If} $R8 < ${SOFA_PURGE_WAIT_SECONDS}
      Goto alt_wait
    ${EndIf}
  StrCpy $SofaPurgeRunning 1
FunctionEnd

; ---------------------------------------------------------------------------------
; Fallback restrito (D-43, DR-06). No maximo `library.json` e `settings.json` na raiz de
; SofaRoot, por handle. Sem curinga, sem subpasta, sem recursao, sem Delete do NSIS.
; Entrada: SofaLocalApp e SofaRoot (do usuario interativo). Saida: SofaDeleted.
; ---------------------------------------------------------------------------------
Function ${un}SofaFallbackDelete
  StrCpy $SofaDeleted 0
  ${If} $SofaLocalApp == ""
  ${OrIf} $SofaRoot == ""
    Return
  ${EndIf}

  ; caminho final do LocalAppData do usuario (referencia da conferencia "um nivel abaixo")
  System::Call 'kernel32::CreateFileW(w "$SofaLocalApp", i 0x80, i 7, p 0, i 3, i 0x02000000, p 0)p .r1'
  ${If} $1 = 0
  ${OrIf} $1 = -1
    Return
  ${EndIf}
  System::Call 'kernel32::GetFinalPathNameByHandleW(p r1, w .r2, i 2048, i 0)i .r3'
  System::Call 'kernel32::CloseHandle(p r1)'
  ${If} $3 = 0
    Return
  ${EndIf}
  StrCpy $4 $2 4
  ${If} $4 == "\\?\"
    StrCpy $2 $2 "" 4
  ${EndIf}
  ; o LocalAppData do perfil nao pode passar por junction nem symlink: o caminho final tem que ser o
  ; mesmo texto do perfil (falha fechada; perfil redirecionado de proposito cai em "nada apagado")
  StrCmp $2 $SofaLocalApp 0 fb_end
  StrCpy $R1 "$2\${BUNDLEID}"          ; caminho final esperado da raiz

  ; 1) abre a RAIZ sem FILE_SHARE_DELETE (nao pode ser renomeada nem trocada por junction enquanto
  ;    o handle vive). FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT.
  System::Call 'kernel32::CreateFileW(w "$SofaRoot", i 0x80, i 3, p 0, i 3, i 0x02200000, p 0)p .r5'
  ${If} $5 = 0
  ${OrIf} $5 = -1
    Return                               ; raiz ausente: nada a apagar
  ${EndIf}

  ; 2) a raiz nao pode ser ponto de reanalise e tem que ser pasta; o caminho final tem que ser
  ;    exatamente <LocalAppData final>\<identificador>
  System::Call '*(i 0, i 0)p .r6'
  System::Call 'kernel32::GetFileInformationByHandleEx(p r5, i 9, p r6, i 8)i .r7'
  System::Call '*$6(i .r8, i .r9)'
  System::Free $6
  ${If} $7 = 0
    Goto fb_close_root
  ${EndIf}
  IntOp $R2 $8 & 0x400
  ${If} $R2 <> 0
    Goto fb_close_root                   ; raiz e junction ou symlink: para
  ${EndIf}
  IntOp $R2 $8 & 0x10
  ${If} $R2 = 0
    Goto fb_close_root                   ; raiz nao e pasta
  ${EndIf}
  System::Call 'kernel32::GetFinalPathNameByHandleW(p r5, w .r2, i 2048, i 0)i .r3'
  ${If} $3 = 0
    Goto fb_close_root
  ${EndIf}
  StrCpy $4 $2 4
  ${If} $4 == "\\?\"
    StrCpy $2 $2 "" 4
  ${EndIf}
  StrCmp $2 $R1 0 fb_close_root          ; case-insensitive, como o NTFS
  StrCpy $R3 $2                          ; caminho final da raiz, guardado

  ; 3) cada um dos dois nomes (nada alem destes)
  StrCpy $R4 "library.json"
  Call ${un}SofaFallbackDeleteOne
  StrCpy $R4 "settings.json"
  Call ${un}SofaFallbackDeleteOne

  fb_close_root:
  System::Call 'kernel32::CloseHandle(p r5)'
  fb_end:
FunctionEnd

; Um arquivo da raiz. Entrada: $R4 = nome, $R3 = caminho final da raiz, $5 = handle aberto da
; raiz. Abre com DELETE | FILE_READ_ATTRIBUTES e FILE_FLAG_OPEN_REPARSE_POINT; confere pelo handle
; (nao e link nem pasta; caminho final = raiz final + nome; raiz final inalterada); apaga pelo handle.
Function ${un}SofaFallbackDeleteOne
  System::Call 'kernel32::CreateFileW(w "$SofaRoot\$R4", i 0x10080, i 7, p 0, i 3, i 0x00200000, p 0)p .r1'
  ${If} $1 = 0
  ${OrIf} $1 = -1
    Return                               ; ausente (ou nao e arquivo comum): ignorado
  ${EndIf}

  ; (a) nao e link nem pasta
  System::Call '*(i 0, i 0)p .r6'
  System::Call 'kernel32::GetFileInformationByHandleEx(p r1, i 9, p r6, i 8)i .r7'
  System::Call '*$6(i .r8, i .r9)'
  System::Free $6
  ${If} $7 = 0
    Goto one_close
  ${EndIf}
  IntOp $R2 $8 & 0x410                   ; REPARSE_POINT | DIRECTORY
  ${If} $R2 <> 0
    Goto one_close
  ${EndIf}

  ; (b) caminho final do arquivo = <raiz final>\<nome>
  System::Call 'kernel32::GetFinalPathNameByHandleW(p r1, w .r2, i 2048, i 0)i .r3'
  ${If} $3 = 0
    Goto one_close
  ${EndIf}
  StrCpy $4 $2 4
  ${If} $4 == "\\?\"
    StrCpy $2 $2 "" 4
  ${EndIf}
  StrCmp $2 "$R3\$R4" 0 one_close

  ; (c) a raiz, relida agora, nao mudou
  System::Call 'kernel32::GetFinalPathNameByHandleW(p r5, w .r2, i 2048, i 0)i .r3'
  StrCpy $4 $2 4
  ${If} $4 == "\\?\"
    StrCpy $2 $2 "" 4
  ${EndIf}
  StrCmp $2 $R3 0 one_close

  ; exclusao PELO HANDLE: FileDispositionInfoEx (21) com DELETE | POSIX_SEMANTICS | IGNORE_READONLY
  System::Call '*(i 0x13)p .r6'
  System::Call 'kernel32::SetFileInformationByHandle(p r1, i 21, p r6, i 4)i .r7'
  System::Free $6
  ${If} $7 = 0
    ; Windows anterior ao 1709: FileDispositionInfo (4), um BOOLEAN
    System::Call '*(&i1 1)p .r6'
    System::Call 'kernel32::SetFileInformationByHandle(p r1, i 4, p r6, i 1)i .r7'
    System::Free $6
  ${EndIf}
  ${If} $7 <> 0
    IntOp $SofaDeleted $SofaDeleted + 1
  ${EndIf}

  one_close:
  System::Call 'kernel32::CloseHandle(p r1)'
FunctionEnd

; Mostra a pasta ao usuario (aviso + Explorer como o usuario, nunca elevado). Entrada: $R5 = texto.
Function ${un}SofaShowFolder
  ${IfNot} ${Silent}
    MessageBox MB_OK|MB_ICONINFORMATION "$R5" /SD IDOK
    ${If} $SofaRoot != ""
      nsis_tauri_utils::RunAsUser "$WINDIR\explorer.exe" '"$SofaRoot"'
      Pop $R6
    ${EndIf}
  ${EndIf}
FunctionEnd

; ---------------------------------------------------------------------------------
; Decisao (ADR-0006 6b, tabela de codigos). Entrada: o estado do purge. Saida: SofaOutcome.
; ---------------------------------------------------------------------------------
Function ${un}SofaPurgeDecide
  StrCpy $SofaOutcome ""
  StrCpy $SofaDeleted 0

  ${If} $SofaPurgeRunning = 1
    StrCpy $SofaOutcome "nada:purge-em-andamento"
    StrCpy $R5 "$(sofaPurgeRunning)"
    Goto decide_show
  ${EndIf}

  ${If} $SofaPurgeRead = 1
    ${If} $SofaPurgeCode = 0
      StrCpy $SofaOutcome "ok:codigo-0"
      Return
    ${ElseIf} $SofaPurgeCode = 2
      StrCpy $SofaOutcome "nada:codigo-2-alvo-invalido"
      StrCpy $R5 "$(sofaPurgeInvalidTarget)"
      Goto decide_show
    ${ElseIf} $SofaPurgeCode = 4
      StrCpy $SofaOutcome "nada:codigo-4-sofa-aberto"
      StrCpy $R5 "$(sofaPurgeAppOpen)"
      Goto decide_show
    ${ElseIf} $SofaPurgeCode = 1
      StrCpy $SofaOutcome "fallback:codigo-1-parcial"
      StrCpy $R5 "$(sofaPurgePartial)"
    ${ElseIf} $SofaPurgeCode = 3
      StrCpy $SofaOutcome "fallback:codigo-3-elevado"
      StrCpy $R5 "$(sofaPurgePartial)"
    ${Else}
      StrCpy $SofaOutcome "fallback:codigo-desconhecido-$SofaPurgeCode"
      StrCpy $R5 "$(sofaPurgeUnread)"
    ${EndIf}
  ${Else}
    StrCpy $SofaOutcome "fallback:codigo-nao-lido"
    StrCpy $R5 "$(sofaPurgeUnread)"
  ${EndIf}

  ; fallback restrito: so com o perfil interativo resolvido (SEC-25)
  ${If} $SofaRoot == ""
    StrCpy $SofaOutcome "nada:perfil-nao-resolvido"
    StrCpy $R5 "$(sofaPurgeNoProfile)"
    Goto decide_show
  ${EndIf}
  Call ${un}SofaFallbackDelete
  StrCpy $SofaOutcome "$SofaOutcome:apagou-$SofaDeleted"

  decide_show:
  ; Sem pasta (perfil nao resolvido) nao ha o que mostrar abaixo: nada de linhas em branco no fim.
  ${If} $SofaRoot != ""
    StrCpy $R5 "$R5$\r$\n$\r$\n$SofaRoot"
  ${EndIf}
  Call ${un}SofaShowFolder
FunctionEnd

; Orquestracao: usuario interativo -> purge -> decisao.
Function ${un}SofaPurgeUserData
  DetailPrint "$(sofaPurgeStart)"
  Call ${un}SofaOpenExplorer
  Call ${un}SofaResolveProfile
  !ifdef SOFA_HARNESS
    ; so no harness (windows/tests): nunca apaga a pasta real do usuario
    ${If} $SofaTestLocalApp != ""
      StrCpy $SofaLocalApp $SofaTestLocalApp
      StrCpy $SofaRoot "$SofaLocalApp\${BUNDLEID}"
    ${EndIf}
  !endif
  StrCpy $SofaPurgeStarted 0
  StrCpy $SofaPurgeRead 0
  StrCpy $SofaPurgeRunning 0
  Call ${un}SofaRunPurgeViaSystem
  ${If} $SofaPurgeStarted = 0
    Call ${un}SofaRunPurgeViaRunAsUser
  ${EndIf}
  ${If} $SofaExplorerH <> 0
    System::Call 'kernel32::CloseHandle(p $SofaExplorerH)'
  ${EndIf}
  Call ${un}SofaPurgeDecide
FunctionEnd

!macroend

!ifndef SOFA_NO_AUTOINSERT
  !insertmacro SOFA_UNINSTALL_FUNCS "un."

  ; Hook de pre-desinstalacao (chamado depois de fechar o Sofa). Atualizar por cima nao passa
  ; por aqui (CA-53). Com "Manter dados" marcada (padrao, tambem sem tela), so o autostart sai.
  Function un.SofaBeforeUninstall
    ${If} $UpdateMode = 1
      Return
    ${EndIf}
    Call un.SofaRemoveAutostartAllProfiles
    ${If} $KeepDataCheckboxState = 1
      DetailPrint "$(sofaKeepData)"
      Return
    ${EndIf}
    Call un.SofaPurgeUserData
  FunctionEnd
!endif
