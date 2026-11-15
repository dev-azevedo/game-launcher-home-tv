---
status: accepted
date: 2026-11-13
feature: launcher-tv
---

# Stack: Tauri 2 (núcleo Rust) + UI React/TypeScript em WebView2, repo único

O launcher é um app desktop Windows 10/11, tela cheia, 10-foot UI, que precisa de muita API Win32 (XInput em thread de fundo, Core Audio, foco de janela, processos, energia, registro) e de uma UI "console moderno" com animação e capas grandes. Escolhemos **Tauri 2**: o núcleo em Rust (`src-tauri/` + crates do workspace) fala com o Windows via `windows-rs`; a UI é React + TypeScript + Vite rodando no WebView2. Tudo no mesmo repositório.

## Opções consideradas

| Opção | A favor | Contra | Veredito |
|---|---|---|---|
| **Tauri 2 (Rust + WebView2)** | Rust com `windows-rs` cobre todas as APIs Win32/COM sem FFI manual; binário pequeno (~10 MB); núcleo ocioso usa pouca CPU/RAM (RN-21); UI web é o caminho mais rápido para o visual "console" (CSS, animação em `transform`/`opacity`); bundler NSIS com `perMachine` e WebView2 embutido no instalador (CA-50); plugin pronto de instância única (o autostart usa adapter próprio, ADR-0006). Atende a preferência do usuário por Rust. | Depende do WebView2 (processos extras, ~100-200 MB somados em primeiro plano); IPC assíncrono entre UI e núcleo; duas linguagens. | **Escolhida** |
| Electron | Ecossistema enorme, Gamepad API e Chromium previsíveis. | Node + Chromium próprios: ~150 MB de instalador e ~200-300 MB de RAM; acesso Win32 exige addon nativo (N-API) ou `ffi-napi`, ou seja, uma terceira linguagem; pior para RN-21. | Descartada |
| Rust nativo (egui / iced / Slint) | Um processo, uma linguagem, RAM mínima, sem WebView2. | egui é modo imediato: redesenha a cada frame e tem visual "ferramenta", pouco adequado a cards grandes com animação; iced e Slint ainda têm pouca maturidade em animação, carregamento de imagens grandes e texto escalado em 4K; navegação espacial por foco teria de ser escrita do zero; Slint tem licença dupla (GPL/royalty-free com atribuição). Mais tempo para chegar no visual pedido. | Descartada (reavaliar se o WebView2 se mostrar inviável no consumo em segundo plano) |
| .NET (WPF / WinUI 3) | Integração Windows de primeira; XAML com foco por controle (WinUI tem `XYFocus`). | WinUI 3 exige Windows App SDK e tem histórico de problemas de empacotamento sem MSIX; WPF não tem navegação espacial nativa e a UI 10-foot dá trabalho; usuário preferiu Rust/Go; runtime .NET no instalador. | Descartada |
| Go (Wails) | Linguagem sugerida pelo usuário; mesma ideia do Tauri. | Bindings Win32/COM em Go são manuais (`golang.org/x/sys/windows` + `go-ole`); COM de áudio e callbacks ficam frágeis; ecossistema de plugins menor. | Descartada |

## WebView2 no Windows 10

O Windows 11 já traz o runtime WebView2 Evergreen. No Windows 10 ele está presente na grande maioria das máquinas atualizadas (é distribuído pelo Windows Update/Edge desde 2021), mas não é garantido. **Decisão do usuário (P-01 a):** o instalador NSIS do Tauri embute o *bootstrapper* (`webviewInstallMode: embedBootstrapper`, +~2 MB). Se o runtime faltar, ele é baixado da Microsoft e instalado durante a instalação, já elevado pelo UAC (CA-50). Essa é a única saída de rede aceita (SEC-26, SEC-30). O runtime é Evergreen, não Fixed Version (AF-07), para receber correções de segurança do Chromium.

Opções descartadas: instalador offline (+~130 MB por um caso raro) e bootstrapper baixado na hora (exige internet sempre que o runtime faltar, igual ao embutido, mas sem ganho).

## Toolchain (rev. 5, SP-06/SP-07)

- **Release, instalador e CI**: Rust estável, alvo **`x86_64-pc-windows-msvc`**. É o único alvo em que o loader do WebView2 é estático e o binário tem CFG, como SEC-23 pressupõe. O teste de DLL plantada só vale nesse build.
- **Desenvolvimento local**: enquanto a máquina de dev não tiver o MSVC Build Tools, usa `x86_64-pc-windows-gnu` (MinGW em escopo de usuário). Nesse alvo o `WebView2Loader.dll` aparece ao lado do exe. Serve para desenvolver e para os spikes, nunca para distribuir.
- **Onde roda o build MSVC (P-08 b, decisão do usuário)**: o usuário instala o Visual Studio Build Tools (MSVC) **nesta máquina de dev**. Depois disso, release, instalador NSIS e teste de DLL plantada (SEC-23) são feitos aqui, no alvo `x86_64-pc-windows-msvc`. Sem CI de nuvem no MVP.
- **`windows-rs` fixado em 0.58** no `Cargo.lock`, compatível com os dois alvos. Atualizar a versão é uma tarefa explícita, não efeito colateral de `cargo update`.
- O `rust-toolchain.toml` fixa a versão do compilador (≥ 1.77.2, SEC-24) sem fixar o alvo. O CI de release passa `--target x86_64-pc-windows-msvc`. O build de release usa `--target x86_64-pc-windows-msvc` na máquina de dev (P-08 b).
- **Manifesto no `-gnu` (ARQ-05)**: o linker do MinGW junta o manifesto padrão dele ao nosso (`sofa.exe.manifest`, via `tauri-build`) e avisa ".rsrc merge failure: multiple non-default manifests". O `sofa.exe` de dev sai com dois `RT_MANIFEST`/1. Os dois pedem `asInvoker`, mas só o nosso traz Common Controls v6. Aceito porque o `-gnu` não é distribuído. O build MSVC de release tem de sair com exatamente um manifesto, conferido por teste antes do instalador (ARQ-BUG-08, BE-39).
- Memória medida no SP-06 (working set privado, árvore inteira com 6 processos do WebView2): 293 MB na frente no pior caso (300 capas montadas, sem virtualização), 127 MB oculto, **118 MB suspenso**; retomada em ~30 ms. As metas estão no §6 da arquitetura.

## Consequências

- Toda chamada ao Windows mora no núcleo Rust. A UI nunca recebe acesso a sistema de arquivos, shell ou processos. A lista fechada de comandos, a capability e a CSP estão em `docs/arch/launcher-tv.md` §4 (SEC-01 a SEC-05).
- Produto: "Sofá"; `identifier` `app.sofa.launcher`; `mainBinaryName` `sofa` (ASCII no nome do exe e da pasta de instalação, conforme o guia da marca).
- O consumo do WebView2 com o jogo rodando é controlado escondendo a janela e suspendendo o WebView2 (`TrySuspend`). Medir no ST-04 (dev) e no QF-07 (QA final); se não ficar abaixo de 1% de CPU, a UI nativa volta a ser considerada.
- Tipos de comandos e eventos são gerados do Rust para TypeScript (`tauri-specta`), para que núcleo e UI não divirjam.
