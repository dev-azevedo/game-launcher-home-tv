# Threat model: launcher-tv

| Campo | Valor |
|---|---|
| Feature | launcher-tv (MVP do Game Launcher) |
| Branch | `feature/launcher-tv-20261113` (base `main`) |
| Modo | Design (threat model) |
| Entradas | `docs/specs/launcher-tv.md` (seção 14 = decisões); rev. 2: `docs/arch/launcher-tv.md` (rev. 2) e `docs/adr/0003` a `0006` |
| Revisões | rev. 1: threat model inicial. rev. 2 (2026-11-13): AF-01 a AF-10 aprovados pelo usuário; P-03 (tarefa agendada por usuário); emendas em SEC-08, SEC-10, SEC-21, SEC-22, SEC-25, SEC-28, SEC-29; novos SEC-31 e SEC-32; revisão da baseline da arquitetura (seção 10). rev. 4 (2026-11-13): arquitetura rev. 4 (D-32 Win10, D-35 teste só no fim); C-04 avaliado; emendas em SEC-18, SEC-22 e SEC-28 (seção 11) |
| Stack considerada | Tauri 2: núcleo Rust + UI web em WebView2. Se o arquiteto escolher outra stack, os SEC-xx de IPC/CSP valem para o equivalente (ex.: Electron `contextIsolation`/preload) |
| Data | 2026-11-13 |

Legenda de responsável sugerido: **back** = núcleo Rust (comandos, integração com Windows, instalador/bundler); **front** = UI web (WebView2); **bd** = persistência local (arquivos de biblioteca/config, leitura de bancos de terceiros).

---

## 0. Resumo executivo

O launcher não tem rede, login nem servidor. O risco está em ele ser um **executor de programas com tela bonita**: apertar A roda um `.exe` cujo caminho veio de um arquivo em disco (biblioteca própria ou manifesto de loja), e o app também mata processos e desliga o PC. Os riscos que mais pesam:

1. **XSS na UI = execução de código.** Nome de jogo vem de manifesto de terceiro e de nome de arquivo. Se a UI renderizar isso como HTML e houver comando IPC genérico (`run(path)`, shell, fs), um manifesto adulterado vira RCE. Mitigação: CSP estrita, nada de HTML a partir de dado, IPC só com comandos de domínio que recebem **ID**, nunca caminho ou PID (SEC-01 a SEC-06).
2. **Injeção via manifesto/URI.** `appid`, `installdir`, `AppName`, `playTasks` montam URI `steam://`, `com.epicgames.launcher://` ou linha de comando. Sem validação, dá para injetar argumentos ou apontar para fora da pasta do jogo (SEC-10 a SEC-14).
3. **Fechamento forçado matando o processo errado.** RN-10 associa processos por pasta de instalação; um manifesto com `InstallLocation = C:\` ou `C:\Windows` faria "Forçar fechamento" matar o sistema. Mitigação: pasta mínima, conjunto rastreado, handle aberto na detecção (SEC-15 a SEC-17).
4. **Rodar elevado.** Se o launcher rodar como admin (ex.: aberto pelo "Executar ao terminar" do instalador elevado), todo jogo roda como admin e uma falha na UI vira elevação. Mitigação: `asInvoker` e instalador que abre o app sem elevação (SEC-20, SEC-21).

Nenhum item exige servidor, criptografia nem gestão de segredos. Não há SQL próprio; o único SQL é a leitura opcional do banco local do GOG Galaxy (SEC-13).

---

## 1. Ativos

| ID | Ativo | Por que vale atacar | Impacto se comprometido |
|---|---|---|---|
| AT-01 | **Capacidade de iniciar processos** como o usuário logado | O launcher cria processos a partir de caminhos guardados em disco e de manifestos de terceiros | Execução de código arbitrário com os direitos do usuário (persistência, roubo de dados do perfil) |
| AT-02 | **Biblioteca local** (jogos manuais, IDs de loja, ocultos, nomes, datas) | Define o que roda ao apertar A | Trocar o executável de um jogo conhecido por outro binário |
| AT-03 | **Binários instalados** em `Program Files` e a entrada de autostart (HKCU `Run`) | Código que roda a cada login | DLL hijacking ou substituição do executável = persistência |
| AT-04 | **Controle de processos** (encerrar jogo, forçar fechamento) | Encerra processos do usuário | Perda de trabalho não salvo, encerramento de processos não relacionados |
| AT-05 | **Ações de energia** (desligar, reiniciar, suspender) via `SeShutdownPrivilege` | Afeta o PC inteiro | Negação de serviço, perda de dados em outros apps |
| AT-06 | **Integridade do sistema durante instalação/desinstalação** (rodam elevados) | Únicos momentos com privilégio de administrador | Elevação de privilégio ou apagar arquivos fora da pasta do app |
| AT-07 | **Dados de uso** (lista de jogos, caminhos com nome de usuário do Windows, horários de jogo) | Privacidade, pouco valor | Exposição baixa; nada sai do PC (RN-01) |
| AT-08 | Volume do sistema | Sem valor de ataque | Incômodo apenas |

---

## 2. Atores e premissas

- **Usuário dono** (confiável): escolhe executáveis, confirma ações. Ele próprio é a fonte de entrada mais comum, mas pode errar (ex.: escolher um `.exe` baixado da internet).
- **Software do mesmo usuário** (não confiável, mesmo nível de privilégio): qualquer processo rodando como o usuário consegue escrever no perfil dele, no HKCU e nos arquivos da biblioteca. **Premissa aceita:** não defendemos contra malware que já roda como o usuário (ele já consegue persistir sozinho via HKCU `Run`). Defendemos contra o launcher **ampliar** o que esse malware consegue: virar admin, executar a partir de dado que deveria ser só texto, matar o processo errado.
- **Lojas e seus arquivos** (Steam, Epic, GOG): escrevem manifestos que o launcher lê. Formato não documentado (R-08), pode mudar ou estar corrompido. Alguns ficam em `C:\ProgramData`, que outros usuários locais podem conseguir gravar dependendo da ACL criada pela loja.
- **Outros usuários locais do PC** (não confiáveis): o PC é pessoal, mas o Windows pode ter outras contas. Não podem gravar nos dados do usuário nem no `Program Files`.
- **Jogos** (semi-confiáveis): o usuário escolheu instalar, mas o launcher não deve dar a eles mais privilégio do que o usuário tem.
- **Fora de escopo:** atacante com acesso físico e teclado, administrador malicioso, anti-cheat de terceiros, vulnerabilidades do próprio jogo ou da loja.

---

## 3. Trust boundaries

| ID | Fronteira | Dado não confiável que entra | Quem escreveu o dado |
|---|---|---|---|
| TB-01 | **UI web (WebView2) → núcleo Rust (IPC)** | Chamadas de comando e argumentos | Código JS da UI. Se a UI for comprometida (XSS), o atacante controla tudo que passa aqui |
| TB-02 | **Manifestos de loja → núcleo** | `libraryfolders.vdf`, `appmanifest_*.acf` (Steam); `*.item` JSON em `ProgramData\Epic\...\Manifests` e `LauncherInstalled.dat`; registro `HKLM\...\GOG.com\Games`, `goggame-*.info`, banco SQLite do Galaxy | Lojas, e qualquer processo que consiga gravar nesses caminhos |
| TB-03 | **Núcleo → UI (renderização)** | Nomes de jogos, nomes de arquivos e pastas, caminhos em mensagens de erro (CA-28), imagens de capa | Manifestos, sistema de arquivos, usuário |
| TB-04 | **Navegador de pastas / cadastro manual** | Caminho escolhido (pode ser UNC, unidade de rede, `\\?\`, ADS, `.lnk`, `.bat`) | Usuário, mas o conteúdo das pastas é de qualquer um |
| TB-05 | **Arquivos de biblioteca e configuração** em disco | Conteúdo recarregado a cada abertura | O próprio app, mas qualquer processo do usuário pode editar |
| TB-06 | **Núcleo → Windows: criação de processo e URIs** | Caminho do executável, diretório de trabalho, argumentos, URIs `steam://`, `com.epicgames.launcher://`, linha de comando do GOG Galaxy | Biblioteca e manifestos (TB-02, TB-05) |
| TB-07 | **Enumeração de processos e janelas** | PIDs, caminhos de imagem, títulos, árvore pai/filho | Outros processos (PID é reaproveitado, nome de processo é falsificável) |
| TB-08 | **Imagens de capa e ícones** | JPG/PNG da pasta de cache da loja; recurso de ícone de `.exe` | Lojas, terceiros |
| TB-09 | **Segunda instância / linha de comando** | `argv` e diretório da segunda instância repassados à primeira (single-instance) | Qualquer processo que execute o launcher |
| TB-10 | **Instalador e desinstalador elevados** | Pasta de onde o instalador roda (ex.: Downloads), perfil resolvido, pasta de dados a apagar | Conteúdo de Downloads é de terceiros; perfil depende da conta usada no UAC (E-19) |
| TB-11 | **Entrada física e entrada sintética** | XInput, teclado de emergência, `SendInput` de outros processos | Usuário, ou processo do mesmo usuário injetando teclas |

---

## 4. STRIDE por fronteira

Severidade = risco **sem a mitigação** (para priorizar). Críticos e altos marcados com **[CRÍTICO]** / **[ALTO]**.

### TB-01 UI web → núcleo (IPC)

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| E | XSS na UI chama um comando genérico (`spawn(path, args)`, plugin `shell`, `fs` com escopo amplo, `opener` com URL arbitrária) e executa qualquer binário ou lê qualquer arquivo | **[CRÍTICO]** | Só comandos de domínio, argumentos tipados por ID/enum (SEC-03); nenhum plugin de shell/fs/http/opener exposto (SEC-04); capabilities só para a janela principal e só com origem local (SEC-05) |
| T | UI chama `close_game(pid)` ou `kill(pid)` com PID qualquer | **[ALTO]** | Comandos de processo não recebem PID; o núcleo usa o estado da sessão (SEC-15) |
| S | Conteúdo remoto carregado na WebView (navegação, iframe, link) ganha acesso ao IPC | **[ALTO]** | Sem conteúdo remoto; navegação bloqueada; CSP sem origens externas (SEC-01, SEC-02) |
| D | Chamadas IPC em loop (bug ou XSS) disparam `refresh_library`/`launch` repetidamente | Baixo | Comandos caros idempotentes e serializados no núcleo (SEC-06) |
| I | Mensagens de erro do núcleo com stack trace/caminhos internos na UI | Baixo | Erros mapeados para códigos de domínio; detalhe só no log (SEC-06) |

### TB-02 Manifestos de loja → núcleo

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| T/E | `appid`/`AppName`/`CatalogNamespace` com caracteres extras injetam parâmetros na URI de lançamento (ex.: `steam://run/<id>//<args>` aceita argumentos de linha de comando do jogo) | **[ALTO]** | Formato estrito por loja antes de montar URI; URI construída por template fixo com percent-encoding (SEC-10, SEC-11) |
| T/E | `installdir` com `..` ou absoluto; `InstallLocation` + `LaunchExecutable` resultando fora da pasta do jogo; executável apontando para `cmd.exe`/script | **[ALTO]** | Canonicalizar e exigir que o resultado fique dentro da raiz de instalação; extensão `.exe`; scripts recusados (SEC-12, SEC-14) |
| E | `InstallLocation` = raiz de unidade, `C:\Windows`, `Program Files` ou o perfil do usuário: a regra de RN-10 passaria a considerar processos do sistema como "do jogo" | **[ALTO]** | Lista de pastas proibidas e profundidade mínima (SEC-16) |
| D | Arquivo gigante, VDF com aninhamento profundo, JSON malformado, SQLite corrompido derruba ou trava o launcher | Médio | Limites de tamanho/profundidade, parser sem `panic`, falha isolada por loja (SEC-13; seção 11 da spec) |
| T | Banco SQLite do GOG Galaxy adulterado (views/triggers) executa funções ao ser lido | Médio | Abrir só leitura, `trusted_schema` desligado, sem extensões, queries fixas parametrizadas (SEC-13) |
| S | Outro usuário local planta manifesto em `ProgramData` (Epic/GOG) e um "jogo" aparece na biblioteca do dono | Baixo (PC pessoal) | Validações de SEC-12/SEC-14 limitam o alvo a `.exe` dentro de uma pasta de instalação plausível; aceito como risco residual (RR-03) |

### TB-03 Núcleo → UI (renderização)

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| E | Nome de jogo `<img src=x onerror=...>` vindo de manifesto ou nome de arquivo é renderizado como HTML → XSS → TB-01 | **[CRÍTICO]** (encadeado) | Renderizar só como texto; proibido `innerHTML`/`v-html`/`dangerouslySetInnerHTML`/`insertAdjacentHTML` com dado; CSP bloqueia script inline (SEC-01, SEC-07) |
| S | Caracteres de controle e bidi (U+202E) fazem um nome parecer outro | Baixo | Normalização de texto exibido (SEC-07) |
| I | Imagem servida por protocolo de assets com escopo amplo deixa a UI ler qualquer arquivo do disco (`asset://C:/Users/...`) | **[ALTO]** (com XSS) | Capas servidas por ID a partir de cache próprio; sem `assetProtocol` com escopo amplo (SEC-08) |

### TB-04 Navegador de pastas / cadastro manual

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| E | Usuário cadastra `.lnk`, `.url`, `.bat`, `.cmd`, `.ps1`, `.scr`, `.com`, `.pif`: execução indireta, ou argumentos mal escapados para `cmd.exe` (classe "BatBadBut", CVE-2024-24576 no `std::process::Command` do Rust < 1.77.2) | **[ALTO]** | Só `.exe` com cabeçalho PE válido; Rust >= 1.77.2 (SEC-09, SEC-24) |
| I | Caminho UNC (`\\host\share\jogo.exe`) ou unidade de rede: ao ler ícone ou iniciar, o Windows autentica no host e vaza hash NTLM; binário remoto pode mudar entre execuções | Médio | Bloquear UNC, `\\?\`, `\\.\` e unidades `DRIVE_REMOTE` (SEC-09; Ask first AF-02) |
| T | Fluxo de dados alternativo (`jogo.exe:payload`) ou nomes de dispositivo (`CON`, `NUL`) | Baixo | Rejeitar `:` fora da letra de unidade e nomes reservados (SEC-09) |
| I | Comando `list_dir` vira um listador de qualquer pasta para a UI | Baixo | Retornar só subpastas e arquivos `.exe`; nunca conteúdo de arquivo (SEC-09) |

### TB-05 Arquivos de biblioteca e configuração

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| T | Processo do mesmo usuário troca o caminho de um jogo manual na biblioteca | Baixo (aceito, RR-01) | Mesmas validações do cadastro reaplicadas na carga e no lançamento (SEC-18) |
| T | Dados gravados em `Program Files` ou `ProgramData` com ACL aberta: outro usuário local altera a biblioteca do dono | Médio | Dados só em `%LOCALAPPDATA%`/`%APPDATA%` do usuário com ACL herdada do perfil (SEC-18) |
| D | Escrita interrompida (queda de energia, suspensão) corrompe a biblioteca | Médio | Escrita atômica (arquivo temporário + `ReplaceFileW`/rename), validação de schema, backup e recriação (SEC-18; seção 11 da spec) |
| T | Biblioteca com campos extras ou tipos errados (ex.: `launch.args` injetado) | Médio | Schema fechado (`deny_unknown_fields`), versão de schema, limites de tamanho (SEC-18) |

### TB-06 Núcleo → Windows (processos e URIs)

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| E | Lançamento via `cmd /c`, `start`, `powershell` ou string única de linha de comando: injeção por `&`, `|`, `"` em caminho ou argumento | **[ALTO]** | `CreateProcessW` com caminho absoluto em `lpApplicationName`, sem shell; `ShellExecuteExW` só para URIs de esquema fixo (SEC-10, SEC-11) |
| E | Caminho com espaço sem aspas (`C:\Program Files\Jogo\x.exe` → `C:\Program.exe`) | Médio | `lpApplicationName` preenchido; autostart com caminho entre aspas (SEC-10, SEC-22) |
| E | Jogo com manifesto `requireAdministrator` dispara elevação automática, ou o launcher tenta contornar com `runas` | Médio | Nunca verbo `runas`; tratamento explícito do erro 740 (Ask first AF-01) |
| S | Esquema `steam://` registrado por outro app no HKCU | Baixo (mesmo usuário, RR-01) | Opção de lançar via `steam.exe -applaunch <appid>` com caminho lido do registro da Steam (decisão do arquiteto) |
| I | Herança de handles/variáveis do launcher para o jogo | Baixo | `bInheritHandles = FALSE`; ambiente padrão do usuário (SEC-10) |

### TB-07 Enumeração e encerramento de processos

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| T/D | PID reaproveitado entre a detecção e o "Forçar fechamento": mata processo não relacionado | **[ALTO]** | Abrir handle na detecção e encerrar pelo handle; conferir tempo de criação (SEC-15) |
| D | Associação por nome de processo (`game.exe`) mata outro programa com o mesmo nome | **[ALTO]** | Associação só por caminho completo canônico + árvore iniciada pelo launcher (SEC-15, SEC-16) |
| D | Encerrar o app da loja, `explorer.exe`, processos de outra sessão, processos protegidos ou o próprio launcher | **[ALTO]** | Lista de exclusão e filtros (SEC-17) |
| E | Habilitar `SeDebugPrivilege` ou abrir processos com `PROCESS_ALL_ACCESS` | Médio | Direitos mínimos: `PROCESS_QUERY_LIMITED_INFORMATION`, `SYNCHRONIZE`, `PROCESS_TERMINATE` só no fechamento (SEC-17) |

### TB-08 Imagens e ícones

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| D | Imagem bomba de descompressão (dimensões enormes) trava ou estoura memória | Médio | Limite de tamanho de arquivo e de dimensões antes de decodificar; decodificação no núcleo com timeout (SEC-08) |
| E | Bug de decodificador do WebView ao abrir imagem crua de terceiro | Baixo | Reencode no núcleo para PNG/WebP de tamanho fixo em cache próprio (SEC-08) |

### TB-09 Segunda instância / linha de comando

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| E | `argv` repassado pelo plugin single-instance vira comando (ex.: `--launch <path>`, deep link) | Médio | Ignorar `argv` da segunda instância, salvo flag `--autostart` sem valor; sem esquema de URI próprio (SEC-19) |
| S | Mutex/pipe de instância única em namespace global deixa outra sessão bloquear o app | Baixo | Nome no namespace `Local\` com identificador do usuário (SEC-19) |

### TB-10 Instalador e desinstalador

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| E | "Executar ao terminar" abre o launcher **elevado** herdado do instalador: todo jogo passa a rodar como admin | **[ALTO]** | App aberto pelo instalador com o token do usuário não elevado (SEC-21) |
| E | DLL planting: instalador elevado rodando de `Downloads` carrega DLL plantada ao lado dele | **[ALTO]** | Bundler com carregamento seguro de DLL (NSIS >= 3.x com `SetDefaultDllDirectories`, ou MSI); verificação (SEC-23) |
| E | Pasta de instalação com ACL afrouxada (gravável por usuários) → troca de binário/DLL que roda a cada login | **[ALTO]** | `Program Files` com ACL herdada; nada gravado na pasta de instalação em tempo de execução (SEC-20, SEC-23) |
| T | Desinstalador elevado apaga pasta de dados derivada de config, de variável errada (perfil do admin do UAC, E-19) ou segue junction/symlink e apaga fora da pasta | **[ALTO]** (precisa verificação no bundler escolhido) | Caminho fixo `%LOCALAPPDATA%\<identificador>`, sem seguir pontos de reanálise, profundidade mínima (SEC-25) |
| S | Instalador sem assinatura (D-11): usuário pode executar uma cópia adulterada | Médio (aceito, RR-02) | Publicar SHA-256 do instalador junto do build; conferir antes de instalar (SEC-26) |
| E | (rev. 2) Desinstalador elevado percorre e apaga uma árvore que o usuário comum pode alterar: entre a checagem de reparse point e a remoção, um processo do usuário troca uma subpasta por junction e transforma a remoção em "apagar arquivo arbitrário como admin" (cadeia conhecida de elevação usuário → admin/SYSTEM) | **[ALTO]** | Remoção de dados feita **sem elevação**, como o próprio usuário (SEC-31) |
| E | (rev. 2) Desinstalador elevado executa `schtasks.exe` por nome relativo ou com linha de comando montada com SID não validado | Médio | Caminho absoluto em `%SystemRoot%\System32`, SID com formato validado (SEC-32) |

### TB-11 Entrada física e sintética

| STRIDE | Ameaça | Sev. | Mitigação |
|---|---|---|---|
| D | Toque acidental ou processo injetando teclas (`SendInput`) dispara desligamento | Baixo | Confirmação com "Cancelar" em foco (RN-14); núcleo nunca usa encerramento forçado (SEC-27) |
| I/R | Ler entrada com hook global de teclado (`WH_KEYBOARD_LL`) para a combinação de retorno: comportamento de keylogger, alerta de anti-cheat (R-07) | Médio | Só XInput por polling; nenhum hook global, injeção de DLL ou leitura de memória de outro processo (SEC-28) |

---

## 5. Abuse cases

| US | Uso esperado | Abuse case | Requisitos que bloqueiam |
|---|---|---|---|
| US-03 | Jogos da Steam/Epic/GOG aparecem sozinhos | Manifesto Epic com `DisplayName = "<img src=x onerror=...>"` executa script na UI e chama IPC para rodar `powershell.exe` | SEC-01, SEC-03, SEC-04, SEC-07 |
| US-03 | Idem | `appmanifest` com `installdir = "..\..\..\Windows"`: sessão associa processos do sistema; "Forçar fechamento" os mata | SEC-12, SEC-16, SEC-17 |
| US-03/US-06 | Iniciar jogo Epic | `AppName = "x?action=launch&args=..."` ou com `/` e `&` injeta parâmetros na URI | SEC-11 |
| US-03/US-06 | Iniciar jogo GOG | `goggame-*.info` com `playTasks.path = "C:\Windows\System32\cmd.exe"` e `arguments = "/c ..."` | SEC-12, SEC-14 |
| US-04 | Adicionar executável | Usuário escolhe `atalho.lnk`/`script.bat` com argumento contendo `&` | SEC-09, SEC-10, SEC-24 |
| US-04 | Adicionar executável | Escolhe `\\192.168.0.50\share\jogo.exe`: hash NTLM vaza para o host; binário remoto troca entre execuções | SEC-09, AF-02 |
| US-05 | Remover jogo | Bug transforma "remover" em apagar arquivo do disco | SEC-29 |
| US-06 | Iniciar com A | Processo do mesmo usuário edita a biblioteca para o "último jogado" apontar para outro `.exe` | RR-01 (aceito), SEC-18 reaplica validação |
| US-07 | Fechar jogo | PID reaproveitado após o jogo morrer: "Forçar fechamento" mata outro processo | SEC-15 |
| US-07 | Fechar jogo | Jogo da Steam: launcher mata `steam.exe` junto | SEC-17 |
| US-10 | Desligar | Chamada IPC direta com flag de força (`EWX_FORCE`) derruba apps com trabalho não salvo | SEC-27 |
| US-12 | Instalar | Instalador abre o launcher elevado ao final; jogos e WebView passam a rodar como admin | SEC-20, SEC-21 |
| US-12 | Instalar | `version.dll` plantada em Downloads é carregada pelo instalador elevado | SEC-23 |
| US-12 | Desinstalar com "apagar dados" | Pasta de dados contém junction para `C:\Users\<user>\Documents`; desinstalador elevado apaga documentos | SEC-25 |
| US-13 | Iniciar com o Windows | Entrada `Run` sem aspas executa `C:\Program.exe` plantado | SEC-22 |
| US-01/US-14 | Abrir/sair | Segunda instância com `--launch C:\malware.exe` faz a primeira executar | SEC-19 |

---

## 6. Requisitos SEC-xx

Cada requisito é testável. "Teste" indica como o QA ou a verificação de segurança confirma.

### UI web e IPC (Tauri)

**SEC-01 [CRÍTICO] CSP estrita e sem conteúdo remoto.** (front + back)
CSP definida na config do Tauri, no mínimo: `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' <protocolo de capas próprio>; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'`. Proibido `unsafe-eval` e `unsafe-inline` em `script-src` (se a lib de UI exigir `unsafe-inline` em `style-src`, registrar no ADR). Nenhum script, fonte ou imagem de CDN; todos os assets empacotados. `dangerousDisableAssetCspModification` não usado.
Teste: inspecionar `tauri.conf.json`; no build de release, tentar `eval('1')` e injetar `<script>` inline pela UI e confirmar bloqueio por CSP.

**SEC-02 [ALTO] Navegação e janelas travadas.** (back)
A WebView só carrega a origem do app. Handler de navegação recusa qualquer URL fora da origem local; `window.open`/novas janelas recusadas; links externos não existem no MVP. DevTools desabilitado no release (sem feature `devtools`); sem `--remote-debugging-port` nem argumentos extras de navegador. Menu de contexto e atalhos do navegador (F5, F12, Ctrl+P, Ctrl+F) desabilitados no release.
Teste: no release, F12/Ctrl+Shift+I não abrem DevTools; `location.href = 'https://example.com'` não navega.

**SEC-03 [CRÍTICO] IPC só com comandos de domínio, por ID.** (back + front)
Comandos expostos à UI são uma lista fechada de ações do domínio (ex.: `library_list`, `library_refresh`, `game_launch(game_id)`, `game_rename(game_id, name)`, `game_hide(game_id)`, `game_restore_name(game_id)`, `session_focus`, `session_close`, `session_force_close`, `volume_get`, `volume_step(dir)`, `volume_toggle_mute`, `power(action)`, `settings_get/set`, `fs_list_dir(path)`, `game_add_manual(path)`, `app_exit`). Proibido comando genérico que receba caminho de executável, argumentos, URI, PID, chave de registro ou comando de shell. Exceção única com caminho: `fs_list_dir` e `game_add_manual`, com as validações de SEC-09. Todo argumento é desserializado em tipo fechado (enum, ID com formato validado, string com limite) e o desconhecido é rejeitado.
Teste: listar handlers registrados (`generate_handler!`) e comparar com a lista aprovada no doc de arquitetura; chamar cada comando com tipo errado/campo extra e confirmar rejeição.

**SEC-04 [CRÍTICO] Nenhum plugin perigoso exposto à UI.** (back)
Não habilitar para a UI: `shell` (execute/spawn/open), `fs` (exceto se nenhum escopo), `http`, `opener` com URL/caminho arbitrário, `process` além do necessário, `updater`, `sql`, `dialog` de arquivo do sistema (o navegador de pastas é próprio, por controle). Se algum for imprescindível, escopo mínimo e item novo no doc com aprovação.
Teste: revisar `capabilities/*.json` e `Cargo.toml`/`package.json`; nenhuma permissão `shell:*`, `fs:*`, `http:*`, `opener:allow-open-url`/`allow-open-path` com escopo amplo.

**SEC-05 [ALTO] Capabilities mínimas.** (back)
Uma capability para a janela `main`, sem `remote` (nenhum domínio externo), contendo só as permissões dos comandos de SEC-03 e as permissões core estritamente necessárias (janela: tela cheia, foco, minimizar). `withGlobalTauri` desligado.
Teste: revisar arquivos de capability; nenhuma entrada com `remote.urls`.

**SEC-06 [MÉDIO] Erros e chamadas repetidas no IPC.** (back)
Erros retornados à UI são códigos de domínio (`GameNotFound`, `ExecutableMissing`, `StoreUnavailable`...) com mensagem amigável; detalhe técnico só no log local. `library_refresh` e `game_launch` serializados (uma execução por vez; chamadas concorrentes descartadas ou enfileiradas), respeitando RN-09.
Teste: disparar `game_launch` 20 vezes seguidas e confirmar no máximo um processo de jogo.

**SEC-07 [CRÍTICO] Dado externo renderizado só como texto.** (front + back)
Nomes de jogo, nomes de arquivo/pasta, caminhos e mensagens são inseridos via escape da framework ou `textContent`. Proibido `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `v-html`, `dangerouslySetInnerHTML`, `{@html}` com qualquer dado que não seja constante. O núcleo normaliza nomes antes de enviar: remove caracteres de controle (C0/C1) e de override bidi (U+202A a U+202E, U+2066 a U+2069) e limita a 256 caracteres; renomear (CA-61) aplica as mesmas regras.
Teste: manifesto de teste com nome `<img src=x onerror=alert(1)>` e `\u202Eexe.txt` aparece literal, sem execução; grep no front pelos sinks listados.

**SEC-08 [ALTO] Capas por ID, cache próprio, sem protocolo de assets amplo.** (back + front)
A UI pede a capa por `game_id` (ex.: protocolo próprio `cover://<game_id>`); o núcleo resolve o arquivo. Não usar `assetProtocol` do Tauri com escopo para unidades inteiras, `$HOME/**` ou pastas das lojas. Imagens de terceiros e ícones de `.exe` são lidos no núcleo com limite de arquivo (ex.: 20 MB) e de dimensões (ex.: 8192x8192) checados no cabeçalho antes de decodificar, e reencodados no núcleo em tamanho fixo numa pasta de cache dentro da pasta de dados. Formato do cache livre entre JPEG, PNG e WebP (rev. 2: a arquitetura usa capas em JPEG q85 até 600x900 e ícones em PNG 256x256, aceito). O que importa: o WebView só recebe bytes produzidos pelo encoder do núcleo, nunca o arquivo original de terceiro. Falha de decodificação = placeholder (CA-24).
Teste: pedir `cover://../../Windows/win.ini` e `cover://<id inexistente>` e confirmar 404; PNG com cabeçalho declarando 100000x100000 vira placeholder sem pico de memória.

### Entrada de caminhos e lançamento

**SEC-09 [ALTO] Validação do executável manual e do navegador de pastas.** (back)
`game_add_manual(path)` só aceita: caminho absoluto em letra de unidade local ou removível (`GetDriveTypeW` = `DRIVE_FIXED` ou `DRIVE_REMOVABLE`); sem prefixo `\\`, `\\?\`, `\\.\`; sem `:` fora da letra de unidade (bloqueia ADS); sem nome reservado (`CON`, `NUL`, `COM1`...); extensão `.exe` (case-insensitive); arquivo regular existente (não diretório, não reparse point para fora da unidade) com cabeçalho `MZ` + `PE\0\0`. Caminho canonicalizado antes de gravar e antes da checagem de duplicata (RN-03). `fs_list_dir(path)` aplica as mesmas regras de raiz e retorna só subpastas e arquivos `.exe`, sem conteúdo.
Teste: tentar adicionar `.lnk`, `.bat`, `.cmd`, `.ps1`, `.txt` renomeado para `.exe`, `\\127.0.0.1\c$\...\x.exe`, `C:\jogo.exe:ads`, unidade de rede mapeada; todos recusados com mensagem.

**SEC-10 [ALTO] Criação de processo sem shell.** (back)
Jogos manuais e executáveis de `playTasks` são iniciados com `CreateProcessW` (ou `std::process::Command` com caminho absoluto, que resolve para o mesmo), com `lpApplicationName` = caminho absoluto canônico, diretório de trabalho = pasta do executável (ou `workingDir` validado por SEC-12), `bInheritHandles = FALSE`, ambiente do usuário sem variáveis extras do launcher. Proibido `cmd /c`, `start`, `powershell`, `ShellExecute` com caminho de `.exe` vindo de dado (exceção única abaixo), e montagem de linha de comando por concatenação de string. Argumentos (só existem para GOG `playTasks`) passados como lista.
**Exceção AF-01 (rev. 2, aprovada):** `ShellExecuteExW` sobre o `.exe` do jogo só quando: `CreateProcessW` do mesmo jogo retornou `ERROR_ELEVATION_REQUIRED` (740) na sessão atual; o usuário confirmou na UI (`session_launch_with_uac` aceito só no estado `NeedsElevation` do mesmo `gameId`); o caminho foi revalidado (SEC-09 ou SEC-12/SEC-14) imediatamente antes e tem extensão `.exe` (evita abrir por associação); verbo `NULL`/padrão, nunca `runas`; jogo manual sem `lpParameters`; GOG com `lpParameters` montado pela mesma rotina de escape usada para a lista de argumentos, nunca passando por `cmd.exe`.
Teste: jogo manual em pasta com `&`, `"` e espaços no nome (`C:\Jogos\A & B\x.exe`) inicia o executável certo; nenhum processo `cmd.exe`/`powershell.exe` aparece como pai.

**SEC-11 [ALTO] URIs de loja com formato estrito e esquema fixo.** (back)
URIs construídas por template fixo e lançadas com `ShellExecuteExW` (verbo padrão), nunca com string livre:
- Steam: `steam://rungameid/<appid>` com `appid` = inteiro decimal `u32` (`^[0-9]{1,10}$`). Nunca usar `steam://run/<id>//<args>` com argumentos.
- Epic: `com.epicgames.launcher://apps/<ns>%3A<itemId>%3A<appName>?action=launch&silent=true`, com cada componente em `^[A-Za-z0-9_.-]{1,128}$` e percent-encoded.
- GOG: lançar pelo `playTasks` (SEC-10) ou `GalaxyClient.exe /command=runGame /gameId=<id>` com `id` = `^[0-9]{1,20}$` e caminho do Galaxy lido do registro e validado como `.exe` existente.
Esquemas aceitos: só `steam` e `com.epicgames.launcher`. ID fora do formato = jogo ignorado na detecção (log), não "sanitizado".
Teste: manifestos de teste com `appid = "10//-console"`, `AppName = "x?action=foo&y"` e `AppName = "a b"`: jogos não aparecem; log registra rejeição.

**SEC-12 [ALTO] Caminhos de manifesto confinados à pasta de instalação.** (back)
Steam: `installdir` é um único componente (sem `\`, `/`, `..`, `:`); pasta = `<library>\steamapps\common\<installdir>`, onde `<library>` vem de `libraryfolders.vdf` e também passa pela validação de raiz. Epic: `InstallLocation` absoluto local; `LaunchExecutable` relativo; `join` canonicalizado (resolvendo links) deve ficar dentro de `InstallLocation`. GOG: `path`/`workingDir` de `playTasks` resolvidos contra a pasta do jogo e confinados a ela. Executável final sempre `.exe` (SEC-14). Comparação de prefixo feita sobre caminhos canônicos e por componente (não por prefixo de string: `C:\Jogos2` não está dentro de `C:\Jogos`).
Teste: manifestos com `..\..\Windows`, `C:\Windows\System32`, link simbólico apontando para fora: jogos rejeitados.

**SEC-13 [MÉDIO] Parsers de terceiros robustos e leitura só leitura.** (back + bd)
Limite por arquivo de manifesto (ex.: 4 MB VDF/ACF/JSON; registro: tamanho de valor limitado); limite de profundidade de aninhamento do VDF (ex.: 32) com parser iterativo ou recursão limitada; nenhum `unwrap`/`expect`/indexação que possa dar `panic` em dado de terceiro; cada loja lida num bloco isolado (erro ou `panic` capturado não afeta as outras nem a biblioteca manual). Banco do GOG Galaxy, se usado (AF-06): abrir com `SQLITE_OPEN_READONLY` (`mode=ro`), `trusted_schema=OFF`, extensões desabilitadas, sem `ATTACH`, queries fixas com parâmetros vinculados, `busy_timeout` curto; nunca escrever no banco de uma loja. Nunca escrever em nenhum arquivo ou chave de loja.
Teste: fuzz/arquivos de teste com VDF de 10 MB, 1000 níveis, JSON truncado, SQLite corrompido: launcher abre, loja afetada some, outras carregam.

**SEC-14 [ALTO] Executável de loja é `.exe` e não é intérprete de sistema.** (back)
O executável resolvido de um jogo de loja (para lançamento direto ou para RN-10) tem extensão `.exe` e não está em `%SystemRoot%`. Scripts (`.bat`, `.cmd`, `.ps1`, `.vbs`, `.js`, `.lnk`) recusados. Se um jogo GOG legítimo depender de script, ele fica fora da detecção e o usuário cadastra manualmente (só `.exe`, SEC-09).
Teste: `playTasks.path = "start.bat"` e `path = "C:\Windows\System32\cmd.exe"`: jogo não aparece.

### Sessão e encerramento de processos

**SEC-15 [ALTO] Encerrar só pelo estado da sessão, por handle.** (back)
`session_close`/`session_force_close` não recebem PID nem caminho. O núcleo mantém o conjunto de processos da sessão; cada processo entra no conjunto com handle aberto no momento da detecção (`PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE`), guardando o tempo de criação. Antes de `TerminateProcess`, reabrir/duplicar com `PROCESS_TERMINATE` a partir do handle guardado (não do PID) e conferir que o tempo de criação bate. Associação por caminho de imagem completo canônico (`QueryFullProcessImageNameW`) e árvore iniciada pelo launcher, nunca só por nome de processo. Pedido de fechamento normal (`WM_CLOSE` nas janelas de topo da sessão) antes de qualquer `TerminateProcess` (RN-12).
Teste: processo de teste fora da pasta com o mesmo nome do jogo não é encerrado; matar o jogo manualmente, abrir outro processo e acionar "Forçar fechamento": nada fora da sessão morre.

**SEC-16 [ALTO] Pasta de instalação válida para associação de processos.** (back)
A pasta usada em RN-10 precisa ter profundidade de pelo menos 2 abaixo da raiz da unidade (`C:\Jogos\X` ok, `C:\Jogos` não) e não pode ser, nem conter: raiz de unidade, `%SystemRoot%`, `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%ProgramData%`, `%USERPROFILE%` ou `%LOCALAPPDATA%`/`%APPDATA%` (a própria pasta), a pasta de instalação da loja (ex.: pasta da Steam), a pasta de instalação do launcher. Pasta inválida = jogo ignorado na detecção, com log.
Teste: manifestos com `InstallLocation` = `C:\`, `C:\Program Files`, pasta da Steam: jogos rejeitados.

**SEC-17 [ALTO] Lista de exclusão e direitos mínimos de processo.** (back)
Nunca entram no conjunto da sessão nem são encerrados: o próprio launcher e seus filhos (WebView2), processos das lojas (`steam.exe`, `steamwebhelper.exe`, `EpicGamesLauncher.exe`, `EpicWebHelper.exe`, `GalaxyClient*.exe` e qualquer processo sob a pasta da loja), `explorer.exe`, processos de outra sessão do Windows (`ProcessIdToSessionId` diferente), processos de outro usuário e processos protegidos. Sem `SeDebugPrivilege`, sem `PROCESS_ALL_ACCESS`. Falha de acesso ao processo (ex.: jogo elevado, UIPI) = mensagem, não tentativa de elevação.
Teste: jogo Steam: "Forçar fechamento" mantém a Steam aberta; grep por `SeDebugPrivilege`/`PROCESS_ALL_ACCESS` vazio.

### Persistência local

**SEC-18 [MÉDIO] Pasta de dados por usuário, escrita atômica, schema fechado.** (bd + back)
Biblioteca, ocultos, configurações, logs e cache de capas ficam em `%LOCALAPPDATA%\<identificador>` (ou `%APPDATA%`, decisão do arquiteto), nunca em `Program Files`, `ProgramData` ou pasta pública, sem alterar ACL herdada do perfil. Escrita atômica (arquivo temporário na mesma pasta + `ReplaceFileW`/`MoveFileExW` com `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`). Formato versionado; leitura com schema fechado (campos desconhecidos rejeitados, limites de tamanho de arquivo e de strings, número máximo de jogos). Jogos manuais carregados passam de novo por SEC-09 (exceto existência, que vira CA-28); jogos de loja não são lançados a partir de caminho gravado na biblioteca: o caminho vem da detecção atual. Arquivo inválido: renomeado para backup e recriado (seção 11 da spec). Logs com rotação e tamanho máximo (ex.: 5 MB x 3), sem dados além de IDs, nomes e caminhos de jogo. **rev. 4:** (a) seção `advanced` de `settings.json` com schema fechado e faixas validadas no núcleo; valor fora da faixa ou do tipo = padrão da spec com log, nunca rejeitar o arquivo inteiro por um valor avançado; nenhuma chave avançada pode desligar confirmações (RN-14), travas de SEC-15 a SEC-17 ou o ticket de SEC-27; (b) `diagnostics.level = debug` registra só caminho de imagem dos processos já associados à sessão, nunca linha de comando, ambiente, títulos de janela de outros apps ou conteúdo de arquivos; (c) `logDir` e `autostartMechanism` no `SettingsDto` são só saída (texto e enum), não aceitos por nenhum comando de escrita; a UI exibe `logDir` como texto; um botão "abrir pasta" exigiria comando novo e passa por revisão (SEC-03/SEC-04).
Teste: editar a biblioteca para um jogo manual apontar para `script.bat` ou caminho UNC: o jogo é recusado/ignorado ao carregar; matar o processo durante gravação não corrompe o arquivo anterior.

### Instância única, privilégio e instalação

**SEC-19 [MÉDIO] Segunda instância sem canal de comando.** (back)
O plugin/mecanismo de instância única só traz a janela existente para a frente. `argv` e `cwd` da segunda instância são ignorados (exceto o reconhecimento de `--autostart` sem valor, se o arquiteto precisar). Sem esquema de URI próprio registrado no MVP. Mutex/pipe no namespace `Local\` com nome que inclua o identificador do app e o SID do usuário.
Teste: com o launcher aberto, executar `launcher.exe C:\Windows\System32\calc.exe --launch x`: só foca a janela.

**SEC-20 [ALTO] Launcher nunca roda elevado; instalação não afrouxa ACL.** (back)
Manifesto de aplicação com `requestedExecutionLevel level="asInvoker" uiAccess="false"`. O app não pede elevação em nenhum fluxo (volume, energia, autostart, detecção). Instalado em `%ProgramFiles%\<nome>` com ACL herdada (Users: leitura e execução). Em tempo de execução, nada é gravado na pasta de instalação. Recomendado: se detectar que está rodando elevado (`TokenElevation`), registrar no log e mostrar aviso discreto nas configurações.
Teste: Process Explorer mostra integridade "Medium" e não elevado após login, após "Iniciar com o Windows" e após abrir pelo menu Iniciar; `icacls` na pasta de instalação sem `Users:(M)`/`(F)`/`Everyone`.

**SEC-21 [ALTO] Instalador abre o app sem elevação.** (back)
Se o instalador oferecer "Executar ao terminar", o app é iniciado com o token do usuário interativo não elevado (ex.: no NSIS do Tauri, a rotina de "run as user"; no MSI, via `explorer.exe`/shell do usuário), nunca como filho direto do processo elevado. Se não for possível garantir, remover a opção. O mesmo vale para o registro de autostart (CA-51/E-19): gravado no HKCU do usuário que abriu o instalador ou, como a spec prevê, pelo próprio launcher no primeiro uso, nunca no HKCU da conta administradora do UAC.
Teste: instalar com conta padrão + credencial de outro admin no UAC; o launcher aberto ao final roda como o usuário padrão, integridade "Medium"; o autostart (tarefa ou `Run`, SEC-22) pertence ao usuário padrão, nunca à conta admin do UAC. O instalador elevado não cria autostart em perfil nenhum; só grava o padrão em `HKLM\Software\<identificador>\InstallDefaults`, que o app lê como booleano (DWORD 0/1; qualquer outro valor = padrão desligado).

**SEC-22 [MÉDIO] Autostart só por usuário e sem elevação: tarefa agendada do próprio usuário (`LeastPrivilege`) ou HKCU Run.** (back) *(emendado na rev. 2, P-03)*
São aceitos dois mecanismos, **um ativo por vez** (ligar um remove o outro; desligar remove os dois):
- **Tarefa agendada** criada pelo próprio app, rodando como o usuário, via COM `ITaskService` montando o objeto `ITaskDefinition` campo a campo (proibido `schtasks.exe`, shell e XML montado por concatenação de string). Travas obrigatórias:
  - `Principal.UserId` = SID do usuário atual; `Principal.LogonType = TASK_LOGON_INTERACTIVE_TOKEN`; **`Principal.RunLevel = TASK_RUNLEVEL_LUA` (`LeastPrivilege`)**. Nunca `TASK_RUNLEVEL_HIGHEST`, senha armazenada, `S4U`, `SYSTEM`, `LOCAL SERVICE`, grupo ou outra conta.
  - Gatilho único `LogonTrigger` com `UserId` = o mesmo SID (nunca "qualquer usuário").
  - Ação única `IExecAction`: `Path` = caminho absoluto do `sofa.exe` instalado em `%ProgramFiles%` (ver abaixo), `Arguments` = `--autostart`, caminho entre aspas onde for concatenado em linha de comando (o QA confere no XML exportado). Sem `WorkingDirectory` gravável pelo usuário.
  - Nome fixo com o SID (`Sofa Autostart (<SID>)`). Ao ler, alterar ou remover, o app só considera a tarefa "dele" se `UserId`, `RunLevel`, `LogonType` e `Path` baterem com o esperado. Tarefa com o mesmo nome e outro conteúdo é recriada pelo app (ou relatada como erro), nunca tratada como válida.
  - Configurações: `ExecutionTimeLimit = PT0S`, `MultipleInstancesPolicy = IgnoreNew`; sem condição de rede ou ociosidade.
- **Fallback `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`**: valor com o caminho absoluto do executável instalado **entre aspas** (`"C:\Program Files\<pasta>\sofa.exe" --autostart`), respeitando `StartupApproved`.
- Em ambos: o alvo é o executável instalado, resolvido a partir da pasta de instalação (`InstallLocation` em HKLM gravado pelo instalador, ou `current_exe()` canonicalizado **e** confirmado dentro de `%ProgramFiles%`). Se o app estiver rodando de outro lugar (cópia portátil, pasta de build), ligar o autostart é recusado. Nunca apontar para a pasta de dados, script ou cópia do executável.
- **Proibidos:** tarefa elevada ou em nome de outra conta, tarefa para todos os usuários, serviço Windows, `HKLM\...\Run`, pasta Inicializar de todos os usuários, Active Setup criado pelo app.
- **Fallback automático tarefa → Run (rev. 4, aceito):** o núcleo pode trocar sozinho para o Run quando `RegisterTaskDefinition` falha ou quando `LastTaskResult` da tarefa indica falha. Condições: (a) o único destino do fallback é o HKCU Run deste requisito; em caso de falha **nunca** tentar outra forma de tarefa (`TASK_RUNLEVEL_HIGHEST`, `S4U`, senha, outra conta), `schtasks.exe`, elevação ou outro mecanismo; (b) remover a tarefa antes de gravar o Run (um ativo por vez) e registrar o motivo no log; (c) códigos de "não é falha" do Agendador (`SCHED_S_TASK_RUNNING` 0x41301, `SCHED_S_TASK_HAS_NOT_RUN` 0x41303, `SCHED_S_TASK_QUEUED` 0x41325 e afins) não disparam o fallback; (d) sem volta automática Run → tarefa a cada abertura (evita alternância e escrita repetida no Agendador); voltar à tarefa só quando o usuário religa a opção ou numa atualização do app, uma vez; (e) tarefa "estranha" (conteúdo não bate) é removida e tratada como falha, nunca reaproveitada.
Teste: exportar a tarefa e conferir `RunLevel=LeastPrivilege`, `LogonType=InteractiveToken`, `UserId` = SID do usuário, ação no `Program Files`; com a tarefa ativa, o processo do Sofá após o logon tem integridade "Medium" e não é elevado; ligar Run remove a tarefa e vice-versa; desligar a opção remove ambos (CA-04); criar manualmente uma tarefa `Sofa Autostart (<SID>)` apontando para `calc.exe`: o app não a trata como válida.

**SEC-23 [ALTO] Higiene de DLL e do instalador.** (back)
Executável do launcher chama `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_APPLICATION_DIR)` (ou equivalente via linker) no início, antes de carregar DLLs por demanda; nenhum `LoadLibrary` com nome relativo de DLL fora do System32; não adicionar pastas à `PATH`. Instalador gerado por bundler com carregamento seguro de DLL (NSIS >= 3.x, ou WiX/MSI) e verificado: rodar o instalador a partir de uma pasta com DLLs falsas comuns (`version.dll`, `uxtheme.dll`, `userenv.dll`, `cryptbase.dll`) não as carrega. Binários com ASLR, DEP e CFG (padrão do toolchain MSVC do Rust; não desligar).
Teste: Process Monitor durante a instalação e a primeira execução: nenhuma tentativa de carregar DLL a partir de Downloads ou da pasta de dados.

**SEC-24 [ALTO] Toolchain e dependências.** (back + front)
Rust estável >= 1.77.2 (correção de CVE-2024-24576 no `Command` com `.bat/.cmd`) fixado em `rust-toolchain.toml`. `Cargo.lock` e lockfile do gerenciador JS commitados. `cargo audit` (ou `cargo deny`) e audit nativo do gerenciador JS sem achado crítico/alto alcançável antes de cada release. Scripts de instalação de pacotes JS bloqueados por padrão (`ignore-scripts` ou política equivalente), com exceções explícitas. Nenhuma dependência de cliente HTTP no núcleo (ex.: `reqwest`) nem plugin `updater` (RN-01, sem atualização automática). WebView2 Evergreen (AF-07).
Teste: `cargo tree | grep -i -E "reqwest|hyper|ureq"` vazio; relatório de audit anexado ao PR de release.

**SEC-25 [ALTO] Desinstalador apaga só a pasta de dados exata.** (back)
Com "Manter dados" desmarcado (CA-68), o desinstalador apaga apenas `%LOCALAPPDATA%\<identificador>` (e `%APPDATA%\<identificador>` se usado), com o identificador fixo no build, nunca lido de config ou registro gravável pelo usuário. Antes de apagar: o alvo resolvido está exatamente um nível abaixo de `%LOCALAPPDATA%`/`%APPDATA%`, e contém o arquivo de biblioteca esperado. A remoção não segue junctions/symlinks (apaga o ponto de reanálise, não o destino). Se o desinstalador elevado não conseguir resolver o perfil do usuário interativo (E-19), não apaga nada e informa. Remover também o autostart do usuário, tarefa e/ou `Run` (CA-52, SEC-22), e a pasta de dados do WebView2 dentro da pasta de dados. A remoção em si segue SEC-31 (sem elevação). **Escopo (rev. 2):** CA-68 fala da pasta de dados "do usuário"; apagar dados de **outros** perfis carregados (troca rápida de usuário) é decisão de produto ainda não tomada (DR-03). Até a decisão, apagar só os dados do usuário interativo que rodou o desinstalador.
Teste: criar junction `%LOCALAPPDATA%\<id>\x` → pasta com arquivos-isca; desinstalar sem manter dados; arquivos-isca intactos. **Precisa verificação** no bundler escolhido (comportamento de `RMDir /r` do NSIS com pontos de reanálise).

**SEC-26 [MÉDIO] Instalador verificável sem assinatura.** (back)
Como o instalador não é assinado (D-11), cada build publica o SHA-256 do instalador ao lado do arquivo (ex.: `SHA256SUMS.txt`), e o README explica como conferir com `Get-FileHash`. Build feito a partir de commit/tag identificável. Instalador não baixa nada além do bootstrapper do WebView2 da Microsoft, quando o runtime não estiver presente.
Teste: hash publicado confere com o arquivo gerado.

### Energia, volume, entrada

**SEC-27 [MÉDIO] Ações de energia sem força e por enum.** (back)
`power(action)` aceita só `Shutdown | Restart | Sleep`. Desligar/reiniciar via `ExitWindowsEx(EWX_POWEROFF | EWX_REBOOT, SHTDN_REASON_FLAG_PLANNED ...)` ou `InitiateShutdownW` sem `EWX_FORCE`/`EWX_FORCEIFHUNG`/`SHUTDOWN_FORCE_*`; suspender via `SetSuspendState(FALSE, FALSE, FALSE)`. `SeShutdownPrivilege` habilitado no token do próprio processo só durante a chamada (`AdjustTokenPrivileges`), sem elevação. Nada de `shutdown.exe` via shell. Confirmação (RN-14) obrigatória na UI; recomendado o núcleo exigir que `power` seja chamado só após `power_request` dentro de 30 s (proteção contra bug de UI disparando direto). `volume_step(dir)` aceita só `Up | Down`, com limite 0 a 100 no núcleo.
Teste: grep por `EWX_FORCE`/`shutdown.exe` vazio; com Bloco de Notas com texto não salvo, "Desligar" deixa o Windows mostrar a tela de bloqueio de desligamento (CA-49).

**SEC-28 [MÉDIO] Sem hooks globais, injeção ou leitura de outro processo.** (back) *(detalhado na rev. 2, C-02 / ADR-0003)*
Entrada do controle só por XInput (polling) e eventos da própria janela. Proibido: `SetWindowsHookEx` de qualquer tipo global (incluindo `WH_KEYBOARD_LL`, `WH_MOUSE_LL`, `WH_GETMESSAGE`, `WH_CBT`), `SetWinEventHook` com `WINEVENT_INCONTEXT`, injeção de DLL, `ReadProcessMemory`/`WriteProcessMemory`, `CreateRemoteThread`, `uiAccess=true`, e mudança de configuração global do sistema para ganhar o primeiro plano (`SystemParametersInfo(SPI_SETFOREGROUNDLOCKTIMEOUT)` e afins).
Permitidos, **só dentro do `ForegroundBroker`/`SystemEvents`** (INV-03, guard por grep no CI):
- `SetForegroundWindow`, `AllowSetForegroundWindow`, `BringWindowToTop`, `ShowWindowAsync` (degraus 1 e 4 do ADR-0003).
- `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, NULL, cb, 0, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS)`: **confirmado dentro de SEC-28** (fora de contexto, sem DLL no processo alvo, recebe só o HWND do evento). O callback não lê nada do outro processo além do PID e do caminho de imagem já permitidos em SEC-15.
- **Degrau 2, `SendInput` sintético: aprovado com condições.** Um único par tecla pressionada/solta, sem outra tecla, mouse ou texto; disparado só pelo núcleo em resposta à combinação de retorno (CA-32) ou ao fim de sessão (CA-30), nunca por comando IPC; no máximo uma vez por tentativa de retorno; registrado no log (degrau e resultado). Preferir uma tecla virtual sem função (ex.: `VK_F24` ou código não atribuído) a `VK_MENU`, que abre menu em alguns jogos; usar Alt só se o SP-01 mostrar que a tecla neutra não libera o primeiro plano.
- **Degrau 3, `AttachThreadInput`: aprovado com condições.** Anexar só à thread dona da janela em primeiro plano naquele instante, numa thread de trabalho com tempo limite; desanexar sempre (guarda RAII, inclusive em erro ou timeout); nunca manter anexado além da tentativa (~150 ms); não tentar contornar a falha com processo elevado (UIPI): seguir para o degrau 4.
**Trava dos degraus 2 e 3 em configuração de runtime (rev. 4, aceito):** com D-35, os degraus podem ser implementados e ficar atrás de `settings.json → advanced.foreground.step2/step3`. Condições: padrão `false` no código (arquivo ausente, chave ausente, tipo errado ou valor fora do schema = `false`); `step2Key` é enum fechado `F24 | Alt` (nunca código de tecla livre, sequência ou texto); nenhum comando IPC, tela, argumento de linha de comando ou variável de ambiente liga os degraus; o disparo continua só no núcleo, nas condições acima; o núcleo grava no log (nível INFO, não só debug) na abertura quando algum degrau 2/3 estiver ligado, com a tecla configurada; gravações do app em `settings.json` (ex.: `settings_set_monitor`) preservam a seção `advanced` sem alterá-la. Processo do mesmo usuário que edite o arquivo ganha no máximo um par de tecla neutra ou Alt e um `AttachThreadInput` temporário, algo que ele já faz sozinho com o mesmo privilégio (RR-01, AF-08): sem ganho de privilégio. Ligar os degraus antes da revisão abaixo é risco de anti-cheat assumido pelo usuário, não falha de segurança do app.
**Condição de revisão (obrigatória antes de ligar os degraus 2 e 3; rev. 4: com o log do QA final, D-35, em vez do SP-01):** o arquiteto traz ao agente `seguranca` a tabela jogo x modo x degrau x ms, o efeito da tecla sintética em cada jogo e o comportamento do jogo com anti-cheat. O degrau correspondente fica vetado se: o anti-cheat sinalizar, bloquear ou fechar o jogo; a tecla sintética causar ação visível no jogo que não se resolva trocando a tecla; o degrau exigir qualquer item da lista de proibidos. Sem os degraus 2 e 3, a escada fica com os degraus 1 e 4, que a spec aceita (CA-63).
Teste: grep pelas APIs proibidas vazio; `SendInput`, `AttachThreadInput` e `SetWinEventHook` aparecem só nos arquivos donos; jogo com anti-cheat no SP-01 não acusa o launcher.

**SEC-29 [MÉDIO] Nenhuma operação destrutiva em arquivos de jogo.** (back)
Nenhum código do launcher apaga, move ou sobrescreve arquivos fora da própria pasta de dados (RN-04). "Remover da biblioteca" só altera a biblioteca. As únicas escritas permitidas em tempo de execução (rev. 2): pasta de dados do app (`%LOCALAPPDATA%\app.sofa.launcher`), e o valor `Run` do HKCU do próprio usuário **ou** a tarefa agendada do próprio usuário, nos termos de SEC-22. Nada em `Program Files`, HKLM, `ProgramData`, arquivos ou chaves de loja, nem tarefas de outros usuários. O instalador e o desinstalador elevados escrevem só a pasta de instalação, `HKLM\Software\<identificador>`, atalhos e a entrada de "Aplicativos instalados" (dados do usuário: SEC-25/SEC-31).
Teste: grep por `remove_file`/`remove_dir_all`/`DeleteFileW`/`rename` e confirmar que os alvos são derivados só da pasta de dados.

**SEC-30 [BAIXO] Sem rede e sem telemetria.** (back + front)
Nenhuma chamada de rede em tempo de execução: CSP sem origens externas (SEC-01), núcleo sem cliente HTTP (SEC-24), sem crash reporter remoto. Únicas saídas de rede aceitas: lojas iniciadas pelo usuário (fora do processo do launcher) e o bootstrapper do WebView2 no instalador.
Teste: Resource Monitor/firewall de saída com o launcher aberto por 10 minutos: nenhuma conexão do processo do launcher nem da WebView2 do app.

### Desinstalação (rev. 2)

**SEC-31 [ALTO] Remoção de dados do usuário sem elevação.** (back)
A remoção da pasta de dados (CA-68, SEC-25) não pode ser feita pelo processo elevado percorrendo uma árvore que o usuário comum controla. Checar `FILE_ATTRIBUTE_REPARSE_POINT` e depois apagar pelo caminho é uma corrida (TOCTOU): um processo do usuário troca uma subpasta por junction entre a checagem e a remoção e o desinstalador apaga arquivos arbitrários como administrador, cadeia conhecida de elevação para admin/SYSTEM. A rotina recursiva própria em NSIS do ADR-0006 §6 tem essa corrida. Recomendado: o desinstalador elevado só decide e registra a intenção; a remoção roda **como o usuário interativo, sem elevação** (ex.: `RunAsUser` do `nsis_tauri_utils` iniciando `"<pasta de instalação>\sofa.exe" --purge-user-data` antes de apagar os binários; o app só apaga o próprio `app_local_data_dir()`, com as travas de SEC-25, abrindo cada entrada com `FILE_FLAG_OPEN_REPARSE_POINT` e apagando pelo handle). Se não for viável, o mínimo aceitável é apagar como admin **só arquivos de nome conhecido** em subpastas conhecidas (`library.json`, `settings.json`, `*.corrupt-*.json`, `logs\*.log*`, `cache\scan.json`, `cache\covers\*`, `cache\icons\*`), deixando `EBWebView\` para o passo sem elevação ou como resto documentado. Nunca descida recursiva genérica elevada.
Teste: desinstalar sem manter dados enquanto um processo do usuário troca em loop `...\app.sofa.launcher\cache\covers` por junction para uma pasta-isca só de administradores (ex.: `C:\iscaadmin`): arquivos-isca intactos; Process Monitor mostra as exclusões na pasta de dados com o token do usuário (integridade Medium).

**SEC-32 [MÉDIO] Comandos externos do instalador e do desinstalador com caminho absoluto.** (back)
Qualquer executável chamado pelo instalador ou desinstalador elevado (ex.: `schtasks.exe` para remover tarefas, ADR-0006 §6) é chamado por caminho absoluto em `$SYSDIR` (`%SystemRoot%\System32`), nunca por nome relativo (a ordem de busca de `CreateProcess` inclui a pasta atual). Valores interpolados na linha de comando são validados antes: SID em `^S-1-5-21(-[0-9]{1,10}){4}$`; nome da tarefa montado só a partir de constante + SID validado. Preferível remover as tarefas sem processo externo, enumerando a pasta raiz do Agendador por prefixo `Sofa Autostart (` via COM, o que também cobre usuários sem hive carregado.
Teste: revisar o script NSIS (`nsExec` só com `$SYSDIR\...`); Process Monitor na desinstalação mostra `C:\Windows\System32\schtasks.exe`, sem busca na pasta temporária do desinstalador.

---

## 7. Ask first

**Status (rev. 2, 2026-11-13): AF-01 a AF-10 aprovados pelo usuário como recomendado.** AF-05 foi aprovado junto com a decisão P-03: tarefa agendada por usuário não elevada (preferida), com fallback HKCU Run, nos termos do SEC-22 emendado. AF-10 passa a valer junto com SEC-31.

| ID | Item | Por que exige aprovação | Recomendação |
|---|---|---|---|
| AF-01 | **Jogos que exigem administrador** (manifesto `requireAdministrator`; `CreateProcessW` retorna erro 740) | Elevação de privilégio; UAC exige teclado/mouse (R-04) | Não elevar o launcher nunca. Ao receber 740, mostrar "Este jogo pede permissão de administrador. O Windows vai pedir confirmação (exige teclado ou mouse)" e, se o usuário confirmar, chamar `ShellExecuteExW` com verbo padrão no `.exe` (o UAC do Windows é o controle). Proibido verbo `runas` proativo. O launcher não conseguirá trazer à frente nem encerrar processo elevado (UIPI): tratar como limitação |
| AF-02 | **Caminhos de rede para jogos manuais** (UNC `\\host\share`, unidades mapeadas) | Nova origem externa de executáveis; vazamento de hash NTLM ao ler ícone/iniciar | Bloquear no MVP (SEC-09). Aceitar só unidades locais fixas e removíveis |
| AF-03 | **Tipos além de `.exe` e argumentos de linha de comando** em jogo manual (emuladores, `.bat`, `.lnk`, parâmetros) | Aumenta muito a superfície de injeção; fora da spec | Manter só `.exe` sem argumentos no MVP. Se pedido depois, nova revisão: argumentos como lista no núcleo, nunca via `cmd.exe` |
| AF-04 | **Uso de `SeShutdownPrivilege` e `TerminateProcess`** (novas capacidades privilegiadas/destrutivas do processo) | Permissão nova no token e encerramento de processos | Aprovar, com as travas de SEC-15 a SEC-17 e SEC-27: privilégio habilitado só durante a chamada, sem força; encerramento só do conjunto rastreado, por handle, após pedido normal e nova confirmação |
| AF-05 | **Mecanismo de autostart** | Persistência no login | **Aprovado (rev. 2):** tarefa agendada do próprio usuário, `LeastPrivilege`, `InteractiveToken`, criada pelo app via `ITaskService`, com fallback HKCU `Run`; um ativo por vez; caminho entre aspas (SEC-22). Rejeitados: tarefa elevada ou de outra conta, serviço, HKLM |
| AF-06 | **Leitura do banco SQLite do GOG Galaxy** em `ProgramData` | Parsing de banco de terceiro, possivelmente gravável por outros usuários | Preferir registro `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games` + `goggame-*.info` na pasta do jogo (também cobre E-17). Usar o banco só se o spike mostrar que é necessário, com SEC-13 |
| AF-07 | **Runtime WebView2: Evergreen x Fixed Version** | Evergreen baixa o runtime da Microsoft na instalação (rede) e se atualiza sozinho; Fixed não recebe correções de segurança | Evergreen (bootstrapper ou instalador offline embutido). Fixed Version acumularia vulnerabilidades do Chromium sem atualização automática do launcher |
| AF-08 | **Risco aceito: adulteração da biblioteca por processo do mesmo usuário** (sem HMAC/assinatura do arquivo) | Decisão de aceitar risco | Aceitar (RR-01). Chave de HMAC ficaria no mesmo nível de privilégio do atacante; ganho nulo. SEC-18 reaplica validações na carga |
| AF-09 | **Risco aceito: instalador sem assinatura** (D-11) | Já decidido; confirmar mitigação compensatória | Manter D-11 e adotar SEC-26 (SHA-256 publicado) |
| AF-10 | **Desinstalador apagando dados do usuário elevado** | Operação destrutiva com privilégio de administrador | **Aprovado (rev. 2)** com SEC-25 e SEC-31: remoção feita sem elevação, como o usuário; se inviável, apagar como admin só arquivos de nome conhecido, sem descida recursiva genérica |

---

## 8. Riscos residuais aceitos

| ID | Risco | Motivo para aceitar |
|---|---|---|
| RR-01 | Malware rodando como o usuário edita a biblioteca, o HKCU `Run`, o handler `steam://` ou variáveis de ambiente do WebView2 | Já tem os mesmos poderes sem o launcher. O launcher não amplia privilégio (SEC-20) |
| RR-02 | Instalador sem assinatura: SmartScreen e risco de executar cópia adulterada | D-11; uso pessoal; SEC-26 |
| RR-03 | Outro usuário local planta manifesto em `ProgramData` (Epic/GOG) | PC pessoal; SEC-12/14/16 limitam o alvo; o dono ainda precisa apertar A no item |
| RR-04 | Processo do mesmo usuário injeta teclas (`SendInput`) e dispara desligamento | Mesmo nível de privilégio; desligamento normal não forçado (SEC-27) |
| RR-05 | Jogos com launcher de publicadora/anti-cheat fora do controle do launcher | R-03 da spec |

---

## 9. Pontos para o arquiteto confirmar

Situação na rev. 2:
- Lista de comandos IPC e capability: entregue em `docs/arch/launcher-tv.md` §4.1 e §4.3 e conferida na seção 10. É a baseline da verificação.
- Bundler: NSIS do Tauri (ADR-0006). SEC-21 e SEC-23 endereçados, pendentes do SP-07. SEC-25: a rotina recursiva própria precisa dar lugar à remoção sem elevação de SEC-31.
- Steam: `steam://rungameid/<appid>` por template fixo (aceito).
- Primeiro plano (R-01): degraus 2 e 3 aprovados com condições em SEC-28; revisão obrigatória com o resultado do SP-01 antes de implementá-los.

---

## 10. Revisão da arquitetura rev. 2 (baseline IPC, capability, CSP)

Fontes: `docs/arch/launcher-tv.md` rev. 2 (§2, §4.1 a §4.5, §5, §7, §10), ADR-0003, ADR-0004, ADR-0005 e ADR-0006.

### Respostas aos pontos C-01 a C-03

| Ponto | Decisão |
|---|---|
| C-01 (SEC-22, SEC-29 x tarefa agendada) | **Aceito.** SEC-22 e SEC-29 emendados para incluir a tarefa do próprio usuário com `LeastPrivilege`, `InteractiveToken`, gatilho de logon só do usuário, ação no `Program Files` entre aspas, criada via `ITaskService` sem shell, um mecanismo por vez. Duas travas novas: o app só reconhece a tarefa se o conteúdo bater (o nome não basta) e recusa ligar o autostart quando roda fora do `Program Files` |
| C-02 (SEC-28: `SendInput`, `AttachThreadInput`, `SetWinEventHook`) | **`SetWinEventHook` fora de contexto: confirmado** dentro de SEC-28 (com `WINEVENT_SKIPOWNPROCESS`; `WINEVENT_INCONTEXT` proibido). **`SendInput` e `AttachThreadInput`: aprovados com condições**, escritas em SEC-28 (uma tecla, de preferência neutra; só no núcleo e só no retorno ou fim de sessão; anexar só durante a tentativa, com timeout, e desanexar sempre). **Revisão obrigatória após o SP-01**, com veto se o anti-cheat reagir ou a tecla causar efeito no jogo |
| C-03 (SEC-08, cache em JPEG/PNG) | **Aceito.** SEC-08 ajustado: formato livre entre JPEG, PNG e WebP, desde que o WebView só receba bytes reencodados pelo núcleo |

### Baseline conferida (sem divergência crítica ou alta)

- **IPC (§4.1):** 30 comandos, todos por ID ou enum; nenhum recebe caminho de executável, argumentos, URI, PID, chave de registro ou shell. Exceções com caminho (`fs_list_dir`, `library_add_manual`) sob SEC-09. `session_launch_with_uac` restrito ao estado `NeedsElevation` do mesmo jogo. `power_prepare`/`power_execute` com ticket de 30 s e uso único (atende à recomendação de SEC-27). `settings_set_monitor` restrito à lista enumerada. Conforme SEC-03 e SEC-06.
- **Capability (§4.3):** uma capability, só a janela `main`, sem `remote`, `withGlobalTauri=false`, só `allow-<comando>` + `core:event:allow-listen/unlisten`, sem `core:default`/`core:window:*` nem plugins perigosos, com teste de CI comparando com §4.1. Conforme SEC-04 e SEC-05.
- **CSP (§4.4):** conforme SEC-01. `data:` em `img-src` aceito (imagem `data:` não executa script; uso restrito a SVG gerado no build). Endurecimento do WebView2 e bloqueio de navegação e de novas janelas conformes SEC-02.
- **`cover://` (§4.5):** por `GameId` existente, limites antes de decodificar, reencode, sem `assetProtocol`. Conforme SEC-08.

### Divergências e achados da revisão de design

| ID | Sev. | Onde | Achado | Ação |
|---|---|---|---|---|
| DR-01 | **ALTO** | ADR-0006 §6 (remoção de dados) | Rotina NSIS recursiva elevada com checagem de reparse point por caminho: corrida TOCTOU que permite apagar arquivos arbitrários como admin (elevação usuário → admin). O teste com junction estática do SP-07 não pega a corrida | SEC-31: remover como o usuário, sem elevação (ou só arquivos de nome conhecido). Responsável: back (instalador) |
| DR-02 | Médio | ADR-0006 §6 (remoção de tarefas) | `nsExec` chamando `schtasks.exe` por nome, a partir do desinstalador elevado, com SID interpolado | SEC-32: `$SYSDIR\schtasks.exe` e SID validado, ou remoção via COM por prefixo |
| DR-03 | Médio (decisão de produto) | ADR-0006 §6 (perfis-alvo) | O desinstalador apaga dados de **todos** os perfis com hive carregado; CA-68 fala dos dados "do usuário". Apagar dados de outra conta logada sem ela pedir não está na spec | Levar ao orquestrador/PO. Até decidir, apagar só os dados do usuário interativo (SEC-25 emendado). Remover o autostart (tarefa/Run) de todos os perfis é aceitável, porque o binário deixa de existir |
| DR-04 | Baixo | §2.1 Autostart / ADR-0006 §2 | Alvo do autostart derivado do executável em execução: uma cópia portátil registraria caminho gravável pelo usuário | Coberto pela emenda de SEC-22 (recusar fora do `Program Files`) |
| DR-05 | Baixo | §2.4 / AF-01 | `ShellExecuteExW` sobre `.exe` de jogo conflitava com o texto de SEC-10 | SEC-10 emendado com a exceção AF-01 e suas condições |

Nenhum risco crítico novo. Um alto novo (DR-01 / SEC-31).

---

## 11. Revisão da arquitetura rev. 4 (C-04)

Fontes: `docs/arch/launcher-tv.md` rev. 4 (§2.1, §4.1, §9, §10 C-04), ADR-0003, ADR-0005 e ADR-0006 rev. 4. Contexto: D-32 (Win10 22H2 é o alvo principal) e D-35 (sem spike com o usuário; teste só no QA final).

| Item | Decisão | Condições |
|---|---|---|
| C-04.1 Trava dos degraus 2/3 em `settings.json → advanced.foreground` | **Aceito** | Escritas em SEC-28: padrão `false` no código, `step2Key` enum fechado, sem IPC/tela/argv/env, disparo só no núcleo, log INFO quando ligado, app preserva a seção ao gravar. A revisão obrigatória continua, agora com o log do QA final |
| C-04.2 Fallback automático tarefa → HKCU Run | **Aceito** | Escritas em SEC-22: destino único é o Run; nunca tentar tarefa elevada, outra conta, `schtasks` ou elevação; remover a tarefa antes; códigos de "não é falha" do Agendador não disparam; sem alternância automática |
| `advanced.session.*`, `input.deadzonePercent`, `diagnostics.level` | **Aceito** | Faixas da §9.3 são adequadas; o mínimo de `endGraceSeconds` (2 s) e `closeTimeoutSeconds` (5 s) não remove a confirmação nem as travas de SEC-15 a SEC-17 do "Forçar fechamento". Condições em SEC-18 (a) e (b) |
| `logDir` e `autostartMechanism` no `SettingsDto` | **Aceito** | Só saída, renderizados como texto; sem comando para abrir a pasta (SEC-18 (c)) |
| D-32 Win10 22H2 | Sem impacto de segurança | `FILE_DISPOSITION_FLAG_POSIX_SEMANTICS` (1709+) no purge com fallback para exclusão comum mantém SEC-31 (o purge roda sem elevação) |

Nenhum risco crítico ou alto novo. Pendências: revisão dos degraus 2/3 com o log do QA final (SEC-28) e as verificações de instalador do QF-05 (SEC-21, SEC-23, SEC-25, SEC-31, SEC-32).
