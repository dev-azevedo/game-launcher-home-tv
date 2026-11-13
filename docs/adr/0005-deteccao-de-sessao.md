---
status: accepted (tolerâncias configuráveis; validar no QA final)
date: 2026-11-13
revised: 2026-11-13 (P-02 decidido; SEC-15/16/17 e AF-01 incorporados; rev. 4: D-35, sem spike com o usuário)
feature: launcher-tv
---

# Sessão de jogo: Job Object para jogo manual e GOG; varredura por pasta de instalação + sinal da loja para Steam e Epic

RN-10 define o fim da sessão como "nenhum processo associado ao jogo segue rodando". O Sofá controla o processo de jogos que ele mesmo cria, mas não o de jogos iniciados pela loja (R-02). Decidimos usar duas estratégias atrás de uma mesma interface (`SessionProbe`):

- **Jogo manual e GOG (exe direto)**: o Sofá cria o processo com `CreateProcessW` (sem shell, `bInheritHandles=FALSE`, SEC-10) **dentro de um Job Object** ligado a uma completion port. O Windows avisa quando o último processo do job termina (`JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO`), o que inclui os filhos, sem polling. Fechar à força usa `TerminateJobObject`, que só atinge processos da árvore criada pelo Sofá.
- **Steam e Epic**: varredura de processos a 1 Hz, **só durante a sessão** (snapshot ToolHelp + `QueryFullProcessImageNameW` com `PROCESS_QUERY_LIMITED_INFORMATION`). Um processo é associado ao jogo quando o executável canônico está sob a pasta de instalação, com comparação por componente. Ao entrar no conjunto, o processo tem handle aberto (`PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE`) e tempo de criação guardado (SEC-15). Na Steam, entra um sinal extra: `HKCU\Software\Valve\Steam\RunningAppID`.
- **Regras comuns (domínio puro)**:
  - Início confirmado no primeiro processo associado, com prazo de 60 s (CA-29).
  - Fim só após 5 s seguidos sem processo associado (E-14).
  - Fechar = `WM_CLOSE` nas janelas de topo visíveis da sessão. Após 10 s o Sofá oferece forçar (CA-35).
  - Forçar: duplicar o handle guardado com `PROCESS_TERMINATE` e conferir o tempo de criação antes de `TerminateProcess`. Nunca por PID solto (SEC-15).
- **Nunca entram no conjunto (SEC-17)**: o próprio Sofá e seus filhos WebView2; processos das lojas (`steam.exe`, `steamwebhelper.exe`, `EpicGamesLauncher.exe`, `EpicWebHelper.exe`, `GalaxyClient*.exe`, qualquer coisa sob a pasta da loja); `explorer.exe`; processos de outra sessão ou de outro usuário; processos protegidos. Sem `SeDebugPrivilege` e sem `PROCESS_ALL_ACCESS`.
- **Pasta de instalação válida (SEC-16)**: profundidade ≥ 2 abaixo da raiz e fora de pastas de sistema, de perfil, da loja e do Sofá. Pasta inválida faz o jogo ser ignorado na detecção.
- **GOG (decisão do usuário, P-02 a)**: lançar o executável do `playTask` primário de `goggame-<id>.info` (ou `exe`/`workingDir` do registro `GOG.com\Games\<id>`), confinado à pasta do jogo (SEC-12, SEC-14), **sem passar pelo GOG Galaxy**. O preço aceito: sem sincronização de saves em nuvem nem overlay do Galaxy.
- **Jogo que exige administrador (AF-01)**: `CreateProcessW` retorna 740 → estado `NeedsElevation` → se o usuário confirmar, `ShellExecuteExW` com verbo padrão (o UAC do Windows decide; nunca `runas`). Esse processo fica fora do Job (UIPI) e é acompanhado por varredura de pasta. Fechar ou voltar a ele pode falhar e mostra mensagem.

## Decisão sem spike com o usuário (rev. 4, D-35)

As estratégias acima são implementadas como estão, e os números da spec viram **padrões configuráveis em runtime** em `settings.json → advanced.session` (doc de arquitetura §9.3). A edição é manual, sem comando IPC, e cada valor tem faixa validada:

| Chave | Padrão (spec) | Faixa |
|---|---|---|
| `startTimeoutSeconds` | 60 (CA-29) | 30..300 |
| `endGraceSeconds` | 5 (E-14) | 2..30 |
| `closeTimeoutSeconds` | 10 (RN-12) | 5..60 |
| `probeIntervalMs` | 1000 | 500..5000 |
| `useSteamRunningAppId` | `true` | bool |

Com `diagnostics.level = "debug"`, cada sessão registra transições, o conjunto de processos (caminho da imagem do jogo) e os tempos, para ajustar os valores pelo log devolvido.

**Validar no QA final**: QF-03 (um jogo por loja + um com launcher de publicadora: sem falso fim, retorno ≤ 3 s + tolerância) e QF-07 (CPU). Jogos problemáticos entram na lista de R-03.

## Opções consideradas

- **WMI `Win32_ProcessStartTrace` / ETW**: exigem administrador. Descartada (RN-26, SEC-20).
- **Varredura por pasta para tudo**: o Job Object é exato e mais barato onde dá. Descartada como estratégia única.
- **GOG via Galaxy (`GalaxyClient.exe /command=runGame`)**: traz saves em nuvem, mas o processo intermediário tira o jogo do Job e acrescenta uma linha de comando montada com ID de terceiro. Descartada pelo usuário (P-02).
- **Associação por nome de processo**: mataria outro programa com o mesmo nome (SEC-15). Descartada.

## Consequências

- Launchers de publicadora fora da pasta (Rockstar, EA app) podem encerrar a sessão antes da hora se a troca demorar mais que 5 s (R-03, aceito; o QA lista os casos).
- Processos protegidos podem negar o handle. A associação usa o caminho do snapshot quando possível, mais o sinal da Steam, mas esses processos nunca são encerrados. O ST-02 (dev, processos de teste) cobre o caminho; o QA final (QF-03) confirma com jogos reais.
- Custo da varredura a 1 Hz medido no ST-02 com ~300 processos (meta < 0,2% de CPU, dentro de RN-21); conferido no QF-07.
