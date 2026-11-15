# Roteiro de teste do Sofá (QA-16, esboço da rodada 1)

Este roteiro é só seu e roda no PC da sala, com Windows 10, na TV. Você não precisa consertar nada nem entender o porquê de cada passo: faça, anote o que viu e mande o zip de logs no fim. Caso que você não consegue fazer (falta o hardware, o jogo não está instalado) vale como "não deu": anote assim, sem tentar compensar.

**Estado do esboço.** Faltam duas coisas que chegam depois: o instalador de teste (BE-39, marcado "depende do BE-39" onde entra) e a sua lista de jogos (D-29, no bloco 5). O resto já pode ser lido e ajustado.

**Tempo total estimado:** 3 a 4 horas, em blocos que podem ser feitos em dias diferentes. Os blocos 1 a 3 são o mínimo para o Sofá servir no dia a dia.

## O que ter à mão

- Teclado e mouse (para o UAC, para renomear jogo e para os casos de emergência).
- Controle Xbox com fio **e** sem fio (adaptador e/ou Bluetooth), se tiver os dois. Se tiver um segundo controle, melhor.
- Notebook ou celular para anotar, ou imprima a tabela de cada bloco.
- Uma segunda tela (monitor ou outra TV), se tiver. Sem ela, os casos de duas telas ficam "não deu".
- A credencial de **outro administrador** do Windows (bloco 11) e uma conta **padrão** (não administradora) para testar a instalação.
- Um jogo com anti-cheat que funcione **offline** (bloco 10).
- O Process Explorer (Sysinternals) para o bloco 11. Baixe de learn.microsoft.com/sysinternals.

**Regra de ouro:** se algo travar a tela e o controle não resolver, use o teclado de emergência: setas, Enter (A), Esc (B) e F1 (Menu). Anote que precisou.

## Como anotar

Cada bloco tem uma tabela. Marque **OK**, **Falhou** ou **Não deu** e escreva uma linha do que viu quando não for OK. Horário aproximado ajuda a achar o ponto no log. Prints com o celular também servem.

---

## Bloco 0. Antes de começar (10 min)

1. Abra o PowerShell e rode `powercfg /a`. Anote se aparece **Suspensão (S3)** ou **Espera (S3)** na lista de estados disponíveis. O resultado vale para o bloco 8 (se "Suspender" deve aparecer no Sofá).
2. Anote o modelo da TV, a resolução usada hoje no Windows (Configurações > Sistema > Vídeo) e a escala (100%, 125%…).
3. Anote o modelo do controle e como ele conecta (cabo, adaptador, Bluetooth).
4. Se o Windows pede PIN ou senha ao ligar, o Sofá só começa depois que você entrar na área de trabalho. Isso é esperado e não é falha (RN-19).

## Bloco 1. Instalação — *depende do BE-39* (20 min)

Faça com a conta **padrão** e a credencial do outro administrador (D-31). Não use sua conta de administrador para clicar "Sim" no UAC.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 1.1 | Confira o SHA-256 do instalador: `Get-FileHash .\Sofa-Setup.exe` e compare com o arquivo `SHA256SUMS.txt` | Os dois iguais (CA-76) | |
| 1.2 | Rode o instalador. O SmartScreen avisa que o editor é desconhecido | Aparece o aviso (sem assinatura, é esperado). "Mais informações" > "Executar assim mesmo" | |
| 1.3 | No UAC, escolha a credencial do outro administrador | Instala em `C:\Program Files` sem outro passo manual (CA-50) | |
| 1.4 | Se o WebView2 não estava instalado: deixe o instalador baixar | Baixa e instala. Sem internet, mostra "Não foi possível baixar o WebView2…" (teste desligando o Wi-Fi só se quiser) | |
| 1.5 | Veja a caixa "Iniciar com o Windows" na instalação | Vem marcada (CA-51) | |
| 1.6 | Ao terminar, deixe abrir o Sofá. Abra o Process Explorer, ache `sofa.exe`, clique 2x > aba Security | Integridade **Medium**, usuário = a conta padrão, **não** a conta do administrador do UAC (SEC-21, E-19) | |
| 1.7 | Há atalho no menu Iniciar? Abra por ele com o Sofá já aberto | Atalho existe. Abrir de novo não cria segunda janela, só traz a atual (CA-02) | |

## Bloco 2. Boot e início automático (15 min)

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 2.1 | Reinicie o PC e entre na conta padrão. Cronometre do momento em que a área de trabalho aparece até o Sofá aceitar um direcional (M-02) | Até 5 s. Anote o tempo de 3 boots | |
| 2.2 | Dentro do Sofá: Menu > Configurações > Geral. Anote o texto de "Iniciar com o Windows" (liga, e qual mecanismo: tarefa agendada ou Run) | Ligado; mecanismo anotado | |
| 2.3 | Desligue "Iniciar com o Windows" e reinicie | O Sofá **não** abre sozinho (CA-04) | |
| 2.4 | Ligue de novo, reinicie | Volta a abrir (CA-04). O UAC **não** apareceu nem ao ligar nem ao desligar (CA-74) | |
| 2.5 | No Process Explorer, o `sofa.exe` aberto no boot | Integridade Medium (M-02, SEC-20) | |

## Bloco 3. Controle e navegação (25 min)

Repita com cada forma de conexão que você tiver. Anote qual.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 3.1 | D-pad e analógico esquerdo na biblioteca | O foco vai na direção certa e dá para ver a 3 m (CA-06) | |
| 3.2 | Segure uma direção por 1 s | Depois de ~0,4 s o foco repete em ritmo regular (CA-07) | |
| 3.3 | Deixe o analógico solto, sem tocar, por 1 minuto, em cada controle que você tiver | O foco **não** anda sozinho (CA-08). Se andar, anote qual controle | |
| 3.4 | Com dois controles ligados, use um e depois o outro | Os dois funcionam do mesmo jeito (CA-11) | |
| 3.5 | B em cada tela que não seja a inicial | Volta uma tela, sem executar nada (CA-10) | |
| 3.6 | Desligue o controle (ou tire o cabo) com o Sofá na frente | Em até 2 s aparece o aviso. O foco não muda (CA-14) | |
| 3.7 | Religue | O aviso some em até 2 s e o controle funciona sem reiniciar (CA-15) | |
| 3.8 | Abra o Sofá **sem** controle ligado | Faixa "Nenhum controle conectado. Ligue o controle ou conecte o cabo." e o teclado funciona (CA-13) | |
| 3.9 | Em qualquer tela, tente fazer tudo só com o controle (exceto renomear) | Dá para concluir (CA-09) | |
| 3.10 | Quanto tempo o foco leva para mexer depois de apertar? Subjetivo | Imediato, sem atraso perceptível (M-05) | |

## Bloco 4. Biblioteca real (30 min)

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 4.1 | Menu > Configurações > Biblioteca > Atualizar biblioteca | Os jogos **Steam** instalados aparecem, com nome e capa da Steam (CA-16). Anote jogos que faltam ou com capa errada | |
| 4.2 | O mesmo para **Epic** e **GOG** (inclusive um jogo GOG instalado sem o Galaxy, se houver) | Aparecem com nome e capa, ou com ícone/placeholder se não houver capa (CA-56, CA-57, E-17) | |
| 4.3 | Itens que não são jogo (redistribuíveis da Steam, ferramentas) | Não aparecem (CA-69). Se aparecer algum, anote o nome exato | |
| 4.4 | Desinstale um jogo pela loja e atualize a biblioteca | O jogo some (CA-17, CA-58) | |
| 4.5 | Se a Epic ou o GOG não estiverem instalados: nada de erro na tela (CA-18, CA-59) | Só as outras origens | |
| 4.6 | **Adicionar jogo** (Configurações > Biblioteca): escolha um `.exe` solto pelo controle, de um disco local | Entra com o nome do arquivo (CA-19) | |
| 4.7 | Veja o card do jogo manual | Mostra o **ícone do executável** como capa, não só o nome em texto. Esta é a correção do commit 10ed47c, nunca vista no app real. Anote se aparece ou se fica o texto | |
| 4.8 | Confira se jogos da Steam/Epic/GOG mostram a **capa** (arte) e não o placeholder | Capa aparece quando a loja tem arte local. Print do que ficou como texto | |
| 4.9 | Tente adicionar o mesmo `.exe` de novo | "Este jogo já está na biblioteca" (CA-20) | |
| 4.10 | Renomeie um jogo com o teclado (View > Renomear). Atualize a biblioteca | O nome novo se mantém (CA-61, CA-62). "Restaurar nome original" existe e funciona | |
| 4.11 | Oculte um jogo. Atualize. Restaure em Configurações > Itens ocultos | Some, não volta sozinho, e volta com nome e data de último jogado (CA-21, CA-70) | |
| 4.12 | Feche o Sofá (Menu > Sair do launcher) e abra de novo | Manuais, ocultos, nomes e ordem de recentes iguais (CA-25) | |
| 4.13 | Se tiver HD externo com jogos cadastrados: desconecte | O jogo avisa só o nome ao tentar abrir (CA-28), sem mostrar caminho. Conectando, volta (E-10, E-18) | |

## Bloco 5. Cada jogo, cada modo de tela, cada degrau (60 a 90 min)

**Lista de jogos (D-29): a preencher.** Quando você me mandar a lista, entra aqui uma linha por jogo. Para cada um:

1. Abra pelo Sofá com **A** (CA-26). Anote o tempo até a janela do jogo aparecer.
2. Com o jogo em primeiro plano, segure **View + Menu por 1,5 s**. O Sofá deve vir para a frente em até 3 s com o painel do jogo (CA-32, M-04). Cronometre.
3. **Voltar ao jogo.** O jogo volta do mesmo ponto (CA-34).
4. Volte ao painel e use **Ir para a biblioteca**. O card mostra "Em andamento" e A volta ao jogo (CA-36).
5. Feche o jogo **pelo menu do próprio jogo**. O Sofá volta sozinho em até 3 s com o jogo em foco na biblioteca (CA-30).
6. Repita 1 a 5 com os degraus **1 e 4** e, depois, com os **degraus 2 e 3** (veja o bloco 12 para trocar).

| Jogo | Origem | Modo de tela (janela / sem borda / tela cheia exclusiva) | Degraus | Abre (s) | Retorno por View+Menu (s) | Fim pelo menu do jogo, volta sozinho (s) | Lançador próprio antes do jogo? | O jogo reage ao controle com o Sofá na frente? | Obs. |
|---|---|---|---|---|---|---|---|---|---|
| *(vem da lista D-29)* | | | | | | | | | |
| | | | | | | | | | |
| | | | | | | | | | |

Mais casos de sessão, com **um** jogo qualquer da lista:

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 5.1 | Segure View + Menu por menos de 1,5 s e solte | Nada acontece e o jogo segue recebendo os botões (CA-33) | |
| 5.2 | Com o jogo aberto, aperte botões do controle sem segurar a combinação, se o Sofá estiver em segundo plano | O Sofá não reage (CA-12) | |
| 5.3 | Painel > **Fechar jogo** > confirmar | Tela "Fechando…", o jogo fecha normalmente (CA-35) | |
| 5.4 | Se um jogo travar: Fechar jogo e aguarde 10 s | Aparece "Forçar fechamento" com Cancelar em foco (CA-35, E-04) | |
| 5.5 | Com um jogo aberto, volte à biblioteca e aperte A em **outro** jogo | "Fechar e abrir…?" com Cancelar em foco (CA-31) | |
| 5.6 | Toque em A duas vezes seguidas na tela "Abrindo…" | O segundo toque não faz nada nos primeiros 0,6 s (CA-77) | |
| 5.7 | Em "Abrindo…", aperte Voltar e espere o jogo abrir | O jogo abre depois e a sessão vale normalmente (CA-77) | |
| 5.8 | Jogo que reinicia sozinho (se algum fizer isso) | O Sofá não acha que o jogo fechou (E-14) | |
| 5.9 | Jogo que abre o lançador da empresa antes do jogo | A sessão continua. Anote se o lançador aceita o controle (E-01, R-03) | |
| 5.10 | Jogo que exige administrador (se tiver; ou um `.exe` marcado "Executar como administrador") | Aviso do Sofá antes do UAC, "Continuar / Voltar". Voltar não abre o UAC (CA-73). No UAC, use o teclado. A combinação de retorno pode falhar aí e o aviso diz isso (E-23) | |
| 5.11 | Abra um jogo pela Steam/atalho, fora do Sofá | Não conta como sessão e a combinação não funciona (E-07). Só para confirmar | |
| 5.12 | Veja o Gerenciador de Tarefas com um jogo aberto e o Sofá em segundo plano por 5 min | CPU do Sofá abaixo de 1% (RN-21). Anote o maior valor visto | |
| 5.13 | Quantos cliques para abrir o último jogo jogado, a partir da tela inicial? | 1 (A) (M-03) | |

## Bloco 6. Volume (15 min)

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 6.1 | Menu > Volume, direita e esquerda | 5 pontos por toque, de 0 a 100. O número aparece **na hora**. Anote se parecer lento (a meta é 200 ms) (CA-38) | |
| 6.2 | RT e LT na biblioteca | Sobe e desce 5 por puxada. O indicador some após 2 s (CA-64) | |
| 6.3 | Mude o volume pelo teclado do PC ou pelo controle remoto da TV (HDMI-CEC) e abra o menu | O nível mostrado é o atual do Windows (CA-41) | |
| 6.4 | Mudo: ligue, aumente o volume | O mudo sai e o volume sobe um passo (CA-39, CA-40) | |
| 6.5 | **Troque o dispositivo de áudio padrão** (de TV para fones, por exemplo) com o Sofá aberto e mexa no volume | O Sofá controla o novo dispositivo e mostra o nível dele | |
| 6.6 | Desative todos os dispositivos de áudio de saída (Configurações do som) e abra o Volume | "Nenhum dispositivo de áudio encontrado", botões desabilitados (CA-42). **Reative depois.** | |
| 6.7 | LT/RT dentro do menu rápido, de um diálogo ou com um jogo em primeiro plano | Nada muda (CA-65) | |

## Bloco 7. Energia (30 min)

Salve seu trabalho antes. Os testes desligam e reiniciam o PC de verdade.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 7.1 | Menu > Reiniciar, abra e depois cancele (B) | Diálogo com Cancelar em foco, B cancela, nada acontece (CA-43) | |
| 7.2 | Abra o **Bloco de Notas**, digite algo e **não salve**. No Sofá, Desligar > confirmar | O Windows mostra a **própria tela** dizendo que o Bloco de Notas impede o desligamento. O Sofá não força nada (CA-49). Cancele a tela do Windows | |
| 7.3 | Com um jogo aberto, escolha qualquer ação de energia | O aviso diz o nome do jogo e que o progresso não salvo pode ser perdido (CA-48) | |
| 7.4 | Reiniciar > confirmar (com o Bloco de Notas fechado) | Reinicia em até ~3 s. Com "Iniciar com o Windows" ligado, o Sofá abre depois do login (CA-45) | |
| 7.5 | Se o bloco 0 mostrou que existe S3: Suspender > confirmar. Acorde pelo teclado ou controle | Suspende. Ao acordar, o Sofá está na mesma tela. O controle Bluetooth volta a funcionar sozinho, ou anote o que precisou fazer (CA-46) | |
| 7.6 | Se o bloco 0 mostrou que **não** existe suspensão | "Suspender" não aparece ou aparece desabilitado com "Suspensão indisponível neste PC" (CA-47) | |
| 7.7 | Desligar > confirmar (por último) | O Windows desliga em até 3 s, sem forçar (CA-44) | |

Anote também: **"Suspender" existe no PC da sala?** (sim/não, e o que o `powercfg /a` disse).

## Bloco 8. Telas e TV (30 min)

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 8.1 | Sentado a 3 m, passe por biblioteca, menu, painel, configurações | Leio tudo sem esforço e o item em foco se destaca (CA-54, CA-55). Anote o que ficou pequeno | |
| 8.2 | Observe as bordas da TV (overscan) | Nenhum texto ou botão cortado. Capas cortadas na ponta da fileira são esperadas (RN-32) | |
| 8.3 | A TV em 1080p e em 4K, e escala 100% a 300% (o que a TV aguentar) | Nada cortado e o texto proporcional (CA-54) | |
| 8.4 | O cursor do mouse some após 3 s parado e volta ao mexer (CA-03) | Não há barra de título nem barra de tarefas | |
| 8.5 | **Duas telas:** Configurações > Tela, escolha a outra tela | O Sofá passa para ela na hora, em tela cheia (CA-66) | |
| 8.6 | Feche e reabra o Sofá | Abre na tela escolhida (CA-66) | |
| 8.7 | Desligue a tela escolhida (tire o cabo HDMI) | O Sofá vai para a principal e a escolha continua gravada (CA-67) | |
| 8.8 | Religue a tela e reabra o Sofá | Abre de novo na tela escolhida (CA-67) | |
| 8.9 | Desligue a TV com o PC ligado e ligue de novo | O Sofá continua em tela cheia na mesma tela (E-09) | |

## Bloco 9. Teclado e mouse de emergência (5 min)

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 9.1 | Sem controle: setas, Enter, Esc, F1 | Navega, abre, volta e abre o Menu (RN-07) | |
| 9.2 | Tente abrir as Opções do jogo (View) só pelo teclado | Anote o que precisou. O teclado de emergência não tem tecla de View, só F1 para o Menu (ponto para o PO) | |

## Bloco 10. Jogo com anti-cheat, offline (20 min)

Use **modo offline** e uma conta que você não se importe de perder, se possível (D-30). O risco é baixo, mas a tecla sintética do degrau 2 (F24) pode ser lida como macro por algum anti-cheat.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 10.1 | Abra o jogo pelo Sofá | Abre e o anti-cheat não reclama do Sofá | |
| 10.2 | Segure View + Menu e volte ao Sofá, 3 vezes | Volta em até 3 s e o anti-cheat não acusa nada. Anote qualquer aviso ou kick | |
| 10.3 | Se houve problema: desligue o degrau 2 (bloco 12) e repita | Anote se o problema parou | |

## Bloco 11. Conta padrão e outro administrador (D-31) (20 min)

Se o bloco 1 foi feito com a conta padrão, boa parte já está feita. Falta conferir:

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 11.1 | Entre na conta **padrão** e abra o Sofá, jogue um jogo, ajuste volume, desligue pelo Sofá | O UAC **nunca** aparece, exceto em jogo que exige administrador (CA-74) | |
| 11.2 | Entre na **conta do outro administrador** (se existir no PC) e abra o Sofá | O Sofá dessa conta tem a própria biblioteca e não herda a da conta padrão (D-31). Sem "Iniciar com o Windows" que você não ligou | |
| 11.3 | Rode `Get-ScheduledTask 'Sofa Autostart*'` no PowerShell, ou veja o resultado do coletor | A tarefa (se existir) é só da conta padrão, nunca da conta do administrador do UAC (SEC-21, SEC-22) | |

## Bloco 12. Os degraus 2 e 3 e os logs para a segurança (SG-10) (20 min)

A escada de retorno tenta 4 jeitos para trazer o Sofá à frente, na ordem **1, 3, 2, 4**: o pedido simples, "vincular à entrada da janela do jogo", **uma tecla F24 sintética** e minimizar o jogo. Hoje os degraus 2 e 3 vêm **ligados** (decisão da segurança, rev. 5). Este bloco serve para a segurança decidir se eles continuam assim.

1. Abra o arquivo `%LOCALAPPDATA%\app.sofa.launcher\settings.json` (Bloco de Notas) **com o Sofá fechado** e acrescente a seção `advanced` (o arquivo precisa manter `"schemaVersion": 1` e os campos que já tem; os nomes distinguem maiúsculas de minúsculas):

   ```json
   "schemaVersion": 1,
   "advanced": { "diagnostics": { "level": "debug" } }
   ```

   Isso liga o log detalhado: ele registra, por retorno, quais degraus foram tentados, quantos ms levou cada um e o resultado.
2. Faça o bloco 5 com os degraus **ligados** (padrão: 1, 3, 2, 4).
3. Para testar só 1 e 4, feche o Sofá e use:

   ```json
   "schemaVersion": 1,
   "advanced": { "diagnostics": { "level": "debug" }, "foreground": { "step2": false, "step3": false } }
   ```

   Repita o bloco 5. Compare os tempos. Se o jogo não voltar com 1 e 4, anote: é o resultado esperado e é o que a segurança quer saber.
4. Se um jogo ou anti-cheat reagir à tecla F24, deixe só `"step2": false` e repita. Anote o que mudou.
5. Se errar um nome ou um valor, o Sofá ignora só aquele campo, usa o padrão dele e avisa no log. Apagar a seção `advanced` volta tudo ao padrão.

## Bloco 13. Desinstalação — *depende do BE-39* (20 min)

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 13.1 | Atualize: rode um instalador mais novo por cima, com a biblioteca já montada | Mantém biblioteca, configurações e "Iniciar com o Windows" (CA-53) | |
| 13.2 | Desinstale por "Aplicativos instalados", **mantendo** os dados | Remove o app, atalhos e o início automático da sua conta. Jogos no disco intactos. A pasta `%LOCALAPPDATA%\app.sofa.launcher` continua (CA-52, CA-68) | |
| 13.3 | Reinstale | Biblioteca e configurações voltam (CA-68) | |
| 13.4 | Desinstale **desmarcando "Manter dados"**, com o Sofá **aberto** (ele usa a pasta `EBWebView`) | Pode sobrar algo da pasta `EBWebView` em uso e o desinstalador conta isso, mas a biblioteca e as configurações somem. Os jogos e outras pastas não são tocados (CA-68, SEC-25). Anote o que sobrou | |
| 13.5 | Confira se a entrada de início automático e a tarefa agendada sumiram | Sumiram (CA-52) | |

## Bloco 14. Coletar os logs e devolver (5 min)

1. Abra o PowerShell na pasta onde está o `coletar-logs.ps1`.
2. Rode: `powershell -ExecutionPolicy Bypass -File .\coletar-logs.ps1`
3. O zip aparece na área de trabalho: `sofa-logs-AAAAMMDD-HHMMSS.zip`. Abra, confira o `LEIA-ME.txt` e envie.

O que vai no zip: os logs do Sofá, o `settings.json`, a versão do Windows, o hardware, as telas, o WebView2, o resultado de `powercfg /a` e o estado do Sofá instalado (integridade, tarefa agendada, entrada de início). O que **não** vai: a lista de jogos, as capas, seu nome de usuário e o nome do PC (viram `<usuario>` e `<pc>`).

Rode o coletor **logo depois** de cada bloco longo (5, 10 e 12) se quiser separar os logs. Os logs rodam em até 3 arquivos de 5 MB e o mais antigo se perde.

## Fechamento

Escreva, em poucas linhas:

- O que atrapalhou mais no dia a dia.
- Qual jogo ou situação você não conseguiu testar e por quê.
- Se o Sofá serviria hoje como tela inicial da sala.

*Casos "Não deu" entram no relatório como não verificados (D-33). Nenhum item aqui bloqueia o aceite de uso de 2 semanas (QA-14).*
