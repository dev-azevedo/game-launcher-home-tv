<p align="center"><img src="docs/brand/sofa/assets/logo-horizontal.svg" alt="SofaLauncher" height="72"></p>

[English](README.md) | **Português (Brasil)**

# SofaLauncher

O SofaLauncher é um launcher de jogos para Windows, feito para usar na TV com um controle Xbox. Ele reúne numa biblioteca os jogos da Steam, da Epic Games e do GOG, mais qualquer `.exe` que você adicionar. Você abre o jogo, joga e volta ao launcher sem teclado nem mouse.

> Status: pré-release `v0.2.0`. O instalador não tem assinatura de código (veja [Download e instalação](#download-e-instalação)).

## Requisitos

- Windows 10 ou 11, 64 bits.
- Um controle Xbox (com fio ou sem fio). O teclado serve só como emergência.
- WebView2. Se faltar (mais comum no Windows 10), o instalador baixa da Microsoft, então esse PC precisa de internet durante a instalação.

## Download e instalação

1. Baixe `SofaLauncher_0.2.0_x64-setup.exe` e `SHA256SUMS.txt` na [release v0.2.0](https://github.com/dev-azevedo/game-launcher-home-tv/releases/tag/v0.2.0). A release traz também `README.txt`, `BUILD-INFO.txt` e `coletar-logs.ps1`.
2. Confira o hash no PowerShell, na pasta do arquivo:

   ```powershell
   Get-FileHash .\SofaLauncher_0.2.0_x64-setup.exe -Algorithm SHA256
   ```

   O valor tem que ser igual ao de `SHA256SUMS.txt`. Se for diferente, não instale.
3. Rode o instalador. Ele **não é assinado**, então o SmartScreen do Windows pode mostrar um aviso. Continue só depois de o hash bater.
4. O instalador pede permissão de administrador (UAC) e instala para o PC inteiro.
5. A pasta é fixa: `C:\Program Files\Sofa`. Não há página de pasta, e a opção `/D=` com outra pasta é recusada. Uma instalação anterior fora de Program Files também é recusada: desinstale-a antes.
6. A página "Iniciar com o Windows" vem marcada. Cada usuário do Windows pode mudar depois, em Configurações.

Para jogar sem teclado logo ao ligar o PC, configure o logon automático ou o Windows Hello no Windows. O SofaLauncher começa na área de trabalho, e o instalador não altera o seu logon.

**Atualizando da 0.1.0.** Rode `SofaLauncher_0.2.0_x64-setup.exe` por cima da instalação existente: a biblioteca, as configurações e o "Iniciar com o Windows" ficam, e o atalho antigo (com o nome da 0.1.0) é trocado pelo do SofaLauncher. O programa agora se chama SofaLauncher, mas os nomes técnicos não mudam: `sofa.exe`, a pasta de dados `%LOCALAPPDATA%\app.sofa.launcher`, a pasta de instalação `C:\Program Files\Sofa` e o identificador do app `app.sofa.launcher`.

## Usando com o controle

| Botão | O que faz |
|---|---|
| D-pad | Move o foco, troca de aba, ajusta o volume no menu rápido |
| A | Jogar, escolher, confirmar, voltar ao jogo em andamento |
| B | Voltar, cancelar, fechar |
| View | Opções do jogo (na biblioteca) |
| Menu | Menu rápido |
| LB / RB | Pular linhas |
| LT / RT | Diminuir / aumentar o volume (na biblioteca) |
| View + Menu | Combinação de retorno: volta ao SofaLauncher com um jogo aberto |

**Adicionar jogos.** Os jogos instalados na Steam, na Epic e no GOG são detectados sozinhos. Para adicionar outro, vá em Configurações, Biblioteca, Adicionar jogo e escolha um `.exe` em um disco do PC. Pasta de rede, argumentos de linha de comando e arquivos que não sejam `.exe` não são aceitos. Cada card mostra a origem (Steam, Epic, GOG ou Manual). "Atualizar biblioteca" procura jogos novos.

**Jogar.** Vá até o card e aperte A. Com o jogo aberto, o card mostra "Em andamento" e A volta para ele. Quando o jogo fecha, o SofaLauncher volta para a frente.

**Voltar ao SofaLauncher durante o jogo.** Aperte View + Menu juntos. Um painel oferece: voltar ao jogo, fechar jogo, ir para a biblioteca (o jogo continua aberto) e menu rápido. "Fechar jogo" pede ao jogo para fechar; se ele não responder, aparece "Forçar fechamento" (o progresso não salvo se perde).

**Menu rápido (Menu).** Volume, Mudo, Desligar, Reiniciar, Suspender, Configurações e Fechar o launcher. Desligar, Reiniciar e Suspender pedem confirmação, com Cancelar em foco. "Fechar o launcher" fica no mesmo grupo, mas só fecha o SofaLauncher: não mexe no PC, e um jogo aberto continua aberto. Ele também pede confirmação (Cancelar em foco) e diz como abrir de novo: pelo atalho do SofaLauncher ou reiniciando o PC. O desligamento é o normal do Windows: se outro app tiver trabalho não salvo, o Windows avisa. Suspender aparece como indisponível em PC que não suporta.

**Volume.** LT e RT na biblioteca, ou o D-pad no menu rápido.

**Opções do jogo (View no card).** Jogar (ou Voltar ao jogo), Renomear, Restaurar nome original, Ocultar. Renomear usa o teclado do PC. Ocultar tira o jogo da biblioteca e das atualizações e não apaga nenhum arquivo. Os itens ocultos voltam em Configurações, Itens ocultos.

**Configurações.** Abas: Tela (em qual monitor o SofaLauncher abre), Biblioteca (resumo, atualizar, adicionar jogo), Geral (Iniciar com o Windows) e Itens ocultos.

**Iniciar com o Windows.** Abre o SofaLauncher quando você entra no Windows. Vale só para o seu usuário.

### Teclado de emergência

| Tecla | Equivalente no controle |
|---|---|
| Setas | D-pad |
| Enter | A |
| Esc ou Backspace | B |
| F1 ou tecla de aplicativo (ContextMenu) | Menu: abre o menu rápido |
| F2 | View: opções do jogo na biblioteca |
| PgUp / PgDn | LB / RB |
| `[` / `]` | LT / RT: volume menos / mais (só na biblioteca) |

Dentro de um campo de texto, só Enter, Esc e ↑↓ saem do campo. Não há tecla para a combinação de retorno.

## Dados e logs

O SofaLauncher guarda biblioteca, itens ocultos, nomes personalizados, datas de último jogo e configurações em `%LOCALAPPDATA%\app.sofa.launcher`. Os logs ficam em `%LOCALAPPDATA%\app.sofa.launcher\logs`. Nada sai do PC sozinho.

**Relatar um problema.** Baixe o `coletar-logs.ps1` da release e rode:

```powershell
powershell -ExecutionPolicy Bypass -File .\coletar-logs.ps1
```

Ele lê a pasta de dados (e nunca a altera) e cria um zip na área de trabalho com os logs, o `settings.json`, informações do Windows, da placa de vídeo e do WebView2, e o local da instalação. Ficam de fora o `library.json`, as capas, o seu nome de usuário e o nome do computador. Envie o zip junto com o relato. Use `-IncluirBiblioteca` só se pedirem: ele inclui a lista de jogos.

## Desinstalar

Configurações do Windows, Aplicativos, Aplicativos instalados, SofaLauncher. O desinstalador pergunta se mantém os dados, e **Manter dados (biblioteca e configurações)** vem marcada. Marcada, uma reinstalação depois encontra a sua biblioteca. Desmarcada, os dados do seu usuário são apagados. Atualizar por cima não apaga dados.

## Problemas comuns

- **O SmartScreen avisa sobre o instalador.** É esperado: ele não é assinado. Confira o SHA-256 e continue só se bater.
- **O instalador falha em PC sem WebView2.** Ele baixa o runtime da Microsoft: conecte o PC à internet e tente de novo. Não há instalador offline.
- **O controle não responde.** O SofaLauncher mostra "Nenhum controle conectado". Ligue o controle ou conecte o cabo. O teclado funciona enquanto isso.
- **O jogo não volta para a frente.** Abra o painel do jogo com View + Menu e escolha "Voltar ao jogo". Se o Windows não deixou o SofaLauncher passar na frente, o jogo foi minimizado e "Voltar ao jogo" restaura a janela. Jogo que roda como administrador pode ignorar a combinação de retorno: use o teclado (F1) ou feche o jogo pelo menu dele.
- **O instalador se recusa a continuar.** Tire o `/D=` da linha de comando, ou desinstale a cópia anterior que está fora de Program Files.

## Para desenvolvedores

Tauri 2 + Rust, com UI em React. A documentação da feature fica em `docs/` (spec, arquitetura, ADRs, segurança, contrato de API em `docs/api/launcher-tv.md`). As notas técnicas completas, abaixo, estão em português.

### Estrutura

| Pasta | Conteúdo |
|---|---|
| `crates/launcher-core` | Domínio e aplicação. Sem Tauri, sem Windows, sem I/O (INV-01) |
| `crates/launcher-win` | Adapters Windows (persistência, log, hardening, e os próximos BE-xx) |
| `src-tauri` | Apresentação e composition root: comandos, DTOs, capability, janela |
| `ui` | Front-end (Vite + React + TS) |
| `scripts` | `ci.ps1` e guards |

### CI local (D-34)

Não há servidor de CI: o `scripts/ci.ps1` roda tudo que precisa passar antes de commitar e antes
de gerar um release.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1               # tudo
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 -SkipAudit    # sem rede (registre o motivo)
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 -SkipUi       # só o back
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 -SelfTestOnly # só o autoteste dos guards
powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 -Release      # CI + release MSVC (instalador e SHA256SUMS)
```

Passos, nesta ordem (para no fim com código 1 se qualquer um falhar):

1. **Autoteste dos guards**: uma violação plantada de cada regra precisa ser detectada, e uma
   árvore limpa não pode acusar nada. Garante que o guard não "passa" por estar quebrado.
2. **Guards por grep** (`scripts/ci-guards.ps1`): chamadas de INV-03, INV-05, INV-08 e INV-09 fora
   dos arquivos donos (`XInputGetState`, `SetForegroundWindow`, `SendInput`, `AttachThreadInput`,
   `EWX_FORCE*`, `SeDebugPrivilege`, `PROCESS_ALL_ACCESS`, `shutdown.exe`, `SetWindowsHookEx`,
   `CreateRemoteThread`, `ReadProcessMemory`, `WriteProcessMemory`, `LoadLibrary*`, `remove_file`,
   `remove_dir_all`, `DeleteFileW`, `fs::rename`, `MoveFileExW`); `WINEVENT_INCONTEXT`,
   `SPI_SETFOREGROUNDLOCKTIMEOUT`, `uiAccess="true"` em qualquer lugar e `SetWinEventHook` fora de
   `foreground.rs`/`system_events.rs`; `nsExec`/`ExecWait` sem `$SYSDIR\` nos scripts NSIS (SEC-32).
   Varre `crates/**/src` e `src-tauri/src`; não varre `tests/` nem comentários.
3. `cargo fmt --check` e `cargo clippy --workspace --all-targets -- -D warnings`.
4. `cargo test --workspace`, que inclui os guards escritos como teste: lista de comandos x
   `capabilities/main.json` x `generate_handler!` x contrato, manifesto `asInvoker` embutido no
   `sofa.exe`, CSP de release sem `unsafe-inline`/`unsafe-eval`/origem externa, e a regeneração dos
   tipos TypeScript.
5. **Bindings TypeScript**: os tipos de `ui/src/shared/ipc/generated/` são gerados por
   `cargo test -p sofa --test bindings` (ts-rs). O passo falha se o que está no git difere do gerado
   (faça o commit da pasta junto com a mudança do DTO). Não edite esses arquivos à mão.
6. `cargo deny check` (SEC-24, SEC-30): sem cliente HTTP no grafo, licenças e advisories.
7. `npm audit --omit=dev --audit-level=high` em `ui/` (precisa de rede).
8. `npm run check` em `ui/` (typecheck, lint, testes).

Para adicionar uma regra de guard, edite `$GuardRules` em `scripts/ci-guards.ps1` e acrescente o
token ao autoteste.

### Release e instalador (BE-37 a BE-39, BE-42, BE-46)

O CI de dev continua no `-gnu` e **não** ganhou passo MSVC por padrão: o `-gnu` é rápido, e o MSVC
só é necessário para o que é distribuído (loader do WebView2 estático, CFG, um único `RT_MANIFEST`;
ADR-0001). O MSVC entra em `scripts/ci.ps1 -Release`, que depois dos passos acima gera o release em
`..\sofa-releases\v<versão>` (ao lado do repo, fora do git; troque com `-ReleasesDir`):

1. `cargo test` no MSVC (`-SkipMsvcTests` pula), na toolchain MSVC **da versão fixada** em
   `rust-toolchain.toml` (ou `stable-...-msvc` se for a mesma versão; senão o script diz o comando
   `rustup toolchain install`). Por padrão sem `session_e2e` e `session_runtime`, que sobem o
   `sofa-test-game.exe`. `-FullMsvcTests` roda tudo. O helper sem metadados era barrado pelo
   Defender (falso positivo `Trojan:Win32/Bearfoos.A!ml`, erro 225, BE-52); ganhou versão, empresa,
   descrição e manifesto (`crates/launcher-win/test-game.rc`, `build.rs`) e passa. **Nada de
   exclusão no Defender**: se o erro 225 voltar, o teste falha e o motivo está aqui.
2. Relatórios de `cargo deny` e `npm audit` (produção, nível alto) em `relatorios\`.
3. `tauri build --target x86_64-pc-windows-msvc --bundles nsis` (a CLI vem do `ui/package.json`, sem
   scripts de instalação): NSIS por máquina, `Program Files\Sofa`, bootstrapper do WebView2 embutido.
4. **Travas do `sofa.exe` MSVC**: `src-tauri/tests/manifest_resource.rs` lê o diretório de recursos e
   exige exatamente um `RT_MANIFEST` (`asInvoker`, `uiAccess="false"`, Common Controls v6;
   ARQ-BUG-08), e `src-tauri/tests/pe_hardening.rs` exige Control Flow Guard (`GUARD_CF`), ASLR de
   64 bits e DEP no cabeçalho do PE (SEC-BUG-06). O CFG vem de `.cargo/config.toml`
   (`-C control-flow-guard` no alvo MSVC; a variável `RUSTFLAGS`, se existir, o substituiria, e o
   teste reprova). No `-gnu` os dois testes só avisam.
5. **Varredura do Defender** (BE-52, RR-02): `MpCmdRun -Scan -ScanType 3 -File` no instalador e no
   `sofa.exe`; se acusar (ou não concluir), o build falha. O resultado vai para `relatorios\defender.txt`
   e para o `BUILD-INFO.txt`. Não muda configuração nem exclusão do Defender.
6. Guards de NSIS (SEC-31, SEC-32) e do BOM no `installer.nsi` gerado.
7. `scripts/test-installer.ps1`: testes do instalador e do desinstalador **sem instalar nada**, sem
   elevação, em pastas temporárias (veja abaixo).
8. Copia o instalador, gera `SHA256SUMS.txt` (formato do `sha256sum -c`), `README.txt` com o
   `Get-FileHash` e `BUILD-INFO.txt`, e confere o hash recalculando.

Release oficial sai **de uma tag** `v<versão>` com a árvore limpa (RN-30); `-AllowUntagged` permite um
build de teste, marcado como tal em `BUILD-INFO.txt`. O instalador não é assinado (D-11): quem recebe
confere o SHA-256. O `target\`, `*-setup.exe` e `SHA256SUMS.txt` estão no `.gitignore`; binário nunca
entra no git.

`scripts/test-installer.ps1` (também roda sozinho, depois de um `tauri build`, que baixa o NSIS):
compila com o `makensis` do Tauri o harness `src-tauri/windows/tests/harness.nsi`, que roda as mesmas
funções de `uninstall-hooks.nsh`, e um stub de `sofa.exe`. Cobre: SID, decisão por código de saída
do purge (0 a 4, não lido, ainda rodando), purge de ponta a ponta (filho do Explorer, integridade
Medium, mesmo usuário), caminho alternativo com o `CreateProcessW` forçado a falhar, fallback restrito
(só `library.json` e `settings.json`; raiz e arquivos como junction, ponto de reanálise, pasta, hard
link, somente leitura, em uso; junction trocada em laço), a remoção da tarefa de autostart por SID,
a **pasta fixa em `Program Files\Sofa`** (`/D=` diferente e instalação anterior fora dela recusados,
sem página de pasta; SEC-BUG-05) e os acentos das demais strings (UTF-8 sem mojibake e um BOM no `.nsi` gerado); o nome SofaLauncher no instalador compilado.
Symlink de verdade exige privilégio: num PowerShell elevado o script o inclui sozinho. O que só vale
instalando (UAC de outra conta, ACL, `HKU` de outros perfis, DLL plantada) fica no QA-17.

### Toolchain

`rust-toolchain.toml` fixa o toolchain estável (piso 1.77.2, SEC-24). Na máquina de desenvolvimento
atual o alvo é `x86_64-pc-windows-gnu` com `windows` 0.58; o release usa MSVC (ver `Cargo.toml` e a
seção "Release e instalador"). Com o VS 2022 instalado, o `rustup` acha o linker MSVC sozinho; não
ponha o MinGW na frente do `PATH` ao compilar para MSVC.

## Licença

A definir. Ainda não há arquivo `LICENSE` neste repositório. As fontes Sora e Figtree mantêm as próprias licenças OFL (`docs/brand/sofa/assets/fonts`).
