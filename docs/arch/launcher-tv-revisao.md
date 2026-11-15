# Revisão de arquitetura: launcher-tv (ARQ-05, rodadas 1 a 3)

> **Rodada 3 (2026-11-15):** ciclo de correção 2 e leva 5 (instalador, desinstalador, release). Fecharam os ARQ-BUG-07, 08, 13 e 14. São novos os ARQ-BUG-15 (modo Job: o fim da sessão depende de uma espera de 500 ms) e 16 (o instalador aceita pasta fora de `Program Files`), ambos médios. Ver a seção "Rodada 3" no fim.

> **Rodada 2 (2026-11-14):** conferência do ciclo de correção 1 (`git diff 85b3174..HEAD`). Fecharam os ARQ-BUG-01 a 06 e 09 a 12. Os ARQ-BUG-07, 08 e 13 seguem abertos como BE-44, BE-46 e BE-45. Há um novo, o ARQ-BUG-14 (baixo). Ver a seção "Rodada 2" no fim deste documento.

| Campo | Valor |
|---|---|
| Diff | `git diff main..feature/launcher-tv-20261113` (repo único, 51 commits até `85b3174`) |
| Base de referência | `docs/arch/launcher-tv.md` (INV-01 a INV-09, 2b, seção 6), ADR-0001 a 0006, spikes, `docs/api/launcher-tv.md` §8 a §10 |
| Data | 2026-11-14 |
| Escopo desta rodada | núcleo (BE-06 a BE-36, BE-40, BE-41) e UI (FE-01 a FE-14). **Fora**: instalador e release (BE-37 a BE-39), BE-42, BE-43 (padrão final dos degraus, SG-10) e QA-13 |
| Veredito | **aprovado com ressalvas** (rodada parcial). Nenhum ARQ-BUG alto. Os dois médios (ARQ-BUG-01 e 02) devem ser corrigidos antes do QA-17 |

Conferido nesta rodada: direção de dependência (INV-01: `launcher-core` sem `tauri`/`windows`, 298 testes de domínio e aplicação passam em 0,09 s, sem I/O), lista fechada de 30 comandos e capability única (4.1 e 4.3: `capabilities/main.json` com os 30 `allow-*` + `core:event:allow-listen/unlisten`, sem `core:default`, sem plugin com permissão JS), CSP igual à 4.4, `command_list.rs` como fonte única, guards de CI (`scripts/ci-guards.ps1`), lazy load de `Settings`, `FolderBrowser` e `RenameDialog`, grade virtualizada acima de 60 itens, termos da spec na UI ("Sair do launcher", "Monitor do launcher", "Ocultar", "Itens ocultos").

## ARQ-BUGs

| ID | Sev. | Responsável | Arquivo:linha | Problema | Correção sugerida |
|---|---|---|---|---|---|
| ARQ-BUG-01 | médio | dev-backend | `crates/launcher-core/src/domain/session/machine.rs:345-347`; `crates/launcher-core/src/application/session_service.rs:449-476` | CA-29 manda o Sofá "voltar ao primeiro plano" quando o início não é confirmado em 60 s. A transição `Launching → NotConfirmed` não gera efeito nenhum: com a tela da loja (login, atualização, E-03/E-16) na frente, a mensagem "Não foi possível confirmar" fica atrás dela e o controle continua indo para a loja | Efeito `BringLauncherFront` (sem `focusGameId` e sem `game_ended`) na entrada em `NotConfirmed`, executado no `TaskRunner` como em `on_ended`, com `set_mode(Foreground)`. Teste de aplicação com `WindowSystem` fake. A arquitetura 2.4 já foi atualizada |
| ARQ-BUG-02 | médio | dev-backend | `session_service.rs:449-476` (`apply` com o `Mutex` preso), `:483-484` (`on_running`), `:512` (`on_ended`); `src-tauri/src/launcher_window.rs:114` e `:137`; `src-tauri/src/single_instance.rs:30-41` | Os efeitos rodam com o `Mutex` da sessão preso: `recorder.mark_played` (lê e grava `library.json` atomicamente), `hide_and_suspend` e `show_and_resume`. Esse último chama `DisplayService::on_launcher_shown` → `TauriLauncherPlacement::place` → `window.is_visible()`, que bloqueia esperando a thread da UI. O callback da instância única roda na thread da UI e pega o mesmo `Mutex` (`session.get()`, `handle_return_combo`). Com uma troca de monitor pendente, se uma segunda instância abrir quando o jogo termina, o Sofá trava (deadlock). Mesmo sem isso, `session_get` espera a gravação em disco. O módulo diz "nenhuma chamada demorada roda com o Mutex preso" (2.6), mas não é o que acontece | `apply` devolve a lista de ações e quem chama executa depois de soltar o guard (ou manda para o `TaskRunner`). `DisplayService` guarda se a janela está visível (já recebe `hide`/`show`) em vez de perguntar ao Tauri com getter bloqueante. Teste: `LauncherWindow` fake que tenta `get()` no serviço dentro de `show_and_resume` não pode travar |
| ARQ-BUG-03 | baixo | dev-backend | `crates/launcher-core/src/application/library_service.rs:102-114`, `:219-232` | Toda alteração (renomear, ocultar, `mark_played`) relê e reparseia `library.json` do disco (até 4 MB) em vez de usar a cópia em memória do dono único. Custo repetido (hoje com o `Mutex` da sessão preso, ARQ-BUG-02). Se o arquivo for corrompido por fora no meio da execução, a próxima alteração recria os padrões e grava por cima do overlay válido que estava em memória | Manter o `StoredLibrary` em memória (carregado uma vez, como já é o snapshot) e só gravar. Teste: corromper o arquivo depois da carga e renomear preserva os outros overlays |
| ARQ-BUG-04 | baixo | dev-backend | `crates/launcher-core/src/domain/session/process_policy.rs:73`; `domain/autostart.rs:52-76`; `domain/data_dir_policy.rs:76`; imports em `domain/library/detect.rs:17` e `install_path_policy.rs:7` | "Caminho dentro da raiz, por componente, sem caixa" tem três implementações (política de sessão, DR-04 do autostart, SEC-25 do purge), todas de segurança. Correção feita em uma (ex.: prefixo `\\?\`, ponto final em componente) não chega às outras. Além disso, `library` depende de `session` só para essa função | Extrair `domain/path.rs` (`path_key`, `path_is_within`, `is_direct_child`) e reaproveitar nos três. Teste por tabela único |
| ARQ-BUG-05 | baixo | dev-backend | `src-tauri/src/runtime.rs:161` (`SystemEvents::spawn(..., true)`); `src-tauri/src/events.rs:233` | O hook `EVENT_SYSTEM_FOREGROUND` é instalado, mas `ForegroundChanged` é descartado. A cada troca de janela no Windows, o callback abre o processo da frente e lê o caminho da imagem sem uso nenhum: é superfície de SEC-28 sem consumidor | Passar `false` até existir consumidor (a arquitetura 2.1 foi atualizada: o MVP não usa o evento). Se um dia for usado, lembrar que `WINEVENT_SKIPOWNPROCESS` não avisa quando o Sofá volta para a frente |
| ARQ-BUG-06 | baixo | dev-backend | `scripts/ci-guards.ps1:32-37` e `:91` | O guard de INV-09 cobre remover e renomear, mas não criar ou gravar (`fs::write`, `File::create`, `OpenOptions`, `create_dir_all`, `CreateFileW` com acesso de escrita). A varredura também para no **primeiro** `#[cfg(test)]` do arquivo: `stores/registry.rs:106` tem `#[cfg(test)] pub(crate) mod fake` no meio do arquivo. Hoje nada de produção fica depois dele, mas é uma exclusão silenciosa | Acrescentar esses padrões com os donos de INV-09 (`persistence/json.rs`, `logging.rs`, `cover_cache.rs`, `purge.rs`). Pular só o bloco do item `#[cfg(test)]`, ou falhar se houver código não-teste depois dele. Plantar os casos novos no autoteste |
| ARQ-BUG-07 | baixo | dev-backend | `crates/launcher-win/src/input_thread.rs:162-170`; `crates/launcher-win/src/logging.rs:101-136` | INV-04 (caminho quente de input sem I/O): a thread de entrada chama `tracing::info!` (nova latência máxima, combinação detectada), e o escritor de log é síncrono, atrás de um `Mutex` global, com rotação por `rename`. É raro, mas a thread pode esperar disco ou outra thread que está logando, justo no caminho do M-04 | Pôr um escritor não bloqueante na frente do `RotatingFile` (thread própria com canal; `tracing-appender::non_blocking` serve, e a rotação por tamanho continua própria). Ou mandar esses eventos por canal para fora da thread de entrada |
| ARQ-BUG-08 | baixo | dev-backend | `src-tauri/tests/hardening.rs:85-93`; `src-tauri/build.rs:8-13` | O `sofa.exe` do build `-gnu` tem **dois** `RT_MANIFEST` com ID 1 e idioma 1033: o nosso (763 bytes, com Common Controls v6 e `supportedOS`) e o padrão do MinGW (599 bytes, sem Common Controls). É a origem do aviso ".rsrc merge failure" (desvio 8). O loader usa um deles; hoje o primeiro é o nosso. O teste só procura o texto `asInvoker` no binário e passa com os dois, inclusive se o ativo fosse o do MinGW, em que `TaskDialogIndirect` não carrega | O teste deve ler o diretório de recursos e exigir **exatamente um** `RT_MANIFEST`/1, com `asInvoker`, `uiAccess="false"` e Common Controls v6. Rodar no build MSVC de release como trava do BE-39. No `-gnu` o teste pode só avisar |
| ARQ-BUG-09 | baixo | dev-frontend | `ui/src/shared/ipc/types.ts:42-69` e `:74-105` | `CONTRACT_ERROR_CODES` e `COMMAND_NAMES` foram escritos à mão e duplicam o que é gerado (`generated/commandNames.ts`, `generated/ErrorCode.ts`). Com um comando ou código novo, a lista manual fica velha sem nenhum teste falhar contra o gerado | Reexportar `COMMAND_NAMES` de `generated/`. Para os códigos, `as const satisfies readonly ErrorCode[]` mais checagem de exaustividade, ou gerar a lista no `bindings.rs` |
| ARQ-BUG-10 | baixo | dev-frontend | `ui/src/features/library/store.ts:59` (`selectGameById`, sem uso) contra `features/session/overlay.ts:105`, `session/panel.ts:9`, `session/LaunchFlow.tsx:46`, `session/CloseConfirmDialog.tsx:10`, `session/SessionPanel.tsx:32`, `session/store.ts:71`, `library/HideConfirmDialog.tsx:18`, `library/GameOptions.tsx:22`, `library/RenameDialog.tsx:25` | A busca "jogo por id → nome, com texto padrão" está repetida em 9 lugares, com três textos de reserva diferentes ("o jogo", "O jogo", `undefined`), e o seletor que já existe não é usado | Um `gameName(games, id, reserva)` em `features/library/domain.ts`, e `selectGameById` nos componentes |
| ARQ-BUG-11 | baixo | dev-frontend (com dev-backend se o contrato mudar) | `ui/src/features/library/domain.ts:9`, `:13-22` contra `crates/launcher-core/src/domain/library/model.rs:56-77` | A RN-05 está implementada duas vezes: o núcleo ordena (NFD sem acento) e a UI reordena tudo com `Intl.Collator('pt-BR', { numeric: true })`. As duas ordens divergem ("Jogo 10" e "Jogo 2", pontuação, ß) e, na prática, a ordem do núcleo nunca aparece na tela | A UI deixa de reordenar o que já vem ordenado. A fileira "Recentes" é filtro e corte da lista do núcleo, que já vem com os jogados primeiro. Para a fileira A–Z com jogados misturados, alinhar o `Collator` ao `sort_key` do núcleo (sem `numeric`) e ter uma tabela de nomes comum aos dois testes, ou pedir ao dev-backend um campo de ordem alfabética (mudança de contrato, passa por SG) |
| ARQ-BUG-12 | baixo | dev-backend | `src-tauri/src/commands/library.rs:34-50` contra `:79-99` | `library_refresh` copia o corpo de `spawn_refresh` (thread, `catch_unwind`, log), que fica no mesmo arquivo | `library_refresh` chama `spawn_refresh`. Se a marca `navegavel` deve ser só a primeira, `marks::navigable` já ignora as seguintes; senão, passar uma flag |
| ARQ-BUG-13 | baixo | dev-backend | `src-tauri/src/settings_store.rs:23-37` | A regra de consistência das configurações (alterar uma cópia, gravar, só então trocar a memória, `advanced` intacto) mora na apresentação, sobre o `AppState`. O módulo **Settings** da 2.1 é do núcleo | Mover para um `SettingsService` em `launcher-core/application` sobre `SettingsRepository` (testável com fake). O `AppSettingsStore` vira só a ligação no composition root |

## Veredito sobre os desvios dos devs

| # | Desvio | Veredito | Onde ficou registrado |
|---|---|---|---|
| 1 | `SystemEvents` com janela de topo oculta, não `HWND_MESSAGE` | **Aceito.** Janela só de mensagem não recebe broadcast, e `WM_DISPLAYCHANGE` e `WM_POWERBROADCAST` chegam por broadcast. A janela é `WS_POPUP` sem `WS_VISIBLE`, `WS_EX_TOOLWINDOW \| WS_EX_NOACTIVATE`, nunca mostrada, e não entra no Alt+Tab nem na barra de tarefas. Ressalva: o hook de primeiro plano da mesma thread não tem consumidor (ARQ-BUG-05) | arquitetura 2.1 (SystemEvents) |
| 2 | Sem a exclusão "filho direto do Sofá" no `classify` | **Aceito, conforme.** SEC-17, ADR-0005 e a 2.4 sempre disseram "o Sofá e seus filhos **WebView2**". Excluir todo filho direto tiraria do conjunto o jogo manual ou GOG e a Steam aberta pelo Sofá (que já cai como loja). A exclusão é pelo fecho de `msedgewebview2.exe` a partir do PID do Sofá. WebView2 de um jogo não entra no fecho e fica fora da pasta de instalação de qualquer jeito | arquitetura 2.4; ADR-0005 |
| 3 | Transição `NotConfirmed → Running` | **Aceito.** Cobre E-03/E-16: a loja pede login e o jogo abre depois dos 60 s. Sem a transição, o jogo rodaria sem sessão, sem combinação de retorno e sem "último jogado". A sonda continua em `NotConfirmed` até "Voltar" ou "Tentar de novo". Na revisão apareceu a lacuna de CA-29 na entrada em `NotConfirmed` (ARQ-BUG-01) | arquitetura 2.4; ADR-0005 |
| 4 | Repetição de 100 ms depois dos 400 ms | **Aceito.** CA-07 e o handoff só pedem "intervalos regulares" depois de 400 ms. 100 ms (10 passos/s) cabe na meta de 50 ms por passo (M-05). Não entra em `advanced` (não é ponto de risco). Fica para o QA-17 observar com 500 jogos | arquitetura 2.1 (Input) |
| 5 | Desligar sem `EWX_HYBRID_SHUTDOWN` | **Revertido em 2026-11-14 (D-44 do usuário; arquitetura rev. 8): "Desligar" usa `EWX_HYBRID_SHUTDOWN \| EWX_POWEROFF` com a inicialização rápida ligada, igual ao menu Iniciar; "Reiniciar" completo.** Parecer original: aceito. RN-15 e CA-44 pedem o caminho normal e não forçado, e `ExitWindowsEx(EWX_POWEROFF)` cumpre. A arquitetura tinha o híbrido como opcional. Trade-off: com a inicialização rápida ligada, o próximo boot a frio fica mais lento que pelo menu Iniciar, mas M-02 conta a partir da área de trabalho e não é afetado. Ganho: drivers e controle Bluetooth reiniciam do zero (RA-05) | arquitetura 2.1 (Power) e RA-05 |
| 6 | SEC-22 (d) sem nova tentativa da tarefa na atualização | **Aceito, sem tarefa nova.** SEC-22 (d) *limita* a volta Run → tarefa ("só quando religa ou numa atualização, uma vez"). Implementar só o "religar" é o lado mais restritivo, e está conforme. A justificativa do dev ("schema fechado") não vale por si só: o `schemaVersion` existe para migrar. O que vale é que o ganho é pequeno: quem caiu no Run por falha passageira volta à tarefa religando a opção. Fica como item de backlog pós-MVP, se o QF-06 mostrar Run estourando M-02 | arquitetura 2.1 (Autostart); ADR-0006 |
| 7 | Commit `d0e2b66` junta BE-35, BE-36 e BE-40 | **Aceito, sem reescrever histórico.** A mensagem cita os três IDs (rastreável). Os três se tocam (`main.rs`, `marks.rs`, `bootstrap.rs`). Reescrever a branch traz mais risco que ganho. Pedido para as próximas tarefas: um commit por BE | — |
| 8 | Aviso ".rsrc merge failure: multiple non-default manifests" no `-gnu` | **Aceito como artefato do `-gnu`, com trava.** O aviso vem do manifesto padrão do MinGW somado ao nosso (dois `RT_MANIFEST`/1). O build `-gnu` nunca é distribuído (ADR-0001, RA-09). No MSVC o manifesto do MinGW não entra, mas isso só vale depois de conferido: ARQ-BUG-08 transforma a conferência em teste e trava do BE-39 | ADR-0001 (toolchain); arquitetura RA-11 |

Nenhum desvio muda RN ou CA da spec. Sugestão ao PO (opcional, sem mudar comportamento já aceito): registrar em CA-29 ou E-03 que o jogo que abre depois da mensagem de 60 s segue como sessão normal.

## Reaproveitamento (o que foi bem feito)

- Uma só `ForegroundBroker` para a combinação de retorno, o fim de sessão (CA-30) e a segunda instância com jogo aberto (`single_instance.rs` chama `handle_return_combo`).
- `src-tauri/command_list.rs` é a fonte única dos 30 comandos: `build.rs` (AppManifest), teste do `generate_handler!`, teste da capability e `generated/commandNames.ts`.
- `AppError` → `ErrorDto` num só lugar (`src-tauri/src/errors.rs`), e catálogo de textos num só lugar na UI (`shared/domain/errors.ts`).
- Políticas puras de caminho e ID (`install_path_policy`, `exe_policy`, `check_launch_exe_path`) usadas pelos três detectores, pelo cadastro manual e pelo lançamento, sem cópia. `path_is_within` reaproveitado pela biblioteca (ver ARQ-BUG-04 sobre onde ele mora).
- `SerialGate` única para `library_refresh` e para as ações serializadas de sessão.
- Na UI, `ConfirmDialog` e `WaitScreen` atendem todos os diálogos e esperas (ocultar, fechar jogo, sair, restaurar, energia, "Abrindo…", "Fechando…"). Tokens e fontes vêm de `@brand`, sem cópia.
- Libs do inventário usadas como previsto (`keyvalues-parser`, `image` com só `jpeg`/`png`, `unicode-normalization`, `tauri-plugin-single-instance`, `norigin`, `zustand`, `@tanstack/react-virtual`). `ts-rs` entrou pelo fallback já previsto (RA-06).
- Ports com fakes reais (`WindowSystem`, `AudioEndpoint`, `SessionProbe`, `StoreScanner`, `Clock`): domínio e aplicação testáveis sem Windows.

## Performance: riscos

| Risco | Situação | Ação |
|---|---|---|
| Memória na frente < 250 MB | Não remedida com a UI real (SP-06: 293 MB no pior caso, sem virtualização). A UI real monta ~42 cards na grade e 8 + 12 na Home | QA-17 (QF-07). Se estourar: menos capas montadas ou cache de imagem do WebView2 menor |
| `Mutex` da sessão com disco e getter bloqueante | ARQ-BUG-02 e ARQ-BUG-03 | Corrigir antes do QA-17 |
| CPU do processo inteiro em sessão | Medida só por partes (sonda 0,049% do total, thread de entrada 20 Hz, WebView2 suspenso no SP-06) | QA-17 (QF-07) com o Sofá real |
| Hook de primeiro plano sem uso | Uma abertura de processo por troca de janela no sistema | ARQ-BUG-05 |
| Dois `windows-rs` (0.58 do workspace e 0.62 do Tauri e de `webview2-com`) | Tempo de build e tamanho do binário | Aceito até o MSVC entrar (ADR-0001); subir o workspace para a versão do Tauri é tarefa explícita |
| Início de jogo de loja antes da primeira varredura | O lançamento faz uma varredura síncrona (~200 ms para 300 jogos) | Aceito (SEC-18). Medir no QA-17 junto com o M-02 |
| Capas | `cover://` com `Cache-Control: private, max-age=600`, `nosniff`, reencode no cache de disco, `loading="lazy"` | Ok |
| Timer de 1 ms | `timeBeginPeriod(1)` só com o Sofá na frente | Ok |

## Pendente para fechar o ARQ-05

- Instalador e release (BE-37 a BE-39), incluindo o primeiro build MSVC e o ARQ-BUG-08 como trava.
- BE-43 e o padrão final dos degraus no ADR-0003 (SG-10).
- QA-13.
- ~~Reconferir os ARQ-BUG-01 a 13 corrigidos (rodada 2).~~ Feito; ver abaixo.

---

## Rodada 2 (2026-11-14): ciclo de correção 1

| Campo | Valor |
|---|---|
| Diff | `git diff 85b3174..HEAD`. Back: 4368015, db4e841, cf0c3cb, b899b2d, a82ec86, 2315f50, de004ea, ce8f9ba, fea6d0f, 3c9eaed. Front: 7e6650f, 5e1a7ea, 1c0052e, 5b50391, f2f0435, b6d9e64 |
| Verificação executada | `cargo test -p launcher-core`: 303 testes passam. UI: `vitest`, 209 testes em 19 arquivos, e `tsc --noEmit` sem erro. `scripts/ci-guards.ps1`: 0 violações e autoteste com 50 violações plantadas, todas detectadas |
| Veredito | **aprovado com ressalvas** (continua parcial). Nenhum ARQ-BUG alto ou médio aberto. Abertos: 07, 08 e 13 (viraram BE-44, BE-46 e BE-45) e o 14, novo. Todos baixos |

### Situação dos ARQ-BUGs

| ID | Sev. | Situação | Como foi conferido |
|---|---|---|---|
| ARQ-BUG-01 | médio | **fechado** (4368015) | `tick` em `Launching → NotConfirmed` devolve `BringLauncherFront { focus: None }` (`machine.rs:345-351`). `on_front` mostra a janela, devolve a entrada ao Sofá e roda a escada sem `game_ended` (`session_service.rs:546-571`). Coberto pelos testes de domínio e de aplicação (`ca29_not_confirmed_traz_o_sofa_a_frente...`) |
| ARQ-BUG-02 | médio | **fechado** (4368015) | `apply` só traduz para `Action` (`MarkPlayed`, `Running`, `Front`), e `run_actions` roda depois do guard sair de escopo em `tick`, `dismiss`, `process_seen` e `processes_gone`. Não sobrou chamada a `apply` sem `run_actions`. `DisplayService` guarda `visible` (`on_launcher_hidden`/`on_launcher_shown`) e `TauriLauncherPlacement` não chama mais `is_visible()`. O teste `efeitos_da_sessao_rodam_fora_do_mutex` reproduz o cenário do callback da instância única. Também ficou bom: a escada que falha desfaz o `TOPMOST` e não entrega a entrada ao Sofá. O que ainda roda com o `Mutex` preso é não bloqueante (`PostMessageW` de `WM_CLOSE`, `TerminateProcess` por handle, `emit`). Restou só o custo do `pids()` no modo Job, que é o ARQ-BUG-14 |
| ARQ-BUG-03 | baixo | **fechado** (cf0c3cb) | `write: Mutex<StoredLibrary>` é a cópia do dono único. `with_overlay` e `add_manual` partem dela, e `commit` só troca a memória depois de gravar. Teste `alteracao_usa_a_copia_em_memoria_e_nao_rele_o_disco` |
| ARQ-BUG-04 | baixo | **fechado** (fea6d0f) | `domain/path.rs` (`path_key`, `path_is_within`, `path_is_strictly_within`, `is_direct_child`) é usado por `process_policy`, `detect`, `install_path_policy`, `autostart` e `data_dir_policy`. `library` não depende mais de `session`. Ignorar `\\?\` e aspas nas pontas é seguro: o `"` não existe em nome de arquivo do Windows; `\\?\UNC\...` vira um caminho sem unidade, que nenhuma raiz confina; `\\.\` cai na regra de `.` e falha fechado. Tudo isso está na tabela de teste |
| ARQ-BUG-05 | baixo | **fechado** (ce8f9ba) | `SystemEvents::spawn(..., false)` em `runtime.rs:165`, com o lembrete sobre `SKIPOWNPROCESS` |
| ARQ-BUG-06 | baixo | **fechado** (3c9eaed) | INV-09 passou a cobrir `fs::write`, `File::create`, `OpenOptions`, `create_dir*`, `fs::copy`, `CreateFileW` e `SetFileInformationByHandle`, cada um com seus donos. `#[cfg(test)]` pula só o item marcado, e quando não acha o fim do item a regra falha fechado (`CFG-TEST`). Entraram donos para `TerminateProcess`, `CreateProcess*`, `ShellExecute*`, `runas`, `Command` e escrita no registro. Rodei o script: árvore limpa e autoteste completo |
| ARQ-BUG-07 | baixo | **aberto → BE-44** | Sem mudança no código (INV-04, log síncrono na thread de entrada) |
| ARQ-BUG-08 | baixo | **aberto → BE-46** | Sem mudança. Continua sendo trava do BE-39 (build MSVC) |
| ARQ-BUG-09 | baixo | **fechado** (7e6650f) | `COMMAND_NAMES` e `CommandName` são reexportados de `generated/commandNames.ts`. A lista de códigos sai de um `Record<ErrorCode, true>`: código novo ou removido no núcleo quebra o typecheck |
| ARQ-BUG-10 | baixo | **fechado** (7e6650f) | `findGame` e `gameName` ficam em `features/library/domain.ts`, `selectGameById` usa `findGame`, e as nove buscas soltas sumiram |
| ARQ-BUG-11 | baixo | **fechado** (7e6650f) | "Recentes" só filtra e corta a lista do núcleo, sem reordenar. A fileira A–Z usa `sortKey` (NFD, sem marcas, minúsculas) e compara por ponto de código, como o `String::cmp` do Rust, com desempate pela id. Diferenças residuais, aceitas: o `toLowerCase` do JS trata sigma final por contexto e o Rust não; o desempate compara a id como texto, e o núcleo usa o `Ord` do `GameId`. Só pesam com nomes idênticos depois da normalização |
| ARQ-BUG-12 | baixo | **fechado** (ce8f9ba) | `library_refresh` chama `spawn_refresh`. `marks::navigable` só registra a primeira vez (`swap`). `Busy` continua com prioridade sobre `Internal` quando a biblioteca não subiu |
| ARQ-BUG-13 | baixo | **aberto → BE-45** | Sem mudança (`settings_store.rs`) |
| ARQ-BUG-14 | baixo | **novo** (dev-backend) | Ver abaixo |

### ARQ-BUG novo

| ID | Sev. | Responsável | Arquivo:linha | Problema | Correção sugerida |
|---|---|---|---|---|---|
| ARQ-BUG-14 | baixo | dev-backend | `crates/launcher-win/src/process_probe.rs` (`scan_job` → `scan_with` → `Self::snapshot()`; `WinWatch::members_fresh`; `job_loop`); `crates/launcher-core/src/application/session_service.rs:416-430` e `:384-389` (`pids()` com o `Mutex` da sessão preso); `docs/adr/0005-deteccao-de-sessao.md:26` | O modo Job novo (ADR-0005 rev. 6, SEC-BUG-01) está correto quanto à segurança: `classify` em cada PID do Job, `IsProcessInJob` contra reuso de PID, encerramento só dos membros, sem `TerminateJobObject`. Mas perdeu o "custo zero" do Job: **toda** mensagem da porta (processo novo, saída, outras) e **toda** chamada de `pids()`, `request_close()` e `force_close()` tiram um snapshot ToolHelp do sistema inteiro (19 a 28 ms medidos no SP-02). Isso só serve para calcular `own_child_pids`, que no modo Job não serve para nada, porque o WebView2 do Sofá nunca entra no Job do jogo. `pids()` roda com o `Mutex` da sessão preso, em `handle_return_combo` (inclusive na thread da UI, pelo callback da instância única) e em `return_to_game`, o que soma esse tempo ao M-04 e prende `session_get`. Jogo que dispara rajadas de processos (lançador, compilador de shader, crash handler) gera uma rajada de snapshots. Além disso, o ADR diz que a lista do Job é relida "a cada varredura de `Running`", mas o modo Job não tem varredura periódica, só reage às mensagens | No modo Job, não tirar snapshot do sistema: ler só `JobObjectBasicProcessIdList`, abrir cada PID novo, conferir `IsProcessInJob` e chamar `classify` com `own_child_pids` vazio. O pai não entra no `classify`. `pids()` deve devolver os membros já conhecidos (a `job_loop` os mantém em dia pelas mensagens) e, se precisar de frescor, reler só a lista do Job, nunca com o `Mutex` da sessão preso. Ajustar o ADR-0005 rev. 6: ou tirar o "a cada varredura de `Running`", ou implementar a releitura periódica barata da lista do Job. Teste: `pids()` no modo Job não chama `CreateToolhelp32Snapshot` (contador no fake ou medida de tempo) |

### Outros pontos conferidos (sem ARQ-BUG)

- **Navegação espacial com `GetBoundingClientRectAdapter`** (f2f0435): correção certa. A grade virtualizada posiciona as linhas com `transform`, e o adapter padrão do norigin mede por `offsetTop`, que dá 0 em todas as linhas. Custo: um `getBoundingClientRect` por focável a cada passo, com até ~42 cards montados na grade e 8 + 12 na Home, o que é desprezível. Risco a observar no QA-17, sem bug: o card em foco cresce com `transform: scale` e é medido escalado. Como a escala é centrada, o centro não muda, mas na repetição de 100 ms durante a animação a borda medida varia. Se aparecer salto diagonal errado na grade, medir sem a escala (por exemplo, com o retângulo do contêiner do card). O jsdom não mede layout, então o teste é manual ou E2E.
- **D-44** (de004ea): "Desligar" usa `EWX_HYBRID_SHUTDOWN | EWX_SHUTDOWN | EWX_POWEROFF` com a inicialização rápida ligada (a forma combinada que o RA-05 indicava como alternativa) e "Reiniciar" é completo. Sem bit de forçar; guard de INV-05 limpo. A arquitetura (2.1 Power, RA-05, §13) já reflete isso.
- **SEC-BUG-03** (b899b2d), **SEC-BUG-02 e QA-BUG-01** (a82ec86) e **QA-BUG-03** (2315f50): nada a apontar do lado da arquitetura. O purge continua fora da IPC e fora da instância única, e a nova saída 4 já está no contrato (§10).
- Front (5e1a7ea, 1c0052e, 5b50391, b6d9e64): `DialogSide` novo e reaproveitado por `ConfirmDialog` e `WaitScreen`. O teclado de emergência continua traduzindo para as mesmas intenções (`shared/input/keyboard.ts`, RN-07). Nenhum `invoke` fora de `shared/ipc`.

### Pendente para fechar o ARQ-05

Instalador e release (BE-37 a BE-39, com o ARQ-BUG-08/BE-46 como trava do build MSVC), BE-43 com o padrão final dos degraus no ADR-0003, QA-13, BE-44, BE-45 e o ARQ-BUG-14 (rodada 3).

---

## Rodada 3 (2026-11-15): ciclo de correção 2 e leva 5 (instalador, desinstalador, release)

| Campo | Valor |
|---|---|
| Diff | Leva curta `7433138..782543b`: SEC-BUG-04 (`GameScope`), ARQ-BUG-14 (BE-48), BE-44 (`AsyncLog`), BE-45 (`SettingsService`), QA-BUG-04 e o guard de `System::Call` (BE-50). Leva 5 `782543b..HEAD`: BE-37, BE-38, BE-39, BE-42 e BE-46 |
| Verificação executada | `cargo test -p launcher-core`: 312 testes passam. `cargo test -p launcher-win --lib`: 109 passam, 2 ignorados. `scripts/ci-guards.ps1`: 0 violações e autoteste com 64 violações plantadas, todas detectadas. Instalador, desinstalador e release só por leitura do código (`installer.nsi`, `*.nsh`, `ci-release.ps1`, `tauri.conf.json`); instalar de verdade é QA-17 |
| Veredito | **aprovado com ressalvas** (continua parcial). Dois ARQ-BUG médios novos (15 e 16), que devem ser corrigidos antes do QA-17 |

### Situação dos ARQ-BUGs

| ID | Sev. | Situação | Como foi conferido |
|---|---|---|---|
| ARQ-BUG-07 | baixo | **fechado** (BE-44, 1b98685) | `AsyncLog`: o `tracing` monta a linha num buffer local e a entrega por `try_send` a uma fila limitada (4096). Uma thread própria grava no `RotatingFile`. Fila cheia descarta e conta, e o próximo registro gravado avisa quantos se perderam. Thread de entrada sem disco nem `Mutex` global (INV-04). `logging::flush` no `exit_hook`. Teste com disco parado |
| ARQ-BUG-08 | baixo | **fechado** (BE-46, 8604e44) | `manifest_resource.rs` lê o diretório de recursos do PE (tipo 24, ID, idioma). Reprova no MSVC sem exatamente um `RT_MANIFEST`/1 com `asInvoker`, `uiAccess="false"` e Common Controls v6; no `-gnu`, avisa. Prova com PE sintético. Roda como trava do `ci.ps1 -Release` (`SOFA_MANIFEST_EXE`). O aviso do linker não aparece no MSVC (desvio 8 resolvido) |
| ARQ-BUG-13 | baixo | **fechado** (BE-45, 1b2ab73) | `SettingsService` em `launcher-core/application/settings_service.rs`, testado com repositório fake. Implementa `SettingsStore` para `DisplayService` e `AutostartService`. `settings_store.rs` foi removido. Arquitetura 2.1 (Settings) atualizada |
| ARQ-BUG-14 | baixo | **fechado** (BE-48, 2aa1114), **com regressão: ARQ-BUG-15** | O modo Job não tira mais snapshot ToolHelp (`scan_job` lê só `JobObjectBasicProcessIdList`, confere `IsProcessInJob` e aplica `classify` com `own_child_pids` vazio; contador `snapshots_taken()` no teste). `pids()`, `request_close()` e `force_close()` usam só os membros em cache. ADR-0005 rev. 7 sem a releitura "a cada varredura de `Running`" |
| ARQ-BUG-15 | médio | **novo** (dev-backend) | Ver abaixo |
| ARQ-BUG-16 | médio | **novo** (dev-backend; a segurança confere) | Ver abaixo |

Conferidos sem achado: **SEC-BUG-04** (`GameScope::{Dir, ExactExe}` no domínio, `for_exe` passa a pasta por `check_install_dir` e reaproveita `domain/path.rs`; `attach` escolhe a pasta ou o exe exato), **QA-BUG-04** (regra pura em `display_name.rs`) e **BE-50** (guard de `System::Call` e de `Delete`/`RMDir` sobre a pasta de dados).

### ARQ-BUGs novos

| ID | Sev. | Responsável | Arquivo:linha | Problema | Correção sugerida |
|---|---|---|---|---|---|
| ARQ-BUG-15 | médio | dev-backend | `crates/launcher-win/src/process_probe.rs`: `wait_member_exit` (`EXIT_SIGNAL_WAIT_MS = 500`) e `job_loop` (espera dentro do `scanner.lock()`; `ActiveZero` sem espera) | Depois do ARQ-BUG-14, o fim da sessão no modo Job depende de um prazo fixo. A porta avisa `EXIT_PROCESS` antes de o processo sinalizar, então a sonda espera **até 500 ms** pelo handle. Jogo grande pode levar mais que isso para desmontar o processo (liberar GBs de memória, driver de GPU). Nesse caso o membro ainda parece vivo, e o `ACTIVE_PROCESS_ZERO` que chega logo depois é tratado **sem espera nenhuma**. Como o modo Job não tem varredura periódica, nenhuma outra mensagem vem: a sessão fica em `Running` para sempre e o Sofá não volta (CA-30, RN-10). Com um acompanhante vivo no Job (o caso do SEC-BUG-01), basta passar dos 500 ms no `EXIT_PROCESS`. Além disso, a espera roda com o `Mutex` do scanner preso, e `pids()` e `force_close()` (chamados com o `Mutex` da sessão preso, inclusive pela thread da UI no callback da instância única) podem esperar até 500 ms por saída | Não depender de prazo: para cada membro, registrar uma espera no handle (`RegisterWaitForSingleObject`, ou a mesma espera nos handles do modo varredura) que, ao sinalizar, posta um pacote próprio na porta do Job (`PostQueuedCompletionStatus` com uma chave `MEMBER_EXITED`). A `job_loop` reavalia quando o processo **de fato** saiu. Nenhuma espera com o `Mutex` do scanner preso. Teste: membro cujo handle sinaliza 2 s depois do `EXIT_PROCESS`, com e sem acompanhante vivo; a sessão tem de terminar |
| ARQ-BUG-16 | médio | dev-backend (a segurança confere) | `src-tauri/windows/installer.nsi:402-403` (`MUI_PAGE_DIRECTORY`), `:520-540` (pasta padrão só quando não há `/D=`), `:372` (`ExecWait "$INSTDIR\uninstall.exe"`); `uninstall-hooks.nsh:386` (purge por `$INSTDIR\sofa.exe`) | O cabeçalho do fork e o ADR-0006 dizem "só `perMachine` em `%ProgramFiles%\Sofa`", mas o instalador mantém a página de pasta e aceita `/D=`. Instalado numa pasta criada fora de `Program Files` (ex.: `C:\Sofa`, em que "Usuários autenticados" podem modificar), o SEC-20 cai: qualquer processo do usuário troca o `sofa.exe`. E a exceção de `$INSTDIR` do SEC-32 vira caminho de elevação: o próximo instalador ou desinstalador, elevado, executa o `$INSTDIR\uninstall.exe` plantado. O `--purge-user-data` também roda o `sofa.exe` dessa pasta. O DR-04 já recusa autostart fora de `Program Files`, ou seja, essa instalação nem é suportada | Tirar a `MUI_PAGE_DIRECTORY`. Fixar `$INSTDIR = $PROGRAMFILES64\Sofa` em `.onInit` e `un.onInit`, ignorando ou recusando `/D=` diferente (com mensagem e saída). Se a chave do HKLM apontar para outra pasta, tratar como instalação anterior inválida (já é o que o `$4 != $INSTDIR` faz). Teste em `scripts/test-installer.ps1`: o script gerado não tem página de pasta, e `/D=C:\X` não muda o destino |

### Veredito sobre os desvios da rodada 3 (dev-backend.md e `docs/api` §11)

| # | Desvio | Veredito | Registro |
|---|---|---|---|
| a | Guard de SEC-32 aceita executável por `$PLUGINSDIR\` e `$INSTDIR\`, além de `$SYSDIR\` | **Aceito, condicionado ao ARQ-BUG-16.** `$PLUGINSDIR` é a pasta temporária privada que o NSIS cria para a conta que instala; `$TEMP\`, nome relativo e variável continuam proibidos. `$INSTDIR` só é seguro com a pasta fixa em `Program Files` | ADR-0006 rev. 7 §1 |
| b | Autostart de todos os perfis por `$SYSDIR\schtasks.exe` com SID validado, e não por COM com prefixo | **Aceito.** O ADR-0006 já admitia as duas formas; o COM exigiria `VARIANT` por valor pelo plugin `System`. A validação caractere a caractere e a tabela de 18 SIDs (inclusive `"/F`, `&calc`, `$(x)`) cobrem o risco de injeção | ADR-0006 rev. 7 §1 |
| c | Usuário interativo por duas fontes (token do Explorer e usuário da sessão), e fallback só sem junção no caminho do perfil | **Aceito.** Mais restritivo que o ADR; a divergência cai em "nada apagado" | ADR-0006 rev. 7 §1 |
| d | Saíram do template a migração de WiX, o WebView2 por download ou offline e a remoção recursiva de dados | **Aceito.** O Sofá nunca foi MSI; P-01 é bootstrapper embutido; a remoção recursiva fere SEC-31. Custo registrado: manter o fork a cada atualização do Tauri | ADR-0006 rev. 7 §1 |
| e | Template próprio (fork) no lugar de só hooks | **Aceito.** Os hooks não removem o `RmDir /r` elevado nem a gravação em `HKCU` do template original | ADR-0006 rev. 7 §1 |
| f | Desinstalador anterior só se estiver em `$INSTDIR` | **Aceito** (mais restritivo), junto com o ARQ-BUG-16 | ADR-0006 rev. 7 §1 |
| g | `CreateProcessW` com o Explorer como pai pelo plugin `System`; se falhar, `RunAsUser` sem ler o código de saída | **Aceito.** É o caminho do ADR-0006 rev. 6, e "código não lido" já está na tabela de decisão (fallback restrito). A conferência elevada (integridade Medium do filho, UAC de outra conta) é QA-17 | — |
| h | CI de dev segue no `-gnu`; MSVC só em `ci.ps1 -Release` | **Aceito.** O release reprova antes de publicar | ADR-0001 rev. 6 |
| i | Primeiro release sem tag (`-AllowUntagged`) | **Aceito só para build de teste.** A entrega ao usuário (QA-17 e depois) sai de tag `v<versão>` e árvore limpa; o `BUILD-INFO.txt` marca o build sem tag | ADR-0001 rev. 6 |
| j | `icon.ico` mantido o da marca, não o gerado pelo `tauri icon` | **Aceito.** É a fonte única da marca (arquitetura 3.5), e o gerado perde detalhe nos tamanhos pequenos | — |
| k | `@tauri-apps/cli` 2.12.4 fixo como devDependency (`ignore-scripts`) | **Aceito.** Fixa o bundler na mesma versão do fork do template | — |

Nenhum desvio muda RN ou CA. Os textos novos das telas do instalador ainda não estão no `microcopy.md` (revisão do designer pendente, já anotada no contrato §11).

### Pendências novas

- **BE-51 (testes intermitentes em `session_runtime.rs`):** aceito como tarefa. A causa provável está nos testes: o acompanhante vive só 6 s, e o Defender atrasa a primeira execução de cada cópia. Mas esses testes são a prova de SEC-BUG-01 e SEC-BUG-04, então têm de fechar (50 rodadas sem falha) **antes do QA-13**. Se a falha voltar com o acompanhante vivendo bem mais que o teste, deixa de ser folga de tempo e vira bug de sessão.
- **BE-52 (Defender barra o `sofa-test-game.exe` no MSVC):** aceito. Enquanto estiver aberto, as provas de sessão rodam só no `-gnu` (ADR-0001 rev. 6), e tem de fechar antes do QA-13. Recomendação: pôr a varredura do Defender (`MpCmdRun -Scan -ScanType 3 -File`) no instalador e no `sofa.exe` como passo do `-Release`. Hoje ela foi feita à mão; um falso positivo no artefato entregue é o RR-02 acontecendo.

### Reaproveitamento e performance (rodada 3)

- Bem reaproveitado: `domain/path.rs` no `GameScope`; `check_install_dir` (SEC-16) reutilizado para decidir o escopo da sessão; `SettingsService` como port único para monitor e autostart; o próprio `sofa.exe --purge-user-data` como removedor do desinstalador; ícone e licenças OFL lidos de `docs/brand/sofa/` sem cópia (`bundle.resources`).
- Performance: o modo Job voltou a não tirar snapshot. Fica o risco do ARQ-BUG-15 (espera de até 500 ms com `Mutex` preso). O `AsyncLog` tira o disco do caminho quente.

### Pendente para fechar o ARQ-05

BE-43 com o padrão final dos degraus no ADR-0003; QA-13; ARQ-BUG-15 e 16; BE-51 e BE-52 antes do QA-13 (rodada 4). Instalar e desinstalar de verdade, integridade Medium, DLL plantada e Windows 10 limpo seguem no QA-17.
