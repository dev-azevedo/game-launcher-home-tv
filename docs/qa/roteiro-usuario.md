# Roteiro de teste do SofaLauncher 0.2.0 (QA-16, QA-19)

Este roteiro é só seu e roda no PC da sala, com Windows 10, na TV. Você não precisa entender o porquê de cada passo nem consertar nada: faça, anote o que viu e mande o zip de logs no fim. O que você não conseguir fazer (falta o hardware, o jogo não está instalado) vale como "Não deu". Anote assim, sem tentar compensar.

**Instale e desinstale o SofaLauncher só no PC da sala.** Não rode o instalador em outro computador.

**Tempo total estimado:** 5 a 6 horas, em 2 ou 3 sessões. Quem manda no tempo é o bloco 5, que depende de quantos jogos você escolher. Para o SofaLauncher servir no dia a dia, o mínimo são os blocos 1 a 3. A desinstalação (bloco 14) fica **por último**, porque apaga o que foi testado.

| Bloco | Assunto | Tempo |
|---|---|---|
| 0 | Antes de começar | 10 min |
| 1 | Instalação ou atualização por cima da 0.1.0 | 40 min |
| 2 | Boot e início automático | 20 min |
| 3 | Controle e navegação | 25 min |
| 4 | Biblioteca real | 30 min |
| 5 | Jogos, modos de tela e degraus | 60 a 90 min |
| 6 | Volume | 15 min |
| 7 | Energia | 35 min |
| 7A | Fechar o launcher | 25 min |
| 8 | Telas e TV | 30 min |
| 9 | Teclado e mouse | 10 min |
| 10 | Jogo com anti-cheat, offline | 20 min |
| 11 | Conta padrão e outro administrador (se você tiver) | 20 min |
| 12 | Degraus 2 e 3 e logs | 20 min |
| 13 | Coletar os logs (antes de desinstalar) | 5 min |
| 14 | Desinstalação | 30 min |
| 15 | Coleta final e envio | 5 min |

## O que ter à mão

- Um **pendrive** com a pasta do instalador (`SofaLauncher_0.2.0_x64-setup.exe`, `SHA256SUMS.txt` e `README.txt`) **e** o arquivo `scripts\coletar-logs.ps1` do repositório. O instalador **não** traz o coletor de logs: leve os dois juntos e copie tudo para uma pasta do PC da sala, por exemplo `Documentos\Sofa-teste`.
- Teclado e mouse (para a permissão do Windows, para renomear jogo e para emergências).
- Controle Xbox com fio **e** sem fio (adaptador e/ou Bluetooth), se tiver os dois. Um segundo controle ajuda.
- Celular para anotar e fotografar a tela, ou a tabela de cada bloco impressa.
- Uma segunda tela (monitor ou outra TV), se tiver. Sem ela, os casos de duas telas ficam "Não deu".
- Opcional: a senha de **outro administrador** do Windows e uma conta **padrão** (comum, sem poderes de administrador). Sem elas, o bloco 11 fica "Não deu" e o resto segue.
- Um jogo com anti-cheat que rode **offline** (bloco 10).

**Regra de ouro:** se algo travar a tela e o controle não resolver, use o teclado: setas, Enter (A), Esc ou Backspace (B) e F1 (Menu). Anote que precisou.

## Como anotar

Cada bloco tem uma tabela. Marque **OK**, **Falhou** ou **Não deu** e escreva uma linha do que viu quando não for OK. A hora aproximada ajuda a achar o ponto no log. Fotos da tela com o celular servem.

---

## Bloco 0. Antes de começar (10 min)

1. Abra o PowerShell e rode `powercfg /a`. Anote se aparece **Suspensão (S3)** ou **Espera (S3)** na lista de estados disponíveis. O resultado vale para o bloco 7 (se "Suspender" deve aparecer no SofaLauncher).
2. Anote o modelo da TV, a resolução usada hoje no Windows (Configurações > Sistema > Vídeo) e a escala (100%, 125%…).
3. Anote o modelo do controle e como ele conecta (cabo, adaptador, Bluetooth).
4. Se o Windows pede PIN ou senha ao ligar, o SofaLauncher só começa depois que você entrar na área de trabalho. Isso é esperado, não é falha (RN-19).
5. Veja se o **WebView2** já está instalado: Configurações > Aplicativos > Aplicativos instalados, e procure "Microsoft Edge WebView2 Runtime". Anote se achou. Ele é um componente da Microsoft que o SofaLauncher usa para desenhar a tela. **Não desinstale**: só anote. Se já estiver instalado, o item 1.6 fica "Não deu".

## Bloco 1. Instalação ou atualização por cima da 0.1.0 (40 min)

O instalador não tem assinatura digital (é uma versão de teste, feita para uso pessoal). Por isso o Windows pode estranhar o arquivo. Os passos abaixo mostram como seguir com segurança.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 1.1 | **Confira o arquivo antes de abrir.** Abra o PowerShell na pasta do instalador e rode `Get-FileHash .\SofaLauncher_0.2.0_x64-setup.exe -Algorithm SHA256`. Compare o valor com o de `SHA256SUMS.txt` (o final do valor da 0.2.0 será anotado aqui depois do release oficial: por enquanto, vale o que está no `SHA256SUMS.txt`) | Os dois iguais (CA-76). Se forem diferentes, **não instale** e me avise. Se houver uma versão nova do instalador, o valor muda: vale sempre o `SHA256SUMS.txt` que veio junto do arquivo | |
| 1.2 | Dê dois cliques no instalador. O **SmartScreen** (a tela azul "O Windows protegeu seu PC") pode aparecer, ou o Defender pode reclamar | Só continue se o 1.1 deu certo: clique em **Mais informações** e depois em **Executar assim mesmo**. Não desligue o antivírus. Se o Defender bloquear ou mandar o arquivo para quarentena, pare, tire uma foto da mensagem e anote | |
| 1.3 | O Windows pede **permissão de administrador** (a tela escura "Deseja permitir que este aplicativo faça alterações?") | Aparece, e é esperado só aqui, na instalação (CA-50). Se você tem a senha de **outro administrador**, use essa; se só tem a sua conta, aceite com ela e anote "uma conta só" | |
| 1.4 | Passe pelas telas do instalador | A caixa **"Iniciar com o Windows"** vem marcada (CA-51). Há um texto orientando a entrar no Windows sem teclado (RN-19). Instala em `C:\Program Files\Sofa` sem pedir nada manual | |
| 1.5 | Termine e deixe o SofaLauncher abrir | Abre em tela cheia **sem** pedir permissão de administrador de novo | |
| 1.6 | **Só se o WebView2 não estava instalado (bloco 0, item 5):** repare no que o instalador fez | Baixou e instalou o WebView2 sozinho (CA-50). Para testar a falta de internet, desligue o Wi-Fi ou o cabo **antes** de instalar: deve aparecer "Não foi possível baixar o WebView2. Conecte o PC à internet e tente de novo." Religue a rede depois | |
| 1.7 | Com o SofaLauncher aberto, rode o coletor: `powershell -ExecutionPolicy Bypass -File .\coletar-logs.ps1`. Abra o zip e leia `sofa.txt` | Em "Processo sofa.exe" aparece **"Medium … elevado=False"**. Isso quer dizer que o SofaLauncher roda como programa comum, sem poderes de administrador (SEC-20, SEC-21). Se aparecer "High" ou "elevado=True", anote | |
| 1.8 | No mesmo `sofa.txt`, procure "ACL da pasta de instalação" (é a lista de quem pode mexer na pasta). Se quiser ver ao vivo: `icacls "C:\Program Files\Sofa"` | Os usuários comuns só **leem e executam**: nenhuma linha `Users:(M)`, `Users:(F)` nem `Everyone` (SEC-20). Cole o que aparecer | |
| 1.9 | Abra o SofaLauncher pelo atalho do menu Iniciar com ele já aberto | O atalho existe. Abrir de novo não cria segunda janela, só traz a atual (CA-02) | |
| 1.10 | Rode `Get-ScheduledTask 'Sofa Autostart*'` no PowerShell (se "Iniciar com o Windows" ficou marcada) | A tarefa existe só para a **sua** conta (e, se você instalou com outra senha de administrador, **não** para a conta do administrador) (SEC-21, SEC-22, E-19). Se não existir tarefa, pode ser o outro mecanismo (Run): anote | |
| 1.11 | Opcional, 1 minuto: no PowerShell, rode o instalador com outra pasta: `.\SofaLauncher_0.2.0_x64-setup.exe /D=C:\Teste` | Recusa e mostra a mensagem de **pasta fixa**: "O SofaLauncher só pode ser instalado em Program Files. Rode o instalador sem a opção /D=." Nada é instalado. Se aparecer UAC antes, aceite. O desinstalador tem uma mensagem parecida ("Desinstale em Aplicativos instalados."), que só aparece se alguém rodar o desinstalador de fora de Program Files: não precisa testar | |
| 1.12 | **Só se o PC da sala já tem a 0.1.0 (o "Sofá").** Antes de atualizar, anote: a versão no rodapé das Configurações (0.1.0), se "Iniciar com o Windows" está ligado, se há jogos renomeados ou ocultos, e abra o menu Iniciar e procure "Sofá". Rode também `Get-ScheduledTask 'Sofa Autostart*'` e anote o resultado | Tudo anotado para comparar depois | |
| 1.13 | **Atualizar por cima, sem desinstalar antes.** Confira o hash (1.1) e rode `SofaLauncher_0.2.0_x64-setup.exe` com o Sofá (0.1.0) fechado ou aberto; o instalador cuida do resto | Atualiza sem apagar nada (CA-53). Pede permissão de administrador uma vez. A pasta continua `C:\Program Files\Sofa` (o nome da pasta não muda) | |
| 1.14 | Abra Configurações > Aplicativos > Aplicativos instalados e procure "Sofa" | **Uma única entrada**, "SofaLauncher", com a versão 0.2.0. Se aparecerem **duas** ("Sofá" e "SofaLauncher"), anote e tire foto (R-11) | |
| 1.15 | Abra o menu Iniciar e procure "Sofá" e "SofaLauncher" | O atalho **SofaLauncher** existe e o atalho antigo **Sofá** sumiu. Se o antigo continuar, anote se ele abre o programa certo | |
| 1.16 | Abra o SofaLauncher e confira: a biblioteca, os nomes trocados, os ocultos e "Iniciar com o Windows" (e o `Get-ScheduledTask` do 1.12) | Tudo igual ao que você anotou (CA-53, CA-25). O rodapé das Configurações mostra **0.2.0**, e o nome na tela é "SofaLauncher". A pasta de dados `%LOCALAPPDATA%\app.sofa.launcher` e a pasta `C:\Program Files\Sofa` continuam com esses nomes, de propósito | |

## Bloco 2. Boot e início automático (20 min)

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 2.1 | Reinicie o PC e entre na conta padrão. Cronometre do momento em que a área de trabalho aparece até o SofaLauncher aceitar um direcional (M-02) | Até 5 s. Anote o tempo de 3 boots | |
| 2.2 | Dentro do SofaLauncher: Menu > Configurações > Geral. Anote o texto de "Iniciar com o Windows" (liga, e qual mecanismo: tarefa agendada ou Run) | Ligado; mecanismo anotado | |
| 2.3 | Desligue "Iniciar com o Windows" e reinicie | O SofaLauncher **não** abre sozinho (CA-04) | |
| 2.4 | Ligue de novo, reinicie | Volta a abrir (CA-04). O UAC **não** apareceu nem ao ligar nem ao desligar (CA-74) | |
| 2.5 | Com o SofaLauncher aberto pelo boot, rode o coletor de logs e leia `sofa.txt` | "Medium … elevado=False" também quando o SofaLauncher abre sozinho no boot (M-02, SEC-20) | |

## Bloco 3. Controle e navegação (25 min)

Repita com cada forma de conexão que você tiver. Anote qual.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 3.1 | D-pad e analógico esquerdo na biblioteca | O foco vai na direção certa e dá para ver a 3 m (CA-06) | |
| 3.2 | Segure uma direção por 1 s | Depois de ~0,4 s o foco repete em ritmo regular (CA-07) | |
| 3.3 | Deixe o analógico solto, sem tocar, por 1 minuto, em cada controle que você tiver | O foco **não** anda sozinho (CA-08). Se andar, anote qual controle | |
| 3.4 | Com dois controles ligados, use um e depois o outro | Os dois funcionam do mesmo jeito (CA-11) | |
| 3.5 | B em cada tela que não seja a inicial | Volta uma tela, sem executar nada (CA-10) | |
| 3.6 | Desligue o controle (ou tire o cabo) com o SofaLauncher na frente | Em até 2 s aparece o aviso. O foco não muda (CA-14) | |
| 3.7 | Religue | O aviso some em até 2 s e o controle funciona sem reiniciar (CA-15) | |
| 3.8 | Abra o SofaLauncher **sem** controle ligado | Faixa "Nenhum controle conectado. Ligue o controle ou conecte o cabo." e o teclado funciona (CA-13) | |
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
| 4.10 | Renomeie um jogo com o teclado (View, ou F2 no teclado, depois Renomear). Atualize a biblioteca | O nome novo se mantém (CA-61, CA-62). "Restaurar nome original" existe e funciona | |
| 4.11 | Oculte um jogo. Atualize. Restaure em Configurações > Itens ocultos | Some, não volta sozinho, e volta com nome e data de último jogado (CA-21, CA-70) | |
| 4.12 | Feche o SofaLauncher (Menu > Sair do launcher) e abra de novo | Manuais, ocultos, nomes e ordem de recentes iguais (CA-25) | |
| 4.13 | Se tiver HD externo com jogos cadastrados: desconecte | O jogo avisa só o nome ao tentar abrir (CA-28), sem mostrar caminho. Conectando, volta (E-10, E-18) | |

## Bloco 5. Cada jogo, cada modo de tela, cada degrau (60 a 90 min)

**Você escolhe os jogos na hora (D-29).** Preencha uma linha por jogo na tabela. Escolha o que você realmente joga, com pelo menos um de cada tipo que você tiver: Steam, Epic, GOG, um jogo cadastrado à mão e, no bloco 10, um com anti-cheat. Se algum jogo funcionar em mais de um modo de tela (janela, sem borda, tela cheia exclusiva), teste cada modo e preencha uma linha por modo. Para cada jogo:

1. Abra pelo SofaLauncher com **A** (CA-26). Anote o tempo até a janela do jogo aparecer.
2. Com o jogo em primeiro plano, segure **View + Menu por 1,5 s**. O SofaLauncher deve vir para a frente em até 3 s com o painel do jogo (CA-32, M-04). Cronometre.
3. **Voltar ao jogo.** O jogo volta do mesmo ponto (CA-34).
4. Volte ao painel e use **Ir para a biblioteca**. O card mostra "Em andamento" e A volta ao jogo (CA-36).
5. Feche o jogo **pelo menu do próprio jogo**. O SofaLauncher volta sozinho em até 3 s com o jogo em foco na biblioteca (CA-30).
6. Repita 1 a 5 com os degraus que o SofaLauncher usa hoje (1, 3, 2 e 4, nessa ordem) e, depois, só com **1 e 4** (veja o bloco 12 para trocar).

As linhas em itálico são exemplos do que escrever; apague ou sobrescreva.

| Jogo | Origem (Steam, Epic, GOG, manual) | Modo de tela (janela, sem borda, tela cheia exclusiva) | Degraus (todos, ou só 1 e 4) | Abre (s) | Retorno por View+Menu (s) | Fim pelo menu do jogo, volta sozinho (s) | Lançador próprio antes do jogo? | O jogo reage ao controle com o SofaLauncher na frente? | Obs. |
|---|---|---|---|---|---|---|---|---|---|
| *Exemplo Steam: nome do jogo* | *Steam* | *sem borda* | *todos* | *12* | *1,5* | *2* | *não* | *não* | *tudo certo* |
| *Exemplo Epic: nome do jogo* | *Epic* | *janela* | *todos* | | | | | | |
| *Exemplo GOG: nome do jogo* | *GOG* | *tela cheia exclusiva* | *todos* | | | | | | |
| *Exemplo manual: `.exe` solto* | *manual* | | *todos* | | | | | | |
| *Exemplo anti-cheat, offline (bloco 10)* | | | *todos* | | | | | | |
| | | | | | | | | | |
| | | | | | | | | | |
| | | | | | | | | | |
| | | | | | | | | | |
| | | | | | | | | | |

Mais casos de sessão, com **um** jogo qualquer da lista:

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 5.1 | Segure View + Menu por menos de 1,5 s e solte | Nada acontece e o jogo segue recebendo os botões (CA-33) | |
| 5.2 | Com o jogo aberto, aperte botões do controle sem segurar a combinação, se o SofaLauncher estiver em segundo plano | O SofaLauncher não reage (CA-12) | |
| 5.3 | Painel > **Fechar jogo** > confirmar | Tela "Fechando…", o jogo fecha normalmente (CA-35) | |
| 5.4 | Se um jogo travar: Fechar jogo e aguarde 10 s | Aparece "Forçar fechamento" com Cancelar em foco (CA-35, E-04) | |
| 5.5 | Com um jogo aberto, volte à biblioteca e aperte A em **outro** jogo | "Fechar e abrir…?" com Cancelar em foco (CA-31) | |
| 5.6 | Toque em A duas vezes seguidas na tela "Abrindo…" | O segundo toque não faz nada nos primeiros 0,6 s (CA-77) | |
| 5.7 | Em "Abrindo…", aperte Voltar e espere o jogo abrir | O jogo abre depois e a sessão vale normalmente (CA-77) | |
| 5.8 | Jogo que reinicia sozinho (se algum fizer isso) | O SofaLauncher não acha que o jogo fechou (E-14) | |
| 5.9 | Jogo que abre o lançador da empresa antes do jogo | A sessão continua. Anote se o lançador aceita o controle (E-01, R-03) | |
| 5.10 | Jogo que exige administrador (se tiver; ou um `.exe` marcado "Executar como administrador") | Aviso do SofaLauncher antes do UAC, "Continuar / Voltar". Voltar não abre o UAC (CA-73). No UAC, use o teclado. A combinação de retorno pode falhar aí e o aviso diz isso (E-23) | |
| 5.11 | Abra um jogo pela Steam/atalho, fora do SofaLauncher | Não conta como sessão e a combinação não funciona (E-07). Só para confirmar | |
| 5.12 | Veja o Gerenciador de Tarefas com um jogo aberto e o SofaLauncher em segundo plano por 5 min | CPU do SofaLauncher abaixo de 1% (RN-21). Anote o maior valor visto | |
| 5.13 | Quantos cliques para abrir o último jogo jogado, a partir da tela inicial? | 1 (A) (M-03) | |
| 5.14 | Cadastre como jogo manual um `.exe` que abre o jogo de **outra pasta** (um atalho-lançador, por exemplo). Abra pelo SofaLauncher, volte ao painel e use **Fechar jogo** | O SofaLauncher só fecha o que ele iniciou e o que pertence ao jogo. Nenhum outro programa seu é fechado (SEC-15, SEC-17). Se o jogo real ficar aberto porque está fora da pasta do lançador, anote: é limitação conhecida da regra de sessão | |

## Bloco 6. Volume (15 min)

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 6.1 | Menu > Volume, direita e esquerda | 5 pontos por toque, de 0 a 100. O número aparece **na hora**. Anote se parecer lento (a meta é 200 ms) (CA-38) | |
| 6.2 | RT e LT na biblioteca | Sobe e desce 5 por puxada. O indicador some após 2 s (CA-64) | |
| 6.3 | Mude o volume pelo teclado do PC ou pelo controle remoto da TV (HDMI-CEC) e abra o menu | O nível mostrado é o atual do Windows (CA-41) | |
| 6.4 | Mudo: ligue, aumente o volume | O mudo sai e o volume sobe um passo (CA-39, CA-40) | |
| 6.5 | **Troque o dispositivo de áudio padrão** (de TV para fones, por exemplo) com o SofaLauncher aberto e mexa no volume | O SofaLauncher controla o novo dispositivo e mostra o nível dele | |
| 6.6 | Desative todos os dispositivos de áudio de saída (Configurações do som) e abra o Volume | "Nenhum dispositivo de áudio encontrado", botões desabilitados (CA-42). **Reative depois.** | |
| 6.7 | LT/RT dentro do menu rápido, de um diálogo ou com um jogo em primeiro plano | Nada muda (CA-65) | |

## Bloco 7. Energia (35 min)

Salve seu trabalho antes. Os testes desligam e reiniciam o PC de verdade.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 7.1 | Menu > Reiniciar, abra e depois cancele (B) | Diálogo com Cancelar em foco, B cancela, nada acontece (CA-43) | |
| 7.2 | Abra o **Bloco de Notas**, digite algo e **não salve**. No SofaLauncher, Desligar > confirmar | O Windows mostra a **própria tela** dizendo que o Bloco de Notas impede o desligamento. O SofaLauncher não força nada (CA-49). Cancele a tela do Windows | |
| 7.3 | Com um jogo aberto, escolha qualquer ação de energia | O aviso diz o nome do jogo e que o progresso não salvo pode ser perdido (CA-48) | |
| 7.4 | Reiniciar > confirmar (com o Bloco de Notas fechado) | Reinicia em até ~3 s. Com "Iniciar com o Windows" ligado, o SofaLauncher abre depois do login (CA-45) | |
| 7.5 | Se o bloco 0 mostrou que existe S3: Suspender > confirmar. Acorde pelo teclado ou controle | Suspende. Ao acordar, o SofaLauncher está na mesma tela. O controle Bluetooth volta a funcionar sozinho, ou anote o que precisou fazer (CA-46) | |
| 7.6 | Se o bloco 0 mostrou que **não** existe suspensão | "Suspender" não aparece ou aparece desabilitado com "Suspensão indisponível neste PC" (CA-47) | |
| 7.7 | Antes de desligar, veja Opções de energia > "Ativar inicialização rápida" e anote se ela está **ligada**. Desligar > confirmar (por último) | O Windows desliga em até 3 s, sem forçar, igual ao Desligar do menu Iniciar. Com a inicialização rápida ligada, ela é usada (CA-44, RN-15) | |
| 7.8 | Ligue o PC pelo botão. Depois de entrar na área de trabalho, não toque no controle por 10 s. Se ele for Bluetooth, anote se reconecta sozinho | O controle reconecta sem parear de novo e o SofaLauncher responde (D-44). Se não reconectar só depois do desligamento com inicialização rápida, anote: é o ponto que a D-44 quer observar. Compare com 7.4 (Reiniciar é completo) | |
| 7.9 | Repita 7.7 e 7.8 com a inicialização rápida **desligada**, se quiser | Desligamento completo, sem diferença visível para você. Anote o tempo do boot | |

Anote também: **"Suspender" existe no PC da sala?** (sim/não, e o que o `powercfg /a` disse).

## Bloco 7A. Fechar o launcher (25 min)

Salve seu trabalho antes. Aqui você fecha **só o SofaLauncher**; o PC e os jogos continuam ligados. Para reabrir, use o atalho do menu Iniciar com teclado ou mouse.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 7A.1 | Abra o menu rápido (Menu no controle ou F1) e leia a lista | "Fechar o launcher" aparece no grupo de Desligar, Reiniciar e Suspender, sem ícone vermelho. **Não existe** "Sair do launcher" (CA-05, CA-82) | |
| 7A.2 | Sem jogo aberto, escolha "Fechar o launcher" | Diálogo "Fechar o launcher?" com "O SofaLauncher fecha e o Windows mostra a área de trabalho." e a orientação para reabrir. **Cancelar** fica à esquerda e já em foco; "Fechar o launcher" à direita (CA-82, RN-34) | |
| 7A.3 | Compare o texto com a configuração: com "Iniciar com o Windows" **ligado**, o texto cita o atalho **ou reiniciar o PC**; com **desligado**, só o atalho (D-58). Se quiser, vire a opção em Configurações e abra o diálogo de novo | O texto muda conforme a opção | |
| 7A.4 | Cancele de três jeitos, um por vez: B no controle, Esc e Backspace no teclado, Enter em "Cancelar" | O diálogo fecha e **nada** é fechado (CA-82, RN-14) | |
| 7A.5 | Repita 7A.1, 7A.2 e 7A.4 só com o **teclado** (F1, setas, Enter, Esc), uma vez com o controle ligado e outra sem | Tudo funciona igual ao controle (RN-07, CA-80, CA-82) | |
| 7A.6 | Confirme "Fechar o launcher" sem jogo aberto | O SofaLauncher fecha e aparece a área de trabalho do Windows (CA-82) | |
| 7A.7 | Abra o SofaLauncher pelo atalho do menu Iniciar (teclado ou mouse) | Abre normal, em tela cheia, com a biblioteca e os dados iguais (CA-25, CA-82). "Iniciar com o Windows" continua como estava | |
| 7A.8 | Abra um jogo pelo SofaLauncher. Com o jogo rodando, volte ao painel (View + Menu, 1,5 s) > Menu rápido > "Fechar o launcher" | O diálogo diz "Só o SofaLauncher fecha.", a orientação de reabrir (com "depois de sair do jogo" se "Iniciar com o Windows" estiver ligado) e um aviso âmbar: "<nome do jogo> continuará aberto. Nenhum progresso se perde." Não fala em perder progresso, como o Desligar fala (CA-82, CA-48) | |
| 7A.9 | Confirme | O SofaLauncher fecha. O jogo **continua aberto**: não fecha, não é minimizado nem pausado, e o SofaLauncher não o puxa para a frente (o Windows decide qual janela aparece) (CA-82, D-57) | |
| 7A.10 | Com o launcher fechado e o jogo aberto, segure View + Menu por 2 s | **Nada acontece.** O jogo recebe os botões normalmente (E-24) | |
| 7A.11 | Saia do jogo pelo menu dele, com o launcher ainda fechado | Aparece a área de trabalho e **nada** volta à frente sozinho (E-25) | |
| 7A.12 | Repita 7A.8 e 7A.9. Com o jogo ainda aberto, reabra o SofaLauncher pelo atalho | Abre normal. O card do jogo **não** mostra "Em andamento", e o jogo está no topo dos recentes (D-54, CA-75). View + Menu **não** traz o launcher para esse jogo (E-07, E-24) | |
| 7A.13 | Ainda com esse jogo aberto, aperte A no card dele | Anote o que acontece. Pode aparecer "Não foi possível confirmar que <jogo> abriu" se o jogo só trouxer a própria janela (E-26, R-12). Não é falha, só observação | |
| 7A.14 | Feche o launcher e reabra pelo atalho **logo em seguida** (1 a 2 s) | Às vezes nada aparece; abrir de novo resolve (E-27, R-13). Anote se aconteceu | |
| 7A.15 | Com "Iniciar com o Windows" **ligado**: feche o launcher e reinicie o PC pelo Windows | O SofaLauncher abre sozinho no login (CA-82 item 5, CA-01) | |
| 7A.16 | Com "Iniciar com o Windows" **desligado**: feche o launcher e reinicie o PC | **Não** abre sozinho; só pelo atalho. Ligue a opção de novo depois | |
| 7A.17 | Depois destes testes, rode o coletor e abra `logs\sofa.log` do zip. Procure a linha "fechar o launcher" | Cada fechamento aparece com o estado da sessão e o identificador do jogo, **sem** caminho de arquivo (SEC-18). Sem jogo: "sem-sessao" ou "Idle" | |

## Bloco 8. Telas e TV (30 min)

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 8.1 | Sentado a 3 m, passe por biblioteca, menu, painel, configurações | Leio tudo sem esforço e o item em foco se destaca (CA-54, CA-55). Anote o que ficou pequeno | |
| 8.2 | Observe as bordas da TV (overscan) | Nenhum texto ou botão cortado. Capas cortadas na ponta da fileira são esperadas (RN-32) | |
| 8.3 | A TV em 1080p e em 4K, e escala 100% a 300% (o que a TV aguentar) | Nada cortado e o texto proporcional (CA-54) | |
| 8.4 | O cursor do mouse some após 3 s parado e volta ao mexer (CA-03) | Não há barra de título nem barra de tarefas | |
| 8.5 | **Duas telas:** Configurações > Tela, escolha a outra tela | O SofaLauncher passa para ela na hora, em tela cheia (CA-66) | |
| 8.6 | Feche e reabra o SofaLauncher | Abre na tela escolhida (CA-66) | |
| 8.7 | Desligue a tela escolhida (tire o cabo HDMI) | O SofaLauncher vai para a principal e a escolha continua gravada (CA-67) | |
| 8.8 | Religue a tela e reabra o SofaLauncher | Abre de novo na tela escolhida (CA-67) | |
| 8.9 | Desligue a TV com o PC ligado e ligue de novo | O SofaLauncher continua em tela cheia na mesma tela (E-09) | |

## Bloco 9. Teclado e mouse (10 min)

O teclado cobre todas as ações do controle, menos a combinação de retorno. Teclas: setas = direcional, Enter = A, Esc ou Backspace = B, F1 ou tecla de aplicativo = Menu, F2 = View (opções do jogo), PgUp e PgDn = LB e RB, `[` e `]` = LT e RT (volume na biblioteca).

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 9.1 | Sem controle: setas, Enter, Esc, Backspace, F1 | Navega, abre, volta e abre o Menu (RN-07, CA-80) | |
| 9.2 | Abra as Opções do jogo com **F2**, renomeie, oculte e restaure só pelo teclado | Funciona. Dentro do campo de nome, Backspace apaga letras e não sai da tela (CA-61, CA-80) | |
| 9.3 | PgDn e PgUp na biblioteca e na tela "Todos os jogos"; setas para cima e para baixo na grade longa | Pula linhas e a grade acompanha o foco, sem travar numa linha | |
| 9.4 | `]` e `[` na biblioteca; dentro do menu rápido e de um diálogo | Sobe e desce o volume na biblioteca; nos outros lugares, nada (CA-64, CA-65) | |
| 9.5 | Com um jogo aberto, aperte F1 e F2 juntas, e várias outras teclas | O SofaLauncher **não** vem à frente: a combinação de retorno só existe no controle (CA-80, RN-07). Anote se alguma tecla trouxe o SofaLauncher | |
| 9.6 | Em todo diálogo com dois botões (Desligar, Ocultar, Fechar jogo) | Cancelar fica à esquerda e a ação principal à direita; o foco começa em Cancelar (CA-81, RN-34) | |

## Bloco 10. Jogo com anti-cheat, offline (20 min)

Use **modo offline** e uma conta que você não se importe de perder, se possível (D-30). O risco é baixo, mas a tecla sintética do degrau 2 (F24) pode ser lida como macro por algum anti-cheat.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 10.1 | Abra o jogo pelo SofaLauncher | Abre e o anti-cheat não reclama do SofaLauncher | |
| 10.2 | Segure View + Menu e volte ao SofaLauncher, 3 vezes | Volta em até 3 s e o anti-cheat não acusa nada. Anote qualquer aviso ou kick | |
| 10.3 | Se houve problema: desligue o degrau 2 (bloco 12) e repita | Anote se o problema parou | |

## Bloco 11. Conta padrão e outro administrador (D-31) (20 min)

Só vale se você tiver uma conta padrão (comum) e a senha de outro administrador. Se o bloco 1 já foi feito com essas contas, boa parte está feita. Falta conferir:

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 11.1 | Entre na conta **padrão** e abra o SofaLauncher, jogue um jogo, ajuste volume, desligue pelo SofaLauncher | O UAC **nunca** aparece, exceto em jogo que exige administrador (CA-74) | |
| 11.2 | Entre na **conta do outro administrador** (se existir no PC) e abra o SofaLauncher | O SofaLauncher dessa conta tem a própria biblioteca e não herda a da conta padrão (D-31). Sem "Iniciar com o Windows" que você não ligou | |
| 11.3 | Rode `Get-ScheduledTask 'Sofa Autostart*'` no PowerShell, ou veja o resultado do coletor | A tarefa (se existir) é só da conta padrão, nunca da conta do administrador do UAC (SEC-21, SEC-22) | |

## Bloco 12. Os degraus 2 e 3 e os logs para a segurança (SG-10) (20 min)

A escada de retorno tenta 4 jeitos para trazer o SofaLauncher à frente, na ordem **1, 3, 2, 4**: o pedido simples, "vincular à entrada da janela do jogo", **uma tecla F24 sintética** e minimizar o jogo. Hoje os degraus 2 e 3 vêm **ligados** (decisão da segurança, rev. 5). Este bloco serve para a segurança decidir se eles continuam assim.

1. Abra o arquivo `%LOCALAPPDATA%\app.sofa.launcher\settings.json` (Bloco de Notas) **com o SofaLauncher fechado** e acrescente a seção `advanced` (o arquivo precisa manter `"schemaVersion": 1` e os campos que já tem; os nomes distinguem maiúsculas de minúsculas):

   ```json
   "schemaVersion": 1,
   "advanced": { "diagnostics": { "level": "debug" } }
   ```

   Isso liga o log detalhado: ele registra, por retorno, quais degraus foram tentados, quantos ms levou cada um e o resultado.
2. Faça o bloco 5 com os degraus **ligados** (padrão: 1, 3, 2, 4).
3. Para testar só 1 e 4, feche o SofaLauncher e use:

   ```json
   "schemaVersion": 1,
   "advanced": { "diagnostics": { "level": "debug" }, "foreground": { "step2": false, "step3": false } }
   ```

   Repita o bloco 5. Compare os tempos. Se o jogo não voltar com 1 e 4, anote: é o resultado esperado e é o que a segurança quer saber.
4. Se um jogo ou anti-cheat reagir à tecla F24, deixe só `"step2": false` e repita. Anote o que mudou.
5. Se errar um nome ou um valor, o SofaLauncher ignora só aquele campo, usa o padrão dele e avisa no log. Apagar a seção `advanced` volta tudo ao padrão.

## Bloco 13. Coletar os logs, antes de desinstalar (5 min)

Faça esta coleta **antes** do bloco 14: a desinstalação pode apagar os logs.

1. Abra o PowerShell na pasta onde copiou o `coletar-logs.ps1` (o pendrive do início). O instalador não traz esse arquivo.
2. Com o SofaLauncher **aberto**, rode: `powershell -ExecutionPolicy Bypass -File .\coletar-logs.ps1`
3. O zip aparece na área de trabalho: `sofa-logs-AAAAMMDD-HHMMSS.zip`. Abra, confira o `LEIA-ME.txt` e guarde.

O que vai no zip: os logs do SofaLauncher, o `settings.json`, a versão do Windows, o hardware, as telas, o WebView2, o resultado de `powercfg /a` e o estado do SofaLauncher instalado (nível de permissão, lista de quem pode mexer na pasta, tarefa agendada, entrada de início). O que **não** vai: a lista de jogos, as capas, seu nome de usuário e o nome do PC (viram `<usuario>` e `<pc>`).

Os logs rodam em até 3 arquivos de 5 MB, e o mais antigo se perde. Se quiser separar por bloco, rode o coletor logo depois dos blocos longos (5, 10 e 12).

## Bloco 14. Desinstalação (30 min)

**Esta é a última etapa.** Desinstalar **sem manter os dados** apaga a biblioteca e as configurações do SofaLauncher (nunca os seus jogos). Depois você refaz a biblioteca pelo "Atualizar biblioteca". Faça só no PC da sala.

| # | O que fazer | Esperado | Resultado |
|---|---|---|---|
| 14.1 | **Reinstalar por cima.** Com a biblioteca montada, rode o **mesmo instalador** de novo (confira o hash outra vez se for uma versão nova) | Atualiza sem apagar nada: biblioteca, configurações e "Iniciar com o Windows" continuam como estavam (CA-53). Abra o SofaLauncher e confira | |
| 14.2 | Desinstale em Configurações > Aplicativos > Aplicativos instalados > SofaLauncher, **mantendo os dados** (a caixa "Manter dados" vem marcada) | Remove o app, os atalhos e o início automático. Em "Aplicativos instalados" não sobra nenhuma entrada do SofaLauncher. Seus jogos no disco ficam intactos. A pasta `%LOCALAPPDATA%\app.sofa.launcher` continua (CA-52, CA-68). O Windows pede permissão de administrador | |
| 14.3 | Reinstale | Biblioteca e configurações voltam (CA-68) | |
| 14.4 | Abra o SofaLauncher e deixe-o **aberto**. Desinstale **desmarcando "Manter dados"** | O desinstalador fecha o SofaLauncher sozinho ou pede para você fechar. Se avisar que o SofaLauncher ainda está aberto, **nada é apagado**: feche e repita. Anote o texto exato (CA-68, SEC-25). Não rode `sofa.exe --purge-user-data` à mão | |
| 14.5 | Se algo travou na 14.4, feche o SofaLauncher, espere uns 10 s (a parte que desenha a tela, a pasta `EBWebView`, pode ficar em uso um pouco) e desinstale de novo, sem manter os dados | Biblioteca e configurações somem. Se algo da `EBWebView` sobrar, a pasta é mostrada para você. Os jogos e outras pastas não são tocados. Anote o que sobrou (CA-68, SEC-25, SEC-31) | |
| 14.6 | Confira que a pasta `C:\Program Files\Sofa` sumiu e que não há mais atalho do SofaLauncher | Sumiram (CA-52) | |
| 14.7 | Confira que o início automático saiu: `Get-ScheduledTask 'Sofa Autostart*'` não acha nada, e reiniciar o PC não abre o SofaLauncher | Sumiu (CA-52, RN-17) | |

## Bloco 15. Coleta final e envio (5 min)

1. Rode o coletor de novo (`powershell -ExecutionPolicy Bypass -File .\coletar-logs.ps1`). Como o SofaLauncher já foi removido, o zip só mostra o que sobrou no PC: o relatório deve dizer que não há tarefa nem entrada de início do SofaLauncher.
2. Envie os **dois zips** (o do bloco 13 e este) junto com as suas anotações.

## Fechamento

Escreva, em poucas linhas:

- O que atrapalhou mais no dia a dia.
- Qual jogo ou situação você não conseguiu testar e por quê.
- Se o SofaLauncher serviria hoje como tela inicial da sala.

*Casos "Não deu" entram no relatório como não verificados (D-33). Nenhum item aqui bloqueia o aceite de uso de 2 semanas (QA-14).*
