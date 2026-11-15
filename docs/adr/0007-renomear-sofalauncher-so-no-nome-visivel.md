---
status: accepted (D-56: identificadores congelados, `Uninstall\Sofa` literal travado por teste; D-57: atualização por cima, versão 0.2.0, limpeza do Sofá.lnk com IsShortcutTarget)
date: 2026-11-15
feature: launcher-tv
---

# SofaLauncher muda só o nome visível; identificador, binário, pasta e chaves técnicas ficam congelados

D-49 renomeia o produto de "Sofá" para **SofaLauncher** (sigla SL). O nome antigo está em dois tipos de lugar. Um é o que o usuário lê: atalho, "Aplicativos instalados", título da janela, Gerenciador de Tarefas, textos da UI e do instalador. O outro é o que o sistema usa para achar o app: identificador, pasta de dados, mutex, binário, pasta de instalação e chaves de registro. A 0.1.0 só existe em até dois PCs (o de dev e talvez o da sala).

## Decisão

Trocar o primeiro tipo e congelar o segundo.

**Muda (visível):** `productName` = `SofaLauncher` (ASCII), `publisher`, `copyright` e `shortDescription` no `tauri.conf.json`; `DISPLAYNAME` no `installer.nsi` e no `harness.nsi`; título da janela (`window.rs`), `<title>` e `alt` da marca na UI; textos do `PortugueseBR.nsh`, da `microcopy.md` e da UI; nome do instalador e notas do release (`ci-release.ps1`); textos do `coletar-logs.ps1`; ícone SL (designer).

**Fica (técnico):** `identifier` `app.sofa.launcher` (pasta de dados, `HKLM\Software\app.sofa.launcher`, mutex `app.sofa.launcher-sim`, AUMID, alvo do purge); `mainBinaryName` `sofa` (`sofa.exe`, ação do Run e da tarefa, `exe_matches`, linha de comando do purge); `INSTALLDIRNAME` `Sofa` (`$PROGRAMFILES64\Sofa`, `SofaFixInstallDir`); valor `Sofa` do Run e do `StartupApproved`; tarefa `Sofa Autostart (<SID>)`; `sofa.log`; nomes de crate, variáveis NSIS e IDs de string.

**Ponto que exige cuidado:** no template, `UNINSTKEY` deriva de `${PRODUCTNAME}`. Se só o `productName` mudar, a chave `Uninstall\Sofa` vira `Uninstall\SofaLauncher`. A 0.2.0 deixaria de achar a 0.1.0, as duas apareceriam em "Aplicativos instalados", e a conferência do SEC-BUG-05 (`PageReinstall`) não rodaria contra o desinstalador antigo. Por isso o template fixa `UNINSTKEY` no literal `...\Uninstall\Sofa`, com comentário, e um teste do `test-installer.ps1` trava esse valor.

**Atualização de quem tem a 0.1.0:** instalar a 0.2.0 por cima. A pasta, a chave `Uninstall`, o `MANUPRODUCTKEY` e o binário são os mesmos, então o `PageReinstall` roda o `uninstall.exe /UPDATE` da pasta fixa (o SEC-BUG-05 continua valendo). Dados e autostart ficam (CA-53). O único resto é o atalho `Sofá.lnk`, que o desinstalador em `/UPDATE` não remove. A instalação nova apaga `Sofá.lnk` (menu Iniciar e área de trabalho) só se `IsShortcutTarget` confirmar que ele aponta para `$INSTDIR\sofa.exe`, com o mesmo macro e o mesmo `UnpinShortcut` que o desinstalador já usa. Não há migração de dados nem de pasta.

## Alternativas

- **Renomear tudo** (`app.sofalauncher`, `sofalauncher.exe`, `$PROGRAMFILES64\SofaLauncher`): rejeitada. Exige migrar a pasta de dados, e o purge e o SEC-25 dependem do identificador fixo. A pasta nova cai no resultado 2 do `SofaFixInstallDir` (instalação anterior fora da pasta fixa) e o instalador aborta. O mutex muda, então 0.1.0 e 0.2.0 abertas ao mesmo tempo não se enxergam. Run e tarefa antigos apontam para um exe que some. Muito custo para algo que o usuário não vê.
- **Manter o `productName` "Sofa" e trocar só o `DISPLAYNAME`:** rejeitada. O `FileDescription` do `sofa.exe` (Gerenciador de Tarefas e aba Inicializar) e o nome do instalador continuariam "Sofa".
- **Desinstalar a 0.1.0 à mão (mantendo dados) e instalar a 0.2.0:** não precisa de código para o atalho, mas o desinstalador remove o autostart. Como `installDefaultsApplied` já está gravado no `settings.json`, a 0.2.0 não religa o autostart sozinha e o usuário tem de religar em Configurações. Fica como plano B se a limpeza do atalho for recusada.

## Consequências

- Nome visível e nome técnico divergem para sempre (`SofaLauncher` vs. `sofa`, `app.sofa.launcher`). Quem lê log, registro ou o Agendador vê "Sofa". O glossário registra essa diferença.
- Trocar o identificador depois continua possível, mas exige migração e uma nova revisão de segurança. Esta decisão adia esse custo e não o cria.
- `src-tauri/tests/command_list.rs` deixa de travar `productName == "Sofa"` e passa a travar `identifier`, `mainBinaryName` e o literal de `UNINSTKEY`.
- A versão tem de subir (0.2.0). Com a mesma versão, o `PageReinstall` cai em "Desinstalar" de verdade.
