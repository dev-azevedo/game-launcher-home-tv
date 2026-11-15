# Threat model: launcher-tv

| Campo | Valor |
|---|---|
| Feature | launcher-tv (MVP do Game Launcher) |
| Branch | `feature/launcher-tv-20261113` (base `main`) |
| Modo | Design (threat model); rev. 7: verificação, rodada 1 (seção 14) |
| Entradas | `docs/specs/launcher-tv.md` (seção 14 = decisões); rev. 2: `docs/arch/launcher-tv.md` (rev. 2) e `docs/adr/0003` a `0006` |
| Revisões | rev. 1: threat model inicial. rev. 2 (2026-11-13): AF-01 a AF-10 aprovados pelo usuário; P-03 (tarefa agendada por usuário); emendas em SEC-08, SEC-10, SEC-21, SEC-22, SEC-25, SEC-28, SEC-29; novos SEC-31 e SEC-32; revisão da baseline da arquitetura (seção 10). rev. 4 (2026-11-13): arquitetura rev. 4 (D-32 Win10, D-35 teste só no fim); C-04 avaliado; emendas em SEC-18, SEC-22 e SEC-28 (seção 11). rev. 5 (2026-11-13): SG-02 (SP-01: degraus 2 e 3 ligados por padrão, ordem 1 → 3 → 2 → 4) e SG-03 parcial (SP-07); emendas em SEC-19, SEC-22 e SEC-28 (seção 12). rev. 6 (2026-11-13): SG-01, revisão de contrato e de `tauri.conf.json` (seção 13). rev. 7 (2026-11-14): verificação rodada 1 (SG-04, SG-05, SG-06, SG-09; SG-07 e SG-08 parciais); SEC-BUG-01 a SEC-BUG-03; DR-06; emendas em SEC-19, SEC-22, SEC-28 e SEC-31 (seção 14) |
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
Desvio aceito (SG-03, SP-07): o `tauri-plugin-single-instance` 2.5.5 usa mutex `{identifier}-sim` sem SID e sem prefixo. Nome sem prefixo criado por processo interativo fica no namespace da sessão, o que equivale a `Local\`; numa sessão só há um usuário, então o SID não é necessário. Um processo do mesmo usuário pode ocupar o nome (o Sofá não abre) ou receber o `WM_COPYDATA` com `argv`: coberto por RR-01, porque o `argv` não carrega dado sensível. Condição: o callback ignora `argv` e `cwd`, salvo `--autostart`, como já exigido.
Emenda (rev. 7, SG-07): o `--purge-user-data` é lido no `main` antes da instância única e nunca passa pelo canal dela (correto). Com o Sofá aberto, o modo de remoção não deve apagar nada: recusa sem apagar (ver SEC-BUG-03). O critério do BE-35 ("`sofa.exe --purge-user-data` só traz a janela, sem apagar nada") passa a ser lido como "não apaga nada com o Sofá aberto".

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
Ajuste de teste (SG-03, SP-07): o XML exportado omite `<RunLevel>` quando o valor é `LeastPrivilege` (padrão). Verificar **ausência** de `<RunLevel>HighestAvailable</RunLevel>` e `IPrincipal.RunLevel == TASK_RUNLEVEL_LUA`; comparar `UserId` por SID (`LookupAccountNameW` sobre o nome devolvido pela API, ou o SID do XML), e `Path` com aspas normalizadas.
Emenda (rev. 7, SG-07): a volta automática Run → tarefa "numa atualização do app, uma vez" do item (d) é **opcional**, não obrigatória. Ficar no Run depois de um fallback é conforme: o Run é mecanismo aceito por este requisito. Se for implementada depois, vale o limite de uma vez por versão.

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
**Padrão no MVP (SG-02, rev. 5):** o SP-01 mostrou que, sem direito de primeiro plano, os degraus 1 e 4 não trazem o Sofá (degrau 1: 1/24; degrau 4: 1/26 em janela, 0/8 sem borda e 0/8 em tela cheia exclusiva), então o padrão 1 + 4 não cumpre CA-32, CA-63 nem CA-30. Nenhum critério de veto foi atingido nos apps sintéticos. Decisão: **degraus 2 e 3 ligados por padrão** (`step2 = true`, `step3 = true`, `step2Key = F24`), na ordem **1 → 3 → 2 → 4**: o degrau 3 não entrega entrada ao jogo e funcionou 8/8; a tecla sintética só é enviada se o degrau 3 falhar na mesma tentativa. Condições extras para o padrão ligado: (a) no máximo um par de F24 por tentativa de retorno, e só depois de o degrau 3 falhar; (b) não rodar a escada com a estação bloqueada ou em desktop seguro (UAC, Ctrl+Alt+Del); (c) degrau 3 com no máximo uma thread de trabalho pendente: se a anterior não voltou do timeout, pular o degrau 3 (sem acumular threads presas num jogo travado); a thread da janela do Sofá nunca bloqueia esperando a escada; (d) processo em primeiro plano elevado (UIPI): não tentar contornar, seguir para o degrau 4 e registrar; (e) log INFO de cada tentativa (degraus, ms, resultado), já exigido; (f) a configuração continua desligável em `advanced.foreground`. A validação com jogos reais e anti-cheat offline fica para QA-17, e o SG-10 pode desligar um degrau ou os dois aplicando os critérios de veto acima.
**Gatilho por segunda instância (rev. 7, SG-06):** abrir o `sofa.exe` de novo com um jogo em tela (janela oculta, sessão em `Running`, `Ending`, `Closing` ou `CloseOverdue`) executa a mesma volta da combinação de retorno, com a mesma escada. Aceito como gatilho equivalente à combinação (CA-32), nas condições: o callback continua sem ler `argv` nem `cwd`; vale o limite de uma escada por vez (`return_in_flight`) e de um par de F24 por tentativa, só depois de o degrau 3 falhar; log INFO com a origem "segunda instância". Quem consegue abrir o `sofa.exe` como o usuário já consegue chamar `SendInput` sozinho (RR-01): sem ganho de privilégio.
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
**Emenda (rev. 7, SG-09, DR-06):** o "mínimo aceitável" acima fica restrito. Curinga em subpasta (`cache\covers\*`, `cache\icons\*`, `logs\*.log*`) **não** é "nome conhecido": o processo elevado resolve a subpasta pelo caminho, e uma junction trocada depois da checagem faz o curinga apagar, como administrador, todos os arquivos de uma pasta qualquer (a mesma corrida de DR-01, só que restrita a um nível). O processo elevado não apaga nada por curinga. Recomendado: sem remoção elevada nenhuma; se o purge sem elevação não rodar ou não devolver código 0 confirmado, o desinstalador não apaga dados e informa a pasta ao usuário (SEC-25 já permite "não apaga nada e informa"). Se o usuário quiser manter um fallback elevado (Ask first AF-11), só os dois arquivos `library.json` e `settings.json` na raiz do alvo, sem curinga e sem subpasta.
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
| AF-11 | **(rev. 7, pendente) Fallback elevado do desinstalador** quando o purge sem elevação falha | Operação destrutiva como administrador; reduz o que o AF-10 aprovou | **Não ter fallback elevado**: sem código 0 confirmado do purge, não apagar nada e mostrar a pasta ao usuário. Se o usuário preferir manter um fallback, só `library.json` e `settings.json` na raiz do alvo, sem curinga nem subpasta (DR-06, SEC-31 emendado) |

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

---

## 12. Pareceres SG-02 (SP-01) e SG-03 (SP-07)

### SG-02: SP-01 técnico, degraus 2 e 3 da escada (SEC-28, ADR-0003)

Evidência: `docs/arch/spikes/SP-01.md` (Win11 22621, usuário comum, apps sintéticos em janela, sem borda e D3D11 em tela cheia exclusiva; o launcher sem direito de primeiro plano).

| Degrau | Resultado | Leitura de segurança |
|---|---|---|
| 1 `SetForegroundWindow` | ~1/24, sucessos atribuídos a entrada real | Sem risco; não confiável |
| 4 minimizar | 1/26 em janela, 0/8 sem borda, 0/8 exclusiva | Sem risco; **não cumpre CA-63**: a ativação vai para outra janela, não para o Sofá |
| 3 `AttachThreadInput` | 8/8 nos três modos, 5 a 30 ms; falha se a thread da janela do Sofá não bombeia mensagens | Nenhuma entrada chega ao jogo; não abre handle para o processo do jogo nem toca a memória dele. Menor perfil para anti-cheat |
| 2 `SendInput` de um par F24 | 8/8 nos três modos, 1 a 5 ms; o jogo recebe `vk=0x87` uma vez por tentativa | Entrada sintética (marcada como injetada) chega ao jogo. F24 sem função padrão; Alt abre menus |

**Parecer: liberado, e com padrão LIGADO no MVP.** Degraus 2 e 3 ligados, `step2Key = F24`, ordem **1 → 3 → 2 → 4**, com as condições de SEC-28 e as condições extras (a) a (f) do "Padrão no MVP". Motivo: com o padrão anterior (1 + 4), o retorno por controle (CA-32, CA-63) e a volta no fim da sessão (CA-30) não funcionam, e nenhum critério de veto foi atingido. A ordem com o degrau 3 antes do 2 evita entregar tecla sintética ao jogo quando o 3 basta, que foi o caso em 8/8.

**Anti-cheat (R-07):** os degraus não abrem handle para o processo do jogo, não injetam código nem leem memória, que é o que anti-cheats em geral bloqueiam. O ponto residual é o degrau 2: um anti-cheat que trate entrada injetada como macro pode registrar o F24. Com a ordem 1 → 3 → 2 → 4 ele só é enviado quando o degrau 3 falha, uma vez, no momento em que o usuário pediu para sair do jogo. Risco baixo, não zero, e o ativo afetado é a conta de jogo do próprio usuário. Não bloqueia o MVP. Fica para QA-17 (QF-02, anti-cheat em modo offline, D-30) e SG-10, que pode desligar o degrau 2 (ou os dois) se um critério de veto aparecer. Recomendado avisar o usuário no roteiro do QA-17 de que existe a chave `advanced.foreground.step2` para desligar a tecla sintética.

**`XInputGetStateEx` (ordinal 100):** não é necessário (a combinação View + Menu está na máscara pública do `XInputGetState`) e **não está aprovado** no MVP. Se for proposto depois para ler o Guide, é aceitável como leitura local com fallback para `XInputGetState` quando a ordinal não existir; precisa de nova revisão curta.

**Itens que exigem jogo real e anti-cheat (QA-17 e SG-10):** efeito do F24 em cada jogo da lista D-29; anti-cheat offline; jogo elevado (UIPI, RA-02); Sofá real com WebView2 (atraso de `WM_ACTIVATE`); Windows 10 22H2.

**Para o PO/arquiteto:** o texto de CA-63 e RN-24 assume que minimizar o jogo traz o Sofá, o que o SP-01 desmentiu. Se o SG-10 vetar os degraus 2 e 3, a spec precisa ser revista (fora do escopo da segurança).

### SG-03: SP-07 técnico, instalador e autostart (parcial)

Evidência: `docs/arch/spikes/SP-07.md` (host Win11, conta comum; sem VM nem UAC).

| Item | Resultado | Parecer |
|---|---|---|
| Tarefa de logon via `ITaskService` como conta comum (SEC-22) | Criada em 19 ms sem UAC; `RunLevel = LUA`, `InteractiveToken`, `UserId` = SID no XML, `Command` entre aspas, `--autostart`, `PT0S`, `IgnoreNew`; processo com integridade Medium, não elevado | **Conforme SEC-22 e SEC-29.** Ajustes de teste incorporados em SEC-22 (ausência de `HighestAvailable`, `UserId` por SID, aspas normalizadas) |
| Tarefa `HIGHEST` ou em nome de outro SID | `E_ACCESSDENIED` | **Conforme AF-05**, reforçado pelo próprio sistema; é o gatilho do fallback. Na falha, sem nova tentativa com outra configuração (SEC-22 (a)) |
| `LastTaskResult = 0x41303` antes de rodar | Confirmado | Já coberto por SEC-22 (c): não é falha |
| Purge por handle (`OPEN_REPARSE_POINT`, atributos pelo handle, caminho final do filho contido no do pai, exclusão por handle) | Junction estática removida sem descer; corrida com 57 trocas e 0 arquivos-isca perdidos | **Conforme SEC-31** (o invariante vem do handle, não do caminho). BE-41 repete com mais volume e aborta se a checagem de contenção falhar. O fallback elevado por nomes conhecidos continua só como fallback |
| Remoção de tarefas por COM por prefixo | Viável no Windows | Preferida para SEC-32; prova no NSIS pendente (BE-42); senão `"$SYSDIR\schtasks.exe"` com SID validado |
| Mutex da instância única sem SID e sem prefixo | Namespace da sessão; `argv` enviado por `WM_COPYDATA` | **Desvio aceito** em SEC-19 (condição: ignorar `argv`/`cwd` salvo `--autostart`); adapter próprio não é necessário |
| RunAsUser com UAC de outra conta (SEC-21), DLL plantada (SEC-23), desinstalador com perfis (SEC-25, SEC-32), Win10 sem WebView2 | **Não testados** (exigem VM/UAC) | **Pendentes para QA-17 (QF-05)** e SG-07. Sem esses itens o SG-03 não fecha |

Nenhum risco crítico ou alto novo nos dois pareceres. A pergunta do SP-07 §6 (criar conta local padrão de teste no host) é do orquestrador com o usuário; se aprovada, adianta os pendentes do SG-03 antes do QA-17.

---

## 13. Revisão de contrato (SG-01)

Modo contrato. Fontes: `docs/api/launcher-tv.md` (BE-07, commit `d982ccc`: 30 comandos, 8 eventos, 26 códigos), `src-tauri/tauri.conf.json` (commit `b54a02e`), `ui/vite.config.ts`, `src-tauri/Cargo.toml`, `src-tauri/build.rs`, contra SEC-01 a SEC-32 (rev. 5) e a baseline da arquitetura §4. Não há banco, schema, SQL nem RLS (persistência JSON, ADR-0004, já revisada; o formato não está no contrato), então `sql-code-review` não se aplica. Decisões consideradas: escada padrão 1 → 3 → 2 → 4 com F24 (cenário A, SG-02), P-08 b (MSVC para release), P-09 (sem conta de teste no host).

**Veredito: aprovado com ressalvas.** Nenhum achado crítico ou alto; nada bloqueia a implementação. Os achados abaixo viram tarefa do PO (ajuste de contrato ou critério de teste).

### O que confere

- **SEC-03 (lista fechada por ID):** os 30 comandos batem com a arquitetura §4.1. Entradas são `GameId`, enums, booleanos, ticket opaco e nome; caminho só em `fs_list_dir` e `library_add_manual`, com as regras de SEC-09 escritas no contrato. `LaunchSpec` montado no núcleo. `--purge-user-data`, escada, `SendInput` e energia sem ticket explicitamente fora da IPC. O teste da §7 compara `generate_handler!` e `main.json` com os 30 nomes.
- **SEC-05 (capability):** `main`, sem `remote`, `allow-<comando>` + `core:event:allow-listen/unlisten`; a UI não emite eventos.
- **SEC-06 (erros e serialização):** `ErrorDto` com `code` fechado e `params` só com `gameId`/`origin`; detalhe técnico só no log; `catch_unwind` por comando; `Busy` nas ações serializadas. `GameFileMissing` sem caminho (D3) é melhor que o texto anterior da arquitetura.
- **SEC-07:** `name` normalizado (C0/C1, bidi, 256) e renderização só como texto.
- **SEC-08:** `cover = "cover://<gameId>"`, sem caminho no `GameDto`.
- **SEC-09/SEC-10/AF-01/AF-02/AF-03:** cadastro sem argumentos; `NetworkPathBlocked` ao iniciar jogo de rede revalidado; `session_launch_with_uac` só em `NeedsElevation` do mesmo `gameId`, revalidando o caminho e com verbo padrão (exceção AF-01 de SEC-10).
- **SEC-15/SEC-17/RN-12:** `session_force_close` só em `CloseOverdue`, sem PID; `AccessDenied` para jogo elevado, sem tentar elevar.
- **SEC-27:** `power_prepare`/`power_execute` com ticket de 30 s e uso único; nenhum `EWX_FORCE*`.
- **SEC-18 (c), DR-04:** `logDir` e `autostartMechanism` só como saída; `advanced.*` sem comando; `settings_set_autostart` recusa exe fora de `%ProgramFiles%`.
- **CA-12:** `input://intent` só com o Sofá em primeiro plano.
- **`session_dismiss`:** nunca cancela início ou fechamento em andamento, e o `CloseOverdue → Running` não encerra nada. Sem risco.

### Achados

| ID | Sev. | Onde | Achado | Correção sugerida | Responsável |
|---|---|---|---|---|---|
| CT-01 | Médio | §2.2 #13 `session_replace`, §4 | "Fecha o atual e abre o novo" não diz como fecha. Aceito em `Running` **e** em `CloseOverdue`; se a implementação forçar o fechamento para abrir o novo jogo, pula a segunda confirmação de RN-12 e INV-05 (perda de progresso, SEC-15) | Escrever no contrato: `session_replace` só faz o pedido normal (`WM_CLOSE`, igual a `session_close`) e abre o novo jogo depois que a sessão chega a `Idle`; se passar dos 10 s, vai a `CloseOverdue` e o forçar exige `session_force_close` confirmado pelo usuário. Em `CloseOverdue`, `session_replace` → `InvalidState` (ou só agenda o novo jogo, sem forçar). Teste: jogo que ignora `WM_CLOSE` + `session_replace` não é encerrado sem `session_force_close` | back |
| CT-02 | Baixo | §2.2 #12, #17; §5 | Listas de erro incompletas: `session_retry` e `session_launch_with_uac` revalidam o jogo, mas não listam `GameFileMissing` (só #17), `NetworkPathBlocked`, `StoreNotInstalled`; #12 lista `ElevationDenied` na §5, não na §2.2 | Alinhar as listas da §2.2 com a §5 para os comandos que revalidam (mesmo conjunto de #11, salvo `SessionAlreadyActive`) | back |
| CT-03 | Baixo | §1 DTOs | Só `GameDto.name` está declarado como normalizado. `originalName` (nome da loja), `MonitorDto.name` (vem do driver) e `DirListing.entries[].name` (nome de arquivo) também chegam à UI | Declarar `originalName` e `MonitorDto.name` normalizados como `name` (SEC-07). Em `DirListing`, manter o nome cru para navegação, mas a UI exibe com os caracteres de controle e bidi neutralizados (ex.: substituídos por símbolo visível) | back + front |
| CT-04 | Baixo | §2.1 #9 `fs_list_dir` | Sem limite de entradas: pasta com dezenas de milhares de arquivos gera resposta enorme e trava a tela | Limite (ex.: 2.000 entradas, pastas antes de `.exe`) e campo `truncated: boolean` | back + front |
| CT-05 | Baixo | §1 GameId Manual | "Hash do caminho canônico" de 16 hex sem algoritmo nem normalização de caixa: dois caminhos que diferem só em maiúsculas viram dois jogos (RN-03), e um hash não criptográfico de 64 bits facilita colisão | Especificar: SHA-256 dos bytes UTF-16 do caminho canônico normalizado para caixa (comparação ordinal do Windows), primeiros 8 bytes em hex | back |
| CT-06 | Baixo | §1 GameId Epic, §1 `cover` | A chave Epic aceita `.` e `..` (`epic:..` é bem formado). Sem efeito se o `GameId` nunca compuser caminho de arquivo, e o contrato não diz isso | Declarar: `GameId` nunca é usado para montar caminho em disco (cache de capas por hash da origem, §4.5 da arquitetura); o handler de `cover://` só faz busca na biblioteca | back |
| CT-07 | Baixo | §1 `PowerTicket` | Não diz como o ticket é gerado nem se fica preso à ação | Gerar com CSPRNG do sistema (`getrandom`/`BCryptGenRandom`), preso à `PowerAction` do `power_prepare`; um `power_prepare` novo invalida o anterior | back |
| CT-08 | Info (implementação, SG-04) | `src-tauri/build.rs`, `src-tauri/capabilities/` | No commit `b54a02e` não há `capabilities/main.json` e o `build.rs` chama `tauri_build::build()` sem `AppManifest::commands`. Sem o manifesto, o Tauri 2 libera os comandos da app sem permissão explícita, o que deixa SEC-05 sem efeito quando os comandos chegarem | BE-08: criar `main.json` e o `AppManifest` com os 30 comandos **no mesmo commit do primeiro comando**; o teste da §7 do contrato passa a valer daí em diante | back |

### `src-tauri/tauri.conf.json` (commit `b54a02e`)

| Item | Parecer |
|---|---|
| `security.csp` (release) | **Aceito.** Idêntica à arquitetura §4.4 e conforme SEC-01: sem `unsafe-inline`/`unsafe-eval`, sem origem externa, `data:` só em `img-src` |
| `security.devCsp` | **Aceita.** Só vale no `tauri dev`; acrescenta `'unsafe-inline'` em `style-src` e `ws://127.0.0.1:5173`/`http://127.0.0.1:5173` em `connect-src` para o HMR. O resto é igual à release |
| `'unsafe-inline'` em `script-src` **só** na `devCsp`, se o preâmbulo do React Refresh (`@vitejs/plugin-react`) exigir | **Aceitaria**, nas condições: (1) só na `devCsp`; preferir hash `'sha256-…'` do preâmbulo, se estável, a `'unsafe-inline'`; (2) **nunca `'unsafe-eval'`**, nem em dev, sem nova revisão; (3) teste de CI que falha se `security.csp` contiver `unsafe-inline` ou `unsafe-eval` ou qualquer origem `http(s)://`/`ws://` fora de `ipc.localhost` e `cover.localhost`; (4) build de dev nunca distribuído (o instalador sai só de `tauri build`) |
| `withGlobalTauri: false`, `windows: []` (janela criada no Rust) | Aceito (SEC-05, SEC-02 via `on_navigation`/`NewWindowRequested` no Rust) |
| `tauri` com `default-features = false`, só `wry` e `compression` | Aceito: sem a feature `devtools` no release (SEC-02) |
| `ui/vite.config.ts`: `host: '127.0.0.1'`, `strictPort`, `fs.allow` só com `ui/` e a pasta da marca | Aceito: o servidor de dev não fica exposto na rede e não serve arquivos fora dessas pastas. Vite 8.3.8 entra no audit do SG-07 (o servidor de dev do Vite já teve CVEs de leitura de arquivo) |
| `bundle.active: false` | Sem efeito agora; o bundle NSIS volta no BE-37 e passa pelo SG-07 |

### Para o PO (tarefas)

- **CT-01 (médio):** ajustar o contrato de `session_replace` antes de implementar o comando (BE de sessão), com o teste descrito.
- **CT-02 a CT-07 (baixos):** um ajuste de contrato do dev-backend (listas de erro, normalização de `originalName`/`MonitorDto.name`, limite do `fs_list_dir`, algoritmo do hash do `manual:`, regra "GameId não compõe caminho", geração do ticket) e, no front, a exibição neutralizada dos nomes do navegador de pastas.
- **CT-08:** critério de aceite do BE-08 (capability e `AppManifest` junto com o primeiro comando), verificado no SG-04.
- **CSP de dev:** se o HMR precisar de `'unsafe-inline'` em `script-src`, registrar no commit e criar o teste de CI da condição (3).

---

## 14. Verificação, rodada 1 (SG-04, SG-05, SG-06, SG-09; SG-07 e SG-08 parciais)

Modo verificação. Diff: `git diff main..feature/launcher-tv-20261113` (até o commit `85b3174`), com o repo inteiro pesquisado para confirmar. Contrato: `docs/api/launcher-tv.md` seções 8 a 10. ADRs 0003 e 0006 da árvore de trabalho. Não há SQL, migration nem RLS (persistência JSON, ADR-0004): `sql-code-review` não se aplica. SG-03 continua bloqueado e SG-10 vem depois do QA-17; não foram executados.

**Veredito da rodada: aprovado com ressalvas.** Nenhum crítico ou alto aberto **no código**: um médio (SEC-BUG-01) e dois baixos. No desenho, um alto (DR-06, fallback elevado do desinstalador), que precisa ser resolvido no ADR-0006 **antes** do BE-38; ele ainda não está implementado.

### Evidência executada

| Item | Resultado |
|---|---|
| `scripts/ci.ps1 -SkipAudit -SkipUi` | CI OK: autoteste dos guards (33 violações plantadas detectadas), guards sem violação, `fmt`, `clippy -D warnings`, `cargo test --workspace` (todos verdes), bindings iguais ao versionado, `cargo deny check` |
| `cargo deny check advisories` (base RustSec baixada em 2026-11-14) | 0 erros, 0 avisos. `cargo audit` não está instalado; o `cargo deny` usa a mesma base |
| `cargo tree -i reqwest` / `hyper` | Estão no `Cargo.lock` só como dependência opcional do `tauri` para alvos móveis; **ausentes** em `x86_64-pc-windows-msvc` e `-gnu` (SEC-24, SEC-30 conformes) |
| `npm audit --omit=dev` (ui) | 0 vulnerabilidades |
| `npm audit` completo | 7 moderadas, todas em dev (`minimatch` ReDoS via `typescript-eslint`, só no lint). Não alcançáveis em runtime. Info: atualizar o `typescript-eslint` quando houver versão corrigida sem `--force` |
| `npm audit signatures` | 369 pacotes com assinatura verificada, 97 com attestation |
| `ui/.npmrc` | `ignore-scripts=true`, `save-exact=true` (SEC-24) |
| `cargo test -p launcher-win --features win-integration --test purge_integration` (só pastas temporárias) | 6/6, inclusive junction trocada em loop durante a remoção: isca intacta (SEC-31) |
| Segredos no diff | Nenhum segredo, chave, `.env` ou credencial. Nenhum caminho de perfil pessoal em fixtures ou spikes |
| Não executado (restrição) | Testes `win-integration` de autostart e de áudio (criam tarefa e Run de teste, mexem no volume), energia real, purge na pasta real |

### Status por SEC

| SEC | Status | Evidência |
|---|---|---|
| SEC-01 | Atendido (código); `eval` e `<script>` inline no build de release ficam para o QA | `src-tauri/tauri.conf.json` `security.csp` igual à §4.4; `src-tauri/tests/csp.rs` reprova `unsafe-inline`/`unsafe-eval`/origem externa na release e `unsafe-eval` na dev; sem `dangerousDisableAssetCspModification` |
| SEC-02 | Atendido (código); F12, Ctrl+P e `location.href` externo no build de release ficam para o QA | `src-tauri/src/webview_hardening.rs:44-58` (origem exata, sem `userinfo`), `:77` (`NewWindowResponse::Deny`), `:82-113` (DevTools, menu, status bar e aceleradores desligados no release); `tauri` sem a feature `devtools` (`src-tauri/Cargo.toml`, `tests/hardening.rs`) |
| SEC-03 | Atendido | `src-tauri/command_list.rs` (30 nomes), `src-tauri/src/lib.rs:45-76`, `src-tauri/src/ipc.rs` (corpo desserializado em `*Args` com `deny_unknown_fields`, `src-tauri/src/dto.rs:411-470`); `tests/command_list.rs`, `tests/ipc_args.rs`; `--purge-user-data` fora da IPC (`tests/purge_mode.rs`) |
| SEC-04 | Atendido | Único plugin: `tauri-plugin-single-instance`; `capabilities/main.json` sem permissão de plugin |
| SEC-05 | Atendido (CT-08 resolvido) | `capabilities/main.json` só `main`, sem `remote`, 30 `allow-*` + `core:event:allow-listen/unlisten`; `src-tauri/build.rs` gera o `AppManifest` da mesma lista; `withGlobalTauri: false`; janela criada no Rust |
| SEC-06 | Atendido | `src-tauri/src/errors.rs` (um lugar, `params` só `gameId`/`origin`), `ipc.rs:67-82` (`catch_unwind`), `SerialGate` em `state.rs` e `commands/session.rs` |
| SEC-07 | Atendido | Normalização no núcleo (`display_name.rs`); ESLint com `react/no-danger` e restrição a `innerHTML`/`insertAdjacentHTML` (`ui/src/test/lint.test.ts`); grep dos sinks em `ui/src` vazio; `neutralizeName` no navegador de pastas (CT-03) |
| SEC-08 | Atendido | `src-tauri/src/cover_protocol.rs:49-73` (só GET, só `GameId` da biblioteca em memória, `nosniff`), `crates/launcher-win/src/cover_cache.rs:81-143` (unidade local, 20 MB, 8192², `max_alloc`, reencode); sem `assetProtocol` |
| SEC-09 | Atendido | `crates/launcher-win/src/manual_exe.rs` (unidade local antes e depois de canonicalizar, `MZ` + `PE\0\0`), `launch.rs:81-120` (forma no lançamento), `folder_browser.rs`. Observação: no lançamento o cabeçalho PE não é relido; sem efeito, porque `CreateProcessW` com `lpApplicationName` não executa arquivo que não é PE e não passa por shell |
| SEC-10 | Atendido | `crates/launcher-win/src/game_starter.rs:187-252` (`CreateProcessW` suspenso, `lpApplicationName` absoluto, `bInheritHandles = false`, linha montada por `quote_arg`); AF-01 em `:304-330` (verbo nulo) e `session_service.rs:188-213` (estado, mesmo jogo, revalidação e identidade do exe do 740) |
| SEC-11 | Atendido | `launch.rs:36-62` (template fixo, percent-encoding), `id_formats.rs`, `detect.rs` (ID fora do formato = jogo ignorado) |
| SEC-12 | Atendido | `detect.rs` `confined_exe` (antes e depois de resolver links); Steam com `installdir` de um componente, confinado a `steamapps\common` |
| SEC-13 | Atendido | `stores/mod.rs` `read_limited` (4 MB antes e durante a leitura), pré-checagem de profundidade do VDF, `catch_unwind` por loja (`library_service.rs`), sem SQLite (AF-06) |
| SEC-14 | Atendido | `exe_policy::check_store_exe` (`.exe`, fora de `%SystemRoot%`); GOG só com `FileTask` primária |
| SEC-15 | Atendido no modo varredura (Steam, Epic, jogo elevado); **modo Job: ver SEC-BUG-01** | `process_probe.rs:244-300` (handle `QUERY_LIMITED \| SYNCHRONIZE` + tempo de criação na detecção), `:376-399` `terminate_if_same`. Reabre por PID em vez de duplicar o handle, mas o handle original continua aberto no conjunto, o que impede o reuso do PID, e o tempo de criação é conferido: equivalente ao exigido |
| SEC-16 | Atendido com ressalva baixa (SEC-BUG-02) | `install_path_policy.rs:103-120` (profundidade 2, "nem conter", `%SystemRoot%`) |
| SEC-17 | **Não atendido no modo Job (SEC-BUG-01)**; atendido no modo varredura | `process_policy.rs` `classify`; guards sem `SeDebugPrivilege`/`PROCESS_ALL_ACCESS` |
| SEC-18 | Atendido | `persistence/json.rs` (temporário + `MoveFileExW` com `REPLACE_EXISTING \| WRITE_THROUGH`, 4 MB, backup e recriação); `settings/advanced.rs` (valor ruim = padrão só naquele valor, nenhuma chave desliga confirmação ou trava); `cache\scan.json` com schema fechado e só para exibir; `logDir` só saída |
| SEC-19 | Atendido com ressalva (desvio aceito no SG-03; SEC-BUG-03 e parecer (d)) | `src-tauri/src/single_instance.rs:22-52` (callback descarta `argv`/`cwd`), `main.rs:15-18` (purge antes do Tauri) |
| SEC-20 | Atendido no código; ACL da pasta de instalação pendente (BE-37) | `src-tauri/sofa.exe.manifest` (`asInvoker`, `uiAccess=false`), manifesto embutido conferido em `tests/hardening.rs`, log de `TokenElevation` em `lib.rs:102-110` |
| SEC-21 | Pendente (instalador, BE-37, QF-05) | — |
| SEC-22 | Atendido no runtime; fluxo real em `Program Files` e logon no QA-17 | `autostart/task_scheduler.rs` (campo a campo, conferência por SID, `LUA`, `InteractiveToken`, gatilho do mesmo SID, `Path` entre aspas), `run_key.rs`, `install_env.rs` (`Program Files` por `SHGetKnownFolderPath`, DR-04); fallback só para o Run; item (d): parecer (b) |
| SEC-23 | Parcial | `SetDefaultDllDirectories` é a 1ª instrução do `main` (`tests/hardening.rs`); sem `LoadLibrary` (guard). Release obrigatoriamente MSVC (no alvo `-gnu` o `WebView2Loader.dll` fica ao lado do exe, ADR-0006); instalador e DLL plantada pendentes (BE-37, QF-05) |
| SEC-24 | Atendido no repo; relatório de release pendente (BE-39) | `rust-toolchain.toml` 1.100.0, lockfiles versionados, `ignore-scripts`, `deny.toml` com bans de cliente HTTP, audits acima |
| SEC-25 | Atendido no modo purge; desinstalador pendente (BE-38) | `data_dir_policy.rs` (identificador fixo, conferido contra `tauri.conf.json` em `tests/purge_mode.rs`), `purge.rs` `inspect` (um nível abaixo de `%LOCALAPPDATA%` pelo caminho final do handle, sem reparse point, com marcador) |
| SEC-26 | Pendente (BE-39) | — |
| SEC-27 | Atendido | `power.rs:36-43` (`EWX_POWEROFF`/`EWX_REBOOT` sem bit de forçar, `FLAG_PLANNED`), `:46-118` (`SeShutdownPrivilege` só durante a chamada, RAII), `:169-183` (`BCryptGenRandom`, 256 bits); `domain/power.rs` (uso único, 30 s em relógio monotônico, preso à ação, `prepare` novo invalida o anterior, comparação em tempo constante) |
| SEC-28 | Atendido (cenário A); emenda do gatilho por segunda instância | `crates/launcher-win/src/foreground.rs` e `application/foreground.rs`: desktop `Default` exigido, elevado → só degrau 4, F24 só depois de o degrau 3 falhar e no máximo uma vez, uma thread pendente do degrau 3 com guarda RAII; `system_events.rs:90-123` (`OUTOFCONTEXT \| SKIPOWNPROCESS`, só PID e imagem, nada vai ao log); guards do CI |
| SEC-29 | Atendido no runtime; instalador pendente | Guards INV-09 (`remove_file`/`rename`/`MoveFileExW`/`DeleteFileW` só nos donos); escritas só na pasta de dados e no Run ou tarefa do usuário |
| SEC-30 | Atendido estaticamente; teste de 10 min sem conexão fica para o QA | CSP, sem cliente HTTP nos alvos Windows |
| SEC-31 | Atendido no modo purge; orquestração pendente (BE-38) e **DR-06** no fallback | `purge.rs` (handle com `OPEN_REPARSE_POINT`, atributos do handle, filho dentro do pai pelo caminho final, pastas abertas sem `FILE_SHARE_DELETE`, `FileDispositionInfoEx`); recusa elevado (código 3); `purge_integration` 6/6 |
| SEC-32 | Pendente (BE-42) | Guard NSIS existe em `ci-guards.ps1:109-118`; ainda não há script NSIS |

Ask first: AF-01, AF-02, AF-03, AF-05 (runtime) e AF-06 atendidos; AF-04 atendido, salvo SEC-BUG-01; AF-08 aceito; AF-07 e AF-09 pendentes (instalador); AF-10 atendido no modo purge, com DR-06 no fallback; AF-11 novo, pendente do usuário.

### Achados SEC-BUG

| ID | Sev. | Onde | Cenário | Correção sugerida | Responsável |
|---|---|---|---|---|---|
| SEC-BUG-01 | **Médio** | `crates/launcher-win/src/process_probe.rs:467-476` (`request_close` e `force_close` no modo Job), `crates/launcher-win/src/job.rs:157-159` (`TerminateJobObject`), `crates/launcher-core/src/application/session_service.rs:291` (jogo manual ou GOG com Job não passa por `classify`) | No modo Job (jogo manual e GOG), todo descendente do jogo entra na sessão, sem as exclusões de SEC-17. Casos reais: (1) `.exe` de jogo da Steam cadastrado à mão com a Steam fechada: o jogo chama `SteamAPI_RestartAppIfNecessary` e o `steam.exe` nasce filho do jogo, dentro do Job; (2) jogo que abre uma URL com o navegador fechado: o navegador nasce no Job. "Fechar" manda `WM_CLOSE` às janelas da Steam ou do navegador; "Forçar fechamento" chama `TerminateJobObject` e mata `steam.exe`, `steamwebhelper.exe` ou o navegador do usuário, com o que estiver aberto (SEC-17 proíbe processos de loja e `explorer.exe`; RN-12). Efeito colateral: a sessão não termina enquanto a Steam ou o navegador estiverem abertos (`ACTIVE_PROCESS_ZERO` não chega, CA-30). Exige a confirmação do usuário depois de 10 s, por isso médio | No modo Job, não usar `TerminateJobObject` nem `WM_CLOSE` no Job inteiro. Listar os PIDs do Job, abrir cada um com `QUERY_LIMITED \| SYNCHRONIZE`, aplicar o mesmo `classify` (exclusões de Sofá, loja, `explorer.exe`, outra sessão ou usuário) com a pasta do exe (manual) ou do jogo (GOG) como `install_dir`, e encerrar só os membros, por handle e tempo de criação (`terminate_if_same`). Para o fim de sessão, tratar "no Job só restam processos excluídos" como fim, além do `ACTIVE_PROCESS_ZERO`. Teste: exe de teste que inicia um filho chamado `steam.exe` fora da pasta do jogo; "Forçar fechamento" mantém o filho vivo e a sessão termina quando o jogo sai. Atualizar o ADR-0005 ("forçar o fechamento usa `TerminateJobObject`") | back |
| SEC-BUG-02 | Baixo | `crates/launcher-win/src/stores/mod.rs:42-66` (`StoreEnv::system`); só o scanner da Steam acrescenta a própria pasta (`stores/steam.rs:271`) | SEC-16 proíbe a pasta da loja como pasta de instalação. A pasta da Steam (ex.: `C:\Program Files (x86)\Steam`, profundidade 2) não está entre as proibidas para Epic e GOG. Um `.item` da Epic em `ProgramData` com `InstallLocation` = pasta da Steam e `LaunchExecutable` = `steamapps\common\X\x.exe` passa; a varredura da sessão associaria processos sob a pasta da Steam (outros jogos da Steam, `gameoverlayui.exe`, que não estão nas subpastas excluídas) e o "Forçar fechamento" poderia encerrá-los. Exige gravar manifesto em `ProgramData` (RR-03, RR-01) | Incluir a pasta do cliente da Steam (`SteamPath` do HKCU, já lido em `process_probe::steam_path`) nas pastas proibidas de `StoreEnv::system`, para todas as lojas. Teste com fixture Epic apontando para uma pasta-Steam de teste | back |
| SEC-BUG-03 | Baixo | `src-tauri/src/main.rs:15-16`, `crates/launcher-win/src/purge.rs:516-525` | `sofa.exe --purge-user-data` apaga a pasta de dados com o Sofá aberto. O Sofá em execução segura `EBWebView\` e o log, a remoção fica parcial (código 1) e o Sofá regrava `settings.json` e `library.json` depois. Pelo ADR-0006 §6b, código ≠ 0 dispara o fallback elevado do desinstalador (DR-06), o que leva este caso comum ao caminho de risco. Diverge do critério do BE-35 ("só traz a janela, sem apagar nada"). Sem ganho de privilégio | No modo purge, antes de apagar, tentar abrir o mutex da instância única (`OpenMutexW(SYNCHRONIZE, FALSE, "app.sofa.launcher-sim")`); se existir, sair sem apagar, com código próprio (ex.: 4 "Sofá aberto") ou 2. No BE-38, o desinstalador fecha o Sofá antes do purge (o template NSIS do Tauri já verifica app em execução). Teste: com o Sofá aberto, o modo sai sem apagar nada | back |

### SG-09: ADR-0003 e ADR-0006 contra o threat model

| Item | Parecer |
|---|---|
| DR-01 (remoção elevada recursiva) | **Fechado** no desenho principal: o elevado só decide e o `--purge-user-data` roda como o usuário (ADR-0006 §6b), implementado no BE-41 e conferido acima |
| DR-02 (`schtasks` por nome) | **Fechado** no desenho: COM por prefixo preferido, senão `"$SYSDIR\schtasks.exe"` com SID validado em `^S-1-5-21(-[0-9]{1,10}){4}$` e nome de constante + SID. Verificação no BE-42 |
| DR-03 (perfis-alvo) | **Fechado**: dados só do usuário interativo; autostart de todos os perfis |
| DR-04 (alvo do autostart) | **Fechado** e implementado: `AutostartFailed` fora de `%ProgramFiles%` (pasta por `SHGetKnownFolderPath`) |
| **DR-06 (novo, ALTO, desenho)** | ADR-0006 §6b, fallback: "o desinstalador elevado apaga só arquivos de nome conhecido ... `logs\*.log*`, `cache\covers\*`, `cache\icons\*`. Cada subpasta conhecida é verificada como não-reparse antes". Checar e depois apagar pelo caminho, como admin, é a mesma corrida de DR-01: o usuário troca `cache\covers` por junction entre a checagem e o `Delete`, e o curinga apaga, como administrador, todos os arquivos de uma pasta qualquer (ex.: uma pasta de sistema): elevação usuário → admin por destruição. Agravantes: (1) com `FindProcess` em laço o desinstalador não tem o código de saída do purge, e "código ≠ 0" vira fallback na prática; (2) com o Sofá aberto o purge sai com 1 (SEC-BUG-03). **Correção (arquiteto, antes do BE-38):** tirar a remoção elevada; se o purge sem elevação não rodar ou não devolver 0 **confirmado** (handle do processo ou código lido), não apagar nada e mostrar a pasta ao usuário. Se o usuário quiser manter um fallback (AF-11), só `library.json` e `settings.json` na raiz do alvo, sem curinga e sem subpasta. Emenda escrita em SEC-31 |
| ADR-0006, demais itens | Conformes com SEC-20 a SEC-26, SEC-29 e SEC-32 no desenho; a verificação real fica no SG-07 (BE-37 a BE-39, BE-42, QF-05). Acrescentar ao §6b: fechar o Sofá antes do purge (SEC-BUG-03) |
| ADR-0003 | Conforme SEC-28 rev. 5 e o código. Diferença sem efeito: o ADR cita `WTSRegisterSessionNotification` para a sessão bloqueada; o código usa só `OpenInputDesktop` + nome `Default`, que falha com a estação bloqueada ou em desktop seguro (mesmo resultado). Acrescentar o gatilho por segunda instância (parecer (d)). Com uma ordem configurada que ponha o 2 antes do 3, o código adia o 2 e não o executa naquela tentativa (lado seguro) |

### SG-07 parcial (persistência, instância única, privilégio, autostart, purge)

Feito: BE-11 (SEC-18), BE-12 (guards), BE-35 (SEC-19), BE-36 (SEC-22, DR-04), BE-41 (SEC-25, SEC-31), SEC-20 no código e SEC-24 no repo. **Pendente:** BE-37, BE-38, BE-39, BE-42 e QA-15 (SEC-20 ACL, SEC-21, SEC-23 do instalador e da DLL plantada, SEC-25 e SEC-31 na orquestração, SEC-26, SEC-29 do instalador, SEC-32, AF-07, AF-09, AF-10 completo).

`scripts/ci-guards.ps1` (sem achado explorável; recomendações para o BE-12):
- O laço para no **primeiro** `#[cfg(test)]` do arquivo (`:91`). Hoje só `stores/registry.rs` tem um `#[cfg(test)]` no meio, seguido de módulo de teste; código de produção posto depois de um `#[cfg(test)]` ficaria fora do grep. Sugestão: pular só o item marcado, ou falhar se houver código fora de teste depois do marcador.
- Faltam donos para APIs que este threat model restringe: `TerminateProcess`/`TerminateJobObject` (só `process_probe.rs` e `job.rs`, INV-05), `CreateProcessW`/`ShellExecuteExW` (só `game_starter.rs`, SEC-10), verbo `runas` (proibido, AF-01), `RegSetValueExW`/`RegDeleteValueW` (só `run_key.rs`, SEC-29), `SetFileInformationByHandle` com disposição (só `purge.rs`, INV-09), `std::process::Command` fora de testes.
- A regra NSIS aceita a linha se `$SYSDIR\` aparecer em qualquer lugar dela (ex.: num argumento). Sugestão: exigir `"$SYSDIR\` logo depois de `nsExec::Exec*`/`ExecWait`. Vale para o BE-42.

### SG-08 parcial (energia e rede)

Energia conforme SEC-27, CT-07 e AF-04: ticket de 256 bits por `BCryptGenRandom`, base64url de 43 caracteres, uso único (consumido também quando vence), 30 s em relógio monotônico (`Instant`), preso à `PowerAction`; um `prepare` novo invalida o anterior; chute errado não cancela o vigente; capacidade revalidada no `execute`; `ExitWindowsEx` sem `EWX_FORCE*`; `SetSuspendState(FALSE, FALSE, FALSE)`; privilégio só durante a chamada. O ticket protege contra bug de UI, não contra XSS (quem controla a UI chama `prepare` e `execute`); é o desenho aceito em SEC-27, e o XSS é barrado por SEC-01 e SEC-07. Rede: sem cliente HTTP nos alvos Windows, CSP sem origem externa. Desligar e suspender de verdade (CA-49) e o teste de rede de 10 min ficam para o QA-17. Parecer consolidado depois do SG-10.

### Pareceres sobre os pontos do dev-backend

- **(a) `--purge-user-data` com o Sofá aberto:** não é falha de segurança: roda como o usuário, sem elevação, e só apaga a pasta dele, o que qualquer processo do usuário já pode fazer (RR-01). Manter o modo fora da instância única está certo (SEC-19). Mas ele não deve apagar com o Sofá aberto: a remoção fica parcial, os arquivos são regravados e, pelo ADR-0006, o fallback elevado é disparado. Registrado como SEC-BUG-03 (baixo): recusar com o Sofá aberto, e o desinstalador fecha o Sofá antes.
- **(b) SEC-22 (d) sem nova tentativa da tarefa numa atualização:** aceito. O item permitia, não exigia; ficar no Run depois do fallback é conforme. SEC-22 emendado (rev. 7): a volta numa atualização é opcional.
- **(c) Desligamento completo, sem `EWX_HYBRID_SHUTDOWN`:** aceito. Para a segurança é neutro ou melhor (não grava a sessão do kernel em hibernação; nenhum bit de forçar). O custo é de produto: o boot seguinte pode ser mais lento que o "Desligar" do menu Iniciar com a Inicialização Rápida ligada, o que pesa em M-02. Decisão do PO e do arquiteto, não da segurança.
- **(d) Callback da instância única executa a volta da combinação com jogo em tela:** aceito com condições (SEC-28 emendado, rev. 7). É o mesmo caminho da combinação, com `return_in_flight` (uma escada por vez), F24 no máximo uma vez e só depois de o degrau 3 falhar, e o callback não lê `argv` nem `cwd`. Quem abre o `sofa.exe` como o usuário já pode injetar teclas sozinho (RR-01). Recomendado, sem bloquear: manter o log INFO com a origem "segunda instância" (já existe em `single_instance.rs:41`) e, se o QA-17 mostrar laço, um intervalo mínimo entre voltas disparadas por segunda instância.

### Observação fora de segurança (para QA e dev)

- `application/foreground.rs:131-135`: quando nenhum degrau traz o Sofá (`FrontOutcome::Failed`), o `HWND_TOPMOST` aplicado em `prepare_launcher_topmost` não é desfeito, e `handle_return_combo` põe a entrada em modo `Foreground` assim mesmo. O Sofá pode ficar por cima do jogo sem foco, gerando `input://intent` enquanto o jogo recebe o mesmo controle. Funcional (CA-12, CA-63); sugerido `release_topmost` no caminho de falha e manter o modo de entrada como estava.

### Para o PO (tarefas)

- **SEC-BUG-01 (médio, back):** exclusões de SEC-17 e encerramento por handle também no modo Job; atualizar o ADR-0005.
- **SEC-BUG-02 (baixo, back):** pasta do cliente da Steam proibida para todas as lojas.
- **SEC-BUG-03 (baixo, back):** purge recusa com o Sofá aberto; o BE-38 fecha o Sofá antes.
- **DR-06 (alto, arquiteto):** ajustar o ADR-0006 §6b antes do BE-38 (sem remoção elevada por curinga; sem fallback elevado, salvo decisão do usuário em AF-11).
- **Guards (baixo, back, BE-12):** recomendações acima.
