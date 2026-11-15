# Spec: launcher-tv

| Campo | Valor |
|---|---|
| Feature | launcher-tv (MVP do SofaLauncher) |
| Produto | SofaLauncher; sigla de texto SL (D-49, D-52). Identificadores técnicos sem mudança (D-56, ADR-0007): `sofa`, `sofa.exe`, identificador do app e pasta de dados `app.sofa.launcher`, pasta `Program Files\Sofa`, chave `Uninstall\Sofa` |
| Branch | `feature/launcher-tv-20261113` |
| Status | Revisada com as decisões do usuário, da arquitetura, de segurança e da rodada 1 de QA e design, mais D-49 a D-51, tomadas depois da versão de teste 0.1.0 (seção 14) |
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
| Menu rápido | Menu aberto pelo botão Menu com volume, energia (inclusive "Fechar o launcher") e configurações. |
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
10. Desligar, reiniciar e suspender o PC, sempre com confirmação. No mesmo grupo do menu rápido, "Fechar o launcher", também com confirmação (RN-35).
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
- **Nome do produto:** SofaLauncher (D-49). SL é só uma sigla de texto e não aparece no logo nem no ícone (D-52). Os identificadores técnicos não mudam: `sofa`, `sofa.exe`, identificador do app e pasta de dados `app.sofa.launcher`, pasta `Program Files\Sofa` e chave `Uninstall\Sofa`, sem migração (D-56, ADR-0007).
- **Marca:** direção A, "Palco", em `docs/brand/sofa/`. Logo aprovado: um controle, no lugar do símbolo de play, com "SofaLauncher" escrito por extenso (D-52). O guia de marca ainda não reflete essa mudança.
- **Mockups:** aprovados pelo usuário, em `docs/design/launcher-tv/mockups/`, com handoff em `docs/design/launcher-tv.md`. Microcopy em `docs/design/launcher-tv/microcopy.md`.
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
| US-14 | Como jogador de sofá, quero fechar o launcher pelo menu de energia e ir para a área de trabalho, para usar o PC de outro jeito quando precisar. | P2 |
| US-15 | Como jogador de sofá com monitor e TV no mesmo PC, quero escolher em qual tela o launcher abre, para ele aparecer na TV sem mudar o monitor principal do Windows. | P2 |

## 8. Mapeamento padrão do controle

Fixo no MVP. Referência para CA e QA.

| Entrada | Na biblioteca | Em menus, diálogos e painel do jogo em andamento | Durante sessão de jogo (launcher em segundo plano) |
|---|---|---|---|
| D-pad / analógico esquerdo | Move o foco | Move o foco | Ignorado pelo launcher |
| A | Inicia o jogo em foco | Confirma a opção em foco | Ignorado |
| B | Sem efeito na tela inicial; volta um nível nas outras | Cancela / fecha | Ignorado |
| Menu (≡) | Abre o menu rápido | Fecha o menu rápido | Ignorado (salvo na combinação de retorno) |
| View (⧉) | Abre opções do jogo em foco (Jogar, Renomear, Restaurar nome original, Ocultar; CA-79) | Sem efeito | Ignorado (salvo na combinação de retorno) |
| LB / RB | Pula uma página/linha para trás / para frente | Sem efeito | Ignorado |
| LT / RT | Diminui / aumenta o volume em 5 pontos | Sem efeito (no menu rápido o volume usa esquerda/direita) | Ignorado |
| View + Menu segurados 1,5 s | Sem efeito | Sem efeito | Traz o launcher para a frente (combinação de retorno) |

LT/RT não tinham outro uso no mapeamento; não há conflito. LB/RB continuam com a paginação.

Teclado de emergência (RN-07, CA-80): cada ação desta tabela tem uma tecla equivalente, salvo a combinação de retorno, que só existe no controle. As teclas ficam no handoff de design ou no front, não nesta spec.

## 9. Critérios de aceite

### Inicialização, tela cheia e instância

**CA-01**: Dado que o launcher está instalado com "Iniciar com o Windows" ligado, quando o usuário entra na área de trabalho do Windows, então o launcher abre em tela cheia sem borda no monitor do launcher (padrão: monitor principal) e fica navegável em até 5 s (M-02).

**CA-02**: Dado que o launcher está aberto, quando o usuário tenta abri-lo de novo (atalho, menu Iniciar), então nenhuma segunda janela é criada e a janela existente vem para a frente.

**CA-03**: Dado que o launcher está em tela cheia, quando qualquer tela é exibida, então não aparecem barra de título, barra de tarefas nem cursor do mouse parado (o cursor some após 3 s sem movimento e volta ao mexer o mouse).

**CA-04**: Dado que o usuário está nas configurações, quando desliga "Iniciar com o Windows" e reinicia o PC, então o launcher não abre sozinho; ao religar a opção, volta a abrir no próximo login.

**CA-05**: Dado o menu rápido aberto, quando o usuário percorre os itens, então não existe "Sair do launcher". O launcher só é fechado por "Fechar o launcher", no grupo de energia (CA-82, D-53).

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

**CA-26**: Dado um jogo em foco e nenhuma sessão ativa, quando o usuário aperta A, então o launcher mostra a tela "Abrindo <nome do jogo>…" (com o botão "Voltar", CA-77) e inicia o jogo. Jogos Steam e Epic iniciam pela loja, que é aberta antes se estiver fechada. Jogos GOG (inclusive os instalados pelo Galaxy) iniciam direto pelo executável do jogo, sem abrir o Galaxy.

**CA-27**: Dado que o jogo abriu, quando a janela do jogo aparece, então o launcher sai da frente e deixa o jogo em primeiro plano.

**CA-28**: Dado um jogo manual cujo executável não existe mais, quando o usuário tenta iniciá-lo, então aparece "Não encontramos o arquivo de <nome do jogo>" com as opções "Ocultar" e "Voltar". A mensagem não mostra o caminho do arquivo (D-42).

**CA-29**: Dado que o jogo foi iniciado, quando nenhum processo do jogo é detectado em até 60 s, então o launcher volta ao primeiro plano com "Não foi possível confirmar que <nome> abriu" e as opções "Tentar de novo" e "Voltar". Se o jogo abrir depois dessa mensagem (por exemplo, após login na loja, E-03), a sessão segue normal: o launcher sai da frente (CA-27), grava o último jogado (RN-05) e a combinação de retorno funciona.

**CA-30**: Dado uma sessão de jogo ativa, quando o jogo é fechado pelo próprio menu do jogo, então o launcher volta ao primeiro plano em até 3 s, na biblioteca, com o jogo que acabou de fechar em foco e no topo dos recentes. Para vir à frente, usa a mesma sequência de tentativas do retorno pela combinação (RN-24); o Windows não devolve o foco sozinho quando o jogo fecha.

**CA-31**: Dado uma sessão de jogo ativa, quando o usuário volta à biblioteca e aperta A em outro jogo, então o launcher pergunta "<jogo atual> está aberto. Fechar e abrir <novo jogo>?" com "Cancelar" em foco por padrão.

### Voltar ao launcher durante o jogo

**CA-32**: Dado uma sessão de jogo ativa com o jogo em primeiro plano (janela, sem borda ou tela cheia), quando o usuário segura View + Menu por 1,5 s, então o launcher vem para a frente em até 3 s (M-04) e mostra o painel do jogo em andamento.

**CA-33**: Dado que o usuário aperta a combinação de retorno por menos tempo que o definido, quando solta, então nada acontece e o jogo segue recebendo os botões normalmente.

**CA-34**: Dado o painel do jogo em andamento, quando o usuário escolhe "Voltar ao jogo" ou aperta B, então o jogo volta ao primeiro plano no estado em que estava.

**CA-35**: Dado o painel do jogo em andamento, quando o usuário escolhe "Fechar jogo" e confirma, então o launcher mostra a tela "Fechando <nome>…" (CA-78) e pede ao jogo para fechar normalmente; se o jogo continuar aberto após 10 s, oferece "Forçar fechamento (o progresso não salvo será perdido)" com "Cancelar" em foco por padrão.

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

**CA-44**: Dado o diálogo de confirmação de "Desligar", quando o usuário confirma, então em até 3 s o Windows inicia o mesmo desligamento do Desligar do menu Iniciar, normal (não forçado): com a inicialização rápida do Windows ligada, ela é usada; com ela desligada, o desligamento é completo (RN-15).

**CA-45**: Dado o diálogo de confirmação de "Reiniciar", quando o usuário confirma, então o Windows faz um reinício completo, sem inicialização rápida (RN-15), e com "Iniciar com o Windows" ligado o launcher abre de novo após o login (CA-01).

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

**CA-54**: Dado a TV em 1080p ou 4K, quando qualquer tela é exibida, então todo texto e elemento interativo fica dentro da área segura (margem de 5% em cada borda) e o texto do corpo tem pelo menos 24 px equivalentes a 1080p, escalando em 4K. Única exceção: card de capa parcialmente cortado na borda lateral da tela, para indicar que a fileira continua, ou embaixo, quando a fileira seguinte aparece sob o rodapé de dicas. O texto e o destaque de foco do item em foco continuam dentro da área segura (RN-32).

**CA-55**: Dado qualquer tela, quando o usuário olha a 3 m, então sempre existe exatamente um item em foco, visível e com contraste suficiente em relação ao fundo.

### Epic Games e GOG

**CA-56**: Dado que o Epic Games Launcher está instalado com jogos baixados, quando o launcher abre ou o usuário escolhe "Atualizar biblioteca", então cada jogo Epic instalado aparece com o nome da Epic e capa local, se existir; senão, ícone do executável ou placeholder (CA-24).

**CA-57**: Dado que há jogos instalados pelo GOG Galaxy, quando o launcher abre ou o usuário escolhe "Atualizar biblioteca", então cada jogo GOG instalado aparece com o nome do GOG e capa local, se existir; senão, ícone do executável ou placeholder (CA-24).

**CA-58**: Dado que um jogo Epic ou GOG foi desinstalado pela loja, quando a biblioteca é atualizada, então o jogo some da biblioteca.

**CA-59**: Dado que o Epic Games Launcher ou o GOG Galaxy não está instalado, quando a biblioteca é carregada, então nenhum erro aparece e só os jogos das outras origens são mostrados.

**CA-60**: Dado qualquer jogo na biblioteca, quando o card aparece, então mostra a origem (Manual, Steam, Epic ou GOG) de forma discreta, para diferenciar o mesmo jogo em lojas diferentes (RN-25).

### Renomear jogo

**CA-61**: Dado um jogo em foco, quando o usuário aperta View (ou a tecla equivalente, RN-07), escolhe "Renomear", digita o novo nome no teclado físico e confirma com Enter, então o card passa a mostrar o novo nome. Esc ou B cancelam sem alterar. Nome vazio não é aceito.

**CA-62**: Dado um jogo de loja renomeado, quando a biblioteca é atualizada, então o nome personalizado é mantido (RN-22). Nas opções do jogo existe "Restaurar nome original".

### Retorno com fallback

**CA-63**: Dado que o usuário segurou a combinação de retorno e o pedido simples para vir à frente falhou, quando isso é detectado, então o launcher segue as próximas tentativas da sequência de RN-24 e mostra o painel do jogo em andamento assim que uma delas funcionar, tudo dentro de 3 s (M-04). Minimizar o jogo é a última tentativa e não garante o retorno sozinho. Se o jogo foi minimizado, "Voltar ao jogo" o restaura. O jogo nunca é pausado.

### Volume pelos gatilhos

**CA-64**: Dado a tela da biblioteca em primeiro plano, quando o usuário puxa LT ou RT, então o volume desce ou sobe 5 pontos percentuais por puxada, limitado a 0 e 100, e um indicador de volume aparece na tela em até 200 ms e some após 2 s sem uso. LT com o sistema no mudo baixa o nível guardado e mantém o mudo.

**CA-65**: Dado o menu rápido, um diálogo, o painel do jogo em andamento ou uma sessão de jogo com o launcher em segundo plano, quando o usuário puxa LT ou RT, então o volume não muda e nada é acionado.

### Monitor do launcher

**CA-66**: Dado mais de um monitor conectado, quando o usuário escolhe outro monitor em Configurações > Tela, então o launcher passa para esse monitor em tela cheia na hora e volta a abrir nele nas próximas inicializações.

**CA-67**: Dado que o monitor escolhido não está conectado (ao abrir o launcher ou durante o uso), quando o launcher percebe a ausência, então passa para o monitor principal do Windows. A preferência é mantida: na próxima inicialização com o monitor escolhido conectado, o launcher abre nele.

### Desinstalador

**CA-68**: Dado o desinstalador, quando ele roda, então pergunta se o usuário quer manter biblioteca e configurações, com "Manter dados" marcado por padrão. Marcado, os dados ficam e uma reinstalação os recupera. Desmarcado, o desinstalador apaga só os arquivos do SofaLauncher na pasta de dados `app.sofa.launcher` do usuário (biblioteca, configurações, logs, cache) e nada fora dela, nem por atalhos ou links dentro da pasta. Essa remoção roda sem privilégio de administrador.

Se a remoção sem administrador falhar (ou não confirmar que terminou com sucesso), o desinstalador apaga como administrador só `library.json` e `settings.json` na raiz da pasta de dados, pelo nome exato, sem curinga e sem entrar em subpastas. Capas, logs e o resto ficam, e o desinstalador mostra a pasta ao usuário.

### Itens que não são jogos e ocultos

**CA-69**: Dado que a Steam tem instalados itens que não são jogos e estão na lista fixa de exclusão (ex.: "Steamworks Common Redistributables"), quando a biblioteca é atualizada, então esses itens não aparecem (RN-28).

**CA-70**: Dado que existem itens ocultos, quando o usuário abre Configurações > Itens ocultos, então vê a lista navegável pelo controle; apertar A num item e confirmar devolve o item à biblioteca, com nome personalizado e data de último jogado preservados. Sem itens ocultos, a tela mostra "Nenhum item oculto".

### Cadastro manual: bloqueios

**CA-71**: Dado o navegador de pastas de "Adicionar jogo", quando o usuário procura uma pasta de rede ou unidade de rede mapeada, então ela não aparece na lista. Cada momento tem mensagem própria:
- no cadastro, se chegar um local fora dos discos deste PC: "Este local não é aceito. O SofaLauncher só abre jogos de discos deste PC.", com "Voltar";
- ao iniciar um jogo cujo caminho é de rede (ex.: biblioteca editada à mão): o jogo não inicia e aparece "Jogos em pastas de rede não são aceitos. Copie o jogo para um disco deste PC.", com "Ocultar" e "Voltar" (foco em "Voltar").

**CA-72**: Dado que o usuário seleciona um arquivo com extensão `.exe` que não é um executável válido do Windows, quando confirma, então aparece "Este arquivo não é um executável válido" e o jogo não é cadastrado.

### Jogo que exige administrador

**CA-73**: Dado um jogo que exige permissão de administrador, quando o usuário o inicia, então acontece em duas etapas, e o launcher continua sem privilégio de administrador:
1. Antes de qualquer UAC, o SofaLauncher mostra "<nome> precisa de permissão de administrador para abrir. O Windows vai pedir a sua confirmação.", com "Continuar" (em foco) e "Voltar". "Voltar" ou B cancelam sem abrir o UAC.
2. Com "Continuar", o Windows mostra a própria confirmação (UAC) do jogo e o SofaLauncher exibe por trás dela: "<nome> pede permissão de administrador. Confirme no Windows com teclado ou mouse. A combinação de retorno pode não funcionar neste jogo."

Se o usuário recusar no UAC, aparece "O Windows não deu permissão para abrir <nome>" com "Voltar".

**CA-74**: Dado o uso diário (abrir o launcher, navegar, iniciar jogo, volume, energia, ligar ou desligar "Iniciar com o Windows"), quando qualquer dessas ações acontece, então o Windows não pede permissão de administrador por causa do launcher. Únicas exceções: jogo que exige administrador (CA-73) e instalar, atualizar ou desinstalar (RN-26).

### Último jogado

**CA-75**: Dado que um jogo foi iniciado e a sessão foi confirmada, quando o launcher fecha ou cai antes do fim da sessão e é aberto de novo, então esse jogo aparece no topo dos recentes (RN-05).

### Verificação do instalador

**CA-76**: Dado um instalador publicado, quando o usuário confere o arquivo, então o SHA-256 do instalador está publicado junto dele e bate com o arquivo baixado (RN-30).

### Telas de espera

**CA-77**: Dado a tela "Abrindo <nome>…", quando ela aparece, então tem o botão "Voltar" em foco e ignora A nos primeiros 600 ms. Ao apertar "Voltar" (A no botão ou B), o SofaLauncher para de esperar e volta à biblioteca, mas não cancela o início: o jogo ainda pode abrir depois. Se abrir, vale o comportamento normal de sessão (CA-27, CA-30).

**CA-78**: Dado a tela "Fechando <nome>…" (após confirmar "Fechar jogo"), quando ela aparece, então tem o botão "Ir para a biblioteca" em foco e ignora A nos primeiros 600 ms. Ao apertar o botão (A ou B), a biblioteca aparece; o pedido de fechamento continua e o card mostra "Em andamento" até o jogo fechar. Se o jogo não fechar, o usuário volta ao painel pelo card (CA-36) e chega ao "Forçar fechamento" (CA-35).

### Opções do jogo

**CA-79**: Dado um jogo em foco, quando o usuário aperta View, então abre a lista de opções nesta ordem: "Jogar" (primeiro item e em foco), "Renomear", "Restaurar nome original" (só se o nome foi alterado) e "Ocultar". "Jogar" faz o mesmo que A no card. B fecha a lista. Não existe tela de detalhe do jogo.

### Teclado de emergência

**CA-80**: Dado o launcher em primeiro plano, com ou sem controle conectado, quando o usuário usa só o teclado, então consegue mover o foco, confirmar, voltar, abrir e fechar o menu rápido, abrir as opções do jogo, trocar de aba ou página, mudar o volume na biblioteca e usar o painel do jogo em andamento, com o mesmo efeito do botão correspondente do controle (seção 8). Durante a sessão de jogo, nenhuma tecla traz o launcher à frente: a combinação de retorno só existe no controle (RN-07).

### Diálogos

**CA-81**: Dado qualquer diálogo com mais de um botão, quando ele aparece, então "Cancelar" ou "Voltar" fica à esquerda e a ação principal à direita, com o foco padrão definido no CA do diálogo (RN-34).

### Fechar o launcher

**CA-82**: Dado o menu rápido aberto (na biblioteca ou no painel do jogo em andamento), quando o usuário escolhe "Fechar o launcher" no grupo de energia, então (RN-35):
1. Aparece um diálogo de confirmação com "Cancelar" à esquerda e em foco e "Fechar o launcher" à direita. O diálogo diz como reabrir: pelo atalho do SofaLauncher ou reiniciando o PC (D-55; texto final na microcopy). "Cancelar" ou B fecham o diálogo sem fechar nada.
2. Sem sessão de jogo ativa, ao confirmar, o launcher fecha e o Windows mostra a área de trabalho.
3. Com sessão de jogo ativa, o diálogo também avisa que <nome do jogo> continua aberto, sem o aviso de progresso perdido do CA-48. Ao confirmar, só o launcher fecha: o jogo segue rodando, não recebe pedido de fechamento, não é minimizado nem pausado, e o launcher não o traz à frente antes de sair; o Windows decide qual janela fica em primeiro plano (D-57).
4. Só com o teclado, com ou sem controle conectado, o usuário abre o menu rápido, chega ao item, confirma e cancela pelas teclas equivalentes a Menu, direções, A e B, com o mesmo efeito do controle (RN-07, CA-80).
5. Depois de fechar, "Iniciar com o Windows" continua como estava. Se estiver ligado, o launcher abre no próximo login (CA-01).
6. Ao ser aberto de novo (atalho, menu Iniciar ou próximo login), o launcher abre como numa inicialização normal: tela cheia no monitor do launcher, biblioteca com os dados preservados (CA-25) e, se o fechamento aconteceu durante uma sessão, o jogo dela no topo dos recentes (CA-75). Se esse jogo ainda estiver aberto, ele é tratado como aberto fora do launcher (E-07): a sessão não é retomada, o card não mostra "Em andamento" e a combinação de retorno não funciona para ele (D-54).

## 10. Regras de negócio

**RN-01**: O launcher funciona só com dados locais. Não há login, conta, servidor nem chamada de rede obrigatória.

**RN-02**: Jogos entram na biblioteca por quatro origens no MVP: `Manual` (executável escolhido pelo usuário), `Steam`, `Epic` e `GOG` (detectados entre os jogos instalados). Só jogos instalados aparecem. Xbox/Game Pass e outras lojas ficam fora.

**RN-03**: Um jogo é único por origem e identificador: jogo manual pelo caminho completo do executável; jogo de loja pelo identificador do jogo naquela loja (ID do app na Steam, identificador do jogo na Epic e no GOG).

**RN-04**: Qualquer item da biblioteca, de qualquer origem, pode ser ocultado pelo controle. Ocultar nunca apaga arquivos. O item vai para a lista de ocultos, não reaparece em atualizações e pode ser restaurado em Configurações > Itens ocultos (CA-70). Não existe "excluir" no MVP.

**RN-05**: Ordem da tela inicial: jogos já iniciados pelo launcher, do último jogado ao mais antigo; depois os nunca iniciados, em ordem alfabética sem diferenciar maiúsculas e acentos, pelo nome exibido (personalizado, se houver). "Último jogado" é a data de início da sessão, gravada quando o launcher confirma que o jogo abriu.

**RN-06**: Entrada do analógico só conta acima de uma zona morta (referência inicial: 25% do curso). Valor final ajustável pelo dev após teste com controle real.

**RN-07**: Teclado e mouse funcionam sempre como alternativa de emergência, mas nenhum fluxo do MVP pode depender deles, salvo renomear jogo (CA-61). O teclado cobre todas as ações do controle da seção 8 (navegar, confirmar, voltar, abas, opções do jogo, volume, menu rápido e painel do jogo em andamento), nos mesmos contextos em que o botão vale. A combinação de retorno (View + Menu) é a única exceção: existe só no controle, e apertar juntas as teclas de View e Menu não a substitui. As teclas são definidas no handoff de design ou no front (CA-80).

**RN-08**: Todo controle XInput conectado (até 4) é tratado como o mesmo usuário. Não há "controle principal".

**RN-09**: No MVP existe no máximo uma sessão de jogo ativa. Iniciar outro jogo exige fechar o atual (CA-31).

**RN-10**: Uma sessão começa quando o launcher detecta o processo do jogo e termina quando nenhum processo associado ao jogo segue rodando. Para jogo manual e jogo GOG (iniciado direto pelo executável), o processo associado é o executável iniciado e seus filhos; para jogo Steam e Epic, os processos que rodam a partir da pasta de instalação do jogo. O app da loja em si nunca conta como processo do jogo. *(Ponto técnico frágil, ver R-02.)*

**RN-11**: A combinação de retorno é View + Menu segurados juntos por 1,5 s e só tem efeito com sessão de jogo ativa. Soltar qualquer um dos dois antes do tempo cancela.

**RN-12**: Fechar jogo segue duas etapas: pedido de fechamento normal; só depois de 10 s sem fechar o launcher oferece forçar, e forçar exige nova confirmação.

**RN-13**: O volume controlado é o volume mestre do dispositivo de saída padrão do Windows. Passo de 5 pontos percentuais, faixa de 0 a 100.

**RN-14**: Toda ação de energia (desligar, reiniciar, suspender) e toda ação destrutiva (fechar jogo, forçar fechamento, ocultar item, fechar o launcher) exige confirmação com "Cancelar" em foco por padrão.

**RN-15**: Ações de energia usam o caminho normal do Windows, sem forçar o encerramento de outros apps. "Desligar" funciona igual ao Desligar do menu Iniciar: usa a inicialização rápida do Windows quando ela está ligada. "Reiniciar" é sempre um reinício completo (CA-44, CA-45).

**RN-16**: Só uma instância do launcher roda por usuário do Windows.

**RN-17**: "Iniciar com o Windows" vale só para o usuário atual e ligar ou desligar a opção nas configurações não exige permissão de administrador, mesmo com o app instalado para todo o PC. O registro é sempre por usuário e sem elevação: entrada de inicialização do usuário no Windows ou, se ela atrasar a abertura além de 5 s, tarefa agendada no logon do usuário (só se der para criá-la sem administrador; senão, aceita-se o atraso). Nunca para todos os usuários, como serviço ou como tarefa elevada.

**RN-18**: Biblioteca, lista de ocultos, nomes personalizados, datas de último jogo e configurações ficam salvas localmente na pasta de dados `app.sofa.launcher` do usuário e sobrevivem a atualizações do launcher. Na desinstalação, são mantidas por padrão e só apagadas se o usuário desmarcar "Manter dados" (CA-68). A remoção roda sem administrador. Se ela falhar, a única remoção como administrador é a de `library.json` e `settings.json` na raiz da pasta de dados, pelo nome exato, sem curinga nem subpastas; o resto fica e a pasta é mostrada ao usuário.

**RN-19**: O launcher começa a funcionar a partir da área de trabalho do Windows. Entrar no Windows sem teclado (logon automático, Windows Hello, PIN) é configuração do usuário, fora do launcher. A spec e o instalador devem orientar isso.

**RN-20**: O launcher roda em tela cheia sem borda no monitor do launcher: o escolhido nas configurações ou, se nenhum foi escolhido ou ele não estiver conectado, o monitor principal do Windows (CA-66, CA-67).

**RN-21**: Enquanto um jogo roda, o launcher fica em segundo plano e deve gastar o mínimo de recursos (meta: menos de 1% de CPU em média e sem quedas de desempenho perceptíveis no jogo).

**RN-22**: O nome personalizado de um jogo tem prioridade sobre o nome do arquivo ou da loja e não é sobrescrito por atualizações da biblioteca. "Restaurar nome original" volta ao nome da origem.

**RN-23**: LT/RT controlam o volume só na tela da biblioteca com o launcher em primeiro plano. Cada puxada conta uma vez ao passar do meio do curso do gatilho; segurar não repete.

**RN-24**: Sempre que o launcher precisa vir à frente de um jogo (combinação de retorno, CA-63, ou fim da sessão, CA-30), tenta nesta ordem e para na primeira que funcionar: pedido simples ao Windows (degrau 1 do ADR-0003); vincular-se à entrada da janela do jogo e pedir de novo (degrau 3); enviar uma única tecla F24 sintética e pedir de novo (degrau 2); minimizar o jogo (degrau 4). A ordem padrão pode ser ajustada e cada tentativa pode ser desligada em configuração avançada. O jogo nunca é pausado pelo launcher.

**RN-25**: O mesmo jogo instalado em lojas diferentes aparece como itens separados, cada um com sua origem (consequência de RN-03). O launcher não tenta unificar.

**RN-26**: A instalação é para todo o PC, em `Program Files`, e exige administrador (UAC) para instalar, atualizar e desinstalar. O uso diário roda sem administrador. Os dados (RN-18) e a inicialização automática (RN-17) são por usuário. Se o runtime WebView2 faltar, o instalador o baixa e instala (CA-50).

**RN-27**: O launcher nunca roda como administrador e nunca pede elevação por conta própria. Jogo que exige administrador só é aberto depois que o usuário confirma o aviso do SofaLauncher; então abre do jeito padrão do Windows, que mostra o UAC do próprio jogo (CA-73).

**RN-28**: Itens da Steam que não são jogos (redistribuíveis, ferramentas, trilhas sonoras) são filtrados por uma lista fixa de identificadores mantida no launcher. O que a lista não cobrir, o usuário oculta pelo controle (RN-04).

**RN-29**: Jogo manual é só um arquivo `.exe` válido, em disco local (fixo ou removível), iniciado sem argumentos. Pastas de rede, unidades de rede mapeadas e outros tipos de arquivo são recusados (CA-71, CA-72).

**RN-30**: O instalador não tem assinatura de código no MVP. Cada versão publicada vem com o SHA-256 do instalador, para conferência antes de instalar (CA-76).

**RN-31**: Telas de espera ("Abrindo…", "Fechando…") sempre têm um botão focável e ignoram A nos primeiros 600 ms, para um toque atrasado não acionar nada. Sair delas só interrompe a espera: não cancela o início nem o fechamento do jogo (CA-77, CA-78).

**RN-32**: A área segura de 5% (CA-54) admite uma única exceção visual: card de capa parcialmente cortado, como sinal de que há mais conteúdo. O corte pode ser lateral (borda da fileira) ou inferior (fileira seguinte sob o rodapé de dicas). Texto e destaque de foco do item em foco ficam sempre dentro da área segura.

**RN-33**: Opções do jogo (View) é o único lugar com ações por jogo, e "Jogar" é sempre o primeiro item, seguido de "Renomear", "Restaurar nome original" (se o nome foi alterado) e "Ocultar". Não há tela de detalhe do jogo no MVP (CA-79).

**RN-34**: Em todo diálogo, "Cancelar" ou "Voltar" fica sempre à esquerda e a ação principal à direita. A posição não muda o foco padrão, que segue RN-14 e o CA de cada diálogo (CA-81).

**RN-35**: "Fechar o launcher" é a única forma de fechar o launcher pelo menu rápido ("Sair do launcher" deixou de existir, D-53). Fica no grupo de energia, junto de "Desligar" e "Reiniciar", mas não é ação de energia: encerra só o launcher e não mexe no PC (D-50, D-51). Sempre pede confirmação com "Cancelar" à esquerda e em foco por padrão (RN-14, RN-34), e a confirmação diz como reabrir: pelo atalho do SofaLauncher ou reiniciando o PC (D-55). O teclado de emergência cobre o item e o diálogo como qualquer ação do controle (RN-07). Com sessão de jogo ativa, o launcher não fecha, não minimiza, não pausa e não traz o jogo à frente antes de sair (D-57); deixa de acompanhar a sessão, e a data de último jogado já gravada no início dela continua valendo (RN-05). Fechar o launcher não altera "Iniciar com o Windows" (RN-17). Enquanto o launcher estiver fechado, a combinação de retorno não tem efeito e o fim do jogo não traz nada à frente. Ao reabrir com o jogo ainda rodando, o jogo é tratado como aberto fora do launcher (E-07): a sessão não é retomada e o jogo fica no topo dos recentes (CA-75, D-54) (CA-82).

## 11. Requisitos não funcionais

- **Compatibilidade:** Windows 10 (versões com suporte da Microsoft na data do lançamento) e Windows 11, 64 bits. Alvo principal: Windows 10 22H2, o PC da sala (D-32).
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
| E-02 | Jogo em tela cheia exclusiva não deixa o launcher vir para a frente | O launcher segue a sequência de tentativas de RN-24 e mostra o painel quando uma funcionar (CA-63). Validar no teste final com jogos reais (R-01). |
| E-03 | A loja (Steam, Epic, GOG Galaxy) pede login ou atualização ao iniciar um jogo | A tela da loja aparece; o launcher espera até o tempo de CA-29; se o jogo abrir depois dele, a sessão segue normal (CA-29). Se a tela da loja não aceitar controle, é limitação da loja (R-08). |
| E-04 | Jogo trava e não responde | "Fechar jogo" chega ao "Forçar fechamento" após 10 s (CA-35). |
| E-05 | Controle desconecta durante o jogo | O launcher não faz nada; ao reconectar, a combinação de retorno volta a funcionar. |
| E-06 | Usuário segura a combinação de retorno com o launcher já na frente | Sem efeito (RN-11). |
| E-07 | Jogo é aberto fora do launcher (pela loja, atalho) | Não conta como sessão no MVP; a combinação de retorno não funciona. Candidato para versão seguinte. |
| E-08 | Monitor e TV ligados ao mesmo tempo | Launcher abre no monitor do launcher (RN-20). |
| E-09 | TV desligada com o PC ligado e depois religada | Ao voltar a imagem, o launcher continua em tela cheia na mesma tela. Se o Windows tiver desconectado a TV nesse meio-tempo, vale CA-67. |
| E-10 | Executável cadastrado em unidade externa desconectada | Mesmo comportamento de CA-28 (só o nome do jogo, sem caminho). |
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
| E-24 | Usuário fechou o launcher com o jogo aberto e depois segura View + Menu | Nada acontece: não há launcher rodando para vir à frente, e o jogo recebe os botões normalmente. Mesmo depois de reabrir o launcher, a combinação não vale para esse jogo, que conta como aberto fora do launcher (RN-35, E-07, D-54). |
| E-25 | Usuário fechou o launcher com o jogo aberto e depois sai do jogo pelo menu dele | Com o launcher fechado, o Windows mostra a área de trabalho e nada volta à frente sozinho. Se o launcher já foi reaberto, ele também não vem à frente, porque não há sessão (RN-35, D-54). |
| E-26 | Launcher reaberto com o jogo da sessão anterior rodando; o usuário aperta A nesse jogo | O jogo conta como aberto fora do launcher (E-07). Se ele só traz a própria janela, sem processo novo, o launcher pode mostrar a mensagem do CA-29 (R-12). |
| E-27 | Usuário reabre o launcher logo depois de fechá-lo, enquanto a instância antiga ainda está saindo | Pode não aparecer nada. Abrir de novo resolve (R-13). |

## 13. Riscos

| ID | Risco | Impacto | Mitigação |
|---|---|---|---|
| R-01 | Windows restringe quem pode roubar o primeiro plano; jogos em tela cheia exclusiva podem não ceder a frente ao launcher. | Quebra US-07, que é o diferencial, e o retorno automático no fim da sessão (CA-30). | Spike técnico SP-01 (`docs/arch/spikes/SP-01.md`): minimizar o jogo trouxe o SofaLauncher à frente em só 1 de 26 tentativas, e fechar o jogo também não devolve o foco. Adotada a sequência de quatro tentativas de RN-24 (D-41), aprovada pela arquitetura (ADR-0003) e pela segurança. Validação com jogos reais no teste final do usuário (QA-17). |
| R-02 | Detectar fim de sessão para jogos de loja é indireto: o launcher pede à loja para iniciar o jogo e não controla o processo. Na Epic e no GOG Galaxy o jogo pode passar por processos intermediários da loja. | Launcher não volta sozinho (CA-30) ou volta antes da hora. | Regra por pasta de instalação (RN-10) + período de tolerância (E-14). Validar no spike com ao menos um jogo de cada loja. |
| R-03 | Launchers de publicadoras e anti-cheat abrem processos e janelas próprios. | Sessão mal detectada; janelas que não aceitam controle. | Aceitar como limitação no MVP e listar jogos problemáticos encontrados nos testes. |
| R-04 | Telas do Windows (UAC, login, atualização) não aceitam controle. | Viola M-01 em casos pontuais. Instalar e atualizar exigem teclado/mouse por causa do UAC (RN-26). | Fora do escopo (RN-19); instalação é rara. Orientar o usuário a configurar logon e janelas de atualização. |
| R-05 | O botão Guide (Xbox) não é exposto pela API XInput documentada e o Xbox Game Bar o captura por padrão. | Motivo para não usar o Guide como retorno. | Combinação definida como View + Menu (RN-11). Afirmação a confirmar pelo arquiteto. |
| R-06 | Instalador sem assinatura de código mostra aviso do SmartScreen. | Atrito só na instalação; uso pessoal. | Aceito no MVP (D-11, D-27). SHA-256 publicado com cada versão (RN-30). |
| R-07 | Anti-cheat de alguns jogos pode desconfiar de processos que leem entrada global ou mexem em janelas. | Risco baixo, mas possível bloqueio. A tecla F24 sintética (degrau 2, terceira tentativa de RN-24) pode ser tratada como macro por algum anti-cheat. | O launcher só lê o controle via API padrão e não injeta nada no jogo (overlay fora de escopo). A tecla F24 é enviada uma vez por retorno e pode ser desligada em configuração avançada. Validação com jogo com anti-cheat no teste final do usuário (QA-17, offline, D-30). |
| R-08 | Epic e GOG guardam a lista de jogos instalados em arquivos locais sem documentação pública; o formato pode mudar com atualizações das lojas. A Epic costuma exigir o próprio launcher aberto e logado para iniciar jogos. | Jogos somem da biblioteca após atualização da loja; início lento ou bloqueado por login. | Falha de leitura de uma loja não derruba as outras (seção 11). Cadastro manual como alternativa. Arquiteto confirma no spike o que cada loja expõe, incluindo jogos GOG sem Galaxy (E-17). |
| R-09 | Outro programa rodando com o mesmo usuário do Windows pode alterar a biblioteca salva. | Jogos ou caminhos adulterados na biblioteca. | Risco aceito (D-26). Ao carregar, o launcher reaplica as mesmas validações do cadastro (RN-29), então caminho de rede ou arquivo inválido não inicia. |
| R-10 | Depois de "Fechar o launcher", só o atalho do SofaLauncher (teclado ou mouse) ou reiniciar o PC trazem o launcher de volta. | Viola M-01 até reabrir, se o usuário fechar sem querer ou durante o jogo. | Consequência de D-51. Confirmação com "Cancelar" em foco, aviso de jogo aberto e instrução de como reabrir (RN-35, CA-82, D-55). |
| R-11 | Com o nome novo, a chave de desinstalação mudaria se não fosse fixada em `Uninstall\Sofa` (D-56). | Duas entradas em "Aplicativos instalados" e o instalador 0.2.0 deixa de conferir e chamar o desinstalador antigo (SEC-BUG-05). | Chave fixada no literal `Uninstall\Sofa`, com teste no script de teste do instalador (arquitetura, seção 14). |
| R-12 | Com o jogo da sessão anterior ainda aberto (D-54), iniciar de novo o mesmo jogo pode cair na mensagem de "não foi possível confirmar" (CA-29), se o jogo só traz a própria janela. | Mensagem de erro com o jogo já aberto. | Comportamento já previsto para E-07 (E-26). O QA-17 observa. |
| R-13 | Reabrir enquanto o processo antigo ainda está saindo pode entregar o pedido de reabertura à janela que está fechando. | Nada aparece. Probabilidade baixa: o fechamento leva menos de 1 s. | Abrir de novo (E-27). |

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
| D-15 | Nome e marca | Produto "Sofá"; identificadores técnicos `sofa`; identificador do app e pasta de dados `app.sofa.launcher`. Marca: direção A, "Palco" (`docs/brand/sofa/`). Nome e logo substituídos por D-49; identificadores mantidos. | Cabeçalho, seção 6, RN-18, CA-68 |
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
| D-28 | AF-10. Desinstalador apagando dados | Apaga só os arquivos da pasta de dados do SofaLauncher, com travas para não sair dela. | CA-68 |
| D-29 | Lista de jogos para teste | A lista de jogos reais do usuário vem depois, para o QA final. | Validação (arquitetura §9) |
| D-30 | Teste de anti-cheat | Feito offline, no QA final. | R-07 |
| D-31 | Conta do teste final | Conta padrão do Windows + credencial de outro administrador no UAC. | E-19, CA-50 a CA-53 |
| D-32 | PC da sala | Windows 10 (22H2) é o alvo principal; Windows 11 é secundário. | Seção 11 (compatibilidade) |
| D-33 | Hardware ausente | O que depender de hardware não disponível no desenvolvimento fica registrado como "não verificado" até o QA final. | Validação |
| D-34 | CI | Script local mais pasta de releases. | RN-30 (publicação do SHA-256) |
| D-35 | Testes do usuário | O usuário testa só no fim; não há spike com jogos, controle ou contas reais dele. | Pendências |
| D-36 | Jogo que exige administrador | Aviso do SofaLauncher antes do UAC ("precisa de permissão… Continuar / Voltar"), depois o UAC do jogo. | CA-73, RN-27 |
| D-37 | Telas de espera | "Abrindo…" tem "Voltar" e "Fechando…" tem "Ir para a biblioteca". Voltar durante "Abrindo" interrompe a espera, mas o jogo pode abrir depois. | CA-26, CA-35, CA-77, CA-78, RN-31 |
| D-38 | Cards cortados na borda | Aceitos como sinal de continuidade, exceção visual à área segura; texto e itens focáveis continuam dentro dela. | CA-54, RN-32 |
| D-39 | Erro de caminho de rede | Mensagem própria no cadastro e outra ao iniciar jogo. | CA-71 |
| D-40 | Opções do jogo | View traz "Jogar" como primeiro item; sem tela de detalhe. | CA-79, RN-33, seção 8 |
| D-41 | Resultado do SP-01 e ambiente de teste | (a) Para vir à frente do jogo, sequência padrão: pedido simples → vincular à entrada do jogo → uma tecla F24 sintética → minimizar, configurável. Vale para a combinação de retorno e para o fim da sessão. Substitui o "se falhar, minimiza" de D-04; "nunca pausar" continua. Aprovada pela arquitetura (ADR-0003) e pela segurança (rev. 5, seção 12). (b) O build de release usa o MSVC instalado pelo usuário. (c) A conta de teste não foi criada; os testes com outra conta ficam para o QA final. | RN-24, CA-30, CA-63, E-02, R-01, R-07 |
| D-42 | Caminho na mensagem de arquivo sumido | A UI mostra só o nome do jogo, porque nenhum caminho passa entre o núcleo e a tela (SEC-03). | CA-28, E-10 |
| D-43 | AF-11 / DR-06. Falha ao apagar os dados na desinstalação | Se a remoção sem administrador falhar, o desinstalador apaga como administrador só `library.json` e `settings.json` na raiz da pasta de dados, sem curinga. Capas e logs ficam e a pasta é mostrada ao usuário. | CA-68, RN-18 |
| D-44 | Tipo de desligamento | "Desligar" igual ao Desligar do menu Iniciar, com a inicialização rápida do Windows quando ela estiver ligada. "Reiniciar" é completo. Substitui o desvio 5 aceito na revisão de arquitetura (desligar sem o modo híbrido). | RN-15, CA-44, CA-45 |
| D-45 | D-QA-02. Cobertura do teclado de emergência | O teclado cobre todas as ações do controle: abas, opções do jogo, volume, voltar, menu rápido e painel. A combinação de retorno (View + Menu) continua só no controle; o teclado usa F1 e F2. As teclas exatas ficam no handoff ou no front. Complementa D-08. | RN-07, CA-61, CA-80, seção 8 |
| D-46 | D-QA-04. Card cortado embaixo | Permitido, com a mesma lógica de D-38: a fileira seguinte pode aparecer cortada sob o rodapé. Texto e foco do item focado sempre dentro da área segura. | CA-54, RN-32 |
| D-47 | DES-BUG-04. Ordem dos botões nos diálogos | Cancelar ou Voltar sempre à esquerda, ação principal à direita. Regra única. | RN-34, CA-81 |
| D-48 | D-QA-01. Ordem das opções do jogo | Jogar, Renomear, Restaurar nome original (se o nome foi alterado) e Ocultar, como nos mockups. | CA-79, RN-33, seção 8 |
| D-49 | Nome do produto (após a v0.1.0) | O produto passa a se chamar SofaLauncher, com a sigla SL no logo e no ícone. O logo passa a ser um controle, no lugar do símbolo de play. Substitui o nome e o logo de D-15. Sigla e logo ajustados por D-52; identificadores por D-56. | Cabeçalho, seção 6, CA-68, CA-71, CA-73, CA-77, RN-27, R-01, D-15 |
| D-50 | Onde fica "Fechar o launcher" (após a v0.1.0) | No menu de energia, junto de Desligar e Reiniciar, com confirmação e "Cancelar"/"Voltar" à esquerda (RN-34). | RN-14, RN-35, CA-82, escopo item 10 |
| D-51 | Fechar o launcher com jogo aberto (após a v0.1.0) | Fecha só o launcher. O jogo continua aberto e "Iniciar com o Windows" segue valendo no próximo boot. | RN-35, CA-82, E-24, E-25, R-10 |
| D-52 | Logo aprovado | O logo é o controle, com "SofaLauncher" escrito por extenso. SL é só uma sigla de texto e não vai no logo nem no ícone. Ajusta D-49. | Cabeçalho, seção 6 |
| D-53 | Q-14. "Sair do launcher" e "Fechar o launcher" | Substituir: fica só "Fechar o launcher", no menu de energia. "Sair do launcher" sai do menu rápido. | CA-05, CA-82, US-14, RN-14, RN-35, seção 4 (menu rápido) |
| D-54 | Q-15. Reabrir com o jogo rodando | O jogo é tratado como aberto fora do launcher (E-07). A sessão não é retomada e o jogo fica no topo dos recentes (CA-75). | CA-82, RN-35, E-24 a E-26, R-12 |
| D-55 | Q-16. Aviso de como reabrir | A confirmação diz como reabrir: pelo atalho do SofaLauncher ou reiniciando o PC. Texto final com o designer. | CA-82, RN-35, R-10 |
| D-56 | Q-18. Identificadores técnicos | Ficam como estão (ADR-0007): `app.sofa.launcher`, `sofa.exe`, pasta `Program Files\Sofa` e chave `Uninstall\Sofa`. Sem migração. | Cabeçalho, seção 6, R-11 |
| D-57 | Q-17. Trazer o jogo à frente antes de fechar | Não trazer. Os PCs com a 0.1.0 são atualizados por cima, com a versão 0.2.0. | CA-82, RN-35, CA-53 |

### Pendências

- **Spike técnico (arquiteto, sem o usuário, D-35; validação com jogos reais no QA final, D-29):** primeiro plano e fallback de minimizar (R-01), fim de sessão por loja (R-02), formato local de Epic e GOG e jogos GOG sem Galaxy (R-08, E-17), registro de "Iniciar com o Windows" com UAC de outra conta (E-19), tempo da inicialização automática e tarefa agendada sem administrador (D-18), confirmação sobre o botão Guide (R-05).
- **Perguntas de D-49 a D-51, todas fechadas:** Q-14 (substituir "Sair do launcher", D-53), Q-15 (jogo aberto fora do launcher ao reabrir, D-54), Q-16 (aviso de como reabrir, D-55), Q-17 (não trazer o jogo à frente, D-57) e Q-18 (identificadores mantidos, D-56).
- **Docs fora desta spec ainda com o nome e o item antigos:** guia de marca (logo com play), microcopy, mockups e roteiro do usuário dizem "Sofá", ainda têm "Sair do launcher" e não têm "Fechar o launcher" nem o aviso de como reabrir. Atualização a cargo de design e QA.
