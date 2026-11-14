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

## Toolchain

`rust-toolchain.toml` fixa o toolchain estável (piso 1.77.2, SEC-24). Na máquina de desenvolvimento
atual o alvo é `x86_64-pc-windows-gnu` com `windows` 0.58; o release usa MSVC (ver `Cargo.toml`).
