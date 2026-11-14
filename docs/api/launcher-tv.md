# Contrato de API: launcher-tv (Sofá)

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
| 30 | `app_exit` | — | `()` (CA-05; o jogo em sessão continua aberto) | — |

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
- **`system://resumed`**: payload `{}`. `display://changed` (BE-34) ainda não é emitido: a mudança de topologia só vai para o log.

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
