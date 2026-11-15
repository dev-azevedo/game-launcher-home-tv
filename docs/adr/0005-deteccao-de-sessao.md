---
status: accepted (fechado pelo SP-02; lojas e jogos reais no QA-17)
date: 2026-11-13
revised: 2026-11-13 (P-02; SEC-15/16/17 e AF-01; rev. 4: D-35, tolerâncias configuráveis; rev. 5: resultado do SP-02); 2026-11-14 (rev. 6: SEC-BUG-01, membros do jogo no modo Job, sem `TerminateJobObject`)
feature: launcher-tv
---

# Sessão de jogo: Job Object para jogo manual e GOG; varredura por pasta de instalação + espera nos handles + sinal da loja para Steam e Epic

RN-10 define o fim da sessão como "nenhum processo associado ao jogo segue rodando". O Sofá controla o processo de jogos que ele mesmo cria, mas não o de jogos iniciados pela loja (R-02). Duas estratégias ficam atrás da mesma interface (`SessionProbe`).

## Decisão

### Jogo manual e GOG (exe direto, P-02)

- `CreateProcessW` **suspenso** (sem shell, `bInheritHandles=FALSE`, SEC-10) → `AssignProcessToJobObject` → `ResumeThread`. O Job Object fica ligado a uma completion port.
- O Job Object serve para **rastrear** a árvore de processos do jogo, não para encerrá-la. Sem `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: fechar o Sofá não fecha o jogo.
- **Membros do jogo (rev. 6, SEC-BUG-01).** Nem todo processo do Job é do jogo. Um `.exe` da Steam cadastrado à mão com a Steam fechada faz o `steam.exe` nascer filho do jogo, dentro do Job. Um jogo que abre uma URL com o navegador fechado faz o mesmo com o navegador. Por isso cada PID do Job passa pelo **mesmo `classify`** do modo varredura (`domain/process_policy`):
  - `install_dir` = pasta do exe (jogo manual) ou pasta do jogo (GOG);
  - processo fora dessa pasta não é membro;
  - valem as exclusões de SEC-17 (Sofá e seus WebView2, processos e pasta das lojas, `explorer.exe`, outra sessão ou usuário, processo que não abre com `QUERY_LIMITED | SYNCHRONIZE`).
  
  Um PID do Job que não passa no `classify` é **acompanhante**: não recebe `WM_CLOSE`, não é encerrado e não mantém a sessão viva.
- **Como o conjunto é mantido.** As mensagens da completion port já tratadas em `job.rs` alimentam o conjunto:
  - em `JOB_OBJECT_MSG_NEW_PROCESS`, o PID é aberto com `PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE`. `IsProcessInJob(handle, job)` confirma que o handle é mesmo do processo do Job (fecha a corrida de reuso de PID). O tempo de criação é guardado e o `classify` decide se o processo é membro ou acompanhante;
  - em `EXIT_PROCESS` e `ABNORMAL_EXIT_PROCESS`, e por segurança a cada varredura de `Running`, a lista do Job (`JobObjectBasicProcessIdList`) é relida e conferida contra o conjunto.
  
  O handle de cada membro fica aberto no conjunto, como no modo varredura (SEC-15).
- **Fim.** A sessão vai para `Ending` quando **não sobra membro do jogo**, mesmo que haja acompanhantes vivos (a Steam ou o navegador). `JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO` continua sendo um sinal de fim (~1 ms, SP-02, S5), mas não é mais o único. A tolerância de `endGraceSeconds` vale igual (E-14).
- **Fechar e forçar no modo Job.** Usam o mesmo caminho do modo varredura, restrito aos membros:
  - "Fechar": `WM_CLOSE` só nas janelas de topo dos membros;
  - "Forçar fechamento": `terminate_if_same` em cada membro, com o handle guardado (reaberto ou duplicado com `PROCESS_TERMINATE`) e o tempo de criação conferido (SEC-15).
  
  **Proibido `TerminateJobObject`** e qualquer ação no Job inteiro. O guard de CI que dá dono a `TerminateProcess` passa a reprovar `TerminateJobObject` em qualquer arquivo.

### Steam e Epic (varredura)

- **Descoberta** de processos novos: snapshot (ToolHelp ou `EnumProcesses`) + `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE)` + `QueryFullProcessImageNameW`, **só para PIDs novos**. Há um cache por PID; os não membros são revalidados a cada ~60 s por causa de reuso de PID.
- **Associação**: caminho canônico sob a pasta de instalação, comparado por componente. Tempo de criação guardado (SEC-15).
- **Intervalo adaptativo**: 1 s em `Launching` e `Ending`, **2 s em `Running`**.
- **Fim imediato**: com o conjunto não vazio, uma thread espera nos handles dos membros (`WaitForMultipleObjects`, ou `RegisterWaitForSingleObject` se passar de 64). A saída do último membro é vista na hora, e a varredura serve só para descobrir processos novos.
- Na Steam, o sinal extra `HKCU\Software\Valve\Steam\RunningAppID` mantém a sessão viva enquanto o app estiver rodando.

### Regras comuns (domínio puro)

- Início: no primeiro processo associado, com prazo de 60 s (CA-29). Medido: 0,5 s após o lançamento.
- Fim: só após 5 s seguidos sem processo associado (E-14). Medido: nenhum falso fim com processo filho (S2), relançamento em menos de 5 s (S3a) ou homônimo fora da pasta (S4).
- Transições exigidas do domínio: `Ending → Running` quando um processo reaparece; aviso do Job Object leva `Running → Ending` direto, sem probe. **`NotConfirmed → Running`** (ARQ-05): a sonda segue ligada depois dos 60 s, e o jogo que abre tarde (loja pedindo login ou atualização, E-03/E-16) vira sessão normal, com `lastPlayedAt`. "Voltar" encerra a sonda.
- Fechar: `WM_CLOSE` nas janelas de topo dos **membros** da sessão (nos dois modos). Após 10 s, o Sofá oferece forçar (CA-35), duplicando o handle guardado com `PROCESS_TERMINATE` e conferindo o tempo de criação (SEC-15). Um só caminho de encerramento para Job e varredura.
- **Nunca no conjunto (SEC-17)**: o Sofá e seus WebView2 (fecho de `msedgewebview2.exe` a partir do PID do Sofá, não todo filho direto, porque o jogo manual ou GOG e a Steam aberta pelo Sofá são filhos diretos), processos e pasta das lojas, `explorer.exe`, outra sessão ou usuário, processos que **não abrem** com `QUERY_LIMITED | SYNCHRONIZE`. O SP-02 viu 179 de 377 processos negados, todos de sistema e serviços. Sem `SeDebugPrivilege` e sem `PROCESS_ALL_ACCESS`.
- Pasta de instalação válida (SEC-16). Jogo que exige administrador segue AF-01 (`NeedsElevation`, `ShellExecuteExW` com verbo padrão, fora do Job, só varredura).

### Configuração em runtime (`settings.json → advanced.session`, sem IPC)

| Chave | Padrão | Faixa |
|---|---|---|
| `startTimeoutSeconds` | 60 (CA-29) | 30..300 |
| `endGraceSeconds` | 5 (E-14) | 2..30 |
| `closeTimeoutSeconds` | 10 (RN-12) | 5..60 |
| `probeIntervalMs` (Launching/Ending) | 1000 | 500..5000 |
| `runningProbeIntervalMs` (Running) | 2000 | 1000..10000 |
| `useSteamRunningAppId` | `true` | bool |

### Meta de CPU (base corrigida pelo SP-02)

As metas de CPU são lidas como **% da CPU total da máquina**, a mesma base do Gerenciador de Tarefas e de RN-21. Para cálculos internos também vale a medida em % de um núcleo.

| Variante (≈ 380 processos) | % de 1 núcleo | % do total (16 lógicos) |
|---|---|---|
| Ingênua (abrir tudo a cada 1 s) | 1,8 a 2,1% | 0,12% |
| Cache por PID, 1 s | 0,7 a 1,3% | 0,04 a 0,08% |
| **Decidido**: cache + 2 s em Running + espera nos handles | ~0,35 a 0,65% (estimado) | **~0,02 a 0,04%** |

Meta: **< 0,1% da CPU total** (equivale a < 1,6% de um núcleo numa máquina de 16 lógicos e a < 0,8% numa de 8). Quase todo o custo restante é a listagem de processos do próprio sistema (19 a 28 ms por chamada).

## Opções consideradas

- **WMI/ETW**: exigem administrador. Descartada.
- **Varredura por pasta para tudo**: o Job Object é exato e grátis onde dá. Descartada.
- **GOG via Galaxy**: descartada pelo usuário (P-02).
- **Associação por nome de processo**: fere SEC-15 (S4 mostra o risco). Descartada.
- **Abrir handle de todo processo a cada varredura**: 1,8 a 2,1% de um núcleo. Descartada em favor do cache.

## Consequências

- Relançamento com intervalo maior que `endGraceSeconds` dá falso fim (S3b). Limitação conhecida (R-03, E-14), ajustável por `endGraceSeconds`.
- Jogo cujo processo não abre com `QUERY_LIMITED` (proteção agressiva) fica invisível para a varredura. Restam o `RunningAppID` (Steam) e o `ACTIVE_PROCESS_ZERO` do Job (manual/GOG). Entra no R-03.
- **Modo Job (rev. 6):** jogo manual cujo processo real roda **fora** da pasta do exe cadastrado (ex.: um lançador em `C:\Jogos\X\` que abre o jogo em outra árvore) tem esse processo tratado como acompanhante. A sessão termina quando o lançador sai (falso fim, como S3b), e "Fechar" e "Forçar" não alcançam o jogo. É o preço de nunca encerrar um processo de fora da pasta (SEC-17). Entra no R-03; o QA-17 observa.
- Teste exigido (SEC-BUG-01): exe de teste que inicia um filho chamado `steam.exe` fora da pasta do jogo. "Forçar fechamento" mantém o filho vivo, e a sessão termina quando o jogo sai, com o filho ainda aberto.

## Validar no QA-17 (QF-03, QF-07)

- Um jogo por loja (Steam, Epic, GOG), com `RunningAppID` real e com Epic aberto e fechado.
- Jogo com launcher de publicadora (Rockstar, EA app) e jogo com anti-cheat ou proteção de processo.
- CPU real durante a sessão, no Windows 10 22H2 da sala.
