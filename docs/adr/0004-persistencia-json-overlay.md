---
status: accepted
date: 2026-11-13
revised: 2026-11-13 (pasta de dados movida para %LOCALAPPDATA%, identifier definido)
feature: launcher-tv
---

# Persistência: arquivos JSON por usuário em %LOCALAPPDATA%\app.sofa.launcher; jogos detectados não são persistidos como verdade

Sem servidor e sem banco (RN-01), os dados do usuário (RN-18) ficam em arquivos JSON versionados numa **única pasta por usuário: `%LOCALAPPDATA%\app.sofa.launcher\`**. É o `app_local_data_dir()` do Tauri, fica fora de `Program Files` e sobrevive a atualizações. Os jogos detectados (Steam, Epic, GOG) são **recalculados a cada atualização da biblioteca**. Fica persistido só o que é do usuário, como uma camada por cima, indexada pelo `GameId` (origem + identificador, RN-03): nome personalizado, oculto e último jogado (início da sessão). Jogos manuais são persistidos inteiros, porque não têm outra fonte.

```text
%LOCALAPPDATA%\app.sofa.launcher\
├─ library.json      manualGames[], overlay{GameId → {customName?, hidden, lastPlayedAt?}}, schemaVersion
├─ settings.json     monitorId?, installDefaultsApplied, schemaVersion
├─ cache\            scan.json, covers\, icons\ (pode ser apagado sem perda)
├─ logs\             5 MB × 3 arquivos
└─ EBWebView\        dados do WebView2 (padrão do Tauri)
```

O estado de "Iniciar com o Windows" **não** é copiado no JSON. A fonte única é o próprio mecanismo do Windows (tarefa agendada do usuário ou `HKCU\...\Run`, ADR-0006).

Escrita atômica (temporário na mesma pasta + `ReplaceFileW`/`MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`), feita por um único dono no núcleo. A leitura usa schema fechado (`deny_unknown_fields`, arquivo ≤ 4 MB, strings ≤ 1.024, ≤ 5.000 jogos manuais), e jogos manuais carregados passam de novo pela validação do cadastro (SEC-18). Arquivo inválido é renomeado para `*.corrupt-<data>.json` e recriado com os padrões (seção 11 da spec).

## Por que %LOCALAPPDATA% e não %APPDATA%

A primeira versão deste ADR usava `%APPDATA%` (Roaming). A segurança (SEC-18, SEC-25) aceita as duas, mas o desinstalador fica mais simples e seguro com **uma pasta exata** para validar e apagar, e os caches e o WebView2 já ficam em `%LOCALAPPDATA%` de qualquer jeito. Roaming só traria ganho com perfil móvel de domínio, que não existe num PC pessoal. Ficou tudo em `%LOCALAPPDATA%`.

## Opções consideradas

- **SQLite**: sobra para algumas centenas de registros, soma uma dependência nativa e não é legível pelo usuário. Descartada.
- **Registro do Windows**: ruim para listas e para limpeza. Descartada.
- **Persistir a biblioteca inteira, com os detectados**: obrigaria a reconciliar com a loja. Com a camada por cima, um jogo desinstalado some sozinho (CA-17, CA-58), e ocultos e nomes sobrevivem à volta da unidade (E-18). Além disso, jogos de loja nunca são lançados a partir de um caminho gravado (SEC-18). Descartada.

## Consequências

- O `identifier` `app.sofa.launcher` é definitivo: trocar exige migração da pasta. O nome exibido ("Sofá") pode mudar sem tocar nele.
- Mudanças de formato passam por `schemaVersion` + migração pura no núcleo, testada com fixtures.
- Overlay de um jogo que sumiu (unidade desconectada) não é apagado automaticamente.
