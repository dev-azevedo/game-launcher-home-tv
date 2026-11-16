# Contrato de API: launcher-tv (SofaLauncher)

| Campo | Valor |
|---|---|
| Feature | launcher-tv |
| Tarefa | BE-07 |
| Base | `docs/arch/launcher-tv.md` seção 4 (lista fechada), `docs/security/launcher-tv.md` (SEC-03, SEC-05, SEC-06), spec (CA-26 a CA-49, CA-71 a CA-78), `docs/design/launcher-tv.md` e `microcopy.md` |
| Resumo | **30 comandos**, **8 eventos**, **26 códigos de erro** |
| Revisão de segurança | SG-01 aprovado com ressalvas; achados CT-01 a CT-07 aplicados nesta revisão (rev. 2), CT-08 em BE-08 |

"API" aqui é a IPC do Tauri: **comandos** (UI → núcleo, `invoke`) e **eventos** (núcleo → UI, `listen`). Não há HTTP. A lista é **fechada**: comando novo exige alterar a arquitetura 4.1 e passar pela segurança.

## 0. Convenções

- **Autorização**: capability única `main` (janela `main`, sem `remote`), com `allow-<comando>` para os 30 comandos e `core:event:allow-listen`/`allow-unlisten`. Não há outro tipo de autenticação: o único chamador é a UI empacotada, e a CSP bloqueia conteúdo remoto. Toda ação é validada de novo no núcleo (a UI não é fronteira de confiança).
- **Sem entrada perigosa (SEC-03)**: nenhum comando recebe caminho de executável, argumentos, URI, PID, chave de registro ou texto de shell. Entradas são **IDs de jogo, enums, booleanos, ticket opaco e texto de nome**. **Únicas exceções com caminho**: `fs_list_dir` (#9) e `library_add_manual` (#3), sob SEC-09. O `LaunchSpec` é sempre montado pelo núcleo.
- **Serialização**: JSON, campos em `camelCase`, enums como string em `PascalCase`. Todo DTO de entrada usa `deny_unknown_fields`: campo extra, campo ausente ou tipo errado → `InvalidRequest`. Tempos são inteiros em milissegundos desde a época Unix (UTC).
- **Erro**: o comando rejeita a promessa com `ErrorDto { code: string, params?: object }`. `code` é da tabela da seção 5. `params` só leva **IDs e enums** (nunca caminho, texto técnico ou mensagem do núcleo). Detalhe técnico vai só para o log (SEC-06, SEC-18). A UI traduz o `code` para pt-BR.
- **Serialização de ações**: `library_refresh`, `session_launch`, `session_launch_with_uac` e `session_replace` aceitam uma execução por vez; chamada concorrente → `Busy`. Comandos de leitura nunca dão `Busy`.
- **Pânico**: cada comando roda sob `catch_unwind`; falha vira `Internal`.
- **Rate limit**: não há (app local). A proteção é a serialização acima e a validação de estado (ex.: `session_force_close` só em `CloseOverdue`).
- **Eventos**: carregam **estado completo**, não deltas. A UI faz uma carga por `invoke` e depois só ouve eventos.

## 1. Tipos básicos

### GameId

String `"<origem>:<chave>"`, validada pelo núcleo em toda entrada. Qualquer outra forma → `InvalidRequest`; ID bem formado que não está na biblioteca → `UnknownGame`.

| Origem | Formato da chave | Exemplo |
|---|---|---|
| Steam | `^[0-9]{1,10}$`, cabe em `u32` | `steam:620` |
| Epic | `^[A-Za-z0-9_.-]{1,128}$` (`AppName`) | `epic:SampleGameA` |
| GOG | `^[0-9]{1,20}$` | `gog:1000000001` |
| Manual | `^[0-9a-f]{16}$` (hash do caminho canônico, ver abaixo; o caminho nunca vai no ID) | `manual:9f3a1c2b7d4e5a60` |

**Hash do `manual:` (CT-05).** `SHA-256` dos bytes UTF-16LE do caminho canônico (resolvido, sem prefixo `\\?\`, separador `\`) **normalizado para caixa** (maiúsculas ordinais, igual à comparação de caminhos do Windows); a chave são os **8 primeiros bytes** do digest em hexadecimal minúsculo (16 caracteres, 64 bits). Dois caminhos que diferem só em caixa dão o mesmo ID (RN-03: um jogo só). Colisão com outro caminho já cadastrado é tratada como `Duplicate` (improvável: no máximo 5.000 jogos manuais). O caminho em si fica só em `library.json`.

**`GameId` nunca compõe caminho em disco (CT-06).** Nem `epic:..` (bem formado pela regex) nem qualquer outro ID entra em `Path::join`, nome de arquivo, URL de arquivo ou argumento de processo. Cache de capas e ícones usa o hash da origem + chave (arquitetura 4.5), e o handler `cover://` só faz busca do `GameId` na biblioteca em memória; ID ausente → 404.

### Enums

| Enum | Valores |
|---|---|
| `Origin` | `Steam`, `Epic`, `Gog`, `Manual` |
| `SessionState` | `Idle`, `Launching`, `NeedsElevation`, `NotConfirmed`, `Running`, `Ending`, `Closing`, `CloseOverdue` (diagrama 2.4 da arquitetura) |
| `InputIntent` | `Up`, `Down`, `Left`, `Right`, `Confirm`, `Back`, `Menu`, `View`, `PagePrev`, `PageNext`, `VolumeDown`, `VolumeUp` |
| `PowerAction` | `Shutdown`, `Restart`, `Suspend` |
| `Availability` | `Available`, `Unavailable` |
| `VolumeDirection` | `Up`, `Down` |
| `FrontOutcome` | `Front`, `FrontAfterMinimize`, `Failed` |
| `AutostartMechanism` | `Task`, `Run`, `None` |
| `DirEntryKind` | `Drive`, `Dir`, `Exe` |
| `LaunchFailure` | `ElevationDenied` (UAC recusado, CA-73) |

### DTOs

```text
GameDto {
  id: GameId,
  name: string,              // nome exibido, ja normalizado (SEC-07), 1..256; a UI renderiza so como texto
  originalName: string|null, // nome da loja/arquivo, normalizado como name (SEC-07, CT-03); nao nulo quando o usuario renomeou (habilita "Restaurar nome", CA-62)
  origin: Origin,
  cover: string,             // "cover://<gameId>" (SEC-08); 404 -> a UI mostra o placeholder (CA-24)
  lastPlayedAt: number|null, // inicio da ultima sessao (P-06)
  inProgress: boolean        // e o jogo da sessao ativa (selo "Em andamento", CA-36)
}
LibrarySnapshot { games: GameDto[], focusGameId: GameId|null }   // sem itens ocultos; ordenado por RN-05
SessionDto {
  state: SessionState,
  gameId: GameId|null,       // nulo em Idle
  since: number|null,        // entrada no estado atual
  dismissed: boolean,        // a UI saiu da tela de espera (session_dismiss); o nucleo continua; a UI nao reabre a tela
  elevated: boolean,         // jogo iniciado com UAC: aviso "pode falhar" (E-23)
  minimized: boolean,        // o retorno minimizou o jogo (degrau 4): aviso do painel
  launchFailure: LaunchFailure|null
}
VolumeState { level: 0..100, muted: boolean, available: boolean }
MonitorDto { id: MonitorId, name: string, primary: boolean, current: boolean }   // MonitorId: string 1..256, opaco; name vem do driver e e normalizado como GameDto.name (SEC-07, CT-03)
SettingsDto { autostart: boolean, autostartMechanism: AutostartMechanism, monitorId: MonitorId|null, logDir: string }
DirListing { path: string|null, parent: string|null, entries: [{ name: string, kind: DirEntryKind }], truncated: boolean }   // max 2000 entradas, pastas antes de .exe (CT-04); name cru (CT-03)
PowerTicket: string opaca ^[A-Za-z0-9_-]{22,64}$, 30 s, uso unico; gerada por CSPRNG do sistema (getrandom/BCryptGenRandom, >= 128 bits), presa a PowerAction do power_prepare; um power_prepare novo invalida o anterior (CT-07)
ErrorDto { code: string, params?: { gameId?: GameId, origin?: Origin } }
```

**Normalização de nomes (CT-03).** `GameDto.name`, `GameDto.originalName` e `MonitorDto.name` saem normalizados pelo núcleo (remove C0/C1 e bidi, corta em 256). `DirListing.entries[].name` é o nome de arquivo **cru**, porque a navegação precisa dele de volta em `fs_list_dir`; a UI o exibe neutralizando controle e bidi (substitui por símbolo visível) e nunca o interpreta como HTML.

`GameDto` não leva caminho de instalação nem do executável. `logDir` é o único caminho que o núcleo devolve (pasta de logs do próprio Sofá, para o usuário copiar).

## 2. Comandos (UI → núcleo): 30

Legenda: "Auth" é sempre a capability `main` (`allow-<comando>`).

### 2.1 Biblioteca

| # | Comando | Entrada | Saída | Erros |
|---|---|---|---|---|
| 1 | `library_get` | — | `LibrarySnapshot` (do cache; não varre) | — |
| 2 | `library_refresh` | — | `()`; o resultado chega em `library://changed` | `Busy` |
| 3 | `library_add_manual` | `{ path: string }`, 1..1024 caracteres | `GameDto` | `Duplicate`, `NotAnExecutable`, `PathNotAllowed`, `NotFound`, `InvalidRequest` |
| 4 | `library_hidden` | — | `GameDto[]` (itens ocultos, CA-70) | — |
| 5 | `game_rename` | `{ gameId: GameId, name: string }`, nome 1..256 após normalizar | `GameDto` | `EmptyName`, `UnknownGame`, `InvalidRequest` |
| 6 | `game_restore_name` | `{ gameId }` | `GameDto` | `UnknownGame` |
| 7 | `game_hide` | `{ gameId }` | `()`. Nunca apaga arquivo (RN-04) | `UnknownGame` |
| 8 | `game_unhide` | `{ gameId }` | `()` | `UnknownGame` |
| 9 | `fs_list_dir` | `{ path: string\|null }`, ≤ 1024; nulo ou vazio = unidades locais | `DirListing` | `PathNotAllowed`, `AccessDenied`, `NotFound`, `InvalidRequest` |

- `library_add_manual`: o núcleo canonicaliza, exige unidade local fixa ou removível (sem UNC, `\\?\`, `\\.\`, ADS, nome reservado), confere `MZ` + `PE`, rejeita duplicata e **nunca** aceita argumentos. Local de rede ou fora dos discos → `PathNotAllowed` (CA-71, 1º momento); `.exe` inválido → `NotAnExecutable` (CA-72).
- `fs_list_dir`: só `DRIVE_FIXED`/`DRIVE_REMOVABLE`; só pastas e `.exe`; nunca lê conteúdo. Devolve no máximo **2.000 entradas**, pastas antes de `.exe` (ordem alfabética sem acento dentro de cada grupo); se houver mais, `truncated = true` e a UI avisa que a lista está cortada (CT-04). Unidade de rede ou mapeada não aparece e, se pedida por caminho, devolve `PathNotAllowed`.
- `game_rename`: o núcleo normaliza (remove C0/C1 e bidi, corta em 256). Nome vazio depois de normalizar → `EmptyName`. Renomear não altera o ID.

Exemplo (#5):
```json
// invoke("game_rename", { "gameId": "steam:620", "name": "Portal 2 (cooperativo)" })
{ "id": "steam:620", "name": "Portal 2 (cooperativo)", "originalName": "Portal 2", "origin": "Steam",
  "cover": "cover://steam:620", "lastPlayedAt": 1794610000000, "inProgress": false }
```

### 2.2 Sessão

| # | Comando | Entrada | Saída | Erros |
|---|---|---|---|---|
| 10 | `session_get` | — | `SessionDto` | — |
| 11 | `session_launch` | `{ gameId }` | `()`; o andamento vem em `session://state` | `SessionAlreadyActive`, `GameFileMissing`, `StoreNotInstalled`, `NeedsElevation`, `NetworkPathBlocked`, `UnknownGame`, `Busy` |
| 12 | `session_launch_with_uac` | `{ gameId }` | `()` | `InvalidState`, `UnknownGame`, `GameFileMissing`, `StoreNotInstalled`, `NetworkPathBlocked`, `ElevationDenied`, `Busy` |
| 13 | `session_replace` | `{ gameId }` (pede o fechamento normal do atual e abre o novo quando a sessão chegar a `Idle`, CA-31) | `()` | os de #11 exceto `SessionAlreadyActive`, mais `InvalidState` |
| 14 | `session_return_to_game` | — | `()` | `NoActiveSession`, `ForegroundDenied` |
| 15 | `session_close` | — | `()`; o prazo de 10 s leva a `CloseOverdue` | `NoActiveSession`, `InvalidState` |
| 16 | `session_force_close` | — | `()` | `NotOverdue`, `AccessDenied`, `NoActiveSession` |
| 17 | `session_retry` | — | `()`; refaz o início do mesmo jogo | `InvalidState`, `UnknownGame`, `GameFileMissing`, `StoreNotInstalled`, `NeedsElevation`, `NetworkPathBlocked`, `Busy` |
| 18 | `session_dismiss` | — | `()`; ver abaixo | — |

Regras:
- `session_launch` resolve o `LaunchSpec` **no núcleo**, revalida o jogo (existência, política de caminho, SEC-09/12/14) e só então inicia. **Caminho de rede** (ex.: biblioteca editada à mão) → `NetworkPathBlocked` (CA-71, 2º momento, **não** `PathNotAllowed`); arquivo ausente → `GameFileMissing{gameId}` (CA-28); loja ausente → `StoreNotInstalled{origin}`; exige administrador → `NeedsElevation` e a sessão vai a `NeedsElevation` (CA-73, 1ª etapa). Com sessão ativa → `SessionAlreadyActive{gameId}` (o jogo **atual**).
- `session_launch_with_uac`: só aceito em `NeedsElevation` e para o **mesmo** `gameId` (AF-01); caso contrário `InvalidState`. Revalida o caminho imediatamente antes e usa o verbo padrão (nunca `runas`). O UAC é do Windows; a sessão fica em `Launching` com `elevated = true` (2ª etapa). UAC recusado → estado volta a `NeedsElevation` com `launchFailure = "ElevationDenied"` (texto "O Windows não deu permissão para abrir <nome>").
- `session_replace` (CT-01): só aceito em `Running`. Faz o **mesmo pedido normal de `session_close`** (`WM_CLOSE`, nunca encerramento forçado, nunca `TerminateProcess`/`TerminateJobObject`) e guarda o novo `gameId` como pendente; quando a sessão chega a `Idle` o núcleo abre o novo jogo pelo fluxo de #11 (revalidação completa; falha de início vira o erro/estado normal de #11 na sessão nova). Se o jogo não fechar em 10 s a sessão vai a `CloseOverdue`, o pendente é **descartado** e o usuário decide: `session_dismiss` (volta a `Running`) ou `session_force_close` confirmado (RN-12, INV-05). Em `CloseOverdue`, `Closing` e em qualquer outro estado, `session_replace` → `InvalidState`. O comando valida o novo jogo (`UnknownGame`, `NetworkPathBlocked`, `StoreNotInstalled`, `GameFileMissing`) **antes** de pedir o fechamento, para não fechar o atual por um destino inválido. Teste obrigatório: jogo de teste que ignora `WM_CLOSE` + `session_replace` não é encerrado sem `session_force_close`.
- `session_retry` e `session_launch_with_uac` revalidam o jogo como #11 (CT-02); por isso repetem `UnknownGame`, `GameFileMissing`, `StoreNotInstalled` e `NetworkPathBlocked`. `ElevationDenied` também é devolvido por #12 (e aparece em `launchFailure`).
- `session_force_close`: só em `CloseOverdue`; fora dele `NotOverdue`. Jogo elevado (UIPI) → `AccessDenied` (texto próprio do forçar).
- **`session_dismiss`** ("Voltar" e B nas telas de espera; CA-77, CA-78, RN-31). Nunca cancela o início nem o fechamento:
  - `Launching`: a UI sai da espera; o núcleo **continua** acompanhando o início (o jogo ainda pode abrir depois e vale o fluxo normal de sessão, CA-27, CA-30). `dismissed = true` até a próxima transição de estado relevante.
  - `Closing`: o pedido de `WM_CLOSE` continua; a biblioteca aparece com o card "Em andamento" até o jogo fechar. `dismissed = true`.
  - `NotConfirmed` e `NeedsElevation` ("Voltar"): a sessão vai a `Idle` (nada rodando ainda, cancelamento real).
  - `CloseOverdue`: a sessão volta a `Running` ("Cancelar", CA-35).
  - `Idle`, `Running`, `Ending`: sem efeito, sem erro (idempotente).
  Sempre responde `()`.

Exemplo (#11, erro):
```json
// invoke("session_launch", { "gameId": "epic:SampleGameA" })  -> rejeita
{ "code": "SessionAlreadyActive", "params": { "gameId": "steam:620" } }
```

### 2.3 Volume

| # | Comando | Entrada | Saída | Erros |
|---|---|---|---|---|
| 19 | `volume_get` | — | `VolumeState` | — |
| 20 | `volume_step` | `{ direction: "Up"\|"Down" }` | `VolumeState` | `NoAudioDevice` |
| 21 | `volume_toggle_mute` | — | `VolumeState` | `NoAudioDevice` |

Passo de 5, limite 0 a 100; RT no mudo desmuta e sobe; LT no mudo baixa o nível guardado e mantém o mudo (RN-13, CA-38 a CA-40, CA-64). Sem dispositivo: `volume_get` devolve `available = false`.

### 2.4 Energia

| # | Comando | Entrada | Saída | Erros |
|---|---|---|---|---|
| 22 | `power_capabilities` | — | `{ shutdown, restart, suspend: Availability }` | — |
| 23 | `power_prepare` | `{ action: PowerAction }` | `PowerTicket` | `Unavailable` |
| 24 | `power_execute` | `{ ticket: PowerTicket }` | `()` | `TicketInvalid`, `Rejected` |

`power_prepare` é chamado quando o diálogo de confirmação abre; `power_execute`, quando o usuário confirma. Ticket expira em 30 s e vale uma vez (SEC-27). Nunca há `EWX_FORCE*`. Com jogo aberto, a UI mostra o aviso de CA-48 antes de `power_prepare`.

### 2.5 Configurações, monitor e app

| # | Comando | Entrada | Saída | Erros |
|---|---|---|---|---|
| 25 | `settings_get` | — | `SettingsDto` | — |
| 26 | `settings_set_autostart` | `{ enabled: boolean }` | `SettingsDto` | `AutostartFailed` |
| 27 | `settings_set_monitor` | `{ monitorId: MonitorId\|null }`; deve estar em `display_monitors`; nulo = principal | `SettingsDto` | `UnknownMonitor` |
| 28 | `display_monitors` | — | `MonitorDto[]` | — |
| 29 | `input_status` | — | `{ connected: 0..4 }` | — |
| 30 | `app_exit` | — | `()` (CA-05, CA-82, RN-35: fecha só o launcher; o jogo em sessão continua aberto) | — |

`app_exit` serve ao item "Fechar o launcher" do grupo de energia do menu rápido (D-50, D-53; o "Sair do launcher" deixou de existir). Não é ação de energia: não passa por `power_prepare`/ticket e não mexe no PC. A confirmação com "Cancelar" em foco é da UI (`ExitDialog`); o núcleo só encerra. Com sessão ativa, não pede fechamento ao jogo, não força, não minimiza, não traz o jogo à frente (`ForegroundBroker`) e não toca em "Iniciar com o Windows" (CA-82 itens 3 e 5, D-51, D-57). Antes de sair grava uma linha `info` "fechar o launcher" com o estado da sessão e o `GameId` (sem caminho, SEC-18), drena o log (até 300 ms) e chama `exit(0)`. Detalhes na seção 13.

`settings_set_autostart`: o núcleo recusa ligar se o exe em execução não estiver em `%ProgramFiles%` (DR-04) → `AutostartFailed`. Os valores `advanced.*` de `settings.json` **não** têm comando (9.3 da arquitetura).

Nunca expostos como comando: `--purge-user-data`, a escada de primeiro plano, `SendInput`, `power` sem ticket e qualquer acesso a caminho de jogo.

## 3. Eventos (núcleo → UI): 8

Permissão de ouvir: `core:event:allow-listen` (a UI não emite eventos).

| # | Evento | Payload | Quando |
|---|---|---|---|
| 1 | `input://intent` | `{ intent: InputIntent, repeat: boolean }` | Sofá em primeiro plano (CA-06 a CA-11). Em segundo plano o núcleo não emite |
| 2 | `input://controllers` | `{ connected: 0..4 }` | Conexão ou desconexão, em ≤ 2 s (CA-13 a CA-15) |
| 3 | `library://changed` | `LibrarySnapshot` | Varredura, renomear, ocultar, mostrar, fim de sessão (com `focusGameId` do jogo que fechou, CA-30) |
| 4 | `session://state` | `SessionDto` | Toda transição do diagrama 2.4 e também mudança de `dismissed`, `elevated`, `minimized` ou `launchFailure` |
| 5 | `session://panel` | `{ open: true, outcome: FrontOutcome }` | Combinação de retorno (CA-32) concluída. `FrontAfterMinimize` faz a UI mostrar o aviso "O jogo foi minimizado" |
| 6 | `volume://changed` | `VolumeState` | Mudança interna ou externa (teclado, mixer) e troca de dispositivo (CA-41, CA-42) |
| 7 | `display://changed` | `MonitorDto[]` | Monitor ligado ou desligado (CA-66, CA-67) |
| 8 | `system://resumed` | `{}` | Volta da suspensão (CA-46) |

Exemplo (#4, jogo aberto e a tela de espera dispensada):
```json
{ "state": "Launching", "gameId": "steam:620", "since": 1794610123456, "dismissed": true,
  "elevated": false, "minimized": false, "launchFailure": null }
```

## 4. Estados de sessão e o que cada comando faz

| Estado | `session_launch` | `session_close` | `session_dismiss` | `session_retry` | `session_return_to_game` | `session_force_close` |
|---|---|---|---|---|---|---|
| `Idle` | inicia | `NoActiveSession` | nada | `InvalidState` | `NoActiveSession` | `NoActiveSession` |
| `Launching` | `SessionAlreadyActive` | `InvalidState` | sai da espera, continua | `InvalidState` | `InvalidState` | `NotOverdue` |
| `NeedsElevation` | `SessionAlreadyActive` | `InvalidState` | → `Idle` | `InvalidState` | `InvalidState` | `NoActiveSession` |
| `NotConfirmed` | `SessionAlreadyActive` | `InvalidState` | → `Idle` | refaz o início | `InvalidState` | `NoActiveSession` |
| `Running` | `SessionAlreadyActive` | → `Closing` | nada | `InvalidState` | volta ao jogo | `NotOverdue` |
| `Ending` | `SessionAlreadyActive` | `InvalidState` | nada | `InvalidState` | `InvalidState` | `NotOverdue` |
| `Closing` | `SessionAlreadyActive` | `InvalidState` | sai da espera, continua | `InvalidState` | `InvalidState` | `NotOverdue` |
| `CloseOverdue` | `SessionAlreadyActive` | `InvalidState` | → `Running` | `InvalidState` | `InvalidState` | força |

`session_launch_with_uac` só em `NeedsElevation`; `session_replace` só em `Running` (em `CloseOverdue` e nos demais → `InvalidState`) e só pede o fechamento normal; o novo jogo abre depois de `Idle` (CT-01).

## 5. Códigos de erro (26)

| Código | `params` | Comandos | Significado |
|---|---|---|---|
| `Busy` | — | #2, #11, #12, #13, #17 | Outra ação serializada em andamento |
| `InvalidRequest` | — | todos | Campo extra/ausente, tipo errado, `GameId`/`MonitorId`/ticket mal formado, comprimento fora do limite |
| `Internal` | — | todos | Falha inesperada ou pânico capturado; detalhe só no log |
| `Duplicate` | — | #3 | Jogo já cadastrado (CA-20) |
| `NotAnExecutable` | — | #3 | `.exe` sem `MZ`/`PE` válidos (CA-72) |
| `PathNotAllowed` | — | #3, #9 | **No cadastro e no navegador de pastas**: local fora dos discos do PC (rede, UNC, mapeada, prefixos especiais, ADS). CA-71, 1º momento |
| `NetworkPathBlocked` | — | #11, #12, #13, #17 | **Ao iniciar jogo** cujo caminho é de rede (ex.: biblioteca editada à mão). CA-71, 2º momento. Distinto de `PathNotAllowed` |
| `NotFound` | — | #3, #9 | Pasta ou arquivo inexistente |
| `AccessDenied` | — | #9, #16 | #9: sem permissão para ler a pasta. #16: jogo elevado, o Windows não deixa encerrar (texto próprio do forçar) |
| `EmptyName` | — | #5 | Nome vazio depois de normalizar |
| `UnknownGame` | — | #5 a #8, #11, #12, #13, #17 | ID bem formado fora da biblioteca |
| `SessionAlreadyActive` | `gameId` (o jogo aberto) | #11, #12, #13 | Já há sessão (CA-31) |
| `GameFileMissing` | `gameId` | #11, #12, #13, #17 | Arquivo do jogo ausente (CA-28). O caminho não vai no `params` |
| `StoreNotInstalled` | `origin` | #11, #12, #13, #17 | Steam/Epic ausente |
| `NeedsElevation` | `gameId` | #11, #13, #17 | Jogo exige administrador; aviso da 1ª etapa (CA-73) |
| `ElevationDenied` | `gameId` | #12 | UAC recusado. Também aparece como `launchFailure` em `SessionDto` |
| `InvalidState` | — | #12, #13 (também em `CloseOverdue`), #15, #17 | Comando fora do estado permitido (seção 4) |
| `NoActiveSession` | — | #14, #15, #16 | Não há jogo aberto |
| `ForegroundDenied` | — | #14 | A escada não trouxe o jogo à frente |
| `NotOverdue` | — | #16 | Forçar fora de `CloseOverdue` |
| `NoAudioDevice` | — | #20, #21 | Sem dispositivo de áudio |
| `Unavailable` | — | #23 | Ação de energia indisponível (CA-47) |
| `TicketInvalid` | — | #24 | Ticket expirado, reutilizado, desconhecido ou invalidado por um `power_prepare` mais novo (CT-07) |
| `Rejected` | — | #24 | O Windows recusou o pedido |
| `AutostartFailed` | — | #26 | Não conseguiu ligar/desligar (inclui exe fora de `%ProgramFiles%`) |
| `UnknownMonitor` | — | #27 | `monitorId` fora de `display_monitors` |

Exemplos:
```json
{ "code": "NetworkPathBlocked" }
{ "code": "GameFileMissing", "params": { "gameId": "gog:1000000001" } }
{ "code": "StoreNotInstalled", "params": { "origin": "Epic" } }
{ "code": "InvalidRequest" }
```

## 6. Alinhamento com `ui/src/shared/domain/errors.ts`

O catálogo da UI cobre **23 dos 26** códigos (faltam `ElevationDenied`, `InvalidRequest`, `Internal`). Divergências e decisões:

| # | Divergência | Decisão |
|---|---|---|
| D1 | `ElevationDenied` não existe em `errors.ts` | Novo no contrato. A UI precisa do texto "O Windows não deu permissão para abrir <nome>" (microcopy) e de lê-lo também em `SessionDto.launchFailure` |
| D2 | `InvalidRequest` e `Internal` não existem em `errors.ts` | Novos. Hoje caem em "Desconhecido" ("Algo deu errado no Sofá. Tente de novo."), que serve para os dois. Basta mapear explicitamente |
| D3 | `GameFileMissing`: a arquitetura 4.1 trazia `{path}` e a microcopy "em <caminho>" | O contrato usa `{ gameId }` (nenhum caminho cruza a IPC, SEC-03). A UI resolve o nome pela biblioteca; `errors.ts` já não mostra caminho |
| D4 | `StoreNotInstalled`: `errors.ts` usa `ctx.storeName` | `params.origin` (`Steam`/`Epic`); a UI traduz para o nome da loja |
| D5 | `AccessDenied` tem dois textos (pasta e forçar) | Mantido um só código; a UI escolhe por comando (`forceCloseDenied` já faz isso) |
| D6 | `NeedsElevation` é erro de `session_launch` **e** estado de `SessionDto` | Mantido: o erro dispara a 1ª etapa; o estado persiste até "Continuar" ou "Voltar" |
| D7 | `errors.ts` usa `NetworkPathBlocked` só em `session_launch`/CA-71 | Confirmado. `PathNotAllowed` só no cadastro e no navegador. Também vale para `session_replace` |
| D8 | `NotConfirmed` (60 s) e UAC aguardando não são códigos | São **estados** de `SessionDto` (`NotConfirmed`; `Launching` + `elevated`), não erros |
| D9 | Esperas "Abrindo…" e "Fechando…" | `session_dismiss` com a semântica da seção 2.2; o campo `dismissed` evita a UI reabrir a tela a cada evento |
| D10 | `errors.ts` não conhece `launchFailure`, `elevated`, `minimized`, `dismissed` | Campos novos de `SessionDto` (aviso de CA-73, aviso do painel E-23 e retorno minimizado). Não são códigos de erro |

Nenhum código de `errors.ts` ficou sem correspondente no contrato.

## 7. Verificação (BE-08 e BE-10)

- Teste do CT-01: jogo de teste que ignora `WM_CLOSE` + `session_replace` não é encerrado; vai a `CloseOverdue` e só `session_force_close` confirmado o encerra; `session_replace` em `CloseOverdue` → `InvalidState`.
- Teste compara `generate_handler!` e `capabilities/main.json` com a tabela da seção 2 (30 nomes) e confirma que `purge` não aparece.
- Cada comando é chamado com campo extra, tipo errado e `GameId` malformado e deve devolver `InvalidRequest`.
- Bindings TypeScript são gerados por `tauri-specta` (fallback `ts-rs`) em `ui/src/shared/ipc/generated/` (combinado com o dev-frontend), sem edição manual.

## 8. Notas de implementação (BE-14, BE-16 a BE-21; rev. 3, sem mudança de comandos, eventos, DTOs nem códigos)

O contrato das seções 1 a 7 vale como está. Esta seção só registra como o núcleo cumpre os pontos que a UI e o QA observam.

- **Sessão (#10 a #18)**: `session_launch`, `session_launch_with_uac`, `session_replace`, `session_retry` e `session_return_to_game` rodam em thread de bloqueio (o UAC e a verificação de primeiro plano podem demorar) e a `SerialGate` é tomada antes de qualquer coisa (`Busy` tem prioridade sobre `Internal`). Sem o serviço de sessão ligado (falha no `setup`), as ações respondem `Internal`, `session_get` devolve `Idle` e `session_dismiss` não faz nada.
- **`session://state`**: emitido a cada mudança do `SessionDto` (inclusive `dismissed`, `elevated`, `minimized`, `launchFailure`); o `Idle` inicial não é evento. `since` é o instante de entrada no estado em ms desde a época Unix.
- **`session://panel`**: emitido **depois** de a escada de primeiro plano terminar (não antes), com o `outcome` real; só existe com sessão em `Running`, `Ending`, `Closing` ou `CloseOverdue`. Uma combinação enquanto outra escada roda é ignorada. A janela e o WebView2 já foram retomados antes da escada e a entrada volta ao modo `Foreground` logo depois dela.
- **`library://changed` no fim de sessão (CA-30)**: leva o último `LibrarySnapshot` conhecido com `focusGameId` do jogo que fechou. Até o BE-29 entrar, a lista de jogos desse evento é a do cache do núcleo (vazia fora do desenvolvimento); a UI deve manter o foco por `focusGameId`.
- **`session_replace`**: valida o novo jogo antes de pedir o fechamento. O jogo novo abre em thread própria quando a sessão chega a `Idle`; sem escada de primeiro plano nesse caso (o Sofá já está na frente, é a UI que pediu a troca).
- **`session_launch_with_uac` (AF-01)**: além de `NeedsElevation` e do mesmo `gameId`, o núcleo guarda a identidade do `.exe` (caminho, tamanho e data) no momento do erro 740 e recusa (`InvalidState`) se mudou até o pedido de UAC.
- **`session_return_to_game`**: devolve o foco ao jogo, confere que ele ficou em primeiro plano (`ForegroundDenied` se não) e só então esconde o Sofá e suspende o WebView2.
- **`input://intent`**: nunca emitido com o jogo na frente (modo `Background`). `input://controllers` é emitido uma vez na partida (estado inicial) e a cada mudança de quantidade.
- **`system://resumed`**: payload `{}`. `display://changed` passou a ser emitido com o BE-34 (ver a seção 10).

## 9. Notas de implementação (BE-22 a BE-30; rev. 4, sem mudança de comandos, eventos, DTOs nem códigos)

As seções 1 a 8 valem como estão. Esta seção registra o que a UI e o QA observam nos comandos de biblioteca (#1 a #9), no protocolo `cover://` e nas varreduras das lojas.

- **`library_get` (#1)**: lê só a cópia em memória: jogos manuais e camada do usuário (`library.json`) mais a lista da última varredura, que vem de `cache\scan.json` (descartável: ausente ou inválido volta vazio). A **primeira** chamada depois da partida (primeiro paint da UI) dispara uma varredura em segundo plano, uma vez só, e o resultado chega em `library://changed` (BE-40). `focusGameId` é o último jogado (CA-22); `null` se nunca jogou nada. O log INFO traz as marcas `primeiro_paint` (primeira leitura) e `navegavel` (primeira varredura concluída), em ms desde o início do processo, para o roteiro de medição do QA (M-02).
- **`library_refresh` (#2)**: devolve `()` assim que a varredura começa (thread própria); a `SerialGate` fica tomada até ela terminar, e chamada concorrente recebe `Busy`. Lojas em paralelo, cada uma isolada (erro ou `panic` de uma loja vira log, a loja some desta varredura e as outras e os jogos manuais carregam). Loja não instalada não é erro. `library://changed` é emitido sempre ao fim, com `focusGameId: null` (só o fim de sessão traz foco, CA-30).
- **Jogo de loja só inicia a partir da detecção atual (SEC-18)**: a lista que veio do cache só serve para mostrar; ao iniciar um jogo de loja antes da primeira varredura desta execução, o núcleo varre na hora (milissegundos) e resolve pelo resultado novo (o que o cache tinha e a loja não detecta mais responde `UnknownGame`). Sem cache e sem varredura ainda, o id de loja responde `UnknownGame`. Jogo manual resolve logo (revalidado a cada início: caminho de rede → `NetworkPathBlocked`, forma inválida → `GameFileMissing`).
- **`library_add_manual` (#3)**: a forma do caminho é checada antes e depois de canonicalizar. `InvalidRequest`: vazio, > 1024, controle, relativo, `C:jogo.exe` ou com `/`. `PathNotAllowed`: UNC, `\\?\`, `\\.\`, unidade de rede ou mapeada, link que resolve para rede. `NotAnExecutable`: extensão diferente de `.exe`, ADS (`jogo.exe:ads`), nome reservado, `..`, pasta chamada `x.exe`, arquivo sem `MZ` + `PE\0\0`. `NotFound`: arquivo ausente. `AccessDenied`: sem permissão de leitura. `Duplicate`: mesmo ID (mesmo caminho em outra caixa, CT-05). O nome é o do arquivo sem extensão, normalizado.
- **`game_rename` (#5)**: id desconhecido tem prioridade sobre nome vazio (`UnknownGame` antes de `EmptyName`). Renomear para o nome da origem não grava nome personalizado. `originalName` só é não nulo quando há nome personalizado.
- **`library_hidden` (#4), `game_hide` (#7), `game_unhide` (#8)**: ocultar nunca apaga arquivo; o oculto não volta nas varreduras; mostrar preserva nome personalizado e data de último jogo. Jogo de unidade desconectada some da biblioteca (E-18) e volta com nome, oculto e data quando a unidade reconecta (a camada do usuário não é apagada).
- **`fs_list_dir` (#9)**: `null`/vazio lista as unidades `DRIVE_FIXED`/`DRIVE_REMOVABLE` (`C:\`). Caminho `C:` vira `C:\`. Na raiz de uma unidade `parent` é `null` e `path` é `C:\`. Pastas antes de `.exe`, ordem sem acento, no máximo 2.000 entradas (`truncated`, CT-04). Ficam de fora arquivos que não são `.exe`, itens ocultos ou de sistema e links/junções (a navegação não sai para outra unidade sem passar pela regra). Erros: `PathNotAllowed` (UNC, `\\?\`, `\\.\`, ADS, `..`, unidade de rede ou mapeada, junção para rede), `NotFound` (inexistente ou não é pasta), `AccessDenied`, `InvalidRequest` (relativo, controle ou > 1024).
- **`cover://<gameId>`**: o campo `GameDto.cover` continua `cover://<gameId>`. O handler só atende `GameId` existente na biblioteca em memória; qualquer outra coisa (`..`, `epic:..`, id inexistente, malformado, método que não é GET) responde 404 e o arquivo de terceiro nunca vai ao WebView (SEC-08). Limites checados no cabeçalho antes de decodificar: arquivo ≤ 20 MB e imagem ≤ 8192×8192, só JPEG ou PNG. Capa reencodada em JPEG q85 até 600×900 (`cache\covers\`); sem capa, ícone do `.exe` em PNG até 256×256 (`cache\icons\`). Não há `assetProtocol`. **Atenção UI (WebView2)**: no Windows o wry só intercepta `http://cover.localhost/<gameId>` (ele converte de e para `cover://localhost/<id>`); um `<img src="cover://steam:620">` cru não chega ao handler. A UI precisa montar a URL de rede por esse caminho (`convertFileSrc(gameId, 'cover')` do `@tauri-apps/api/core`, só em `shared/ipc`). A CSP já libera `cover:` e `http://cover.localhost`.
- **Lojas (BE-25 a BE-27)**: arquivos de terceiros ≤ 4 MB, checado antes de ler. VDF: pré-checagem de profundidade 32 antes do parser (`STATUS_STACK_OVERFLOW` não é capturável). Steam: bit 4 de `StateFlags`, lista fixa de não-jogos, `installdir` de um componente, pasta confinada a `<biblioteca>\steamapps\common`. Epic: BOM removido, campos desconhecidos ignorados, DLC (`AppName` ≠ `MainGameAppName`) e instalação incompleta ignoradas, `LaunchExecutable` relativo e confinado. GOG: registro + `goggame-<id>.info` (`playTask` primário `FileTask`; `.info` presente e inválido derruba o jogo, não cai no registro), sem SQLite. Nenhum `unwrap` em dado de terceiro; item recusado vai para o log com o motivo.

## 10. Notas de implementação (BE-31 a BE-36, BE-40, BE-41, BE-19; rev. 5, sem mudança de comandos, DTOs nem códigos)

As seções 1 a 9 valem como estão. Esta seção registra o que a UI e o QA observam em volume, energia, monitor, "Iniciar com o Windows", instância única e no modo de remoção de dados. **Nenhum comando, evento, DTO ou código de erro novo**; os bindings TypeScript não mudam. `display://changed` (#7) e `volume://changed` (#6) passam a ser emitidos.

### Volume (#19 a #21, evento #6)

- Regra única em `domain::volume`: passo 5, faixa 0 a 100, mudo preserva o nível. RT no mudo desmuta e sobe; LT no mudo baixa o nível guardado e **mantém** o mudo; "Mudo" alterna e devolve o mesmo nível. No limite (100 subindo, 0 descendo) o comando devolve o estado sem gravar nada.
- O Core Audio roda numa thread MTA própria (`sofa-audio`) seguindo o dispositivo de saída padrão (`eRender`, `eConsole`). Os comandos esperam em thread de bloqueio, com limite de 2 s por chamada.
- `volume://changed` (estado completo, `available` incluso) sai: depois de `volume_step` e `volume_toggle_mute` (uma vez, sem repetir o mesmo estado), por mudança **externa** (teclado, mixer: callback do `IAudioEndpointVolume`) e por troca, entrada ou saída do dispositivo padrão (`IMMNotificationClient`). A gravação feita pelo Sofá leva um GUID de contexto próprio e o callback a ignora, então não há evento duplicado.
- Sem dispositivo de saída: `volume_get` devolve `{ level: 0, muted: false, available: false }` e `volume_step`/`volume_toggle_mute` respondem `NoAudioDevice`. Quando um dispositivo aparece, o evento chega com `available = true`.

### Energia (#22 a #24)

- `power_capabilities`: desligar e reiniciar sempre `Available`; suspender só se `GetPwrCapabilities` informa S1, S2 ou S3 **e** `IsPwrSuspendAllowed()` permite (CA-47).
- `power_prepare` emite o ticket por CSPRNG do sistema (`BCryptGenRandom`, 256 bits, 43 caracteres em base64url sem preenchimento, dentro de `^[A-Za-z0-9_-]{22,64}$`), preso à `PowerAction`. **Um `power_prepare` novo invalida o anterior** (CT-07). Ação indisponível: `Unavailable`, sem ticket.
- `power_execute`: o ticket manda, a ação é a que ficou presa a ele. Ticket desconhecido, expirado (> 30 s), reutilizado ou invalidado: `TicketInvalid`. O ticket é consumido ao ser resgatado (também quando vence ou quando o Windows recusa). Um chute errado não cancela o ticket vigente. Se a capacidade sumiu entre o prepare e a confirmação: `Rejected`. `Rejected` também quando o Windows recusa o pedido.
- Desligar e reiniciar: `ExitWindowsEx(EWX_POWEROFF | EWX_REBOOT)` com `SHTDN_REASON_FLAG_PLANNED`, **sem nenhum bit de forçar**; o Windows mostra a tela de apps impedindo o desligamento (CA-49). Suspender: `SetSuspendState(FALSE, FALSE, FALSE)`; a chamada só volta quando o PC acorda. `SeShutdownPrivilege` é habilitado no token do processo só durante a chamada e desabilitado em seguida. Sem `shutdown.exe`.

### Monitor e configurações (#25 a #28, evento #7)

- `display_monitors`: lista dos monitores ligados, principal primeiro, depois por posição. `id` é o `monitorDevicePath` (estável entre sessões; cai para o nome GDI `\\.\DISPLAYn` se o DisplayConfig falhar), `name` é o nome do driver normalizado (SEC-07; vazio vira "Monitor N"), `current` marca onde o launcher está agora (que pode ser o principal por fallback, não a preferência).
- Regra do monitor (RN-20): o escolhido, se estiver ligado; senão o principal. O launcher abre na tela cheia sem borda no monitor da regra; só move a janela quando ele não é o principal.
- `settings_set_monitor`: id fora da lista **de agora** -> `UnknownMonitor` e nada é gravado; `null` volta ao principal. Grava a preferência em `settings.json` (a seção `advanced` fica intacta) e move o launcher na hora (CA-66); emite `display://changed`.
- Monitor desligado ou ligado (`WM_DISPLAYCHANGE`, avisos próximos juntados em uma reavaliação após 400 ms): se o monitor atual sumiu, o launcher cai no principal e **a preferência continua gravada** (CA-67). Se a TV volta com o Sofá aberto, ele **não pula de volta sozinho** (E-09); na próxima abertura volta ao escolhido. Mudança de resolução no monitor atual reajusta a tela cheia. `display://changed` leva a lista completa e só é emitido quando ela (ou o monitor atual) mudou.
- Com jogo aberto (janela oculta), o movimento fica pendente e acontece quando a janela volta.
- `settings_get.autostart` e `autostartMechanism` agora vêm do Windows (abaixo).

### Iniciar com o Windows (#25, #26)

- Mecanismo preferido: tarefa de logon do próprio usuário (`ITaskService`, montada campo a campo; nome `Sofa Autostart (<SID>)`; `LeastPrivilege`, `InteractiveToken`, gatilho do mesmo SID, ação `"<exe>" --autostart`, `PT0S`, `IgnoreNew`; sem UAC). Se a criação falha (ex.: `E_ACCESSDENIED`) ou o conteúdo gravado não confere, o núcleo remove a tarefa e grava `HKCU\...\Run` (`"<exe>" --autostart`) sozinho, com o motivo no log. Destino único do fallback: o Run. Nunca outra forma de tarefa, `schtasks.exe` ou elevação. Um mecanismo por vez; desligar remove os dois.
- Na abertura (só com o exe em `Program Files`, em thread própria): tarefa com o nome do Sofá e conteúdo diferente (`UserId`, `RunLevel`, `LogonType`, gatilho, ação ou `Path`) é removida e nunca tratada como válida; tarefa nossa cuja última execução falhou de verdade (`LastTaskResult` fora de 0 e da faixa 0x41300 a 0x4132F, que inclui `0x41301`, `0x41303` e `0x41325`) troca para o Run e **não volta sozinha**; religar a opção volta a tentar a tarefa. Run desligado no Gerenciador de Tarefas (`StartupApproved`) aparece como `autostart = false` com `autostartMechanism = "Run"`; ligar de novo tira o "desligado".
- `settings_set_autostart(true)` fora de `%ProgramFiles%` (cópia portátil, build de desenvolvimento) responde `AutostartFailed` e não cria nada (DR-04). Desligar funciona de qualquer pasta. O mesmo `AutostartFailed` vale se o usuário do Windows não pôde ser identificado.
- Padrão do instalador: na primeira abertura do perfil com o Sofá instalado (e `installDefaultsApplied` ausente), `HKLM\Software\app.sofa.launcher\InstallDefaults` DWORD 1 liga; 0, 7, qualquer outro número, ausente ou de outro tipo deixa desligado. Marca como aplicado; se ligar falhar, não marca e tenta de novo na abertura seguinte. A cópia de desenvolvimento (compartilha `settings.json`) não aplica nem marca.
- **Como testar em desenvolvimento** (o build de dev nunca está em `Program Files` e o usuário de dev é comum): os testes `win-integration` (`cargo test -p launcher-win --features win-integration --test autostart_integration`) usam, em cada teste, uma tarefa `Sofa QA Autostart <pid>x<n> (<SID>)` e um valor `SofaQaTest<pid>x<n>` de nome único (rodam em paralelo, sem `--test-threads=1`) e apagam o que criam. O fluxo de ponta a ponta com o `settings_set_autostart` real só roda com o Sofá instalado em `Program Files` (QA-17).

### Instância única e linha de comando (SEC-19, SEC-31)

- `tauri-plugin-single-instance` 2.5.5 (mutex `app.sofa.launcher-sim` no namespace da sessão, sem SID: desvio aceito no SG-03). O callback **descarta `argv` e `cwd`** e só traz a janela: com a janela visível, restaura e foca; com jogo em tela (janela oculta), faz a volta da combinação de retorno. `sofa.exe C:\Windows\System32\calc.exe --launch x` só traz a janela; nenhuma ação sai da linha de comando de uma segunda instância.
- A linha de comando é lida uma vez no `main`, antes de Tauri, WebView2 e instância única (`domain::startup`): `--autostart` como único argumento marca a origem `autostart` nas marcas de partida do log (`primeiro_paint` e `navegavel` ganham o campo `origem`); `--purge-user-data` como único argumento entra no modo de remoção; `--purge-user-data` com qualquer outro argumento sai com código 2 sem apagar nada; todo o resto é ignorado e abre normal. **Atenção ao QA**: com o Sofá aberto, `sofa.exe --purge-user-data` sai com código 4 sem apagar nada (o modo é um processo à parte, nunca passa pela instância única, mas confere o mutex dela); o teste de "só traz a janela" vale para os argumentos que o processo ignora.

### Modo `--purge-user-data` (BE-41)

- Sem janela, sem WebView2, sem IPC (não está em `command_list`, nem na capability, nem no `generate_handler!`). Códigos de saída: **0** ok (ou pasta inexistente), **1** parcial (log em `%TEMP%\sofa-purge.log` com o que ficou), **2** alvo inválido ou argumento extra, **3** rodando elevado (`TokenElevation`), **4** Sofá aberto (o mutex da instância única `app.sofa.launcher-sim` existe; SEC-BUG-03). Nada é apagado em 2, 3 e 4. O desinstalador fecha o Sofá e roda o purge de novo; o 4 nunca dispara o fallback.
- O alvo é calculado no processo: `FOLDERID_LocalAppData` + `app.sofa.launcher` (fixo no build, conferido contra o `tauri.conf.json`). Travas (SEC-25), lidas do handle aberto sem seguir link: não é ponto de reanálise, o caminho final está exatamente um nível abaixo de `%LOCALAPPDATA%` e contém `library.json` ou `settings.json`.
- Remoção por handle (`FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`, atributos do handle, caminho final do filho dentro do caminho final do pai, `FileDispositionInfoEx` com fallback): ponto de reanálise é apagado sem descer; pastas são abertas sem `FILE_SHARE_DELETE` enquanto percorridas.
- Remoção parcial (ex.: arquivo de `EBWebView\` em uso): o resto sai e o código é 1. Como `library.json` e `settings.json` já saíram, uma segunda rodada recusa (código 2): o que sobrar é o "resto documentado" do desinstalador (ADR-0006 §6b).

## 11. Notas de implementação (BE-37, BE-38, BE-39, BE-42, BE-46; rev. 6, sem mudança de comandos, eventos, DTOs nem códigos)

O instalador, o desinstalador e o release não têm IPC. Esta seção é o contrato do que o instalador **grava, lê e executa**, e dos pontos em que a implementação detalhou ou divergiu do ADR-0006 rev. 6.

### Instalador (BE-37)

- Artefato: `Sofa_<versão>_x64-setup.exe` (NSIS 3.13 do Tauri, `installMode: perMachine`, pede UAC). Pasta `%ProgramFiles%\Sofa` (ASCII); nome exibido **"Sofá"** (atalho, "Aplicativos instalados", telas) vem do template, porque `productName` fica "Sofa" para o nome do arquivo e da pasta. Binário `sofa.exe`, identificador `app.sofa.launcher`.
- Template próprio `src-tauri/windows/installer.nsi`, fork do template do Tauri 2.12.4 (diferenças listadas no cabeçalho do arquivo). Arquivos: `hooks.nsh` (`installerHooks`), `install-hooks.nsh`, `uninstall-hooks.nsh` (dono do guard de `System::Call`; a lista `NsisSystemCallOwners` ficou como estava), `PortugueseBR.nsh` (todas as telas).
- **Grava só** (SEC-29): a pasta de instalação, os atalhos (menu Iniciar de todos os usuários; o da área de trabalho é opção da página final, **desmarcada**), a entrada de "Aplicativos instalados" e `HKLM\Software\app.sofa.launcher`:

  | Valor | Tipo | Conteúdo |
  |---|---|---|
  | (padrão) e `InstallLocation` | REG_SZ | pasta de instalação, sem aspas |
  | `InstallDefaults` | REG_DWORD | `1` ou `0`, o que a caixa "Iniciar com o Windows" disse. O app só trata DWORD 0/1 (SEC-21) |

  Visão de 64 bits (`SetRegView 64`), a mesma que o app lê. **Não grava** em `HKCU`, não cria tarefa nem `Run`, não mexe em ACL (a pasta herda `Program Files`: Users só lê e executa, SEC-20) e não escreve na pasta de dados.
- Página "Iniciar com o Windows" (depois da pasta, antes de instalar): caixa **marcada** e a orientação de RN-19. Ao reinstalar, a caixa parte do `InstallDefaults` gravado (só o DWORD 0 desmarca); sem tela (`/S`, `/P`, `/UPDATE`) vale o que já estava gravado, ou 1.
- Abrir ao terminar: caixa da página final (marcada) com `nsis_tauri_utils::RunAsUser`: o app abre com o token do shell do usuário interativo (Medium), nunca como filho do instalador elevado (SEC-21).
- WebView2 (P-01): bootstrapper embutido, executado de `$PLUGINSDIR` (não de `$TEMP`), só se o runtime faltar e fora de atualização. Falha (sem internet) mostra o texto de CA-50 sem alteração: "Não foi possível baixar o WebView2. Conecte o PC à internet e tente de novo."
- Atualizar por cima (CA-53): o desinstalador anterior roda sempre com `/UPDATE`, que **não** apaga dados, não remove tarefa/`Run` e não apaga a chave do HKLM; o instalador novo grava de novo o mesmo `InstallDefaults`. Escolher "desinstalar" na mesma versão roda o desinstalador de verdade (sem `/UPDATE`).
- Licenças OFL (Figtree e Sora) em `<pasta>\licenses\`. Sem MSI, sem página de licença, sem seletor de idioma.
- Textos novos das telas (ainda não estavam no `microcopy.md`; revisão de design pendente): "Iniciar com o Windows" / "Escolha se o Sofá abre sozinho quando você entra no Windows." / "Vale para cada pessoa que usar o Sofá neste PC. Dá para mudar depois, em Configurações." / "Para usar o Sofá só com o controle, deixe o Windows entrar sozinho na área de trabalho: entrada automática, PIN ou Windows Hello. Isso é uma configuração do Windows, fora do Sofá, e o instalador não altera o seu login." / "Manter dados (biblioteca e configurações)". Mensagens do desinstalador: ver `PortugueseBR.nsh` (`sofaPurge*`).

### Desinstalador (BE-38, BE-42)

Ordem no `Section Uninstall`: **1.** fecha o Sofá (verificação do template, RestartManager; se não fechar, a desinstalação aborta e nada é apagado); **2.** `un.SofaBeforeUninstall` (só fora de `/UPDATE`); **3.** apaga os binários; **4.** apaga `HKLM\Software\app.sofa.launcher`.

- **Autostart de todos os perfis (BE-42, SEC-32)**, sempre fora de `/UPDATE`: para cada chave da `ProfileList` com SID em `^S-1-5-21(-[0-9]{1,10}){4}$` (validado caractere a caractere; outro formato é ignorado sem executar nada): `"$SYSDIR\schtasks.exe" /Delete /TN "Sofa Autostart (<SID>)" /F` (caminho absoluto, sem `cmd`; no stub de 32 bits `$SYSDIR` é `SysWOW64`, que opera o mesmo Agendador) e remoção do valor `Sofa` em `HKU\<SID>\...\Run` e `...\Explorer\StartupApproved\Run` (só hive carregado; hive descarregado deixa entrada órfã que o Windows ignora). Escolhido o `schtasks.exe` por SID, não o COM por prefixo: enumerar o Agendador por COM exigiria vtable de `ITaskService` com `VARIANT` por valor pelo plugin `System`, que o NSIS não faz com segurança.
- **"Manter dados"** marcada por padrão (CA-68); sem tela, vale "manter". Desmarcada, o desinstalador **decide** e o `sofa.exe --purge-user-data` (BE-41) apaga:
  1. Usuário interativo (DR-03, E-19): abre o Explorer da sessão (`GetShellWindow`, `PROCESS_CREATE_PROCESS | PROCESS_QUERY_LIMITED_INFORMATION`, sem `SeDebugPrivilege`). O SID vem do token do Explorer e, de outra fonte, do usuário da sessão deste processo (`WTSQuerySessionInformationW` + `LookupAccountNameW`); se as duas respondem e divergem, ou nenhuma responde, nada é apagado. O `LocalAppData` é `ProfileImagePath` (da `ProfileList`, nunca o `%LOCALAPPDATA%` do processo elevado) + `\AppData\Local`.
  2. Purge: `CreateProcessW` com `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS` = Explorer, `bInheritHandles = FALSE`, linha de comando constante `"<pasta>\sofa.exe" --purge-user-data`; espera até 60 s e lê `GetExitCodeProcess`. Se o `OpenProcess` ou o `CreateProcessW` falhar, cai no `RunAsUser` do plugin + `FindProcess` em laço de 60 s (código **não lido**).
  3. Decisão:

  | Resultado do purge | O desinstalador |
  |---|---|
  | código 0 | nada mais |
  | 1 (parcial) ou 3 (elevado) | fallback restrito; mostra a pasta |
  | 2 (alvo inválido) ou 4 (Sofá aberto) | **não** apaga nada; mostra a pasta |
  | não lido, ou outro valor | fallback restrito; mostra a pasta |
  | ainda rodando depois de 60 s (qualquer código) | **não** apaga nada; mostra a pasta |
  | perfil interativo não resolvido | **não** apaga nada; informa `app.sofa.launcher` em `AppData\Local` do usuário |

  "Mostra a pasta" = aviso com o texto e o caminho + `explorer.exe` aberto **como o usuário** (`RunAsUser`, caminho absoluto), nunca um Explorer elevado. Sem tela (`/S`), só o aviso é omitido; nada é apagado a mais.
- **Fallback restrito (D-43)**, elevado, **no máximo `library.json` e `settings.json`** na raiz, pelo nome exato (sem curinga, subpasta, recursão nem `Delete` do NSIS): (1) o `LocalAppData` do perfil é aberto e o caminho final tem que ser o mesmo texto do perfil (sem junction no caminho; perfil redirecionado de propósito cai em "nada apagado"); (2) a raiz é aberta **sem `FILE_SHARE_DELETE`** (`OPEN_REPARSE_POINT | BACKUP_SEMANTICS`) e tem que ser pasta, não ponto de reanálise, com caminho final = `LocalAppData\app.sofa.launcher`; (3) cada arquivo é aberto com `DELETE | FILE_READ_ATTRIBUTES` e `OPEN_REPARSE_POINT`; pelo handle: não é link nem pasta, caminho final = raiz final + nome, raiz final relida inalterada; (4) exclusão **pelo handle** (`FileDispositionInfoEx` com POSIX e ignora somente leitura; `FileDispositionInfo` no Windows anterior ao 1709). Hard link: some só o nome da raiz.
- Desvios e detalhes do ADR-0006: (a) o ADR dá o `System` como caminho do `CreateProcessW`; implementado como descrito, com o filho herdando o token do Explorer. A conferência **elevada** (integridade Medium do filho, usuário do Explorer e não da conta do UAC, `OpenProcess` com UAC de outra conta) só vale instalando: QA-17. (b) o guard de SEC-32 passou a aceitar executável por `$PLUGINSDIR\` e `$INSTDIR\` além de `$SYSDIR\` (bootstrapper do WebView2 e desinstalador anterior); `$TEMP\`, nome relativo e variável continuam proibidos. (c) o template do Tauri tinha migração de WiX (`ExecWait` de `msiexec` lido do registro), WebView2 por download e instalador offline: saíram (o Sofá nunca foi MSI). (d) o desinstalador anterior só roda se estiver em `$INSTDIR` e `$INSTDIR` for a pasta fixa; **a leva 6 (SEC-BUG-05) tirou a página de pasta e recusa `/D=` diferente e instalação anterior fora de `Program Files`** (seção 12).

### Release e travas (BE-39, BE-46)

- `scripts/ci.ps1 -Release` (detalhes no README): MSVC na versão fixada, `cargo test` no MSVC, relatórios, `tauri build`, trava do manifesto, guards no `installer.nsi` gerado, testes do instalador, `SHA256SUMS.txt` (formato `sha256sum -c`, hash em minúsculas, dois espaços, LF) e `README.txt` com `Get-FileHash`. O CI de dev segue no `-gnu` e **não** ganhou passo MSVC por padrão (decisão documentada no README).
- **BE-46 (ARQ-BUG-08)**: `src-tauri/tests/manifest_resource.rs` lê o diretório de recursos do PE (tipo 24, por ID e idioma). MSVC: exatamente um `RT_MANIFEST`, ID 1, com `asInvoker`, `uiAccess="false"` e Common Controls v6, senão reprova. `-gnu`: só avisa (o `sofa.exe` de dev tem 2: 763 e 599 bytes, ID 1, idioma 1033) e exige que nenhum peça elevação. `SOFA_MANIFEST_EXE=<exe>` aponta o teste para o `sofa.exe` de release. O teste prova que reprova: PE sintético com um manifesto passa; com dois, sem Common Controls, com `requireAdministrator`, com `uiAccess="true"` ou com ID 2, reprova.
- Testes do instalador: `scripts/test-installer.ps1` (ver README). O que não roda aqui e vai para o QA-17: instalar, desinstalar, UAC de outra conta, `RunAsUser` e `CreateProcessW` com Explorer como pai estando elevado, `icacls` da pasta, `HKU` de outros perfis, DLL plantada ao lado do instalador e do `sofa.exe`, Windows 10 22H2 limpo (sem WebView2), upgrade por cima (CA-53), symlink real (script elevado).

## 12. Notas de implementação (ARQ-BUG-15, SEC-BUG-05, SEC-BUG-06, BE-51, BE-52; rev. 7, sem mudança de comandos, eventos, DTOs nem códigos)

Leva 6: correções do instalador e da sessão antes do teste do usuário. Nada muda na IPC. Esta seção é o contrato do que mudou por baixo.

### Fim da sessão no modo Job sem prazo fixo (ARQ-BUG-15)

- A porta do Job avisa `JOB_OBJECT_MSG_EXIT_PROCESS` **antes** de o objeto do processo sinalizar. A rev. 6 esperava até 500 ms (`EXIT_SIGNAL_WAIT_MS`) pelo handle com o `Mutex` do scanner preso; jogo grande que leva mais para desmontar o processo ficava para sempre em `Running` (o modo Job não tem varredura periódica), e `pids()` e `force_close()` podiam esperar 500 ms.
- Agora, para cada membro, a sonda registra uma espera no handle (`RegisterWaitForSingleObject`, `WT_EXECUTEONLYONCE | WT_EXECUTEINWAITTHREAD`). Quando o handle **de fato** sinaliza, o sistema chama uma callback que só faz `PostQueuedCompletionStatus` de um pacote próprio na porta do Job (chave `MEMBER_KEY`, PID em `dwNumberOfBytes`; `JobMessage::MemberExited(pid)`). A `job_loop` reavalia os membros. **Nenhuma espera com o `Mutex` preso e nenhum prazo**: a mensagem de saída do kernel só reavalia, e o pacote próprio encerra a sessão quando o processo saiu de verdade, com ou sem acompanhante vivo no Job.
- A espera sai (`UnregisterWaitEx` bloqueante, a callback só posta) antes de o handle fechar e antes de o Job fechar (o contexto da espera guarda um `Arc<Job>`).
- Testes: `process_probe::tests` (mensagem `EXIT_PROCESS` injetada com o membro vivo e acompanhante vivo: o `Mutex` do scanner não fica preso e a sessão termina quando o handle sinaliza; o pacote só chega depois de o handle sinalizar) e dois de ponta a ponta em `session_runtime.rs` com saída segurada por depurador (2 s), com e sem acompanhante. Medido: o kernel posta o `EXIT_PROCESS` junto com a liberação do processo, então a ordem "mensagem antes do handle" só é reproduzível injetando a mensagem.

### Instalador: pasta fixa (SEC-BUG-05, ARQ-BUG-16)

- A pasta é **sempre** `$PROGRAMFILES64\Sofa`. Saíram a página de pasta (`MUI_PAGE_DIRECTORY`) e o `RestorePreviousInstallLocation` do `.onInit`. `src-tauri/windows/install-dir.nsh` (`SofaFixInstallDir`, e a versão `un.`) recebe `$INSTDIR` (o que o `/D=` pôs) e `$SofaPrevDir` (valor de `HKLM\Software\app.sofa.launcher`) e devolve 0 (ok), 1 (`/D=` diferente) ou 2 (instalação anterior fora da pasta fixa).
- `.onInit`: 1 e 2 mostram a mensagem (`sofaInstallDirFixed`, `sofaPreviousInstallElsewhere`; `/SD IDOK` no modo silencioso) e abortam o instalador **antes de qualquer página e de qualquer `ExecWait`** do desinstalador anterior. `PageLeaveReinstall` repete a conferência logo antes do `ExecWait` (`Quit`). `un.onInit`: desinstalador fora da pasta fixa recusa (`sofaUninstallDirFixed`).
- `/D=` igual à pasta fixa é aceito. Qualquer outro (`C:\X`, `D:\Jogos\Sofa`, subpasta, sufixo) é recusado com a saída do NSIS, nunca ajustado em silêncio.
- Com a pasta fixa, a exceção `$INSTDIR\` do guard de SEC-32 deixa de ser condicionada (parecer da segurança, ponto (a)).

### Control Flow Guard (SEC-BUG-06)

- `.cargo/config.toml`: `[target.x86_64-pc-windows-msvc] rustflags = ["-C", "control-flow-guard"]`. Vale só para o alvo MSVC; a variável `RUSTFLAGS`, se existir, substituiria o arquivo. Trava: `src-tauri/tests/pe_hardening.rs` lê o `DllCharacteristics` do PE e reprova no MSVC sem `GUARD_CF`, ASLR (`DYNAMIC_BASE`), ASLR de 64 bits (`HIGH_ENTROPY_VA`) ou DEP (`NX_COMPAT`); no `-gnu` só avisa. Roda no `-Release` com `SOFA_MANIFEST_EXE` apontando o `sofa.exe` de release.

### Textos e encoding do instalador (revisao-instalador.md)

- 22 strings do `PortugueseBR.nsh` trocadas pelo catálogo final (`microcopy.md` §4) e as 3 `webview2Download*` removidas (não usadas); `{{product_name}}` virou `${DISPLAYNAME}`; `publisher` e `copyright` do `tauri.conf.json` passam a "Sofá" (`productName` segue "Sofa"); `allowDowngrades: false` em `bundle.windows`. Textos novos desta leva (não estão no catálogo do designer): `sofaInstallDirFixed`, `sofaPreviousInstallElsewhere`, `sofaUninstallDirFixed`. Boas-vindas e tela final com texto próprio (`MUI_WELCOMEPAGE_*`, `MUI_FINISHPAGE_TITLE/TEXT`).
- **BOM (medido, contraria a hipótese do designer):** o bundler do Tauri grava o próprio BOM UTF-8 no `installer.nsi` renderizado e no arquivo de idioma copiado. Fonte **com** BOM vira **dois** BOMs e o `makensis` para na linha 1 (`Invalid command`). Por isso as fontes ficam **sem** BOM, o gerado tem exatamente um, e o NSIS lê UTF-8. Guard `NSIS-BOM` em `scripts/ci-guards.ps1` (fonte sem BOM; gerado com um) e testes no `test-installer.ps1` (inclusive `ProductName`, `FileDescription` e `LegalCopyright` do instalador compilado com o acento).

### Testes de sessão e o helper (BE-51, BE-52)

- BE-51: o acompanhante dos testes de SEC-BUG-01 e SEC-BUG-04 vive 2 minutos (era 6 s e 20 s) e é derrubado no fim do teste (`kill_and_wait`); a asserção "segue vivo" passa a ler o handle do processo (código de saída), não uma busca por nome.
- BE-52: o `sofa-test-game.exe` ganhou `VERSIONINFO` e manifesto (`crates/launcher-win/test-game.rc`, `test-game.manifest`, `build.rs` com `embed-resource`, só para esse binário). Sem os metadados, o Defender o barra no MSVC (erro 225); com eles, `session_e2e` e `session_runtime` passam no MSVC. Nenhuma exclusão do Defender.
- `-Release` ganhou o passo `MpCmdRun -Scan -ScanType 3 -File` no instalador e no `sofa.exe`; acusou ou não concluiu, o build falha (`Invoke-DefenderScan`, com autoteste de um `MpCmdRun` de mentira).

## 13. Notas de implementação (BE-61 a BE-74; rev. 8, versão 0.2.0, sem mudança de comandos, eventos, DTOs nem códigos)

Versão 0.2.0: o produto passa a se chamar **SofaLauncher** (D-49, ADR-0007) e ganha "Fechar o launcher" (D-50, D-51, D-53). Continuam **30 comandos, 8 eventos e 26 códigos de erro**. Esta seção é o contrato do que mudou por baixo.

### "Fechar o launcher" reusa o `app_exit` (#30, BE-71)

- Sem comando novo, sem evento, sem DTO, sem código de erro e sem `power_prepare`/ticket: não é ação de energia (RN-35). A UI abre o `ExitDialog` (Cancelar à esquerda e em foco) e, ao confirmar, chama `app_exit` sem argumentos.
- `src-tauri/src/exit.rs`: `close_launcher(&dyn ExitEnv)` faz três coisas, nesta ordem: (1) uma linha `info` `fechar o launcher` com `estado` (`SessionState`) e `jogo` (`GameId`, ou `-`), sem caminho (SEC-18); (2) `logging::flush(300 ms)`; (3) `exit(0)`. O hook de `AppState.exit_hook` só chama isso.
- O `ExitEnv` tem só `session()` (leitura do estado), `flush_log()` e `exit()`. Por construção o caminho de saída **não** chama `force_close`, pedido de fechamento, `ForegroundBroker` (INV-03, SEC-28) nem `AutostartRegistry` (CA-82 itens 3 e 5, D-57). O jogo segue rodando porque o Job não tem `KILL_ON_JOB_CLOSE`: quando o processo sai, o kernel fecha o handle do Job e o jogo continua (INV-05).
- Reabrir com o jogo ainda aberto: primeira instância normal, sessão em `Idle`, jogo tratado como aberto fora do launcher (E-07, D-54).
- Travas: `src-tauri/tests/exit.rs` (ordem sessão, flush, exit; a linha chega ao arquivo antes do `exit`; o texto não cita `force_close`, `ForegroundBroker`, autostart, e o hook só chama `close_launcher`), guard `INV-05` para `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` em `scripts/ci-guards.ps1` (violação plantada no autoteste) e `session_runtime::fechar_o_handle_do_job_nao_encerra_o_jogo` (o jogo de teste segue vivo depois do `drop` do Job; vermelho com o limite ligado).

### O que muda e o que fica no nome (ADR-0007)

| Muda (o que o usuário lê) | Fica (o que o sistema usa) |
|---|---|
| `productName` `SofaLauncher`, `publisher`, `copyright`, `shortDescription`; título da janela (`window::MAIN_TITLE`); `DISPLAYNAME` do instalador (atalhos `SofaLauncher.lnk`, "Aplicativos instalados"); textos do `PortugueseBR.nsh`; textos do `coletar-logs.ps1`; nome do instalador `SofaLauncher_<versão>_x64-setup.exe`; READMEs; ícones | `identifier` `app.sofa.launcher` (pasta de dados, `HKLM\Software\app.sofa.launcher`, mutex), `mainBinaryName` `sofa` (`sofa.exe`, Run `Sofa`, tarefa `Sofa Autostart (<SID>)`), `INSTALLDIRNAME` `Sofa` (`C:\Program Files\Sofa`), `MANUKEY`/`MANUPRODUCTKEY`, `sofa.log`, `sofa.txt`, crates e variáveis NSIS |

- **R-11: `UNINSTKEY` fixa em `Software\Microsoft\Windows\CurrentVersion\Uninstall\Sofa`** no `installer.nsi` (antes derivava de `${PRODUCTNAME}`). Sem isso a 0.2.0 gravaria `Uninstall\SofaLauncher`, não acharia a 0.1.0 e o `PageReinstall` perderia a conferência do SEC-BUG-05. Trava: `test-installer.ps1` confere a definição no template **e** no `.nsi` gerado, com `MANUKEY`, `MANUPRODUCTKEY` e `INSTALLDIRNAME`; vermelho com a derivação de volta (conferido).
- `src-tauri/tests/command_list.rs` passa a travar `productName == "SofaLauncher"`, `identifier == "app.sofa.launcher"` e `mainBinaryName == "sofa"`, a versão única nos manifestos (`tauri.conf.json`, workspace, `ui/package.json` e `package-lock.json`; maior que a 0.1.0, com `allowDowngrades: false`) e o `icon.ico` igual ao da marca. `src-tauri/tests/window_title.rs` trava o título e a ausência do nome antigo nos textos de `src`.
- `PortugueseBR.nsh`: cada `LangString` é comparado, caractere a caractere, com a seção 4 do `microcopy.md` por `test-installer.ps1`. Fontes `.nsi` e `.nsh` seguem **sem BOM** (guard `NSIS-BOM` inalterado; o nome novo é ASCII, mas as outras strings têm acento). `test-installer.ps1` deixou de exigir acento em `DISPLAYNAME`/`ProductName` e trava `SofaLauncher`.

### Atualização por cima da 0.1.0 e o atalho legado (D-57, CA-53, BE-64, BE-68)

- Versão **0.2.0**. Com pasta, `UNINSTKEY`, `MANUPRODUCTKEY` e binário iguais, o `PageReinstall` confere a pasta fixa e roda o `uninstall.exe /UPDATE` da 0.1.0; dados e "Iniciar com o Windows" ficam.
- O desinstalador da 0.1.0 em `/UPDATE` não remove atalhos. `src-tauri/windows/legacy-shortcut.nsh` (`SofaRemoveLegacyShortcuts`, chamada no `NSIS_HOOK_POSTINSTALL`, portanto também em `/UPDATE`) apaga o `Sofá.lnk` antigo do menu Iniciar e da área de trabalho **só se** `IsShortcutTarget` confirmar que ele aponta para `$INSTDIR\sofa.exe` (mesmo macro e mesmo `UnpinShortcut` do desinstalador). Caminho fixo `<pasta>\Sofá.lnk`, sem curinga; só `$SMPROGRAMS` e `$DESKTOP`. Atalho ausente, inválido, para outro exe ou para o `sofa.exe` de outra pasta: nada acontece, sem erro. O nome com acento não está escrito na fonte (as `.nsh` não têm BOM): o caractere sai de `IntFmt "%c" 225`.
- Testes no harness (`tests/harness.nsi`, cenário `legacylnk`, só roda com o `utils.nsh` que o bundler gera em `target\<triplo>\release\nsis\x64`): remove o apontado para o `sofa.exe` da pasta, deixa o de outro exe, o de outra pasta, o ausente (sem erro), o arquivo inválido, os outros `.lnk` da pasta e o `SofaLauncher.lnk` novo; vermelho sem a conferência (conferido).

### Release e scripts (BE-66, BE-67)

- `ci-release.ps1`: instalador `SofaLauncher_${version}_x64-setup.exe` (igual ao que o Tauri gera, `productName_versão`), `SHA256SUMS.txt`, `README.txt` e `BUILD-INFO.txt` com o nome novo e a instrução de atualizar por cima da 0.1.0; a pasta `..\sofa-releases` fica.
- `coletar-logs.ps1`: textos exibidos com "SofaLauncher"; caminhos (`%LOCALAPPDATA%\app.sofa.launcher`, `sofa.log`), `sofa.txt`, o zip `sofa-logs-*.zip` e a tarefa `Sofa Autostart` ficam. Mesmo conjunto de arquivos no zip.
