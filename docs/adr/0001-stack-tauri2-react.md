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

## Consequências

- Toda chamada ao Windows mora no núcleo Rust. A UI nunca recebe acesso a sistema de arquivos, shell ou processos. A lista fechada de comandos, a capability e a CSP estão em `docs/arch/launcher-tv.md` §4 (SEC-01 a SEC-05).
- Produto: "Sofá"; `identifier` `app.sofa.launcher`; `mainBinaryName` `sofa` (ASCII no nome do exe e da pasta de instalação, conforme o guia da marca).
- O consumo do WebView2 com o jogo rodando é controlado escondendo a janela e suspendendo o WebView2 (`TrySuspend`). Medir no ST-04 (dev) e no QF-07 (QA final); se não ficar abaixo de 1% de CPU, a UI nativa volta a ser considerada.
- Tipos de comandos e eventos são gerados do Rust para TypeScript (`tauri-specta`), para que núcleo e UI não divirjam.
