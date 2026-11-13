---
status: accepted
date: 2026-11-13
feature: launcher-tv
---

# Entrada do controle: XInput direto, lido só pelo núcleo Rust, numa thread própria

A combinação de retorno (View + Menu por 1,5 s, RN-11) precisa ser lida **com o jogo em primeiro plano**, quando a janela do launcher não tem foco e o WebView2 pode estar suspenso. Decidimos que **um único caminho** lê o controle: uma thread dedicada no núcleo que chama `XInputGetState` (xinput1_4.dll, via `windows-rs`), converte o estado bruto em *intenções* (`Up`, `Confirm`, `Back`, `Menu`, `VolumeUp`...) com zona morta, repetição e borda de gatilho, e as envia à UI por evento. A UI não lê o controle (não usa a Gamepad API do navegador). O teclado de emergência é traduzido na UI para as mesmas intenções.

## Opções consideradas

- **Gamepad API do WebView2 em primeiro plano + Rust só para a combinação**: duas implementações de zona morta, repetição e gatilho, que tendem a divergir. A Gamepad API só funciona com a página visível e focada. Descartada.
- **`gilrs`**: no Windows usa por padrão o backend Windows.Gaming.Input, que pode deixar de entregar entrada para apps sem foco, justamente o caso da combinação de retorno. Além disso, a abstração multi-plataforma não traz ganho, porque o MVP só usa XInput. Descartada.
- **Windows.Gaming.Input / GameInput**: WGI tem a mesma dúvida sobre foco. GameInput exige um redistribuível e é novo demais para o Windows 10. Descartada.
- **Botão Guide (Xbox)**: só aparece pela função não documentada `XInputGetStateEx` (ordinal 100), e o Xbox Game Bar o captura por padrão. Confirma R-05: o Guide não é usado (a confirmação final fica no QA final, QF-01).

## Consequências

- O núcleo decide **quando** a entrada vale: com a janela do launcher em primeiro plano, emite todas as intenções; durante uma sessão de jogo com o launcher em segundo plano, não emite nada, salvo a combinação de retorno (CA-12). A UI decide **o que** cada intenção faz em cada tela (ex.: LT/RT só mudam o volume na biblioteca, RN-23).
- Frequência de leitura: ~120 Hz com o launcher em primeiro plano e ~20 Hz em segundo plano. Slots sem controle são consultados só a cada 1 s, porque `XInputGetState` em slot vazio é caro (CA-15 aceita até 2 s).
- Limitação aceita: XInput é lido por todos os processos, então um jogo que lê o controle sem checar o foco também recebe os botões enquanto o launcher está na frente (ver risco RA-01 em `docs/arch/launcher-tv.md`).
