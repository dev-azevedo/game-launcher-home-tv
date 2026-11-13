# Spec: launcher-tv

| Campo | Valor |
|---|---|
| Feature | launcher-tv (MVP do Sofá) |
| Produto | Sofá (identificadores técnicos: `sofa`; identificador do app e pasta de dados: `app.sofa.launcher`) |
| Branch | `feature/launcher-tv-20261113` |
| Status | Revisada com as decisões do usuário, da arquitetura e de segurança (seção 14) |
| Plataforma | Windows 10 e Windows 11, app desktop instalado |
| Stack provável | Tauri (Rust + UI web), repo único. Decisão final do arquiteto |
| Data | 2026-11-13 |

---

## 1. Problema

O usuário joga no PC ligado à TV, sentado no sofá, com um controle Xbox. Hoje, para abrir um jogo, precisa levantar ou buscar teclado e mouse: achar o atalho na área de trabalho, abrir a Steam (ou outra loja), clicar no jogo. O mesmo vale para sair do jogo, mexer no volume do Windows e desligar o PC. A área de trabalho do Windows não foi feita para ser lida a 3 metros nem navegada com controle.

O que ele quer: ligar o PC, cair numa tela cheia com os jogos dele e fazer tudo (escolher, jogar, voltar, ajustar volume, desligar) só com o controle.

Existem alternativas parecidas (Steam Big Picture, a experiência de tela cheia do app Xbox, Playnite em modo tela cheia). O usuário não testou nenhuma e decidiu seguir com um app próprio, simples, que junta jogos de origens diferentes e cobre volume e energia do PC no mesmo lugar (D-12).

## 2. Objetivo e métricas de sucesso

**Objetivo:** do boot ao desligamento, usar o PC na TV sem tocar em teclado e mouse em uso normal.

Uso pessoal, um usuário, sem telemetria. As métricas são verificadas pelo próprio usuário e pelo QA:

| ID | Métrica | Meta |
|---|---|---|
| M-01 | Sessões (boot até desligar) concluídas sem teclado/mouse, em 2 semanas de uso | 100% das sessões em que o Windows entra direto na área de trabalho (ver RN-19) |
| M-02 | Tempo entre o Windows terminar de carregar e o launcher estar navegável | até 5 s |
| M-03 | Cliques no controle para abrir o último jogo jogado, a partir da tela inicial | 1 (botão A) |
| M-04 | Tempo entre segurar a combinação de retorno e o launcher aparecer na frente do jogo | até 3 s |
| M-05 | Tempo de resposta do foco a um comando de direção no launcher | até 100 ms percebidos |

## 3. Personas e cenários

### Persona única: jogador de sofá

- Dono do PC, único usuário. Sabe usar Windows e configurou o PC sozinho.
- Joga na TV da sala (1080p ou 4K), a cerca de 2 a 3 metros da tela.
- Usa controle Xbox (com fio, adaptador sem fio ou Bluetooth), que o Windows reconhece sem driver extra.
- Tem jogos na Steam, na Epic Games e no GOG, além de alguns executáveis soltos.
- Teclado e mouse existem, mas ficam guardados ou longe do sofá.

### Cenários (interface de 10 pés, lida a distância)

**C-01: Noite de jogo.** Liga o PC pelo botão do gabinete. O Windows entra na área de trabalho e o launcher abre sozinho em tela cheia na TV. Ele vê o jogo que jogou ontem em destaque, aperta A e o jogo abre.

**C-02: Trocar de jogo.** No meio da partida, segura View + Menu. O launcher aparece por cima com "Voltar ao jogo / Fechar jogo". Escolhe fechar, confirma e volta à biblioteca para abrir outro.

**C-03: Volume alto demais.** Na biblioteca, o som está alto. Ele puxa o LT algumas vezes e vê o nível baixar na tela, sem pegar o controle remoto da TV.

**C-04: Hora de dormir.** Fecha o jogo, abre o menu rápido, escolhe "Desligar", confirma. O PC desliga.

**C-05: Controle sem bateria.** O controle desconecta. O launcher mostra que não há controle e como reconectar. Ao trocar as pilhas, ele retoma de onde parou.

## 4. Termos

Usar estes nomes na UI, no código e nos próximos documentos.

| Termo | Definição |
|---|---|
| Biblioteca | Conjunto de jogos que o launcher conhece e mostra. |
| Jogo | Item da biblioteca que o usuário pode iniciar. Tem nome, origem, capa e forma de iniciar. |
| Origem | De onde o jogo veio para a biblioteca: `Manual` (cadastrado pelo usuário), `Steam`, `Epic` ou `GOG` (detectados entre os jogos instalados). |
| Item oculto | Jogo que o usuário escondeu da biblioteca. Não aparece na tela inicial, não volta em atualizações e pode ser restaurado nas configurações. Nenhum arquivo é apagado. |
| Último jogado | Data de início da sessão de jogo mais recente iniciada pelo launcher. |
| Loja | App de uma origem detectada (Steam, Epic Games Launcher, GOG Galaxy). |
| Sessão de jogo | Período entre iniciar um jogo e o launcher detectar que ele terminou. No MVP há no máximo uma sessão ativa. |
| Combinação de retorno | View + Menu segurados por 1,5 s durante uma sessão de jogo; traz o launcher para a frente. |
| Painel do jogo em andamento | Tela do launcher mostrada por cima do jogo após a combinação de retorno. |
| Menu rápido | Menu aberto pelo botão Menu com volume, energia, configurações e sair. |
| Ação de energia | Desligar, reiniciar ou suspender o PC. |
| Monitor do launcher | Tela onde o launcher abre. Padrão: monitor principal do Windows; pode ser trocado nas configurações. |

## 5. Escopo

### Dentro do MVP

1. App instalado para todo o PC, tela cheia, uma instância por vez.
2. Navegação 100% por controle Xbox (XInput), com teclado e mouse como alternativa de emergência.
3. Biblioteca com cadastro manual de executável (só `.exe` em disco local) e detecção automática dos jogos instalados na Steam, na Epic Games e no GOG, filtrando itens da Steam que não são jogos.
4. Renomear jogo pelo teclado físico; ocultar qualquer item pelo controle e restaurá-lo nas configurações.
5. Capa por jogo: imagem local da loja quando houver; senão ícone do executável ou placeholder com o nome.
6. Ordenação por último jogado e acesso à lista completa em ordem alfabética.
7. Iniciar jogo, acompanhar a sessão e voltar ao launcher quando o jogo fecha.
8. Combinação de retorno durante o jogo, com opções de voltar ao jogo ou fechar o jogo.
9. Volume do sistema: aumentar, diminuir e mudo pelo menu rápido; aumentar e diminuir com RT/LT na biblioteca; indicador na tela.
10. Desligar, reiniciar e suspender o PC, sempre com confirmação.
11. Iniciar com o Windows (opção ligada/desligada, por usuário).
12. Escolha do monitor do launcher nas configurações.
13. Comportamento definido sem controle conectado e ao desconectar.
14. Instalador para Windows 10 e 11, com desinstalador.

### Fora do MVP

- Detecção de Xbox/Game Pass, EA, Ubisoft e outras lojas.
- Jogos não instalados das lojas (para instalar pela loja).
- Download de capas e metadados da internet (SteamGridDB, IGDB etc.).
- Instalar, atualizar ou desinstalar jogos.
- Teclado virtual na tela.
- Jogo manual com argumentos de linha de comando ou com arquivo que não seja `.exe` (`.lnk`, `.bat`, `.cmd`, `.ps1` etc.).
- Jogo manual em pasta de rede ou unidade de rede mapeada.
- Rodar o launcher como administrador.
- Abrir jogos GOG pelo GOG Galaxy (sincronização de saves na nuvem e overlay do Galaxy).
- Login, contas, nuvem, sincronização, conquistas, amigos, chat.
- Vários perfis de usuário.
- Substituir o shell do Windows (rodar no lugar do Explorer).
- Login do Windows pela tela de bloqueio com controle (configurar logon automático ou Windows Hello é responsabilidade do usuário, ver RN-19).
- Controles que não são XInput (DualShock/DualSense, genéricos DirectInput) sem camada de terceiros.
- Mapeamento de botões configurável pelo usuário (inclusive a combinação de retorno).
- Overlay desenhado dentro do jogo (estilo Steam overlay). O retorno traz a janela do launcher para a frente; não injeta nada no jogo.
- Pausar o jogo automaticamente ao voltar ao launcher.
- Hibernar, desligar a tela, bloquear a tela, trocar usuário.
- Controle de volume por aplicativo e troca de dispositivo de saída de áudio.
- Atalho de volume durante o jogo.
- Detecção automática da TV entre os monitores.
- Assinatura de código do instalador.
- Telemetria e envio de dados para qualquer servidor.
- Atualização automática do launcher.

## 6. Premissas de UI

Definidas pelo usuário. A spec não fixa identidade visual; logo, cores e tipografia estão no guia de marca.

- **Público:** só o usuário. Não há onboarding de marketing nem textos para terceiros.
- **Nome do produto:** Sofá. Identificadores técnicos usam `sofa`; identificador do app e pasta de dados, `app.sofa.launcher`.
- **Marca:** direção A, "Palco", em `docs/brand/sofa/`.
- **Linguagem visual:** "console moderno", na linha do dashboard do Xbox e do PS5. Cards grandes, capas em destaque, o jogo em foco ampliado ou realçado, pouca informação por tela.
- **Leitura a distância:** valem CA-54 e CA-55.

## 7. User stories

| ID | História | Prioridade |
|---|---|---|
| US-01 | Como jogador de sofá, quero que o launcher abra sozinho em tela cheia quando o PC liga, para não precisar de teclado e mouse. | P1 |
| US-02 | Como jogador de sofá, quero navegar por todos os meus jogos com o controle, para escolher o que jogar de longe. | P1 |
| US-03 | Como jogador de sofá, quero que meus jogos da Steam, da Epic Games e do GOG apareçam sozinhos, para não cadastrar um por um. | P1 |
| US-04 | Como jogador de sofá, quero adicionar um jogo apontando para o executável e poder renomeá-lo, para incluir jogos fora das lojas com um nome legível. | P1 |
| US-05 | Como jogador de sofá, quero ocultar qualquer item da biblioteca e restaurá-lo depois, para esconder o que não jogo mais ou o que não é jogo. | P2 |
| US-06 | Como jogador de sofá, quero iniciar um jogo apertando A, para começar a jogar rápido. | P1 |
| US-07 | Como jogador de sofá, quero voltar ao launcher durante o jogo com uma combinação de botões, para fechar o jogo ou trocar de jogo sem teclado. | P1 |
| US-08 | Como jogador de sofá, quero que o launcher volte para a frente quando eu saio do jogo pelo menu dele, para não cair na área de trabalho. | P1 |
| US-09 | Como jogador de sofá, quero aumentar, diminuir e silenciar o volume do PC pelo launcher, inclusive com os gatilhos na biblioteca, para não depender do controle remoto da TV ou do teclado. | P1 |
| US-10 | Como jogador de sofá, quero desligar, reiniciar ou suspender o PC pelo launcher com confirmação, para encerrar a noite sem levantar e sem desligar sem querer. | P1 |
| US-11 | Como jogador de sofá, quero ver um aviso claro quando não há controle conectado, para saber o que fazer. | P1 |
| US-12 | Como jogador de sofá, quero instalar o launcher com um instalador comum, para configurar o PC uma vez e esquecer. | P1 |
| US-13 | Como jogador de sofá, quero ligar e desligar a inicialização com o Windows, para usar o PC normalmente quando não estou na TV. | P2 |
| US-14 | Como jogador de sofá, quero sair do launcher para a área de trabalho, para usar o PC de outro jeito quando precisar. | P2 |
| US-15 | Como jogador de sofá com monitor e TV no mesmo PC, quero escolher em qual tela o launcher abre, para ele aparecer na TV sem mudar o monitor principal do Windows. | P2 |

## 8. Mapeamento padrão do controle

Fixo no MVP. Referência para CA e QA.

| Entrada | Na biblioteca | Em menus, diálogos e painel do jogo em andamento | Durante sessão de jogo (launcher em segundo plano) |
|---|---|---|---|
| D-pad / analógico esquerdo | Move o foco | Move o foco | Ignorado pelo launcher |
| A | Inicia o jogo em foco | Confirma a opção em foco | Ignorado |
| B | Sem efeito na tela inicial; volta um nível nas outras | Cancela / fecha | Ignorado |
| Menu (≡) | Abre o menu rápido | Fecha o menu rápido | Ignorado (salvo na combinação de retorno) |
| View (⧉) | Abre opções do jogo em foco (renomear, ocultar) | Sem efeito | Ignorado (salvo na combinação de retorno) |
| LB / RB | Pula uma página/linha para trás / para frente | Sem efeito | Ignorado |
| LT / RT | Diminui / aumenta o volume em 5 pontos | Sem efeito (no menu rápido o volume usa esquerda/direita) | Ignorado |
| View + Menu segurados 1,5 s | Sem efeito | Sem efeito | Traz o launcher para a frente (combinação de retorno) |

LT/RT não tinham outro uso no mapeamento; não há conflito. LB/RB continuam com a paginação.

Teclado de emergência (RN-07): setas = D-pad, Enter = A, Esc = B, tecla de aplicativo ou F1 = Menu.

## 9. Critérios de aceite

### Inicialização, tela cheia e instância

**CA-01**: Dado que o launcher está instalado com "Iniciar com o Windows" ligado, quando o usuário entra na área de trabalho do Windows, então o launcher abre em tela cheia sem borda no monitor do launcher (padrão: monitor principal) e fica navegável em até 5 s (M-02).

**CA-02**: Dado que o launcher está aberto, quando o usuário tenta abri-lo de novo (atalho, menu Iniciar), então nenhuma segunda janela é criada e a janela existente vem para a frente.

**CA-03**: Dado que o launcher está em tela cheia, quando qualquer tela é exibida, então não aparecem barra de título, barra de tarefas nem cursor do mouse parado (o cursor some após 3 s sem movimento e volta ao mexer o mouse).

**CA-04**: Dado que o usuário está nas configurações, quando desliga "Iniciar com o Windows" e reinicia o PC, então o launcher não abre sozinho; ao religar a opção, volta a abrir no próximo login.

**CA-05**: Dado que o usuário escolhe "Sair do launcher" no menu rápido, quando confirma, então o launcher fecha e o Windows mostra a área de trabalho. Se houver sessão de jogo ativa, o diálogo avisa que o jogo continuará aberto.

### Navegação por controle

**CA-06**: Dado um controle Xbox conectado (com fio, adaptador ou Bluetooth), quando o usuário usa D-pad ou analógico esquerdo, então o foco se move entre os itens na direção pressionada e o item em foco fica destacado de forma visível a 3 m.

**CA-07**: Dado que o usuário segura uma direção, quando passam 400 ms, então o foco passa a repetir o movimento em intervalos regulares até soltar (navegação contínua em listas longas).

**CA-08**: Dado que o analógico esquerdo está em repouso com folga mecânica (drift leve), quando o launcher lê a entrada, então o foco não se move sozinho (zona morta, RN-06).

**CA-09**: Dado qualquer tela, menu ou diálogo do launcher, quando o usuário tenta concluir a ação dela, então é possível fazer isso só com o controle, sem teclado e mouse. Exceções: as situações listadas em RN-07 e RN-19 e renomear jogo (CA-61).

**CA-10**: Dado qualquer tela que não seja a inicial, quando o usuário aperta B, então volta para a tela anterior ou fecha o diálogo, sem executar ação.

**CA-11**: Dado mais de um controle conectado, quando qualquer um deles envia comandos, então o launcher responde a todos da mesma forma.

**CA-12**: Dado que o launcher está em segundo plano durante uma sessão de jogo, quando o usuário aperta botões no controle, então o launcher não executa nenhuma ação, exceto ao detectar a combinação de retorno.

### Controle ausente ou desconectado

**CA-13**: Dado que nenhum controle está conectado, quando o launcher abre, então exibe a biblioteca normalmente e uma faixa visível "Nenhum controle conectado. Ligue o controle ou conecte o cabo.", e aceita teclado e mouse.

**CA-14**: Dado que o controle desconecta com o launcher em primeiro plano, quando a desconexão é detectada, então aparece um aviso em até 2 s, o foco atual é mantido e nada é acionado.

**CA-15**: Dado o aviso de controle ausente, quando um controle conecta, então o aviso some em até 2 s e o controle passa a funcionar sem reiniciar o launcher.

### Biblioteca

**CA-16**: Dado que a Steam está instalada com jogos baixados (em qualquer pasta de biblioteca da Steam), quando o launcher abre ou o usuário escolhe "Atualizar biblioteca", então cada jogo Steam instalado aparece com o nome da Steam e a capa local da Steam, se existir.

**CA-17**: Dado que um jogo Steam foi desinstalado pela Steam, quando a biblioteca é atualizada, então o jogo some da biblioteca.

**CA-18**: Dado que a Steam não está instalada, quando a biblioteca é carregada, então nenhum erro aparece e só os jogos das outras origens são mostrados.

**CA-19**: Dado que o usuário escolhe "Adicionar jogo" nas configurações, quando navega pelas pastas com o controle e seleciona um `.exe`, então o jogo entra na biblioteca com o nome do arquivo (sem extensão) e o ícone do executável como capa. O navegador mostra só discos locais (fixos e removíveis), pastas e arquivos `.exe`, e não há campo para argumentos (RN-29).

**CA-20**: Dado que o usuário tenta adicionar um executável que já está na biblioteca, quando confirma, então o launcher avisa "Este jogo já está na biblioteca" e não cria duplicata.

**CA-21**: Dado qualquer item em foco (de qualquer origem), quando o usuário aperta View, escolhe "Ocultar" e confirma, então o item sai da biblioteca, nenhum arquivo é apagado do disco e o item não volta nas próximas atualizações (RN-04). Restaurar: CA-70.

**CA-22**: Dado uma biblioteca com jogos já iniciados pelo launcher, quando a tela inicial abre, então os jogos aparecem do mais recente para o mais antigo jogado (pela data de início da sessão, RN-05), e os nunca jogados vêm depois em ordem alfabética. O jogo em foco ao abrir é o último jogado.

**CA-23**: Dado uma biblioteca vazia (nenhuma loja com jogos e nenhum jogo manual), quando a tela inicial abre, então aparece uma mensagem com a ação "Adicionar jogo" já em foco.

**CA-24**: Dado um jogo sem capa disponível, quando aparece na biblioteca, então mostra um placeholder com o nome do jogo legível a 3 m.

**CA-25**: Dado que o launcher é fechado e aberto de novo, quando a biblioteca carrega, então jogos manuais, itens ocultos, nomes personalizados e datas de último jogo continuam iguais (persistência local, RN-18).

### Iniciar jogo e sessão

**CA-26**: Dado um jogo em foco e nenhuma sessão ativa, quando o usuário aperta A, então o launcher mostra "Abrindo <nome do jogo>…" e inicia o jogo. Jogos Steam e Epic iniciam pela loja, que é aberta antes se estiver fechada. Jogos GOG (inclusive os instalados pelo Galaxy) iniciam direto pelo executável do jogo, sem abrir o Galaxy.

**CA-27**: Dado que o jogo abriu, quando a janela do jogo aparece, então o launcher sai da frente e deixa o jogo em primeiro plano.

**CA-28**: Dado um jogo manual cujo executável não existe mais, quando o usuário tenta iniciá-lo, então aparece "Não encontramos o arquivo do jogo em <caminho>" com as opções "Ocultar" e "Voltar".

**CA-29**: Dado que o jogo foi iniciado, quando nenhum processo do jogo é detectado em até 60 s, então o launcher volta ao primeiro plano com "Não foi possível confirmar que <nome> abriu" e as opções "Tentar de novo" e "Voltar".

**CA-30**: Dado uma sessão de jogo ativa, quando o jogo é fechado pelo próprio menu do jogo, então o launcher volta ao primeiro plano em até 3 s, na biblioteca, com o jogo que acabou de fechar em foco e no topo dos recentes.

**CA-31**: Dado uma sessão de jogo ativa, quando o usuário volta à biblioteca e aperta A em outro jogo, então o launcher pergunta "<jogo atual> está aberto. Fechar e abrir <novo jogo>?" com "Cancelar" em foco por padrão.

### Voltar ao launcher durante o jogo

**CA-32**: Dado uma sessão de jogo ativa com o jogo em primeiro plano (janela, sem borda ou tela cheia), quando o usuário segura View + Menu por 1,5 s, então o launcher vem para a frente em até 3 s (M-04) e mostra o painel do jogo em andamento.

**CA-33**: Dado que o usuário aperta a combinação de retorno por menos tempo que o definido, quando solta, então nada acontece e o jogo segue recebendo os botões normalmente.

**CA-34**: Dado o painel do jogo em andamento, quando o usuário escolhe "Voltar ao jogo" ou aperta B, então o jogo volta ao primeiro plano no estado em que estava.

**CA-35**: Dado o painel do jogo em andamento, quando o usuário escolhe "Fechar jogo" e confirma, então o launcher pede ao jogo para fechar normalmente; se o jogo continuar aberto após 10 s, oferece "Forçar fechamento (o progresso não salvo será perdido)" com "Cancelar" em foco por padrão.

**CA-36**: Dado o painel do jogo em andamento, quando o usuário escolhe "Ir para a biblioteca", então a biblioteca aparece com o jogo ainda aberto em segundo plano, sem minimizar o jogo, e um indicador "Em andamento" no card do jogo; apertar A nesse card volta ao jogo.

**CA-37**: Dado o painel do jogo em andamento, quando o usuário abre o menu rápido, então volume e energia funcionam igual à biblioteca.

### Volume

**CA-38**: Dado o menu rápido aberto com "Volume" em foco, quando o usuário aperta direita/esquerda, então o volume do dispositivo de saída padrão do Windows sobe/desce 5 pontos percentuais por toque, limitado a 0 e 100, e o nível aparece na tela em até 200 ms.

**CA-39**: Dado o menu rápido aberto, quando o usuário escolhe "Mudo", então o áudio do sistema é silenciado e o indicador mostra o estado mudo; escolher de novo tira o mudo e o volume volta ao nível anterior.

**CA-40**: Dado que o sistema está no mudo, quando o usuário aumenta o volume (no menu rápido ou com RT na biblioteca), então o mudo é desativado e o volume sobe um passo.

**CA-41**: Dado que o volume foi alterado fora do launcher (teclado, TV ligada ao PC, outro app), quando o usuário abre o menu rápido, então o nível mostrado é o atual do Windows.

**CA-42**: Dado que não há dispositivo de saída de áudio ativo, quando o usuário abre o controle de volume, então aparece "Nenhum dispositivo de áudio encontrado" e os botões de volume ficam desabilitados.

### Energia

**CA-43**: Dado o menu rápido, quando o usuário escolhe "Desligar", "Reiniciar" ou "Suspender", então aparece um diálogo de confirmação com o nome da ação e "Cancelar" em foco por padrão; B cancela.

**CA-44**: Dado o diálogo de confirmação de "Desligar", quando o usuário confirma, então o Windows inicia o desligamento normal (não forçado) em até 3 s.

**CA-45**: Dado o diálogo de confirmação de "Reiniciar", quando o usuário confirma, então o Windows reinicia, e com "Iniciar com o Windows" ligado o launcher abre de novo após o login (CA-01).

**CA-46**: Dado o diálogo de confirmação de "Suspender", quando o usuário confirma, então o PC entra em suspensão; ao acordar, o launcher está na mesma tela em que estava e responde ao controle assim que o controle reconectar.

**CA-47**: Dado que a suspensão não está disponível no PC (desabilitada pelo Windows ou pelo hardware), quando o menu rápido abre, então "Suspender" não aparece ou aparece desabilitado com a explicação "Suspensão indisponível neste PC".

**CA-48**: Dado uma sessão de jogo ativa, quando o usuário escolhe qualquer ação de energia, então a confirmação avisa "<nome do jogo> está aberto. O progresso não salvo pode ser perdido."

**CA-49**: Dado que o Windows bloqueia o desligamento (por exemplo, outro app com trabalho não salvo), quando isso acontece, então o launcher não força nada; o usuário vê a tela do próprio Windows.

### Instalador

**CA-50**: Dado um PC com Windows 10 (versão com suporte) ou Windows 11, quando o usuário executa o instalador, então o Windows pede permissão de administrador (UAC) e o launcher é instalado em `Program Files` sem exigir etapas manuais extras. Se o runtime WebView2 faltar (comum só no Windows 10), o próprio instalador o baixa da Microsoft e instala; nesse caso precisa de internet e, sem conexão, mostra "Não foi possível baixar o WebView2. Conecte o PC à internet e tente de novo."

**CA-51**: Dado o instalador, quando a instalação termina, então há atalho no menu Iniciar, a opção "Iniciar com o Windows" foi oferecida como caixa marcada por padrão e a escolha foi respeitada para o usuário que executou o instalador.

**CA-52**: Dado o launcher instalado, quando o usuário desinstala por "Aplicativos instalados" do Windows (com UAC), então o app, os atalhos e a entrada de inicialização automática do usuário são removidos. Os jogos no disco não são tocados. Biblioteca e configurações seguem CA-68.

**CA-53**: Dado uma versão nova do instalador, quando o usuário a executa sobre uma versão instalada (com UAC), então o app é atualizado e a biblioteca, as configurações e a opção "Iniciar com o Windows" são mantidas.

### Interface de 10 pés

**CA-54**: Dado a TV em 1080p ou 4K, quando qualquer tela é exibida, então todo texto e elemento interativo fica dentro da área segura (margem de 5% em cada borda) e o texto do corpo tem pelo menos 24 px equivalentes a 1080p, escalando em 4K.

**CA-55**: Dado qualquer tela, quando o usuário olha a 3 m, então sempre existe exatamente um item em foco, visível e com contraste suficiente em relação ao fundo.

### Epic Games e GOG

**CA-56**: Dado que o Epic Games Launcher está instalado com jogos baixados, quando o launcher abre ou o usuário escolhe "Atualizar biblioteca", então cada jogo Epic instalado aparece com o nome da Epic e capa local, se existir; senão, ícone do executável ou placeholder (CA-24).

**CA-57**: Dado que há jogos instalados pelo GOG Galaxy, quando o launcher abre ou o usuário escolhe "Atualizar biblioteca", então cada jogo GOG instalado aparece com o nome do GOG e capa local, se existir; senão, ícone do executável ou placeholder (CA-24).

**CA-58**: Dado que um jogo Epic ou GOG foi desinstalado pela loja, quando a biblioteca é atualizada, então o jogo some da biblioteca.

**CA-59**: Dado que o Epic Games Launcher ou o GOG Galaxy não está instalado, quando a biblioteca é carregada, então nenhum erro aparece e só os jogos das outras origens são mostrados.

**CA-60**: Dado qualquer jogo na biblioteca, quando o card aparece, então mostra a origem (Manual, Steam, Epic ou GOG) de forma discreta, para diferenciar o mesmo jogo em lojas diferentes (RN-25).

### Renomear jogo

**CA-61**: Dado um jogo em foco, quando o usuário aperta View, escolhe "Renomear", digita o novo nome no teclado físico e confirma com Enter, então o card passa a mostrar o novo nome. Esc ou B cancelam sem alterar. Nome vazio não é aceito.

**CA-62**: Dado um jogo de loja renomeado, quando a biblioteca é atualizada, então o nome personalizado é mantido (RN-22). Nas opções do jogo existe "Restaurar nome original".

### Retorno com fallback

**CA-63**: Dado que o usuário segurou a combinação de retorno e o Windows não deixou o launcher vir para a frente, quando isso é detectado, então o launcher minimiza o jogo e mostra o painel do jogo em andamento, ainda dentro de 3 s (M-04). "Voltar ao jogo" restaura o jogo minimizado.

### Volume pelos gatilhos

**CA-64**: Dado a tela da biblioteca em primeiro plano, quando o usuário puxa LT ou RT, então o volume desce ou sobe 5 pontos percentuais por puxada, limitado a 0 e 100, e um indicador de volume aparece na tela em até 200 ms e some após 2 s sem uso. LT com o sistema no mudo baixa o nível guardado e mantém o mudo.

**CA-65**: Dado o menu rápido, um diálogo, o painel do jogo em andamento ou uma sessão de jogo com o launcher em segundo plano, quando o usuário puxa LT ou RT, então o volume não muda e nada é acionado.

### Monitor do launcher

**CA-66**: Dado mais de um monitor conectado, quando o usuário escolhe outro monitor em Configurações > Tela, então o launcher passa para esse monitor em tela cheia na hora e volta a abrir nele nas próximas inicializações.

**CA-67**: Dado que o monitor escolhido não está conectado (ao abrir o launcher ou durante o uso), quando o launcher percebe a ausência, então passa para o monitor principal do Windows. A preferência é mantida: na próxima inicialização com o monitor escolhido conectado, o launcher abre nele.

### Desinstalador

**CA-68**: Dado o desinstalador, quando ele roda, então pergunta se o usuário quer manter biblioteca e configurações, com "Manter dados" marcado por padrão. Marcado, os dados ficam e uma reinstalação os recupera. Desmarcado, o desinstalador apaga só os arquivos do Sofá na pasta de dados `app.sofa.launcher` do usuário (biblioteca, configurações, logs, cache) e nada fora dela, nem por atalhos ou links dentro da pasta.

### Itens que não são jogos e ocultos

**CA-69**: Dado que a Steam tem instalados itens que não são jogos e estão na lista fixa de exclusão (ex.: "Steamworks Common Redistributables"), quando a biblioteca é atualizada, então esses itens não aparecem (RN-28).

**CA-70**: Dado que existem itens ocultos, quando o usuário abre Configurações > Itens ocultos, então vê a lista navegável pelo controle; apertar A num item e confirmar devolve o item à biblioteca, com nome personalizado e data de último jogado preservados. Sem itens ocultos, a tela mostra "Nenhum item oculto".

### Cadastro manual: bloqueios

**CA-71**: Dado o navegador de pastas de "Adicionar jogo", quando o usuário procura uma pasta de rede ou unidade de rede mapeada, então ela não aparece na lista. Se um caminho de rede chegar por outro meio (ex.: biblioteca editada à mão), o jogo não inicia e aparece "Jogos em pastas de rede não são aceitos. Copie o jogo para um disco deste PC."

**CA-72**: Dado que o usuário seleciona um arquivo com extensão `.exe` que não é um executável válido do Windows, quando confirma, então aparece "Este arquivo não é um executável válido" e o jogo não é cadastrado.

### Jogo que exige administrador

**CA-73**: Dado um jogo que exige permissão de administrador, quando o usuário o inicia, então o launcher continua sem privilégio de administrador, o Windows mostra a própria confirmação (UAC) e o launcher exibe por trás dela: "<nome> pede permissão de administrador. Confirme no Windows com teclado ou mouse. A combinação de retorno pode não funcionar neste jogo." Se o usuário recusar no UAC, aparece "O Windows não deu permissão para abrir <nome>" com "Voltar".

**CA-74**: Dado o uso diário (abrir o launcher, navegar, iniciar jogo, volume, energia, ligar ou desligar "Iniciar com o Windows"), quando qualquer dessas ações acontece, então o Windows não pede permissão de administrador por causa do launcher. Únicas exceções: jogo que exige administrador (CA-73) e instalar, atualizar ou desinstalar (RN-26).

### Último jogado

**CA-75**: Dado que um jogo foi iniciado e a sessão foi confirmada, quando o launcher fecha ou cai antes do fim da sessão e é aberto de novo, então esse jogo aparece no topo dos recentes (RN-05).

### Verificação do instalador

**CA-76**: Dado um instalador publicado, quando o usuário confere o arquivo, então o SHA-256 do instalador está publicado junto dele e bate com o arquivo baixado (RN-30).

## 10. Regras de negócio

**RN-01**: O launcher funciona só com dados locais. Não há login, conta, servidor nem chamada de rede obrigatória.

**RN-02**: Jogos entram na biblioteca por quatro origens no MVP: `Manual` (executável escolhido pelo usuário), `Steam`, `Epic` e `GOG` (detectados entre os jogos instalados). Só jogos instalados aparecem. Xbox/Game Pass e outras lojas ficam fora.

**RN-03**: Um jogo é único por origem e identificador: jogo manual pelo caminho completo do executável; jogo de loja pelo identificador do jogo naquela loja (ID do app na Steam, identificador do jogo na Epic e no GOG).

**RN-04**: Qualquer item da biblioteca, de qualquer origem, pode ser ocultado pelo controle. Ocultar nunca apaga arquivos. O item vai para a lista de ocultos, não reaparece em atualizações e pode ser restaurado em Configurações > Itens ocultos (CA-70). Não existe "excluir" no MVP.

**RN-05**: Ordem da tela inicial: jogos já iniciados pelo launcher, do último jogado ao mais antigo; depois os nunca iniciados, em ordem alfabética sem diferenciar maiúsculas e acentos, pelo nome exibido (personalizado, se houver). "Último jogado" é a data de início da sessão, gravada quando o launcher confirma que o jogo abriu.

**RN-06**: Entrada do analógico só conta acima de uma zona morta (referência inicial: 25% do curso). Valor final ajustável pelo dev após teste com controle real.

**RN-07**: Teclado e mouse funcionam sempre como alternativa de emergência (mapeamento na seção 8), mas nenhum fluxo do MVP pode depender deles, salvo renomear jogo (CA-61).

**RN-08**: Todo controle XInput conectado (até 4) é tratado como o mesmo usuário. Não há "controle principal".

**RN-09**: No MVP existe no máximo uma sessão de jogo ativa. Iniciar outro jogo exige fechar o atual (CA-31).

**RN-10**: Uma sessão começa quando o launcher detecta o processo do jogo e termina quando nenhum processo associado ao jogo segue rodando. Para jogo manual e jogo GOG (iniciado direto pelo executável), o processo associado é o executável iniciado e seus filhos; para jogo Steam e Epic, os processos que rodam a partir da pasta de instalação do jogo. O app da loja em si nunca conta como processo do jogo. *(Ponto técnico frágil, ver R-02.)*

**RN-11**: A combinação de retorno é View + Menu segurados juntos por 1,5 s e só tem efeito com sessão de jogo ativa. Soltar qualquer um dos dois antes do tempo cancela.

**RN-12**: Fechar jogo segue duas etapas: pedido de fechamento normal; só depois de 10 s sem fechar o launcher oferece forçar, e forçar exige nova confirmação.

**RN-13**: O volume controlado é o volume mestre do dispositivo de saída padrão do Windows. Passo de 5 pontos percentuais, faixa de 0 a 100.

**RN-14**: Toda ação de energia (desligar, reiniciar, suspender) e toda ação destrutiva (fechar jogo, forçar fechamento, ocultar item, sair do launcher) exige confirmação com "Cancelar" em foco por padrão.

**RN-15**: Ações de energia usam o caminho normal do Windows, sem forçar o encerramento de outros apps.

**RN-16**: Só uma instância do launcher roda por usuário do Windows.

**RN-17**: "Iniciar com o Windows" vale só para o usuário atual e ligar ou desligar a opção nas configurações não exige permissão de administrador, mesmo com o app instalado para todo o PC. O registro é sempre por usuário e sem elevação: entrada de inicialização do usuário no Windows ou, se ela atrasar a abertura além de 5 s, tarefa agendada no logon do usuário (só se der para criá-la sem administrador; senão, aceita-se o atraso). Nunca para todos os usuários, como serviço ou como tarefa elevada.

**RN-18**: Biblioteca, lista de ocultos, nomes personalizados, datas de último jogo e configurações ficam salvas localmente na pasta de dados `app.sofa.launcher` do usuário e sobrevivem a atualizações do launcher. Na desinstalação, são mantidas por padrão e só apagadas se o usuário desmarcar "Manter dados" (CA-68).

**RN-19**: O launcher começa a funcionar a partir da área de trabalho do Windows. Entrar no Windows sem teclado (logon automático, Windows Hello, PIN) é configuração do usuário, fora do launcher. A spec e o instalador devem orientar isso.

**RN-20**: O launcher roda em tela cheia sem borda no monitor do launcher: o escolhido nas configurações ou, se nenhum foi escolhido ou ele não estiver conectado, o monitor principal do Windows (CA-66, CA-67).

**RN-21**: Enquanto um jogo roda, o launcher fica em segundo plano e deve gastar o mínimo de recursos (meta: menos de 1% de CPU em média e sem quedas de desempenho perceptíveis no jogo).

**RN-22**: O nome personalizado de um jogo tem prioridade sobre o nome do arquivo ou da loja e não é sobrescrito por atualizações da biblioteca. "Restaurar nome original" volta ao nome da origem.

**RN-23**: LT/RT controlam o volume só na tela da biblioteca com o launcher em primeiro plano. Cada puxada conta uma vez ao passar do meio do curso do gatilho; segurar não repete.

**RN-24**: Quando o launcher volta pela combinação de retorno, a ordem é: tentar trazer a janela para a frente; se não conseguir, minimizar o jogo e mostrar o launcher (CA-63). O jogo nunca é pausado pelo launcher.

**RN-25**: O mesmo jogo instalado em lojas diferentes aparece como itens separados, cada um com sua origem (consequência de RN-03). O launcher não tenta unificar.

**RN-26**: A instalação é para todo o PC, em `Program Files`, e exige administrador (UAC) para instalar, atualizar e desinstalar. O uso diário roda sem administrador. Os dados (RN-18) e a inicialização automática (RN-17) são por usuário. Se o runtime WebView2 faltar, o instalador o baixa e instala (CA-50).

**RN-27**: O launcher nunca roda como administrador e nunca pede elevação por conta própria. Jogo que exige administrador é aberto do jeito padrão do Windows, que mostra o UAC do próprio jogo, e o usuário é avisado (CA-73).

**RN-28**: Itens da Steam que não são jogos (redistribuíveis, ferramentas, trilhas sonoras) são filtrados por uma lista fixa de identificadores mantida no launcher. O que a lista não cobrir, o usuário oculta pelo controle (RN-04).

**RN-29**: Jogo manual é só um arquivo `.exe` válido, em disco local (fixo ou removível), iniciado sem argumentos. Pastas de rede, unidades de rede mapeadas e outros tipos de arquivo são recusados (CA-71, CA-72).

**RN-30**: O instalador não tem assinatura de código no MVP. Cada versão publicada vem com o SHA-256 do instalador, para conferência antes de instalar (CA-76).

## 11. Requisitos não funcionais

- **Compatibilidade:** Windows 10 (versões com suporte da Microsoft na data do lançamento) e Windows 11, 64 bits.
- **Permissões:** instalar, atualizar e desinstalar pedem administrador (UAC). O uso diário, inclusive ações de energia e volume, roda sem administrador.
- **Desempenho:** M-02, M-04 e M-05; RN-21.
- **Resolução:** 1920x1080 e 3840x2160 com escala do Windows de 100% a 300% sem cortar conteúdo.
- **Acessibilidade de 10 pés:** CA-54 e CA-55; contraste de texto mínimo 4,5:1.
- **Privacidade:** nenhum dado sai do PC (RN-01).
- **Robustez:** falha ao ler uma loja, imagem corrompida ou arquivo de configuração inválido não pode derrubar o launcher; o item com problema é ignorado e o resto carrega. Configuração inválida é renomeada como backup e recriada com padrões.

## 12. Casos de borda e erros

| # | Situação | Comportamento esperado |
|---|---|---|
| E-01 | Jogo abre um launcher próprio antes (ex.: launcher da publicadora) | A sessão continua ativa enquanto houver processo associado (RN-10). O usuário pode precisar do controle dentro desse launcher externo; se ele não aceitar controle, é limitação do jogo. Registrar em R-03. |
| E-02 | Jogo em tela cheia exclusiva não deixa o launcher vir para a frente | O launcher minimiza o jogo e mostra o painel (CA-63, RN-24). Validar em teste com jogos reais (R-01). |
| E-03 | A loja (Steam, Epic, GOG Galaxy) pede login ou atualização ao iniciar um jogo | A tela da loja aparece; o launcher espera até o tempo de CA-29. Se a tela da loja não aceitar controle, é limitação da loja (R-08). |
| E-04 | Jogo trava e não responde | "Fechar jogo" chega ao "Forçar fechamento" após 10 s (CA-35). |
| E-05 | Controle desconecta durante o jogo | O launcher não faz nada; ao reconectar, a combinação de retorno volta a funcionar. |
| E-06 | Usuário segura a combinação de retorno com o launcher já na frente | Sem efeito (RN-11). |
| E-07 | Jogo é aberto fora do launcher (pela loja, atalho) | Não conta como sessão no MVP; a combinação de retorno não funciona. Candidato para versão seguinte. |
| E-08 | Monitor e TV ligados ao mesmo tempo | Launcher abre no monitor do launcher (RN-20). |
| E-09 | TV desligada com o PC ligado e depois religada | Ao voltar a imagem, o launcher continua em tela cheia na mesma tela. Se o Windows tiver desconectado a TV nesse meio-tempo, vale CA-67. |
| E-10 | Executável cadastrado em unidade externa desconectada | Mesmo comportamento de CA-28, com o caminho exibido. |
| E-11 | Windows abre janela por cima (atualização, UAC, notificação) | Fora do controle do launcher. UAC exige teclado/mouse ou toque; documentar em R-04. |
| E-12 | Duas pastas de biblioteca Steam (ou Epic/GOG) com o mesmo jogo | Um único item (RN-03). |
| E-13 | Nome de jogo muito longo | Truncado com reticências no card; nome completo no painel/rodapé quando em foco. |
| E-14 | Jogo fecha e reabre sozinho (alguns jogos reiniciam para aplicar configurações) | A sessão só termina se nenhum processo associado aparecer por 5 s seguidos. |
| E-15 | Mesmo jogo instalado na Steam e na Epic | Dois itens, cada um com sua origem (RN-25, CA-60). |
| E-16 | Jogo da Epic iniciado com o Epic Games Launcher fechado ou deslogado | O launcher abre a Epic; se ela pedir login, vale E-03. O tempo de CA-29 conta a partir do pedido de início. |
| E-17 | Jogo GOG instalado pelo instalador avulso (sem GOG Galaxy) | Entra na biblioteca se a detecção o encontrar; senão, o usuário cadastra manualmente. O arquiteto confirma o que é detectável (R-08). |
| E-18 | Biblioteca da loja em unidade externa desconectada | Os jogos dessa unidade somem na atualização (como desinstalados) e voltam quando a unidade reconectar. Ocultos e nomes personalizados são preservados. |
| E-19 | Instalação feita com credenciais de outra conta administradora no UAC | "Iniciar com o Windows" deve ficar registrado para o usuário que abriu o instalador. Se o instalador não conseguir garantir isso, a opção é ligada pelo próprio launcher no primeiro uso (CA-04). Arquiteto valida. |
| E-20 | Usuário puxa LT/RT segurando a direção ao navegar | O volume muda e o foco continua se movendo; um não cancela o outro. |
| E-21 | Com o launcher na frente e o jogo aberto em segundo plano (painel ou biblioteca), o jogo também reage aos botões do controle | Aceito no MVP: o Windows não dá a nenhum app uso exclusivo do controle e o jogo não é minimizado (CA-36). O QA lista os jogos afetados. |
| E-22 | Jogo GOG que usa saves na nuvem do Galaxy | Aberto pelo executável (CA-26), o Galaxy não sincroniza os saves. Para sincronizar, o usuário abre o Galaxy. |
| E-23 | Jogo que roda como administrador | A combinação de retorno pode falhar, porque o Windows impede um app comum de mexer na janela de um app elevado. O aviso de CA-73 avisa disso. |

## 13. Riscos

| ID | Risco | Impacto | Mitigação |
|---|---|---|---|
| R-01 | Windows restringe quem pode roubar o primeiro plano; jogos em tela cheia exclusiva podem não ceder a frente ao launcher. | Quebra US-07, que é o diferencial. | Fallback de minimizar (RN-24). Spike técnico antes do resto (arquiteto), com 3 a 5 jogos reais do usuário, em janela, sem borda e tela cheia exclusiva. |
| R-02 | Detectar fim de sessão para jogos de loja é indireto: o launcher pede à loja para iniciar o jogo e não controla o processo. Na Epic e no GOG Galaxy o jogo pode passar por processos intermediários da loja. | Launcher não volta sozinho (CA-30) ou volta antes da hora. | Regra por pasta de instalação (RN-10) + período de tolerância (E-14). Validar no spike com ao menos um jogo de cada loja. |
| R-03 | Launchers de publicadoras e anti-cheat abrem processos e janelas próprios. | Sessão mal detectada; janelas que não aceitam controle. | Aceitar como limitação no MVP e listar jogos problemáticos encontrados nos testes. |
| R-04 | Telas do Windows (UAC, login, atualização) não aceitam controle. | Viola M-01 em casos pontuais. Instalar e atualizar exigem teclado/mouse por causa do UAC (RN-26). | Fora do escopo (RN-19); instalação é rara. Orientar o usuário a configurar logon e janelas de atualização. |
| R-05 | O botão Guide (Xbox) não é exposto pela API XInput documentada e o Xbox Game Bar o captura por padrão. | Motivo para não usar o Guide como retorno. | Combinação definida como View + Menu (RN-11). Afirmação a confirmar pelo arquiteto. |
| R-06 | Instalador sem assinatura de código mostra aviso do SmartScreen. | Atrito só na instalação; uso pessoal. | Aceito no MVP (D-11, D-27). SHA-256 publicado com cada versão (RN-30). |
| R-07 | Anti-cheat de alguns jogos pode desconfiar de processos que leem entrada global ou mexem em janelas. | Risco baixo, mas possível bloqueio. | O launcher só lê o controle via API padrão e não injeta nada no jogo (overlay fora de escopo). Minimizar o jogo (RN-24) é uma ação de janela comum. |
| R-08 | Epic e GOG guardam a lista de jogos instalados em arquivos locais sem documentação pública; o formato pode mudar com atualizações das lojas. A Epic costuma exigir o próprio launcher aberto e logado para iniciar jogos. | Jogos somem da biblioteca após atualização da loja; início lento ou bloqueado por login. | Falha de leitura de uma loja não derruba as outras (seção 11). Cadastro manual como alternativa. Arquiteto confirma no spike o que cada loja expõe, incluindo jogos GOG sem Galaxy (E-17). |
| R-09 | Outro programa rodando com o mesmo usuário do Windows pode alterar a biblioteca salva. | Jogos ou caminhos adulterados na biblioteca. | Risco aceito (D-26). Ao carregar, o launcher reaplica as mesmas validações do cadastro (RN-29), então caminho de rede ou arquivo inválido não inicia. |

## 14. Decisões

Respostas do usuário às perguntas abertas da versão anterior.

| ID | Pergunta | Decisão | Itens afetados |
|---|---|---|---|
| D-01 | Q-01. Como os jogos entram na biblioteca | Manual + Steam + Epic Games + GOG no MVP. Xbox/Game Pass fica fora. | RN-02, RN-03, RN-04, RN-10, RN-25, CA-18, CA-21, CA-23, CA-26, CA-56 a CA-60, US-03, E-03, E-12, E-15 a E-18, R-02, R-08 |
| D-02 | Q-02. Combinação de retorno | View + Menu segurados por 1,5 s. Não configurável no MVP. | RN-11, CA-32, seção 8 |
| D-03 | Q-03. Cadastro manual | Navegador de pastas pelo controle, nome vem do arquivo, renomear pelo teclado físico. Sem teclado virtual. | CA-09, CA-61, CA-62, RN-07, RN-22, US-04 |
| D-04 | Q-04. O que a combinação faz | Traz o launcher para a frente; se falhar, minimiza o jogo. Sem pausa automática. | RN-24, CA-63, E-02, R-01 |
| D-05 | Q-05. Atalho de volume | Menu rápido e também LT (baixa) / RT (sobe) na tela da biblioteca, passo de 5, com indicador. LT/RT não tinham outro uso: sem conflito. | CA-40, CA-64, CA-65, RN-23, US-09, seção 8, E-20 |
| D-06 | Q-06. Ações extras de energia | Nenhuma. | Sem mudança |
| D-07 | Q-07. Monitor | Escolha nas configurações; padrão monitor principal; se o escolhido sumir, cai no principal. | CA-01, CA-66, CA-67, RN-20, US-15, E-08, E-09 |
| D-08 | Q-08. Teclado e mouse de emergência | Sempre ativos. | RN-07, CA-13 (sem mudança) |
| D-09 | Q-09. Tipo de instalação | Para todo o PC (`Program Files`), com UAC. "Iniciar com o Windows" continua por usuário. | RN-17, RN-26, CA-50 a CA-53, E-19, R-04 |
| D-10 | Q-10. Dados ao desinstalar | O desinstalador pergunta, com "Manter dados" marcado. | RN-18, CA-52, CA-68 |
| D-11 | Q-11. Assinatura de código | Sem assinatura no MVP. | R-06 |
| D-12 | Q-12. Ferramentas existentes | Usuário não testou; seguir com o app próprio. | Seção 1 |
| D-13 | Q-13. Jogos não instalados | Não mostrar. | RN-02 |
| D-14 | Marca e visual | Público só o usuário; visual "console moderno". | Seção 6 |
| D-15 | Nome e marca | Produto "Sofá"; identificadores técnicos `sofa`; identificador do app e pasta de dados `app.sofa.launcher`. Marca: direção A, "Palco" (`docs/brand/sofa/`). | Cabeçalho, seção 6, RN-18, CA-68 |
| D-16 | P-01. WebView2 no instalador | Bootstrapper embutido no instalador; baixa o runtime só se faltar. | CA-50, RN-26 |
| D-17 | P-02. Como abrir jogos GOG | Pelo executável direto, inclusive os instalados pelo Galaxy. | CA-26, RN-10, E-22, escopo |
| D-18 | P-03. Atraso da inicialização automática | Se a entrada de inicialização do usuário passar de 5 s, usar tarefa agendada no logon do usuário, sem elevação, se o spike confirmar que dá sem administrador; senão, aceitar o atraso. | RN-17 |
| D-19 | P-04. Minimizar ao ir para a biblioteca | Não minimizar. | CA-36, E-21 |
| D-20 | P-05. Itens da Steam que não são jogos | Lista fixa de identificadores; o usuário oculta qualquer item pelo controle. | CA-21, CA-28, CA-69, CA-70, RN-04, RN-14, RN-28, US-05, seção 8 |
| D-21 | P-06. Definição de último jogado | Data de início da sessão. | CA-22, CA-75, RN-05 |
| D-22 | AF-01. Elevação | O launcher nunca é elevado. Jogo que exige administrador abre do jeito padrão, com o UAC do jogo, e o usuário é avisado. | CA-73, CA-74, RN-27, E-23 |
| D-23 | AF-02. Caminhos de rede | Bloqueados no cadastro manual. | CA-19, CA-71, RN-29 |
| D-24 | AF-03. Tipo de arquivo e argumentos | Jogo manual só `.exe`, sem argumentos. | CA-19, CA-72, RN-29, escopo |
| D-25 | AF-05. Mecanismo de inicialização automática | Só por usuário (entrada de inicialização do usuário ou tarefa por usuário não elevada, D-18). Nunca para todos os usuários, serviço ou tarefa elevada. | RN-17 |
| D-26 | AF-08. Adulteração da biblioteca | Risco aceito para processos do mesmo usuário. | R-09 |
| D-27 | AF-09. Instalador sem assinatura | Mantido, com SHA-256 publicado. | CA-76, RN-30, R-06 |
| D-28 | AF-10. Desinstalador apagando dados | Apaga só os arquivos da pasta de dados do Sofá, com travas para não sair dela. | CA-68 |

### Pendências

- **Spike técnico (arquiteto):** primeiro plano e fallback de minimizar (R-01), fim de sessão por loja (R-02), formato local de Epic e GOG e jogos GOG sem Galaxy (R-08, E-17), registro de "Iniciar com o Windows" com UAC de outra conta (E-19), tempo da inicialização automática e tarefa agendada sem administrador (D-18), confirmação sobre o botão Guide (R-05).
