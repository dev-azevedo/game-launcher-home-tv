; =====================================================================================
; Sofa: limpeza do atalho legado da 0.1.0, "Sofa.lnk" com acento agudo no a
; (D-57, CA-53, ADR-0007, INV-09, SEC-29, SEC-32).
;
; Com o nome novo o instalador cria "SofaLauncher.lnk". O desinstalador da 0.1.0, chamado em
; `/UPDATE` pelo PageReinstall, nao remove atalhos, entao o "Sofa.lnk" com acento agudo ficaria ao lado do novo.
; Esta limpeza apaga o atalho antigo SO se `IsShortcutTarget` confirmar que ele aponta para
; `$INSTDIR\${MAINBINARYNAME}.exe` (o mesmo macro e o mesmo `UnpinShortcut` do desinstalador).
;
;   - caminho fixo: <pasta>\Sofa.lnk (acento agudo no a), sem curinga e sem varrer pasta;
;   - so `$SMPROGRAMS` e `$DESKTOP` (no contexto do instalador: todos os usuarios);
;   - atalho ausente, ilegivel ou apontando para outro exe: nada acontece, sem erro;
;   - roda tambem em `/UPDATE` (chamada pelo NSIS_HOOK_POSTINSTALL, que nao depende do modo).
;
; O nome com acento NAO esta escrito nesta fonte: ela e UTF-8 sem BOM (o bundler so grava o BOM no
; que copia) e o NSIS leria o "a" agudo como ANSI. O caractere sai de `IntFmt "%c"` (U+00E1).
; Precisa de MAINBINARYNAME e dos macros de `utils.nsh` (IsShortcutTarget) e `Integration.nsh`
; (UnpinShortcut) incluidos antes.
; =====================================================================================

!include LogicLib.nsh

; Entrada: pilha = pasta. Saida: pilha = 1 se removeu o atalho, 0 se nao.
Function SofaRemoveLegacyLnkIn
  Exch $R0
  Push $R1
  Push $R2
  Push $0
  Push $1
  Push $2
  Push $3
  StrCpy $R1 0
  IntFmt $0 "%c" 225
  StrCpy $R2 "$R0\Sof$0.lnk"
  ${If} ${FileExists} "$R2"
    !insertmacro IsShortcutTarget "$R2" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $1
    ${If} $1 = 1
      !insertmacro UnpinShortcut "$R2"
      Delete "$R2"
      StrCpy $R1 1
    ${EndIf}
  ${EndIf}
  Pop $3
  Pop $2
  Pop $1
  Pop $0
  Pop $R2
  Exch $R1
  Exch
  Pop $R0
FunctionEnd

Function SofaRemoveLegacyShortcuts
  Push $0
  Push "$SMPROGRAMS"
  Call SofaRemoveLegacyLnkIn
  Pop $0
  Push "$DESKTOP"
  Call SofaRemoveLegacyLnkIn
  Pop $0
  Pop $0
FunctionEnd
