---
status: accepted (SP-01 + parecer SG-02 rev. 5: cenário A é o padrão do MVP)
date: 2026-11-13
revised: 2026-11-13 (rev. 3: condições de SEC-28; rev. 4: D-35, degraus configuráveis em runtime; rev. 5: resultado do SP-01; rev. 6: cenário A aprovado pela segurança)
feature: launcher-tv
---

# Trazer o Sofá para a frente: escada de tentativas com verificação, sem mudar configuração do sistema

O Windows só deixa um processo tomar o primeiro plano em situações restritas: ele recebeu a última entrada, é o processo em primeiro plano, ou recebeu permissão. Quando a combinação de retorno é detectada (CA-32) ou o jogo termina (CA-30), o Sofá está em segundo plano e não tem esse direito, porque XInput não conta como entrada para o Windows. Um único módulo, o `ForegroundBroker`, executa uma **escada de tentativas**. Cada degrau é verificado com `GetForegroundWindow`, consultado a cada 5 ms até `verifyDelayMs`, e a escada para no primeiro degrau que der certo, dentro do orçamento de 3 s (M-04).

## Evidência do SP-01 (`docs/arch/spikes/SP-01.md`)

Cenário medido: Sofá sem direito de primeiro plano, que é o caso real. Apps sintéticos: janela comum, sem borda e D3D11 em tela cheia exclusiva.

| Degrau | Janela | Sem borda | Tela cheia exclusiva | Tempo |
|---|---|---|---|---|
| 1 `SetForegroundWindow` | 1/8 (não confiável) | 0/8 | 0/8 | falha em 150 a 200 ms |
| 4 minimizar o jogo + `SetForegroundWindow` | 1/26 | 0/8 | 0/8 | — |
| 2 `SendInput` (F24) + `SetForegroundWindow` | 8/8 | 8/8 | 8/8 | 1 a 5 ms |
| 3 `AttachThreadInput` + `SetForegroundWindow` + `BringWindowToTop` | 8/8 | 8/8 | 8/8 | 5 a 30 ms |

Conclusões que mudam este ADR:
- **O padrão anterior (degraus 1 + 4) não funciona.** Minimizar o jogo devolve a ativação a outra janela, não ao Sofá. O fallback de RN-24/CA-63 só funciona combinado com o degrau 2 ou 3.
- **O fim de sessão (CA-30) também precisa da escada.** Fechar o jogo não devolve o primeiro plano ao Sofá, mesmo com a janela visível e `TOPMOST`.
- **O degrau 3 exige que a thread da janela do Sofá continue bombeando mensagens.** Com a thread da janela bloqueada esperando o resultado, falhou 3/3.

## Decisão

### Ordem recomendada: 1 → 3 → 2 → 4

1. `SetForegroundWindow` simples. Passa quando o Sofá herdou o direito, por exemplo logo após uso de teclado ou mouse.
2. **`AttachThreadInput`** (degrau 3 da numeração) à thread dona da janela em primeiro plano naquele instante, + `SetForegroundWindow` + `BringWindowToTop`, desanexando em seguida. Vem antes da tecla porque **não entrega nada ao jogo**.
3. **Tecla sintética neutra** (degrau 2 da numeração): um único par down/up de `VK_F24` + `SetForegroundWindow`. É o mais rápido, mas o jogo recebe a tecla.
4. Minimizar as janelas de topo do jogo (`ShowWindowAsync(SW_MINIMIZE)`) + `SetForegroundWindow`. Último recurso, mantido por RN-24.

A numeração dos degraus (1 a 4) e as chaves `step1`..`step4` continuam as mesmas; muda só a ordem de execução, guardada em `advanced.foreground.order` (padrão `[1, 3, 2, 4]`). Pior caso medido com degrau 1 falhando e o seguinte funcionando: ~160 ms em janela e sem borda, ~350 ms em tela cheia exclusiva, muito abaixo de M-04.

### Padrão dos degraus 2 e 3: cenário A aprovado (segurança rev. 5, §12)

| Cenário (A aprovado pela segurança) | `step1` | `step3` | `step2` | `step4` | Ordem efetiva | Atende CA-32/CA-30/CA-63? |
|---|---|---|---|---|---|---|
| **A, padrão do MVP**: 2 e 3 ligados | on | on | on (F24) | on | 1 → 3 → 2 → 4 | Sim, com redundância |
| B: só o 3 liberado (sem injetar tecla) | on | on | off | on | 1 → 3 → 4 | Sim (8/8 no SP-01) |
| C: só o 2 liberado | on | off | on (F24) | on | 1 → 2 → 4 | Sim (8/8 no SP-01) |
| D: nenhum liberado | on | off | off | on | 1 → 4 | **Não.** Retorno e fim de sessão ficam sem garantia. A spec precisa ser revista pelo PO e pelo usuário (ex.: retorno só quando o Sofá tiver o direito, ou orientar Alt+Tab) |

- **Decidido: cenário A** é o padrão do MVP (`step1..step4 = true`, `order = [1, 3, 2, 4]`, `step2Key = "F24"`). O degrau 3 cobre os casos medidos, e o 2 dá redundância quando o anexo de entrada falha.
- **O build de desenvolvimento também usa o cenário A**, para que os testes reflitam o produto. Os cenários B a D continuam possíveis só por `settings.json`, para diagnóstico no QA-17.

### Regras de implementação (ForegroundBroker, BE-18)

- A escada roda numa **thread de trabalho**, nunca na thread da janela, e a thread da janela não espera o resultado de forma bloqueante.
- Antes do 1º degrau, a janela do Sofá já está visível e `HWND_TOPMOST` (mostrada com `SWP_NOACTIVATE`), e o WebView2 já foi retomado.
- Vale igual para a combinação de retorno (CA-32) e para o fim de sessão (CA-30).
- `verifyDelayMs` = 150 (faixa 50..500). Os degraus que funcionam respondem em ≤ 30 ms. Baixar para 50 a 80 ms é uma otimização permitida pela faixa, com o pior caso caindo de ~600 para ~250 ms.
- Cada tentativa registra no log a ordem, o degrau, os ms e o `FrontOutcome`.

### Condições de SEC-28 (rev. 5 da segurança)

- **Proibidos**: `SetWindowsHookEx` global de qualquer tipo; `SetWinEventHook` com `WINEVENT_INCONTEXT`; injeção de DLL; `Read/WriteProcessMemory`; `CreateRemoteThread`; `uiAccess=true`; `SPI_SETFOREGROUNDLOCKTIMEOUT` e qualquer mudança global do sistema.
- **Degrau 2**: um único par down/up de `VK_F24`, sem outra tecla, mouse ou texto. **No máximo um por tentativa, e só depois de o degrau 3 falhar** na mesma tentativa. Disparado só pelo núcleo (CA-32 ou CA-30), nunca por IPC, com log. `Alt` só por `step2Key` e após nova revisão.
- **Estação bloqueada ou desktop seguro** (tela de bloqueio, UAC, Ctrl+Alt+Del): a escada **não roda**. O `ForegroundBroker` checa antes se o desktop de entrada é o desktop do usuário (`OpenInputDesktop` + nome `Default`, mais o estado de sessão bloqueada via `WTSRegisterSessionNotification`). Se não for, registra e desiste.
- **Jogo elevado**: se a janela em primeiro plano pertence a processo de integridade maior (`TokenElevation`/nível de integridade do processo da janela), a escada **pula direto para o degrau 4** e registra no log. Não tenta os degraus 2 e 3 contra UIPI.
- **Degrau 3**: anexar só durante a tentativa (~150 ms), em thread de trabalho com timeout, com guarda RAII que sempre desanexa. **No máximo uma thread pendente do degrau 3**: se uma tentativa anterior ainda não voltou (thread do jogo travada), a nova tentativa não cria outra e segue para o degrau 2.
- `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, ..., WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS)` só em `system_events.rs`.
- `SendInput`, `AttachThreadInput`, `SetForegroundWindow` e `BringWindowToTop` só em `foreground.rs`. O CI verifica por grep (INV-03, INV-08).

### XInput e botão Guide (R-05)

- A leitura usa só **`XInputGetState`** (função pública de `xinput1_4.dll`). A combinação View + Menu (D-02) está na máscara pública. **`XInputGetStateEx` (ordinal 100) não é usado**: ele só serviria para ler o Guide.
- O Guide não entra em nenhum fluxo. A Game Bar capturar ou não o Guide não afeta o Sofá. O SP-01 não tinha controle, então a confirmação com controle real fica no QA-17 (QF-01, QF-08).
- Custo medido do polling com slot vazio: 20 Hz × 4 slots = 0,52% de um núcleo (0,03% do total); 20 Hz × 1 slot ≈ 0,1% de um núcleo. Confirma a regra do ADR-0002 de consultar slot vazio só a cada 1 s.

Para devolver o foco ao jogo ("Voltar ao jogo", CA-34), o Sofá já está em primeiro plano e chama `SetForegroundWindow`/`ShowWindowAsync(SW_RESTORE)` diretamente. Antes de iniciar um jogo, chama `AllowSetForegroundWindow(ASFW_ANY)` (CA-27).

## Opções descartadas

- **Padrão 1 + 4 apenas**: falhou no SP-01 (degrau 4 deu certo 1 vez em 26).
- **Variantes sem tecla e sem anexo** (`SwitchToThisWindow`; minimizar + `SW_SHOWNORMAL`; minimizar + espera): de 0/8 a 2/4 no SP-01. Não confiáveis.
- **`SPI_SETFOREGROUNDLOCKTIMEOUT` = 0, `uiAccess=true`, overlay injetado, hook global**: proibidos por SEC-28.
- **`RegisterHotKey`**: exigiria simular a tecla, ou seja, o degrau 2 com mais peças.

## Validar no QA-17 (não coberto pelo SP-01)

- Jogo ou processo **elevado** (RA-02, UIPI). A expectativa documentada é que a escada falhe, o que é limitação aceita.
- Jogos reais da lista D-29 × modos, efeito do F24 em jogo, anti-cheat offline (D-30): QF-01, QF-02.
- Controle real, Guide, Game Bar ligada e desligada: QF-01, QF-08.
- Sofá real com Tauri/WebView2: o BE-18 mede de novo e registra no log.
- Windows 10 22H2 (D-32).
