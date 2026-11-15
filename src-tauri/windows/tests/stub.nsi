; Stub do `sofa.exe` para o harness do desinstalador (windows/tests/harness.nsi). Nao faz purge
; nenhum: registra como foi iniciado e sai com o codigo pedido.
;
;   <pasta do exe>\exitcode.txt   codigo de saida (se nao existir, 0)
;   <pasta do exe>\sleep.txt      se existir, dorme 3 s antes de sair (da tempo de olhar o processo)
;   <pasta do exe>\stub.log       linha de comando, pid, integridade e SID do dono do token
Unicode true
RequestExecutionLevel user
SilentInstall silent
OutFile "${OUT_EXE}"

!include LogicLib.nsh

Section
  FileOpen $1 "$EXEDIR\stub.log" w
  FileWrite $1 "args=$CMDLINE$\r$\n"
  System::Call 'kernel32::GetCurrentProcessId()i .r2'
  FileWrite $1 "pid=$2$\r$\n"

  System::Call 'kernel32::GetCurrentProcess()p .r3'
  System::Call 'advapi32::OpenProcessToken(p r3, i 8, *p .r4)i .r5'
  ${If} $5 <> 0
    ; TokenIntegrityLevel = 25: o ultimo sub-identificador do SID e o nivel (0x2000 = Medium)
    System::Alloc 512
    Pop $6
    System::Call 'advapi32::GetTokenInformation(p r4, i 25, p r6, i 512, *i .r7)i .r5'
    ${If} $5 <> 0
      System::Call '*$6(p .r7)'
      System::Call 'advapi32::GetSidSubAuthorityCount(p r7)p .r8'
      System::Call '*$8(&i1 .r8)'
      IntOp $8 $8 - 1
      System::Call 'advapi32::GetSidSubAuthority(p r7, i r8)p .r8'
      System::Call '*$8(i .r8)'
      FileWrite $1 "integrity=$8$\r$\n"
    ${EndIf}
    ; TokenUser = 1
    System::Call 'advapi32::GetTokenInformation(p r4, i 1, p r6, i 512, *i .r7)i .r5'
    ${If} $5 <> 0
      System::Call '*$6(p .r7)'
      System::Call 'advapi32::ConvertSidToStringSidW(p r7, *p .r8)i .r5'
      ${If} $5 <> 0
        System::Call '*$8(&w128 .r9)'
        FileWrite $1 "user=$9$\r$\n"
      ${EndIf}
    ${EndIf}
    System::Free $6
  ${EndIf}
  FileClose $1

  ${If} ${FileExists} "$EXEDIR\sleep.txt"
    Sleep 3000
  ${EndIf}

  StrCpy $0 0
  ClearErrors
  FileOpen $3 "$EXEDIR\exitcode.txt" r
  ${IfNot} ${Errors}
    FileRead $3 $0
    FileClose $3
    IntOp $0 $0 + 0
  ${EndIf}
  SetErrorLevel $0
SectionEnd
