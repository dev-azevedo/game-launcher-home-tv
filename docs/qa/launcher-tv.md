# QA: launcher-tv (Sofá), rodada 1

| Campo | Valor |
|---|---|
| Etapa | 8, rodadas 1 e 2 (QA-01 a QA-12, parte do QA-15 e esboço do QA-16; a rodada 2 reteste as correções do ciclo 1 e a regressão está na seção 11) |
| Data | 2026-11-14 |
| Branch | `feature/launcher-tv-20261113` (commit base `85b3174`) |
| Máquina | Windows 11 Enterprise 22621, 1 tela, sem Steam, Epic, GOG, sem controle Xbox, sem dispositivo de áudio ativo, sem VM, sem `tauri-driver` |
| Veredito parcial | **Aprovado com ressalvas, sem veredito final (rodada 2).** QA-BUG-01, 02 e 03 fechados; 1 bug novo, baixo (QA-BUG-04). Nenhum CA falhou. 11 CA e boa parte de QA-08, QA-10 e QA-15 seguem dependendo de VM, instalador (BE-37 a BE-39) e do PC da sala |

Legenda de quem executa: **AUTO** = teste automatizado do repo; **QA-UI** = navegador (Playwright) contra o devCore; Q = `qa_adversarial.rs`;  **QA-VM** = VM Windows 10/11 (QA-08, QA-10, QA-15; ainda sem VM); **USER** = roteiro do usuário (`docs/qa/roteiro-usuario.md`, QA-16/QA-17).
Status: **passou**, **parcial** (a parte automatizável passou, falta a real), **não verificado** (precisa de VM, instalador ou hardware), **falhou**.

## 1. Execução desta rodada (evidência)

| Execução | Resultado |
|---|---|
| `scripts/ci.ps1 -SkipAudit` (toolchain `-gnu`) | **OK**: autoteste dos guards (33 violações plantadas detectadas), guards INV/SEC, `fmt`, `clippy -D warnings`, `cargo test --workspace`, bindings, `cargo deny`, UI `typecheck` + `lint` + `vitest`. `npm audit` não rodou (offline, `-SkipAudit`) |
| `cargo test --workspace` | 298 testes do `launcher-core` + 102 do `launcher-win` (2 ignorados) + 3 + 4 de sessão + 12 + 9 + 3 + 8 + 9 + 10 + 8 + 4 + 3 de `src-tauri`, todos verdes |
| `vitest` (UI) | 19 arquivos, **193 testes verdes** |
| `--features win-integration`: `purge_integration` (6), `library_integration` (3), `session_runtime` (12 + 3 ignorados), `session_e2e` (4 + 2 ignorados) | Verdes. Rodei também os ignorados `cpu_da_varredura_em_sessao` (**0,156% de 1 núcleo = 0,0098% do total**), `fechar_pelo_painel_pede_wm_close…`, `replace_com_jogo_que_ignora_wm_close…` e `wm_close_fecha_jogo_normal…`. Não rodei `escada_real_traz_a_janela…` (manda F24 sintético ao que estiver em foco na máquina do dev) nem `audio_integration` (volume real) |
| `--features win-integration`: `autostart_integration` (7) | **7/7 em paralelo na rodada 2 (3 execuções seguidas); na rodada 1 falhavam 7/7 por colisão de nomes (QA-BUG-03, corrigido). Depois das rodadas: nenhuma tarefa `Sofa*`, nenhum `Sofa*` em `HKCU\...\Run` nem em `StartupApproved` |
| Novos testes meus | `crates/launcher-core/tests/qa_adversarial.rs` (11, 1 ignorado = QA-BUG-01) e `crates/launcher-win/tests/qa_robustez.rs` (6, 1 ignorado = QA-BUG-02). Sem commit |
| Navegador (vite na porta 5179, encerrado ao fim) | Telas conferidas em 1920x1080 e 3840x2160 (texto ≥ 24 px equivalentes, nada de scroll horizontal, nome HTML literal); 375/768/1280 só para ver que nada quebra |
| Greps estáticos | Sem `innerHTML`/`dangerouslySetInnerHTML`/`eval`/`fetch`/`WebSocket`/`localStorage` no front (fora de testes); sem cliente HTTP nem plugin `shell/fs/http/opener/updater/dialog` na árvore do Cargo; sem `EWX_FORCE`, `shutdown.exe`, `SeDebugPrivilege`, `PROCESS_ALL_ACCESS`, hooks globais, `schtasks` no código; capability `main` só com os 30 comandos e `listen/unlisten`, sem `remote`; manifesto `asInvoker` `uiAccess=false` |
| Script coletor | `scripts/coletar-logs.ps1` rodado nesta máquina: zip com `logs\sofa.log`, `sistema.txt`, `sofa.txt`, `LEIA-ME.txt`; nome de usuário e do PC mascarados (0 ocorrências); integridade lida com a mesma rotina (Medium, não elevado) |

## 2. Bugs

| ID | Sev. | Onde | Passos | Esperado | Obtido | CA/RN/SEC | Responsável |
|---|---|---|---|---|---|---|---|
| QA-BUG-01 (**fechado r2**) | baixo | `crates/launcher-core/src/domain/library/display_name.rs:7-27` | `normalize_display_name("\u{200B}")` (também U+2060, U+FEFF, U+3164, U+2800, U+00AD, U+115F). Teste: `qa_adversarial.rs::ca61_nome_so_com_invisiveis_vira_vazio` (ignorado) | "Nome vazio não é aceito": nome só com caracteres invisíveis vira vazio e `EmptyName` | Vira nome válido: o card fica com título em branco; o mesmo vale para nome de manifesto de loja | CA-61, SEC-07 (bloqueio de bidi está completo, falta o resto dos invisíveis) | dev-backend |
| QA-BUG-02 (**fechado r2**) | médio | `crates/launcher-win/src/persistence/json.rs:210-218` (ramo `Err` de `load_file`) e `crates/launcher-core/src/application/library_service.rs:228,297` (ler-alterar-gravar com `load_stored()`) | Dados em `library.json`; leitura falha de forma transitória (antivírus, backup ou indexador abrem sem compartilhar leitura); ocultar, renomear ou iniciar um jogo. Teste: `qa_robustez.rs::leitura_transitoriamente_travada_nao_pode_levar_a_perda_dos_dados_no_save_seguinte` (ignorado) | Falha de leitura não pode levar à perda: ou o save recusa, ou o conteúdo antigo vai para backup | `load` devolve a biblioteca vazia **sem `recovery`** e sem tocar o arquivo; a mutação seguinte lê vazio e grava `library.json` só com a mudança nova: jogos manuais e ocultos somem, sem backup | CA-25, RN-18, seção 11 da spec | dev-backend |
| QA-BUG-03 (**fechado r2**) | baixo | `crates/launcher-win/tests/autostart_integration.rs` (nomes fixos `Sofa QA Autostart (<SID>)` e `SofaQaTest`) e `docs/api/launcher-tv.md` seção 10 ("Como testar em desenvolvimento") | `cargo test -p launcher-win --features win-integration --test autostart_integration` | 7 verdes | 7/7 vermelhos em paralelo (testes pisam na tarefa e no valor `Run` um do outro). Verdes com `-- --test-threads=1`. Risco de resíduo de teste na tarefa e no `Run` do dev | CI/QA-10 (SEC-22) | dev-backend (nome único por teste, ou `serial_test`, ou documentar `--test-threads=1`) |

Sem bug aberto de front. Observações abaixo não são bug.

## 3. Dúvidas para o PO, o arquiteto e o designer (nenhum item aprovado sobre elas)

| ID | Dúvida | Onde |
|---|---|---|
| D-QA-01 (**fechada: D-48**) | **Ordem das opções do jogo.** CA-79 lista "Jogar, Renomear, Ocultar e, se alterado, Restaurar nome original". A UI e o microcopy (`microcopy.md:53`) mostram "Jogar, Renomear, Restaurar nome original, Ocultar". A spec manda; falta decidir qual dos dois muda | CA-79, RN-33 |
| D-QA-02 (**fechada: D-45, CA-80**) | **Teclado de emergência não tem View, LT, RT, LB nem RB** (RN-07 só lista setas, Enter, Esc e F1/tecla de aplicativo). Sem controle, o usuário não abre opções do jogo, então não renomeia nem oculta, e não muda volume pelo teclado fora do menu rápido. CA-61 diz "View, escolhe Renomear, digita no teclado": exige controle e teclado. Está conforme a spec; confirmar que é isso mesmo | RN-07, CA-09, CA-13, CA-61 |
| D-QA-03 (**aberta, só documentação**) | **Padrão dos degraus 2 e 3.** `qa.md` (QA-06 "2 e 3 ligados por configuração", QA-16 "perfil de diagnóstico que liga os degraus 2 e 3") está desatualizado: pelo SG-02 rev. 5, `arch 9.3` e `advanced.rs:59-60` os dois vêm **ligados**, ordem 1, 3, 2, 4. O roteiro já parte disso e traz o passo para testar "só 1 e 4" (bloco 12). Nenhum perfil novo é necessário em BE-39/QA-16 | RN-24, SEC-28 |
| D-QA-04 (**fechada: D-46, RN-32**) | **Segunda linha da biblioteca sob o rodapé.** Na tela inicial, os cards da fileira "Biblioteca" aparecem cortados pelo rodapé de dicas (uns 40 px, sem texto legível, y 932 a ~970 em 1080p), abaixo da área segura de 5% (y 1026). RN-32 só prevê corte horizontal. Provável intenção do desenho; confirmar com o designer | CA-54, RN-32 |
| D-QA-05 (**fechada: correção do front, RN-32**) | **Anel de foco fora da área segura.** O anel do item da lista de configurações começa em x≈75 (margem de 5% = 96 px). O elemento está dentro da área, só o anel (sombra) sai. Aceitar? | CA-54 |
| D-QA-06 (**fechada: texto novo do herói**) | `Em andamento. A volta ao jogo.` (herói com jogo aberto, `microcopy.md:20`) lê-se estranho. Conforme o microcopy; sugestão ao designer revisar | CA-36 |

## 4. Plano de testes e resultado por CA (QA-01)

Cobertura por teste existente: `ui/src/**/*.test.*` (U), `crates/launcher-core` (C), `crates/launcher-win` (W), `src-tauri/tests` (T), navegador (N). Casos de USER são os blocos do `roteiro-usuario.md`.

### Inicialização, tela cheia e instância

| ID | Passos e esperado | Quem | Status e evidência |
|---|---|---|---|
| CA-01 | Instalado com autostart ligado, logar: tela cheia sem borda no monitor do launcher, navegável ≤ 5 s | QA-VM, USER (bl. 2) | não verificado. `BE-40` registra `primeiro_paint` e `navegavel`; medir na VM e no PC da sala |
| CA-02 | Abrir o app de novo (atalho, Iniciar): nenhuma 2ª janela, a existente vem à frente | AUTO, USER (1.7) | **passou**. T `single_instance::callback_descarta_argv_e_cwd`; BE-35 conferiu `calc.exe --launch x` no binário real. Falta o app instalado |
| CA-03 | Tela cheia sem barra de título e de tarefas; cursor some após 3 s | AUTO, USER (8.4) | parcial. U `CursorAutoHide`; janela real sem borda não verificada |
| CA-04 | Desligar autostart, reiniciar: não abre; religar: abre | AUTO, QA-VM, USER (2.3, 2.4) | parcial. W `autostart_integration` 7/7 (paralelo na r2); reinício real pendente |
| CA-05 | Sair do launcher confirma e fecha; com jogo ativo, avisa que continua aberto | AUTO, USER | parcial. U `quickmenu` "Sair do launcher… app_exit; o jogo segue aberto"; fechamento real pendente |

### Navegação por controle

| ID | Passos e esperado | Quem | Status e evidência |
|---|---|---|---|
| CA-06 | D-pad e analógico movem o foco, visível a 3 m | AUTO, USER (3.1) | parcial. C `engine::analogico_acima_da_zona_morta…`, `dpad_e_botoes_viram_intencoes…`; N foco visível; controle físico pendente (sem Xbox na máquina) |
| CA-07 | Segurar direção: repete após 400 ms em intervalo regular | AUTO | **passou**. C `engine::segurar_direcao_repete_depois_de_400_ms…`, `leitura_atrasada_nao_gera_rajada…`; U `dispatch` |
| CA-08 | Analógico em repouso com drift não move | AUTO, USER (3.3) | **passou** (domínio). C `drift_leve_nao_move_o_foco`, `limite_exato_da_zona_morta`, `histerese…`; drift real do controle fica no USER |
| CA-09 | Toda ação só com o controle (exceto RN-07/RN-19/renomear) | AUTO, QA-UI | **passou**. U `intentMap` por tela; N Opções, ocultar, volume, energia e configurações alcançados por intenções. Ver D-QA-02 sobre o teclado |
| CA-10 | B volta ou fecha sem executar; na inicial, sem efeito | AUTO | **passou**. U `intentMap::as_demais_telas_voltam_com_B`, `a_Home_ignora_B`, `dispatch` |
| CA-11 | Vários controles respondem igual | AUTO, USER (3.4) | **passou** (domínio). C `varios_controles_valem_como_um_so`, `combinacao_vale_com_view_num_controle_e_menu_em_outro` |
| CA-12 | Launcher em segundo plano ignora botões, salvo a combinação | AUTO | **passou**. C `em_segundo_plano_nenhuma_intencao_sai`; T/W `session_runtime` |
| CA-13 | Sem controle: biblioteca + faixa "Nenhum controle conectado. Ligue o controle ou conecte o cabo." | AUTO, QA-UI | **passou**. U `input`; N `#ctrl=none` mostra a frase exata |
| CA-14 | Controle desconecta: aviso ≤ 2 s, foco mantido | AUTO, QA-UI, USER (3.6) | parcial. N `#ctrl=lost`: aviso no mesmo instante do evento; núcleo lê slot vazio a cada 1 s (BE-14); medição com controle real pendente |
| CA-15 | Controle volta: aviso some ≤ 2 s e funciona | AUTO, QA-UI, USER (3.7) | parcial. N "Controle conectado." some 1,5 s depois; real pendente |

### Biblioteca

| ID | Passos e esperado | Quem | Status e evidência |
|---|---|---|---|
| CA-16 | Steam com jogos em qualquer pasta de biblioteca: nome e capa locais | AUTO, USER (4.1) | parcial. W `stores::steam` (fixtures SP-08, 2 bibliotecas, 2 layouts de capa), 300 jogos em ~0,2 s; **sem Steam na máquina**: loja real fica no USER |
| CA-17 | Jogo desinstalado some na atualização | AUTO | parcial. C `jogo_desinstalado_some_na_proxima_atualizacao`; real no USER |
| CA-18 | Sem Steam: sem erro, só as outras origens | AUTO | **passou**. C/W `epic_ausente…`, `gog_ausente…`, `refresh_e_serializado_loja_com_defeito_nao_derruba_as_outras` |
| CA-19 | Adicionar jogo: navegador só mostra discos locais, pastas e `.exe`; nome do arquivo, ícone como capa; sem campo de argumentos | AUTO, QA-UI | **passou**. W `folder_browser`, `manual_exe`; U `folders`; N lista com Drive/Dir/Exe. Ícone no card real: USER (4.7) |
| CA-20 | Duplicata: "Este jogo já está na biblioteca" | AUTO | **passou**. C `cadastro_manual_canonicaliza_deduplica…`; U `folders` (Duplicate) |
| CA-21 | View > Ocultar > confirmar: sai, nada apagado, não volta | AUTO | **passou**. U `options` (Cancelar em foco); C `oculto_nao_volta_apos_refresh…`; T `renomear_ocultar_restaurar_pela_ipc` |
| CA-22 | Ordem: recentes primeiro, nunca jogados A a Z; foco no último | AUTO | **passou**. C `ordem_recentes_primeiro_depois_a_z…`; U `library` |
| CA-23 | Biblioteca vazia: mensagem, "Adicionar jogo" em foco | AUTO, QA-UI | **passou**. U `HomeScreen vazia`; N `#lib=vazia` |
| CA-24 | Sem capa: placeholder com nome legível | AUTO, QA-UI | **passou**. U `CoverImage`; N placeholders a 24+ px equivalentes |
| CA-25 | Fechar e abrir: manuais, ocultos, nomes, datas iguais; leitura que falha não pode apagar dados | AUTO | **passou** (r2). W `persistence`, `qa_robustez::leitura_transitoriamente_travada…` e `trava_curta_de_leitura_e_superada…` (QA-BUG-02 corrigido) |

### Iniciar jogo e sessão

| ID | Passos e esperado | Quem | Status e evidência |
|---|---|---|---|
| CA-26 | A: "Abrindo <nome>…" com Voltar; Steam/Epic pela loja; GOG direto pelo `.exe` | AUTO, USER | **passou** (núcleo e UI). U `launch`; C `launch::steam_usa_o_template_fixo`; W `session_runtime`. Loja real e GOG sem Galaxy: USER |
| CA-27 | Janela do jogo aparece: launcher sai da frente | AUTO, USER | parcial. W `sessao_completa_inicia_marca_jogado_esconde…` com jogo sintético; jogos reais no USER |
| CA-28 | Exe sumiu: "Não encontramos o arquivo de <nome>", Ocultar e Voltar, sem caminho | AUTO | **passou**. U `launch` GameFileMissing; W `arquivo_ausente_nao_deixa_sessao_nem_processo`; `errors.test` (nome, nunca caminho) |
| CA-29 | 60 s sem processo: volta com "Não foi possível confirmar…", Tentar de novo e Voltar; se o jogo abrir depois, sessão normal (CA-27, RN-05, retorno) | AUTO | **passou** (r2, texto novo). C `jogo_que_abre_depois_dos_60_s_segue_o_fluxo_normal`, `ca29_sessenta_segundos…`, `retry_so_em_not_confirmed`; commit 4368015 traz o Sofá à frente em `NotConfirmed`; U `launch` |
| CA-30 | Fim pelo menu do jogo: launcher volta em ≤ 3 s com o jogo em foco | AUTO, USER (bl. 5) | **passou** (sintético). C `fim_pelo_menu_do_jogo_traz_o_sofa_apos_5_s`, `ca30_cinco_segundos…`; W sessão com jogo sintético. Jogo real e degrau usado: USER |
| CA-31 | A em outro jogo: pergunta com Cancelar em foco | AUTO | **passou**. U `launch` troca de jogo; C `replace_*` |
| CA-32 | View+Menu 1,5 s: launcher à frente ≤ 3 s com painel | AUTO, USER (bl. 5) | parcial. C `combinacao_de_retorno…`; W escada real ~0,2 s em janela sintética (BE-21); WebView2 e jogo real, tela cheia exclusiva: USER |
| CA-33 | Combinação curta não faz nada | AUTO | **passou**. C `ca33_apertar_menos_que_o_tempo…`, `soltar_qualquer_um_antes…` |
| CA-34 | Voltar ao jogo (ou B): jogo à frente como estava | AUTO | **passou**. U `panel`; C `voltar_ao_jogo_*` |
| CA-35 | Fechar jogo: "Fechando…", WM_CLOSE; após 10 s oferece forçar, Cancelar em foco | AUTO | **passou**. W `fechar_pelo_painel_pede_wm_close…` e `replace_com_jogo_que_ignora_wm_close…` (rodados); C `ca35_dez_segundos…`; U `panel` |
| CA-36 | Ir para a biblioteca: jogo aberto, "Em andamento", A volta | AUTO, QA-UI | **passou**. U `panel`, `library`; N selo "Em andamento" |
| CA-37 | Menu rápido no painel igual à biblioteca | AUTO | **passou**. U `intentMap` (no painel Menu abre o menu rápido), `quickmenu` |

### Voltar ao launcher durante o jogo, volume e energia

| ID | Passos e esperado | Quem | Status e evidência |
|---|---|---|---|
| CA-38 | Volume: ±5 por toque, 0 a 100, número em ≤ 200 ms | AUTO, USER (6.1) | parcial. C `volume::regras_por_tabela`, `nunca_sai_de_0_a_100…`; U `quickmenu`; N HUD instantâneo. **Sem dispositivo de áudio na máquina**: o teste de 200 ms (`audio_integration`) cai no ramo `NoAudioDevice`; tempo real no USER |
| CA-39 | Mudo liga e desliga, volta ao nível | AUTO | **passou** (domínio e UI). U `Mudo liga e desliga pelo núcleo`; C regras |
| CA-40 | No mudo, aumentar tira o mudo e sobe 1 passo | AUTO | **passou**. C `regras_por_tabela` (RT no mudo) |
| CA-41 | Mudança externa: menu mostra o nível atual | AUTO, USER (6.3) | parcial. U (nível vem de `volume://changed`); callback do Core Audio com dispositivo real pendente |
| CA-42 | Sem dispositivo: "Nenhum dispositivo de áudio encontrado", botões desabilitados | AUTO | **passou**. U `quickmenu` sem dispositivo; W `audio_integration` ramo sem dispositivo (esta máquina) |
| CA-43 | Desligar/Reiniciar/Suspender: confirmação com Cancelar em foco, B cancela | AUTO | **passou**. U `quickmenu`, `ConfirmDialog` |
| CA-44 | Confirmar Desligar: em ≤ 3 s o mesmo desligamento do menu Iniciar, normal; com inicialização rápida ligada, ela é usada | AUTO, QA-VM, USER (7.7) | não verificado (r2). W `power::tests::flags_de_desligar_e_reiniciar_nunca_forcam`, `inicializacao_rapida_so_com_hiberboot_ligado_e_hibernacao_disponivel`: `EWX_HYBRID_SHUTDOWN\|EWX_SHUTDOWN\|EWX_POWEROFF` com fallback único para `EWX_POWEROFF`, sem bit de forçar. Execução real não feita (restrição) |
| CA-45 | Confirmar Reiniciar: reinício completo, sem inicialização rápida; autostart reabre o Sofá | QA-VM, USER (7.4) | não verificado (r2). W `EWX_REBOOT` sem híbrido (teste de flags); real pendente |
| CA-46 | Suspender e acordar: mesma tela, controle reconecta | QA-VM, USER (7.5) | não verificado. U `system.test` (volta da suspensão recarrega sem trocar de tela) cobre só a UI |
| CA-47 | Suspensão indisponível: "Suspensão indisponível neste PC" | AUTO | **passou** (UI). U `quickmenu::suspensao_indisponivel…`; núcleo `GetPwrCapabilities`/`IsPwrSuspendAllowed` sem teste real. Esta máquina oferece S3, o PC da sala: USER (0 e 7.6) |
| CA-48 | Energia com jogo ativo: avisa nome e progresso | AUTO | **passou**. U `com jogo aberto a confirmação avisa o nome` |
| CA-49 | Windows bloqueia o desligamento: launcher não força | QA-VM, USER (7.2) | não verificado (Bloco de Notas não salvo). Grep sem `EWX_FORCE*` passou |

### Instalador, interface de 10 pés e demais

| ID | Passos e esperado | Quem | Status e evidência |
|---|---|---|---|
| CA-50 | Instalador: UAC, `Program Files`, WebView2 baixado se faltar, mensagem sem internet | QA-VM, USER (bl. 1) | não verificado. **BE-37 a BE-39 a fazer** |
| CA-51 | Atalho no Iniciar, "Iniciar com o Windows" marcada, vale para o usuário que executou | QA-VM, USER | não verificado (idem) |
| CA-52 | Desinstalar remove app, atalhos e autostart; jogos intactos | QA-VM, QA-15, USER (bl. 13) | não verificado (idem) |
| CA-53 | Atualizar por cima mantém biblioteca, configurações e autostart | QA-VM, USER | não verificado (idem) |
| CA-54 | 1080p e 4K: tudo na área segura de 5%, texto ≥ 24 px equiv.; exceção: card cortado na lateral ou embaixo; texto e destaque de foco dentro | QA-UI, QA-VM, USER (bl. 8) | parcial (r2). N: anel de foco da lista de Configurações agora com borda a 103 px do canto (margem de 96 px); texto ≥ 24 px (1080p e 4K, rodada 1); escalas de Windows e TV real pendentes |
| CA-55 | Sempre exatamente um item em foco, visível | AUTO, QA-UI | **passou**. U `library` "um único foco", `AllGamesScreen`; N anel laranja em todas as telas vistas. Visibilidade a 3 m: USER |
| CA-56 | Epic: nome e capa locais; senão ícone ou placeholder | AUTO, USER (4.2) | parcial. W `stores::epic` (fixtures); sem Epic na máquina |
| CA-57 | GOG idem | AUTO, USER | parcial. W `stores::gog` (registro + `.info`); sem GOG na máquina |
| CA-58 | Jogo desinstalado na Epic ou GOG some | AUTO | parcial. C `jogo_desinstalado_some…` (genérico); real no USER |
| CA-59 | Epic ou GOG ausente: sem erro | AUTO | **passou**. W `epic_ausente_e_not_installed_sem_erro`, `gog_ausente…` |
| CA-60 | Card mostra a origem de forma discreta | AUTO, QA-UI | **passou**. U `origem e andamento`; N selos Steam, Epic, GOG, Manual |
| CA-61 | View ou F2, Renomear, digitar, Enter; Esc/B cancelam; vazio não aceito | AUTO, QA-UI | **passou** (r2). U `options`; N F2 abre opções, Backspace edita o texto (não volta), Cancelar à esquerda e Salvar à direita; Q `ca61_nome_so_com_invisiveis_vira_vazio` e `ca61_mais_invisiveis…` (QA-BUG-01 corrigido; QA-BUG-04 residual) |
| CA-62 | Nome mantido ao atualizar; "Restaurar nome original" | AUTO | **passou**. C `renomear_normaliza_restaura_e_valida`, `nome_personalizado_vence`; U `options` |
| CA-63 | Retorno com fallback em ≤ 3 s; minimizar é o último; "Voltar ao jogo" restaura | AUTO | **passou** (sintético). C `foreground::*` (ordem 1, 3, 2, 4; tudo falha em orçamento < 3 s); W escada real. Anti-cheat e tela cheia exclusiva: USER |
| CA-64 | LT/RT: ±5 por puxada, HUD ≤ 200 ms, some em 2 s, LT no mudo mantém o mudo | AUTO, QA-UI | **passou**. U `VolumeHud e LT/RT`; N HUD aparece e some em 2 s |
| CA-65 | LT/RT sem efeito no menu, diálogo, painel e em segundo plano | AUTO, QA-UI | **passou**. U `ignorados no menu rápido, em diálogos e no painel`; N RT no menu não mexeu (50 → 50); C `so_em_primeiro_plano_o_gatilho_aciona_volume` |
| CA-66 | Escolher outro monitor: passa na hora e persiste | AUTO, QA-VM, USER (8.5) | parcial. C/T `display` com topologia falsa; **uma tela na máquina**; U `settings`; reais no USER |
| CA-67 | Monitor ausente: cai no principal, preferência mantida | AUTO, USER (8.7) | parcial. C `display`, T `rajada_de_display_changed…`; real no USER |
| CA-68 | Desinstalador pergunta "Manter dados"; desmarcado apaga só a pasta do Sofá, sem seguir links, sem administrador | AUTO, QA-15, USER | parcial (r2). W `purge_integration` 6/6; `purge::sofa_aberto_recusa_com_codigo_4_e_nao_apaga_nada` e `app_is_open_enxerga_o_mutex…` (código 4 com o Sofá aberto); diálogo, `RunAsUser` e a remoção como administrador só de `library.json` e `settings.json` (RN-18): BE-38 a fazer |
| CA-69 | Itens fixos da Steam não aparecem | AUTO | **passou**. C `steam_non_games`, `steam_filtra_nao_jogos…` |
| CA-70 | Itens ocultos: lista, A restaura com nome e data; vazio "Nenhum item oculto" | AUTO | **passou**. U `settings` Itens ocultos (3 casos); C `unhide_preserva…` |
| CA-71 | Pasta de rede não aparece; `PathNotAllowed` no cadastro, `NetworkPathBlocked` ao iniciar (Ocultar e Voltar, foco em Voltar) | AUTO, QA-UI | **passou**. U `errors`, `launch`, `folders`; W `manual_editado_a_mao_para_script_ou_rede_nao_inicia`; Q `qa_adversarial` (15 formas de caminho). Unidade de rede mapeada real: não testável aqui, resolve para UNC (BE-24) |
| CA-72 | `.exe` inválido: "Este arquivo não é um executável válido" | AUTO | **passou**. W `cadastro_manual_no_disco_recusa_cada_caso…` (MZ, PE); U `folders` |
| CA-73 | Jogo que exige admin: aviso antes do UAC (Continuar, Voltar), depois aviso por trás do UAC; recusa: "O Windows não deu permissão…" | AUTO, USER (5.10) | **passou** (sintético). W `exe_com_require_administrator_devolve_needs_elevation…`; U `launch` (2 etapas) |
| CA-74 | Uso diário sem UAC | QA-VM, USER (11.1) | não verificado. Manifesto `asInvoker` (T `manifesto_embutido_no_exe_e_as_invoker`); ausência de UAC no uso real depende do instalado |
| CA-75 | Sessão confirmada e queda: jogo no topo dos recentes | AUTO | **passou**. C `iniciar_ate_running_marca_jogado…`; W `dismiss_em_launching_continua…_com_mark_played` |
| CA-76 | SHA-256 do instalador publicado e confere | AUTO, USER (1.1) | não verificado (BE-39 a fazer) |
| CA-77 | "Abrindo…" com Voltar em foco, A ignorado por 600 ms, sair não cancela o início | AUTO, QA-UI | **passou**. U `WaitScreen` (A antes de 600 ms ignorado), `launch`; N segundo A em 80 ms ignorado; W `dismiss_em_launching…` |
| CA-78 | "Fechando…" com "Ir para a biblioteca", A ignorado 600 ms, fechamento continua | AUTO | **passou**. U `panel` ("A em Fechando nos primeiros 600 ms", "B em Fechando…"); C `dismiss_em_closing…` |
| CA-79 | View: Jogar (em foco), Renomear, Restaurar nome original (se alterado), Ocultar; B fecha; sem tela de detalhe | AUTO, QA-UI | **passou** (r2, ordem decidida em D-48). U `options`; N ordem vista: Jogar, Renomear, Restaurar nome original, Ocultar |
| CA-80 | Só com teclado: foco, confirmar, voltar, menu rápido, opções, abas/páginas, volume, painel; nada traz o launcher à frente durante o jogo | AUTO, QA-UI | **passou** (r2). U `keyboard`; N F2 (opções), F1 (menu), PgDn/PgUp (a linha pula e volta), `]` (volume 45 para 50, HUD), `[`/`]` ignorados no menu rápido, Backspace e Esc voltam, F9 (só do devCore) abre o painel e F2 com jogo aberto abre as opções. Combinação de retorno sem tecla: por desenho (RN-07) |
| CA-81 | Diálogo com mais de um botão: Cancelar/Voltar à esquerda, ação principal à direita | AUTO, QA-UI | **passou** (r2). N: Ocultar (Cancelar@880, Ocultar@1172), Renomear (Cancelar@888, Salvar@1172), Desligar, Fechar jogo (Cancelar@871, Fechar jogo@1163) e Forçar fechamento (Cancelar@764, Forçar@1056); foco inicial em Cancelar no Desligar (print `r2-desligar`) |

**Resumo de CA (81, rodada 2, com CA-80 e CA-81 novos):** passou **51** · parcial **19** · não verificado **11** (CA-01, 44, 45, 46, 49, 50, 51, 52, 53, 74, 76) · falhou **0**.

## 5. Regras de negócio (RN)

| ID | Caso e esperado | Status e evidência |
|---|---|---|
| RN-01 | Sem rede: nenhum cliente HTTP, CSP sem origem externa | **passou**. `cargo tree` vazio de http, `cargo deny`, T `csp`. Rede zero 10 min no Resource Monitor: não verificado |
| RN-02 | 4 origens, só instalados | **passou**. C `model`, `detect` |
| RN-03 | Unicidade por origem e ID | **passou**. C `mesmo_jogo_em_duas_pastas…`, `steam_e_epic_do_mesmo_jogo_sao_dois_itens` |
| RN-04 | Ocultar nunca apaga; restaura nas configurações | **passou**. C, U `options`/`settings`; grep sem remoção de arquivo fora da pasta de dados |
| RN-05 | Ordem recentes, depois A a Z sem acento | **passou**. C `ordem_recentes_primeiro_depois_a_z_sem_acento_nem_caixa` |
| RN-06 | Zona morta 25% (ajustável 10 a 50) | **passou** (domínio). C `zona_morta_configuravel_e_limitada_a_10_50`; valor final com controle real: USER |
| RN-07 | Teclado cobre todas as ações do controle, menos a combinação de retorno | **passou** (r2). U `keyboard`; N (ver CA-80) |
| RN-08 | Até 4 controles, mesmo usuário | **passou**. C `varios_controles…` |
| RN-09 | Uma sessão ativa | **passou**. C `launch_com_sessao_em_qualquer_estado_nao_idle…`; W 20 `session_launch` seguidos |
| RN-10 | Processos do jogo por job (manual e GOG) e por pasta (Steam e Epic) | parcial. W `process_probe`, `varredura_*`; loja real: USER |
| RN-11 | Combinação 1,5 s só com sessão | **passou**. C `combo::dispara_so_aos_1500_ms_cravados_e_uma_vez` |
| RN-12 | Fechar em 2 etapas, forçar com nova confirmação | **passou**. C, W, U |
| RN-13 | Volume mestre do dispositivo padrão, passo 5 | parcial. Domínio passou; dispositivo real pendente |
| RN-14 | Ações destrutivas com Cancelar em foco | **passou**. U `ConfirmDialog`, `options`, `quickmenu`, `panel` |
| RN-15 | Energia sem forçar; Desligar igual ao menu Iniciar (híbrido se ligado); Reiniciar completo | parcial (r2). W `power::tests` (flags, inicialização rápida); greps limpos; execução real pendente (CA-44, CA-45, CA-49) |
| RN-16 | Uma instância por usuário | **passou**. T `single_instance` |
| RN-17 | Autostart por usuário e sem elevação | parcial. W `autostart_integration` 7/7 (paralelo na r2) (tarefa `LeastPrivilege`, `InteractiveToken`, SID, fallback Run); VM pendente |
| RN-18 | Dados em `%LOCALAPPDATA%\app.sofa.launcher`; remoção sem administrador, só `library.json` e `settings.json` como exceção | parcial (r2). W `persistence`, `purge_integration`; QA-BUG-02 corrigido; instalador (BE-38) pendente |
| RN-19 | Orientar logon sem teclado | não verificado (texto do instalador, BE-37) |
| RN-20 | Tela cheia no monitor do launcher | parcial. C/T com topologia falsa |
| RN-21 | < 1% de CPU em segundo plano | parcial. W varredura 0,0098% do total (medido); `system.test` pausa animações; processo inteiro com a UI real: USER (5.12) |
| RN-22 | Nome personalizado prevalece | **passou**. C |
| RN-23 | Gatilho conta uma vez, segurar não repete | **passou**. C `gatilho_conta_uma_vez…`, `tremor_do_gatilho…` |
| RN-24 | Escada 1, 3, 2, 4, configurável, nunca pausa | **passou** (domínio e escada real sintética). C `foreground::*`; jogos reais: USER |
| RN-25 | Mesmo jogo em lojas diferentes são itens separados | **passou**. C |
| RN-26 | Instalar, atualizar, desinstalar com UAC; uso diário sem | não verificado (BE-37 a BE-39) |
| RN-27 | Launcher nunca elevado, UAC só do jogo | **passou** (T manifesto; W `NeedsElevation`). Integridade Medium real: QA-10, USER |
| RN-28 | Lista fixa de não-jogos da Steam | **passou**. C `steam_non_games` |
| RN-29 | Manual só `.exe` local, sem argumentos | **passou**. C, W, Q (`qa_adversarial`) |
| RN-30 | SHA-256 publicado | não verificado (BE-39) |
| RN-31 | Telas de espera: botão focável, A ignorado 600 ms | **passou**. U `WaitScreen` |
| RN-32 | Única exceção à área segura: card cortado na lateral ou embaixo; texto e foco do item em foco dentro | **passou** (r2). N: anel de foco da lista de Configurações e do navegador de pastas dentro da margem (borda do anel a 103 px, margem de 96 px); segunda fileira sob o rodapé aceita (D-46) |
| RN-33 | Opções: Jogar, Renomear, Restaurar nome original, Ocultar | **passou** (r2). U `options`, N |
| RN-34 | Cancelar/Voltar sempre à esquerda, ação principal à direita; não muda o foco padrão | **passou** (r2). N (ver CA-81) |

## 6. Segurança funcional (SEC)

Execução dos "Teste:" da seção 6 de `docs/security/launcher-tv.md`. Revisão de código fica com o agente `seguranca`.

| ID | Teste executado | Status e evidência |
|---|---|---|
| SEC-01 | CSP de release sem `unsafe-eval/inline` nem origem externa; `eval` e `<script>` inline no release | parcial. T `csp_de_release_do_projeto_esta_limpa`, `verificador_reprova_csps_ruins`; teste no release real: QA-02 bloqueado (sem `tauri-driver`) |
| SEC-02 | Navegação travada; F12 e `location.href` externo | parcial. T `release_so_navega_para_a_origem_do_app`, `release_sem_console_e_sem_devtools_no_tauri`; no release real: bloqueado |
| SEC-03 | Lista de handlers vs lista aprovada; tipo errado e campo extra rejeitados | **passou**. T `generate_handler_registra_exatamente_a_lista`, `todo_comando_rejeita_campo_extra…`, `comandos_com_argumentos_rejeitam_tipo_errado…` |
| SEC-04 | Capabilities e Cargo sem `shell/fs/http/opener` | **passou**. T `capability_nao_libera_nada_proibido`, `tauri_sem_features_de_devtools_nem_plugins`; revisão do `main.json` |
| SEC-05 | Uma capability, sem `remote`, `withGlobalTauri` desligado | **passou**. T `conf_do_tauri_nao_expoe_global…`, `capability_main_so_tem_allow_dos_30_comandos_e_listen` |
| SEC-06 | `game_launch` 20 vezes seguidas: no máximo 1 processo | **passou**. T/W `vinte_session_launch_seguidos_criam_no_maximo_um_processo`; `Busy` por `SerialGate` |
| SEC-07 | Nome `<img onerror>` e `\u202E` literais | **passou** (r2: QA-BUG-01 fechado). N `#lib=longo`: 0 `<img>` injetado, texto literal; U em 5 telas; C `display_name`; Q `qa_adversarial`. 
| SEC-08 | `cover://..`, id inexistente: 404; imagem gigante vira placeholder | **passou**. T `cover_so_serve_game_id_existente_e_nunca_arquivo`, `nenhum_asset_protocol_habilitado`; W `cover_cache` (20 MB, 8192x8192 no cabeçalho). Capas reais no WebView2: USER (4.7, 4.8) |
| SEC-09 | `.lnk .bat .cmd .ps1 .txt`, UNC, ADS, nomes reservados, mapeada | **passou**. Q 33 formas hostis; W integração em disco; T `cadastro_manual_recusa…`. Unidade mapeada real: não testável aqui |
| SEC-10 | `CreateProcessW` sem shell; pasta `A & B` com aspas | **passou**. W `exe_em_pasta_com_e_comercial_e_aspas_inicia_o_arquivo_certo_e_o_pai_e_o_sofa`; exceção AF-01 em `uac_fora_de_needs_elevation…` |
| SEC-11 | `appid=10//-console`, `AppName=x?action=foo&y`, `a b` | **passou**. Q `sec11_*`; C `steam_rejeita_appid…`, `epic_rejeita_ids…` |
| SEC-12 | `..\..\Windows`, `C:\Windows\System32`, link para fora | **passou**. Q `sec12_*`; C `steam_junction_para_fora…`, `epic_link_que_escapa…` |
| SEC-13 | VDF de 10 MB, 1000 níveis, JSON truncado | **passou**. W `stores::steam` (4 MB, profundidade 32 antes do parser); `qa_robustez` com 12 formas de arquivo corrompido e JSON com 100.000 níveis: sem pânico. SQLite não é usado |
| SEC-14 | `playTasks=start.bat` e `cmd.exe` | **passou**. Q `sec14_*`; C `gog_rejeita_script_cmd_ads_unc_lnk…` |
| SEC-15 | Homônimo fora da pasta; PID reaproveitado | **passou**. W `varredura_por_pasta_ignora_homonimo…`, `pid_reaproveitado_nao_e_encerrado_mas_o_mesmo_processo_e` |
| SEC-16 | `InstallLocation` = `C:\`, `C:\Program Files`, pasta da Steam | **passou**. Q `sec16_*`; C `install_path_policy` |
| SEC-17 | Forçar mantém a Steam; sem `SeDebugPrivilege`/`PROCESS_ALL_ACCESS` | **passou**. W `o_proprio_sofa_e_o_webview2…`, `varredura_exclui_processos_do_cliente_da_loja`; grep vazio. Steam real: USER |
| SEC-18 | Biblioteca editada para `script.bat` ou UNC; matar durante a gravação | **passou** (r2). W `manual_editado_a_mao…`, `gravacao_interrompida_nao_corrompe…`, `qa_robustez` (QA-BUG-02 corrigido) |
| SEC-19 | `launcher.exe C:\Windows\System32\calc.exe --launch x` só foca | **passou**. T `callback_descarta_argv_e_cwd`; BE-35 com binário real |
| SEC-20 | Manifesto `asInvoker`; integridade Medium; `icacls` | parcial. T manifesto embutido no exe; integridade e `icacls` no instalado: QA-10 |
| SEC-21 | Instalar com conta padrão e outro admin: app Medium, autostart do padrão | não verificado (BE-37 a BE-39; roteiro bl. 1 e 11). W `padrao_do_instalador_com_adaptadores_reais`: valor inválido de `InstallDefaults` = desligado |
| SEC-22 | XML da tarefa, forjada, Run, fora de `Program Files`, fallback | parcial. W 7 testes (paralelo na r2) cobrem todos os itens, em conta comum; tarefa em `Program Files` e logon real: VM e USER |
| SEC-23 | `SetDefaultDllDirectories` primeiro; sem `LoadLibrary` relativo; DLL plantada | parcial. T `set_default_dll_directories_e_a_primeira_instrucao_do_main`, `nenhum_codigo_do_app_chama_load_library_relativo`; Process Monitor: QA-10 |
| SEC-24 | Toolchain, `cargo tree` sem http, audits | parcial. `cargo deny` OK, `rust-version` no piso (T); `npm audit` **não rodou** (offline) |
| SEC-25 | Junction-isca durante a desinstalação | parcial. W `junction_trocada_em_loop_durante_a_remocao_deixa_a_isca_intacta` e 5 outros; script NSIS (BE-38) a fazer |
| SEC-26 | Hash publicado confere | não verificado (BE-39) |
| SEC-27 | Sem `EWX_FORCE`/`shutdown.exe`; Bloco de Notas não salvo | parcial. Grep vazio; ticket de energia: T `energia_ticket_de_ponta_a_ponta_pela_ipc`; tela do Windows (CA-49): USER |
| SEC-28 | Sem hooks globais/injeção; APIs só nos donos | **passou**. Greps limpos; guard INV-03 no CI (33 violações plantadas detectadas); degraus 2 e 3 condicionados (C `foreground`). Anti-cheat: USER (bl. 10) e SG-10 |
| SEC-29 | Nenhuma operação destrutiva fora da pasta de dados | **passou**. W `repositorios_nao_escrevem_fora_da_pasta_de_dados`; greps |
| SEC-30 | Rede zero 10 min | parcial. Sem cliente HTTP e CSP; Resource Monitor 10 min não feito (sem o app real automatizável) |
| SEC-31 | Remoção por handle contra troca de junction | parcial. W `purge_integration` (6); remoção com o token do usuário no desinstalador (`RunAsUser`, BE-38/QF-05) a fazer |
| SEC-32 | NSIS só com `$SYSDIR\...` | não verificado (BE-37/BE-38 a fazer) |

**Resumo de SEC (32):** passou **19** · parcial **10** · não verificado **3** (SEC-21, 26, 32) · falhou **0**. SEC-07 e SEC-18 ficaram sem ressalva na rodada 2 (QA-BUG-01 e QA-BUG-02 fechados).

## 7. Casos de borda e métricas

| ID | Status e evidência |
|---|---|
| E-01 a E-04 | **passou** (domínio e sintético). C `jogo_que_abre_depois_dos_60_s…`; W escada; E-02 e E-03 com jogos e lojas reais: USER |
| E-05 | **passou**. C `e05_desconectar_durante_a_pressao…` |
| E-06 | **passou**. C `e06_com_o_launcher_na_frente_a_combinacao…` |
| E-07 | **passou**. Só a sessão aberta pelo Sofá conta (C) |
| E-08, E-09 | parcial. C/T com topologia falsa; USER (8.9) |
| E-10 | **passou**. U `GameFileMissing` |
| E-12, E-15 | **passou**. C |
| E-13 | **passou**. N `#lib=longo`: nome longo truncado no card e inteiro no herói; U `truncateName` |
| E-14 | **passou**. C `e14_processo_que_reaparece_em_menos_de_5_s…`, `jogo_que_fecha_e_reabre_varias_vezes…` |
| E-16, E-17, E-18 | parcial. Fixtures (W `stores`); lojas reais: USER |
| E-19 | não verificado (BE-37 a BE-39) |
| E-20 | **passou**. C `e20_volume_e_direcao_ao_mesmo_tempo_nao_se_cancelam` |
| E-21 | não verificado (jogos reais: USER, bl. 5) |
| E-23 | **passou** (domínio). W `NeedsElevation`; C `jogo_elevado_vai_direto_ao_degrau_4` |
| M-01 | USER, QA-14 |
| M-02 | não verificado (VM, PC da sala); marcas no log prontas (BE-40) |
| M-03 | **passou** em desenho: A no card em foco abre o jogo (U); contagem real no USER (5.13) |
| M-04 | parcial. Escada real ~0,2 s e retomada ~30 ms (BE-21, SP-06); jogo real no USER |
| M-05 | parcial. Domínio 120 Hz e UI sem atraso em N; percepção com controle: USER |

## 8. Telas por breakpoint

O produto é de TV (1080p e 4K); 375, 768 e 1280 não são alvo. Tirei prints deles só para ver que nada quebra. Prints em `docs/qa/launcher-tv/screens/`.

| Tela | 375 | 768 | 1280 | 1920x1080 | 3840x2160 |
|---|---|---|---|---|---|
| Biblioteca (heroi, recentes, grade) | sem scroll horizontal, legível | sem scroll horizontal | sem scroll horizontal | **passou** | **passou** (texto ≥ 24 px equiv.) |
| Menu rápido | não visto | não visto | não visto | **passou** | **passou** |
| Opções do jogo | não visto | não visto | não visto | **passou** | não visto |
| Painel do jogo | não visto | não visto | não visto | **passou** (texto) | não visto |
| Configurações (Tela) | não visto | não visto | não visto | **passou** | não visto |
| Biblioteca vazia | não visto | não visto | não visto | **passou** | **passou** |
| Nomes longos e HTML (`#lib=longo`) | não visto | não visto | não visto | **passou** | **passou** |
| Navegador de pastas, Itens ocultos, Todos os jogos | não visto | não visto | não visto | cobertos por testes de UI, não revistos em tela | não visto |

Alvos de toque, teclado virtual e zoom do campo não se aplicam (controle e TV). Contraste de texto: inspeção visual boa, não medi a razão de contraste de cada par (4,5:1, seção 11 da spec).

## 9. Estado por tarefa de QA nesta rodada

| ID | Status | Resumo |
|---|---|---|
| QA-01 | feito | Este documento |
| QA-02 | bloqueado | Sem `tauri-driver` nem `msedgedriver`, janela do Tauri em tela cheia não é controlada pelo Playwright; instalar o driver não foi pedido. Cobertura substituta: T `release_so_navega_para_a_origem_do_app`, `release_sem_console_e_sem_devtools`, `csp`, `single_instance` e a UI no navegador. Falta: F12, Ctrl+P, `eval` e `location.href` no release real |
| QA-03 | parcial | Domínio de entrada e UI por intenções: passou. Sem controle físico |
| QA-04 | parcial | Fixtures e UI: passou. Nenhuma loja instalada na máquina |
| QA-05 | parcial | CA-19, 20, 71, 72 e SEC-09/10 passaram; unidade de rede mapeada real pendente |
| QA-06 | parcial | Máquina de estados, sonda, job, WM_CLOSE e CPU com sintético passaram; matriz app × modo × degrau real com o Sofá real depende de VM e do roteiro |
| QA-07 | parcial | Domínio e UI passaram; tempo ≤ 200 ms e mudança externa precisam de dispositivo de áudio |
| QA-08 | bloqueado | Energia nunca exercitada (restrição); VM necessária. Feito: UI, tickets, greps e adaptadores com fake. Vai para QA-17 |
| QA-09 | parcial | 1080p e 4K por viewport; VM com escalas e dois monitores virtuais pendente. Vai para QA-17 |
| QA-10 | bloqueado | VM e BE-37 a BE-39 pendentes; autostart e DLL: testes que existem passaram em série. Vai para QA-17 |
| QA-11 | parcial | Todos os testes dos SEC-06 a SEC-17 e SEC-30 estáticos executados; falta rede zero por 10 minutos |
| QA-12 | feito | Corrupção, imagem, loja, interrupção e pasta no lugar do arquivo passaram; QA-BUG-02 fechado na rodada 2 |
| QA-15 | bloqueado | VM, BE-38, BE-42 pendentes. Feito: `purge_integration` 6/6 com junction em loop. Vai para QA-17 |
| QA-16 | parcial | Esboço em `docs/qa/roteiro-usuario.md` e `scripts/coletar-logs.ps1` testado; falta o ensaio na VM Windows 10, a lista D-29 e a parte do instalador (BE-39) |

## 10. Veredito (rodada 1; atualizado nas seções 11 e 12)

**Aprovado com ressalvas (parcial, rodada 1).** Nenhum CA falhou. Pendências antes do veredito final: corrigir QA-BUG-02; decidir D-QA-01 a D-QA-05; construir BE-37 a BE-39 e BE-42; VM para QA-08, QA-10 e QA-15; roteiro do usuário (QA-17).

## 11. Rodada 2: reteste e regressão

Commits desde `85b3174` até `a5b88a4`. Spec mudou em CA-29, 44, 45, 54, 61, 68, 79, 80 (novo), 81 (novo), RN-07, 15, 18, 32, 33, 34 (novo) e D-43 a D-48.

### 11.1 Execução

| Execução | Resultado |
|---|---|
| `scripts/ci.ps1 -SkipAudit` (com UI) | **CI OK**: guards (33 violações plantadas detectadas), `fmt`, `clippy -D warnings`, `cargo test --workspace` (508 verdes, 7 ignorados), bindings, `cargo deny`, UI `typecheck` + `lint` + **209 testes de UI verdes** (eram 193). `npm audit` não rodou (offline) |
| `qa_adversarial` (core) | 12 verdes, 1 ignorado (QA-BUG-04). `ca61_nome_so_com_invisiveis_vira_vazio` passa sem `#[ignore]` |
| `qa_robustez` (win) | 7 verdes, sem ignorados. `leitura_transitoriamente_travada…` passa; os devs acrescentaram `trava_curta_de_leitura_e_superada_pelas_novas_tentativas` |
| `autostart_integration` | 7/7 em paralelo, 3 execuções seguidas. Depois: nenhuma tarefa `Sofa*`, nenhum `Sofa*` em `Run` nem em `StartupApproved` |
| `purge_integration` 6, `library_integration` 3, `session_runtime` 14 (3 ignorados), `session_e2e` 4 (2 ignorados) | Verdes |
| Navegador (vite 5179, encerrado) | Regressão da seção 11.3 |

### 11.2 Bugs

| ID | Estado | Detalhe |
|---|---|---|
| QA-BUG-01 | **fechado** | `is_invisible` em `display_name.rs`: ZWSP, ZWNJ, BOM, U+3164, U+2800, U+00AD, U+115F e demais passam a valer como vazio; ZWJ, seletor de variação e acento combinado dentro de nome com texto ficam. Reteste: `qa_adversarial::ca61_nome_so_com_invisiveis_vira_vazio` e `ca61_mais_invisiveis_e_texto_legitimo_preservado` (14 formas, inclusive espaços Unicode) |
| QA-BUG-02 | **fechado** | Leitura que falha não grava por cima; o serviço usa a cópia em memória (commit `cf0c3cb`). Reteste: `qa_robustez::leitura_transitoriamente_travada_nao_pode_levar_a_perda_dos_dados_no_save_seguinte` sem `ignore` |
| QA-BUG-03 | **fechado** | Nomes únicos por teste (commit `2315f50`); 7/7 em paralelo, 3 vezes, sem resíduo |
| QA-BUG-04 | **novo, baixo, aberto** (dev-backend) | `crates/launcher-core/src/domain/library/display_name.rs` (`is_invisible`): U+1D173 a U+1D17A (formatos musicais invisíveis, categoria Cf) não valem como invisíveis; nome só com eles vira nome válido em branco. Cosmético e obscuro. Teste: `qa_adversarial::ca61_formatos_musicais_invisiveis_viram_vazio` (ignorado). CA-61 |

Sem bug aberto de front. Dúvidas D-QA-01, 02, 04, 05 e 06 fechadas (D-48, D-45, D-46, correção do front e texto novo do herói "Em andamento. Aperte A para voltar ao jogo."). **D-QA-03 segue aberta**, só de documentação: a tarefa BE-18 em `dev-backend.md` ainda diz "padrão só 1 e 4; 2 e 3 desligados", mas código, ADR e `arch 9.3` têm os dois ligados (ordem 1, 3, 2, 4).

### 11.3 Regressão da UI (1920x1080, teclado real no navegador)

| Verificação | Resultado |
|---|---|
| F2 abre as opções do jogo; ordem Jogar, Renomear, Restaurar nome original, Ocultar | passou (CA-79, CA-80) |
| Esc e Backspace voltam; Backspace dentro do campo de renomear apaga o texto ("Vale dos Ventos" para "Vale dos Vento") e não sai | passou |
| PgDn e PgUp pulam e voltam a linha; `]` sobe o volume de 45 para 50 na biblioteca; `[` e `]` não mudam nada no menu rápido | passou |
| F9 abre o painel do devCore; F2 com jogo aberto abre as opções, não o painel | passou |
| Diálogos com o nome do jogo no título: "Ocultar Vale dos Ventos?", "Abrindo Vale dos Ventos…", painel "Vale dos Ventos" | passou |
| Cancelar à esquerda e ação à direita: Ocultar, Renomear (Salvar), Desligar, Fechar jogo e Forçar fechamento. Cancelar em foco no Desligar | passou (CA-81, RN-34) |
| Grade "Todos os jogos" (500 jogos): Baixo avança uma linha (7 colunas), Baixo ×4 vai à linha 4, Cima ×2 volta, Direita e Esquerda, PgDn/PgUp, Esc volta à tela inicial | passou (corrigido) |
| Anel de foco: item da lista de Configurações a 103 px do canto, com margem de 96 px | passou (RN-32, CA-54) |

Prints novos em `docs/qa/launcher-tv/screens/r2-*.png`. Não revi 3840x2160 nem as telas Todos os jogos e navegador de pastas em imagem nesta rodada.

### 11.4 Cobertura de CA depois da rodada 2 (81)

- **Passou 51:** os 49 da rodada 1 mais CA-80 e CA-81. CA-25 e CA-61 saem da ressalva. CA-29, CA-79 continuam passou com o texto novo da spec.
- **Parcial 19:** CA-03, 04, 05, 06, 14, 15, 16, 17, 27, 32, 38, 41, 54, 56, 57, 58, 66, 67, 68.
- **Não verificado 11:** CA-01, 44, 45, 46, 49, 50, 51, 52, 53, 74, 76. CA-44 e CA-45 ganharam o código novo (`EWX_HYBRID_SHUTDOWN`) e seguem sem execução real.
- **Falhou 0.**

RN com mudança: RN-07, RN-32, RN-33 e RN-34 **passou**; RN-15 e RN-18 **parcial**. SEC: 19 passou, 10 parcial, 3 não verificado, 0 falhou.

### 11.5 Itens novos que o roteiro cobre

D-44 (Desligar com inicialização rápida e controle Bluetooth reconectando), código 4 do `--purge-user-data` com o Sofá aberto, jogo manual com processo fora da pasta, teclado completo e teclas novas. Detalhes em `docs/qa/roteiro-usuario.md`.

## 12. Veredito atualizado

**Aprovado com ressalvas (parcial, rodada 2).** Sobram QA-BUG-04 (baixo), D-QA-03 (documentação) e o que depende de BE-37 a BE-39 e BE-42, de VM e do roteiro do usuário (QA-17).
