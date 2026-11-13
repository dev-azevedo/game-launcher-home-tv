---
status: accepted (degraus 2 e 3 desligados por padrão até a revisão da segurança com o resultado do QA final)
date: 2026-11-13
revised: 2026-11-13 (rev. 3: condições de SEC-28; rev. 4: D-35, sem spike com o usuário; degraus configuráveis em runtime)
feature: launcher-tv
---

# Trazer o Sofá para a frente: escada de tentativas com verificação, sem mudar configuração do sistema

O Windows só deixa um processo tomar o primeiro plano em situações restritas: ele recebeu a última entrada, é o processo em primeiro plano, ou recebeu permissão. Quando a combinação de retorno é detectada (CA-32) ou o jogo termina (CA-30), o Sofá está em segundo plano e não tem esse direito, porque XInput não conta como entrada para o Windows. Decidimos que um único módulo, o `ForegroundBroker`, executa uma **escada de tentativas**. Cada degrau é verificado com `GetForegroundWindow` após ~150 ms, e a escada para no primeiro que der certo, sempre dentro do orçamento de 3 s (M-04):

1. `SetForegroundWindow` simples, com a janela do Sofá visível e marcada `HWND_TOPMOST` enquanto o painel estiver aberto.
2. **Tecla sintética neutra**: `SendInput` de **um único par** tecla pressionada/solta de uma tecla virtual sem função (`VK_F24` preferida, ou um código não atribuído), seguido de `SetForegroundWindow`. Com isso o Sofá passa a ser o processo que recebeu a última entrada. `VK_MENU` (Alt) só entra se o QA final mostrar (com `advanced.foreground.step2Key = "Alt"`, após a revisão da segurança) que a tecla neutra não libera o primeiro plano, porque Alt abre menu em alguns jogos.
3. `AttachThreadInput` à thread dona da janela em primeiro plano **naquele instante**, seguido de `SetForegroundWindow` + `BringWindowToTop`, e desanexar.
4. **Fallback da spec (RN-24 / CA-63)**: `ShowWindowAsync(SW_MINIMIZE)` nas janelas de topo do jogo, depois `SetForegroundWindow` do Sofá.

Para devolver o foco ao jogo ("Voltar ao jogo", CA-34), o Sofá já está em primeiro plano e pode chamar `SetForegroundWindow`/`ShowWindowAsync(SW_RESTORE)` diretamente. Antes de iniciar um jogo, o Sofá chama `AllowSetForegroundWindow(ASFW_ANY)` para que a loja e o jogo possam assumir a frente (CA-27).

## Condições de SEC-28 (segurança rev. 2): obrigatórias na implementação

**Proibido em qualquer degrau ou módulo**:
- `SetWindowsHookEx` de qualquer tipo global (incluindo `WH_KEYBOARD_LL`, `WH_MOUSE_LL`, `WH_GETMESSAGE`, `WH_CBT`);
- `SetWinEventHook` com `WINEVENT_INCONTEXT`;
- injeção de DLL, `ReadProcessMemory`/`WriteProcessMemory`, `CreateRemoteThread`;
- **`uiAccess=true`** no manifesto;
- **`SystemParametersInfo(SPI_SETFOREGROUNDLOCKTIMEOUT)`** e qualquer mudança de configuração global para ganhar o primeiro plano.

**Degrau 2 (`SendInput`)**:
- um único par down/up, sem outra tecla, mouse ou texto;
- disparado **só pelo núcleo**, em resposta à combinação de retorno (CA-32) ou ao fim de sessão (CA-30), **nunca por comando IPC** (não existe comando na lista fechada que o acione);
- no máximo uma vez por tentativa de retorno;
- registrado no log (degrau, tecla, resultado);
- tecla neutra preferida a Alt.

**Degrau 3 (`AttachThreadInput`)**:
- anexar só à thread dona da janela em primeiro plano naquele instante, numa thread de trabalho com tempo limite (~150 ms);
- desanexar **sempre**, por guarda RAII (um `Drop` que chama `AttachThreadInput(.., FALSE)`), inclusive em erro, `panic` ou timeout;
- nunca manter anexado além da tentativa;
- com processo elevado (UIPI), não tentar contornar: segue para o degrau 4.

**Eventos de primeiro plano (`SystemEvents`)**: `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, NULL, cb, 0, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS)`. O callback só usa o HWND, o PID e o caminho de imagem, já permitidos em SEC-15.

**Localidade**: `SendInput`, `AttachThreadInput`, `SetForegroundWindow` e `BringWindowToTop` só aparecem em `launcher-win/src/foreground.rs`; `SetWinEventHook` só em `system_events.rs`. O CI verifica por grep (INV-03, INV-08), junto com o grep vazio das APIs proibidas.

**Revisão obrigatória antes de ligar os degraus 2 e 3** (D-35: com o resultado do QA final, não de um spike): o arquiteto leva ao agente `seguranca` o log de diagnóstico com a tabela jogo × modo × degrau × ms, o efeito da tecla sintética em cada jogo e o comportamento do jogo com anti-cheat. O degrau fica **vetado** se:
- o anti-cheat sinalizar, bloquear ou fechar o jogo;
- a tecla causar ação visível no jogo que não se resolva trocando a tecla;
- o degrau exigir qualquer item proibido.

Sem os degraus 2 e 3, a escada fica com os degraus 1 e 4, que a spec aceita (CA-63).

## Decisão sem spike com o usuário (rev. 4, D-35)

O usuário só testa no fim, então a escada é implementada inteira e **cada degrau é ligável ou desligável em runtime**, em `settings.json → advanced.foreground` (doc de arquitetura §9.3):

| Chave | Padrão |
|---|---|
| `step1` (SetForegroundWindow) | ligado |
| `step2` (tecla sintética) | **desligado** (SEC-28) |
| `step2Key` | `F24` (`Alt` só como alternativa) |
| `step3` (AttachThreadInput) | **desligado** (SEC-28) |
| `step4` (minimizar o jogo) | ligado |
| `verifyDelayMs` | 150 (faixa 50..500) |

- Não há comando IPC nem tela para mudar esses valores. A edição é manual no arquivo, no QA final, e o padrão vale para qualquer valor fora do schema. O caminho que dispara o degrau 2 continua só no núcleo (SEC-28).
- Com `diagnostics.level = "debug"`, cada tentativa registra os degraus tentados, ms e `FrontOutcome`, além da versão do Windows. O usuário devolve o log depois do QA final.
- O padrão entregue (degraus 1 + 4) já cumpre a spec (RN-24, CA-63).

**Validar no QA final**: QF-01 (retorno ≤ 3 s por jogo e modo), QF-02 (anti-cheat offline, D-30). Ligar os degraus 2/3 só depois da revisão da segurança.

**Para a segurança reavaliar (SEC-28)**: a trava dos degraus 2/3 passou de constante de compilação para configuração em arquivo do usuário (editável por processo do mesmo usuário, premissa RR-01). O padrão continua desligado.

## Opções descartadas

- **Mudar `SPI_SETFOREGROUNDLOCKTIMEOUT` para 0**: muda o sistema inteiro, de forma persistente. Proibido por SEC-28.
- **`uiAccess=true`**: daria o direito ao primeiro plano, mas exige binário assinado em pasta segura e amplia o privilégio da UI. Proibido por SEC-28.
- **Overlay injetado no jogo / hook global**: fora de escopo, arriscado com anti-cheat (R-07), proibido por SEC-28.
- **`RegisterHotKey`**: exigiria simular a tecla via `SendInput`, ou seja, o degrau 2 com mais peças.

## Consequências e limites conhecidos

- Jogo rodando **como administrador**: a UIPI bloqueia `SendInput` e pode bloquear `ShowWindow` vindos de um processo sem elevação. O retorno pode falhar. É limitação documentada (RA-02); o Sofá nunca eleva (RN-26).
- O dev mede os tempos com apps sintéticos (ST-01: janela, borderless, D3D em tela cheia exclusiva de teste, processo elevado de teste). A validação com jogos reais é o QF-01/QF-02 do QA final.
