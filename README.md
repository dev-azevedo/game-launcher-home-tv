# Game Launcher (Sofá)

Launcher de jogos para TV (Tauri 2 + Rust, UI em React). Documentação da feature em `docs/`
(spec, arquitetura, ADRs, segurança, contrato de API em `docs/api/launcher-tv.md`).

## Estrutura

| Pasta | Conteúdo |
|---|---|
| `crates/launcher-core` | Domínio e aplicação. Sem Tauri, sem Windows, sem I/O (INV-01) |
| `crates/launcher-win` | Adapters Windows (persistência, log, hardening, e os próximos BE-xx) |
| `src-tauri` | Apresentação e composition root: comandos, DTOs, capability, janela |
| `ui` | Front-end (Vite + React + TS) |
| `scripts` | `ci.ps1` e guards |

## CI local (D-34)

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

## Release e instalador (BE-37 a BE-39, BE-42, BE-46)

O CI de dev continua no `-gnu` e **não** ganhou passo MSVC por padrão: o `-gnu` é rápido, e o MSVC
só é necessário para o que é distribuído (loader do WebView2 estático, CFG, um único `RT_MANIFEST`;
ADR-0001). O MSVC entra em `scripts/ci.ps1 -Release`, que depois dos passos acima gera o release em
`..\sofa-releases\v<versão>` (ao lado do repo, fora do git; troque com `-ReleasesDir`):

1. `cargo test` no MSVC (`-SkipMsvcTests` pula), na toolchain MSVC **da versão fixada** em
   `rust-toolchain.toml` (ou `stable-...-msvc` se for a mesma versão; senão o script diz o comando
   `rustup toolchain install`). Por padrão sem `session_e2e` e `session_runtime`, que sobem o
   `sofa-test-game.exe`: o Defender classifica esse helper (compilado no MSVC) como
   `Trojan:Win32/Bearfoos.A!ml`, falso positivo, e bloqueia a execução (erro 225). `-FullMsvcTests`
   roda tudo (adicione `target\` às exclusões do Defender se quiser).
2. Relatórios de `cargo deny` e `npm audit` (produção, nível alto) em `relatorios\`.
3. `tauri build --target x86_64-pc-windows-msvc --bundles nsis` (a CLI vem do `ui/package.json`, sem
   scripts de instalação): NSIS por máquina, `Program Files\Sofa`, bootstrapper do WebView2 embutido.
4. **Trava do ARQ-BUG-08**: `src-tauri/tests/manifest_resource.rs` lê o diretório de recursos do
   `sofa.exe` MSVC e exige exatamente um `RT_MANIFEST` (`asInvoker`, `uiAccess="false"`, Common
   Controls v6). No `-gnu` o mesmo teste só avisa (o MinGW soma o manifesto dele ao nosso).
5. Guards de NSIS (SEC-31, SEC-32) no `installer.nsi` gerado.
6. `scripts/test-installer.ps1`: testes do instalador e do desinstalador **sem instalar nada**, sem
   elevação, em pastas temporárias (veja abaixo).
7. Copia o instalador, gera `SHA256SUMS.txt` (formato do `sha256sum -c`), `README.txt` com o
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
link, somente leitura, em uso; junction trocada em laço) e a remoção da tarefa de autostart por SID.
Symlink de verdade exige privilégio: num PowerShell elevado o script o inclui sozinho. O que só vale
instalando (UAC de outra conta, ACL, `HKU` de outros perfis, DLL plantada) fica no QA-17.

## Toolchain

`rust-toolchain.toml` fixa o toolchain estável (piso 1.77.2, SEC-24). Na máquina de desenvolvimento
atual o alvo é `x86_64-pc-windows-gnu` com `windows` 0.58; o release usa MSVC (ver `Cargo.toml` e a
seção "Release e instalador"). Com o VS 2022 instalado, o `rustup` acha o linker MSVC sozinho; não
ponha o MinGW na frente do `PATH` ao compilar para MSVC.
