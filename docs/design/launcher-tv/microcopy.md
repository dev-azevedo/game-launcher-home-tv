# Microcopy do SofaLauncher (pt-BR)

Revisão pós-0.1.0 (D-49, D-50, D-51, D-52, D-53, D-55): o produto passa a se chamar SofaLauncher e "Sair do launcher" vira "Fechar o launcher", no grupo de energia. Nome por extenso em toda a interface. "SL" é sigla só de texto, para espaço estreito (título de janela estreita, tooltip, identificador curto): na TV, sem uso hoje. O logo é o controle de videogame (`assets/logo-horizontal.svg`, `assets/splash.svg`).

Voz: frases curtas, verbo no botão, diz o que houve e o que fazer. Sem "ops", sem exclamação, sem "por favor". Termos da spec: Biblioteca, Jogo, Origem, Item oculto, Ocultar, Itens ocultos, Combinação de retorno, Menu rápido.

Todo texto com dado externo (nome de jogo, caminho, nome de monitor) entra como texto puro (SEC-07). `<nome>` e `<caminho>` abaixo são esses dados.

Frases marcadas com (spec) são literais da spec e não mudam sem mudar a spec.

## 1. Textos de tela

### Geral
| Onde | Texto |
|---|---|
| Dicas de botão (rodapé) | A Jogar · View Opções · Menu Menu · LT RT Volume |
| Indicador de controle | "Controle conectado" / "Sem controle" |
| Indicador de volume | número de 0 a 100, "Mudo", "Sem áudio" |
| Origem no card | Steam, Epic, GOG, Manual |
| Selo no card | Em andamento |
| Último jogado | Jogado hoje / Jogado ontem / Jogado há N dias / Nunca jogado |
| Herói com jogo em andamento | Em andamento. Aperte A para voltar ao jogo. |

### Abertura
| Elemento | Texto |
|---|---|
| Status | Carregando biblioteca… |

### Controle
| Situação | Texto |
|---|---|
| Nenhum controle ao abrir (spec) | Nenhum controle conectado. Ligue o controle ou conecte o cabo. |
| Controle desconectou | Controle desconectado. Ligue o controle ou conecte o cabo. |
| Controle voltou (some em até 2 s) | Controle conectado. |

### Início e Todos os jogos
| Elemento | Texto |
|---|---|
| Linhas | Recentes · Biblioteca |
| Último card | Todos os jogos |
| Título de Todos os jogos | Todos os jogos · "N jogos, de A a Z" |
| Dicas de Todos os jogos | A Jogar · View Opções · LB RB Pular linhas · B Voltar |
| Biblioteca vazia, título | Nenhum jogo por aqui |
| Biblioteca vazia, texto | Instale jogos na Steam, na Epic ou no GOG, ou adicione um executável. |
| Biblioteca vazia, botões | Adicionar jogo (em foco) · Atualizar biblioteca |
| Carregando, título | Procurando jogos… |
| Carregando, texto | Lendo a Steam, a Epic e o GOG deste PC. |
| Atualizando (chip) | Atualizando biblioteca… |
| Atualização concluída | Biblioteca atualizada. |
| Atualização sem novidade (biblioteca vazia) | Nenhum jogo novo. |

### Opções do jogo
| Elemento | Texto |
|---|---|
| Itens | Jogar (ou Voltar ao jogo se já está aberto) · Renomear · Restaurar nome original · Ocultar |
| Dica em Renomear | Pelo teclado do PC |
| Dica em Ocultar | Nenhum arquivo é apagado |
| Dica em Restaurar | o nome original, por extenso |
| Renomear, título | Renomear jogo |
| Renomear, linha de apoio | Nome original: <nome> |
| Renomear, instrução | Digite no teclado do PC. Enter salva, Esc cancela. |
| Renomear, botões | Cancelar · Salvar |
| Nome vazio | O nome não pode ficar vazio. |
| Nome salvo | Nome salvo. |
| Nome restaurado | Nome original restaurado. |
| Ocultar, título | Ocultar <nome>? |
| Ocultar, texto | O jogo sai da biblioteca e não volta nas atualizações. Nenhum arquivo é apagado. / Para restaurar, abra Configurações e escolha Itens ocultos. |
| Ocultar, botões | Cancelar (em foco) · Ocultar |
| Ocultar, confirmação | <nome> oculto. Restaure em Configurações, Itens ocultos. |

### Abrir jogo e sessão
| Elemento | Texto |
|---|---|
| Abrindo, título (spec) | Abrindo <nome>… |
| Abrindo, apoio e botão | Aguarde. O jogo vai aparecer na frente. · Voltar |
| Trocar de jogo (spec) | <atual> está aberto. Fechar e abrir <novo>? |
| Trocar, apoio e botões | O progresso não salvo de <atual> pode ser perdido. · Cancelar (em foco) · Fechar e abrir |
| Painel do jogo, selo | Em andamento |
| Painel do jogo, apoio | Jogando agora / Jogando há N min / Jogando há H h M min |
| Painel do jogo, itens | Voltar ao jogo (em foco) · Fechar jogo · Ir para a biblioteca ("O jogo continua aberto") · Menu rápido |
| Painel, jogo minimizado | O Windows não deixou o SofaLauncher passar na frente. O jogo foi minimizado. Voltar ao jogo restaura a janela. |
| Painel, jogo como administrador | Este jogo roda como administrador. A combinação de retorno pode falhar nele. |
| Fechar jogo, título | Fechar <nome>? |
| Fechar jogo, texto | O SofaLauncher pede ao jogo para fechar. / O progresso não salvo pode ser perdido. |
| Fechar jogo, botões | Cancelar (em foco) · Fechar jogo |
| Fechando | Fechando <nome>… · Esperando o jogo fechar. Pode levar alguns segundos. · botão: Ir para a biblioteca |
| Forçar, título | <nome> não fechou |
| Forçar, texto | O jogo não respondeu ao pedido para fechar. / O progresso não salvo será perdido. (spec: "Forçar fechamento (o progresso não salvo será perdido)") |
| Forçar, botões | Cancelar (em foco) · Forçar fechamento |
| Jogo fechou | <nome> fechado. |

### Menu rápido
| Elemento | Texto |
|---|---|
| Itens, em ordem | Volume · Mudo · (grupo de energia) Desligar · Reiniciar · Suspender · Fechar o launcher · (grupo) Configurações. "Sair do launcher" não existe mais (Q-14) |
| Dica de gatilhos | Na biblioteca, LT diminui e RT aumenta o volume. |
| Sem áudio (spec) | Nenhum dispositivo de áudio encontrado |
| Sem suspensão (spec) | Suspensão indisponível neste PC |
| Dicas | D-pad Ajustar volume · A Escolher · B Fechar |
| Desligar | Desligar o PC? / O Windows desliga do jeito normal. Se outro app tiver trabalho não salvo, o Windows avisa. / Cancelar (em foco) · Desligar |
| Reiniciar | Reiniciar o PC? / O Windows reinicia do jeito normal. (com Iniciar com o Windows ligado: " O SofaLauncher abre de novo quando você entrar.") / Cancelar · Reiniciar |
| Suspender | Suspender o PC? / O PC entra em suspensão. Ao acordar, o SofaLauncher volta para esta tela. / Cancelar · Suspender |
| Com jogo aberto (spec) | <nome> está aberto. O progresso não salvo pode ser perdido. |
| Em execução | Desligando… / Reiniciando… / Suspendendo… · "O Windows está encerrando." / "…reiniciando." / "O PC está entrando em suspensão." · botão Voltar |
| Fechar o launcher, título (os dois casos) | Fechar o launcher? |
| Fechar o launcher, sem jogo aberto | O SofaLauncher fecha e o Windows mostra a área de trabalho. / Para reabrir, use o atalho do SofaLauncher (com teclado ou mouse) ou reinicie o PC. |
| Fechar o launcher, com jogo aberto | Só o SofaLauncher fecha. / Para reabrir, use o atalho do SofaLauncher ou reinicie o PC depois de sair do jogo. / Aviso âmbar: <nome> continuará aberto. Nenhum progresso se perde. |
| Fechar o launcher, botões | Cancelar (em foco, à esquerda) · Fechar o launcher (à direita, sem vermelho: não desliga o PC nem perde progresso) |
| Fechar o launcher, nome do diálogo (leitor de tela) | Fechar o launcher |
| Fechado (só no mockup, simula a área de trabalho) | Simulação: o SofaLauncher fechou e o Windows mostra a área de trabalho. / Abrir o SofaLauncher |

Regras do diálogo de Fechar o launcher:
- Sem o aviso de progresso perdido do CA-48: nada se perde. O aviso âmbar é só o nome do jogo que segue aberto (CA-82, item 3).
- Não promete que o controle reabre o launcher. O texto sem jogo diz que o atalho precisa de teclado ou mouse. A combinação de retorno também para (R-10, Q-16).
- Reiniciar o PC fecha o jogo aberto. Por isso, com jogo, o texto diz "depois de sair do jogo".
- Se "Iniciar com o Windows" estiver desligado, reiniciar o PC não reabre o launcher. Variante proposta nesse caso (dúvida 1): trocar a segunda frase por "Para reabrir, use o atalho do SofaLauncher (com teclado ou mouse)." e, com jogo, "Para reabrir, use o atalho do SofaLauncher." O mockup mostra só o texto pedido.
- Item sem ícone vermelho: Desligar, Reiniciar e Suspender são vermelhos (afetam o PC); Fechar o launcher usa o ícone neutro de saída.

### Configurações
| Elemento | Texto |
|---|---|
| Título e abas | Configurações · Tela · Biblioteca · Geral · Itens ocultos (com contagem) |
| Dicas | Abas: A Entrar · B Voltar. Painel: A Escolher · B Abas |
| Tela | Monitor do launcher / O SofaLauncher abre em tela cheia no monitor escolhido. / <monitor> · <resolução> · Principal / Em uso |
| Um monitor | Só há um monitor conectado. |
| Monitor ausente | O monitor escolhido não está conectado. O SofaLauncher está no monitor principal e volta ao monitor escolhido quando ele reconectar. |
| Monitor trocado | SofaLauncher movido para <monitor>. |
| Biblioteca | Resumo: "N jogos: 6 Steam, 3 Epic…" · Atualizar biblioteca ("Procura jogos novos na Steam, na Epic e no GOG.") · Adicionar jogo ("Escolha um arquivo .exe em um disco deste PC.") |
| Iniciar com o Windows | Iniciar com o Windows / Abre o SofaLauncher quando você entra no Windows. Vale só para o seu usuário. |
| Aviso de logon (RN-19) | Para ligar o PC e jogar sem teclado, configure o logon automático ou o Windows Hello no Windows. O SofaLauncher começa na área de trabalho. |
| Aviso de elevação | O SofaLauncher nunca roda como administrador. Jogos que exigem administrador pedem confirmação no Windows, com teclado ou mouse. |
| Rodapé | SofaLauncher 0.2.0 · Fontes Sora e Figtree sob licença OFL |
| Itens ocultos | Escolha um item para voltar à biblioteca. |
| Itens ocultos vazio (spec) | Nenhum item oculto / Os jogos que você ocultar aparecem aqui. |
| Restaurar | Restaurar <nome>? / O jogo volta para a biblioteca com o nome e a data de último jogo que tinha. / Cancelar · Restaurar (em foco) |
| Restaurado | <nome> voltou para a biblioteca. |

### Navegador de pastas
| Elemento | Texto |
|---|---|
| Título e apoio | Adicionar jogo / Escolha o arquivo .exe do jogo. Só aparecem discos deste PC. |
| Raiz | Este computador |
| Linha de arquivo | Adicionar |
| Dicas | A Abrir ou escolher · B Pasta anterior (na raiz: Voltar) |
| Pasta vazia | Nenhuma pasta ou .exe aqui / Esta pasta não tem pastas nem arquivos .exe. / Voltar |
| Sem permissão | Sem permissão para abrir esta pasta / O Windows não deixa o SofaLauncher ver o que há aqui. / Voltar |
| Pasta não existe | Não encontramos esta pasta / Ela pode ter sido removida, ou o disco foi desconectado. / Voltar |
| Confirmar | Adicionar este jogo? / O jogo entra na biblioteca como <nome>. Dá para renomear depois. / <caminho> / Voltar · Adicionar (em foco) |
| Adicionado | <nome> adicionado. |

## 2. Erros e avisos por código

A UI traduz o `code` do `ErrorDto`. Nenhuma mensagem mostra detalhe técnico, caminho do sistema do SofaLauncher nem texto vindo do núcleo (SEC-06). Catálogo fechado contra o contrato rev. 2 (`docs/api/launcher-tv.md`, seção 5): os 26 códigos têm texto aqui e em `ui/src/shared/domain/errors.ts`. Código fora da tabela cai em "Algo deu errado no SofaLauncher."

| Código | Comando | Onde aparece | Texto | Botões (foco em negrito) |
|---|---|---|---|---|
| `Busy` | `library_refresh`, `session_launch` | Diálogo | Um momento. O SofaLauncher ainda está terminando outra ação. Tente de novo em instantes. | **Voltar** |
| `Duplicate` | `library_add_manual` | Diálogo | Este jogo já está na biblioteca (spec). Apoio: Não criamos uma segunda cópia. | **Voltar** |
| `NotAnExecutable` | `library_add_manual` | Diálogo | Este arquivo não é um executável válido (spec). Apoio: O arquivo tem a extensão .exe, mas o Windows não o reconhece como programa. Escolha outro arquivo. | **Voltar** |
| `PathNotAllowed` | `library_add_manual`, `fs_list_dir` | Diálogo | Este local não é aceito. O SofaLauncher só abre jogos de discos deste PC. | **Voltar** |
| `NetworkPathBlocked` | `session_launch` | Diálogo | Jogos em pastas de rede não são aceitos. Copie o jogo para um disco deste PC. (spec, CA-71) | **Voltar** · Ocultar |
| `NotFound` (pasta) | `fs_list_dir` | Estado da lista | Não encontramos esta pasta. Ela pode ter sido removida, ou o disco foi desconectado. | **Voltar** |
| `AccessDenied` (pasta) | `fs_list_dir` | Estado da lista | Sem permissão para abrir esta pasta. O Windows não deixa o SofaLauncher ver o que há aqui. | **Voltar** |
| `EmptyName` | `game_rename` | Linha no diálogo | O nome não pode ficar vazio. | (campo em foco) |
| `UnknownGame` | `game_rename`, `game_hide`, `game_unhide`, `game_restore_name` | Diálogo | Este jogo não está mais na biblioteca. | **Voltar** |
| `SessionAlreadyActive` | `session_launch` | Diálogo | <atual> está aberto. Fechar e abrir <novo>? (spec) | **Cancelar** · Fechar e abrir |
| `GameFileMissing` | `session_launch` | Diálogo | Título: <nome>. Texto: Não encontramos o arquivo do jogo. O caminho não cruza a IPC (contrato, SEC-03), então a frase da spec (CA-28) perde o "em <caminho>". Pendência de spec para o PO | **Voltar** · Ocultar |
| (estado) sem processo em 60 s | `SessionDto` | Diálogo | Não foi possível confirmar que <nome> abriu (spec) | Voltar · **Tentar de novo** |
| `StoreNotInstalled` | `session_launch` | Diálogo | Não encontramos a <loja> neste PC. Instale a loja para abrir este jogo. | **Voltar** |
| `NeedsElevation` | `session_launch` | Diálogo, 1ª etapa | <nome> precisa de permissão de administrador para abrir. O Windows vai pedir a sua confirmação. | Voltar · **Continuar** |
| (estado) esperando o UAC | `session_launch_with_uac` | Diálogo | <nome> pede permissão de administrador. Confirme no Windows com teclado ou mouse. A combinação de retorno pode não funcionar neste jogo. (spec) | **Voltar** |
| `ElevationDenied` (e `launchFailure` do `SessionDto`) | `session_launch_with_uac` | Diálogo | O Windows não deu permissão para abrir <nome>. (spec) | **Voltar** |
| `InvalidState` | vários | Diálogo | Isso não pode ser feito agora. Volte e tente de novo. | **Voltar** |
| `NoActiveSession` | `session_return_to_game`, `session_close` | Diálogo | Nenhum jogo está aberto. | **Voltar** (leva à biblioteca) |
| `ForegroundDenied` | `session_return_to_game` | Diálogo | O Windows não deixou voltar ao jogo. Tente de novo. | Ir para a biblioteca · **Tentar de novo** |
| `NotOverdue` | `session_force_close` | Diálogo | O jogo ainda está fechando. Espere alguns segundos. | **Voltar** |
| `AccessDenied` (forçar) | `session_force_close` | Diálogo | Não foi possível fechar <nome>. O jogo roda como administrador. Feche pelo menu do próprio jogo. | **Voltar** |
| `NoAudioDevice` | `volume_step`, `volume_toggle_mute` | Indicador e menu | Nenhum dispositivo de áudio encontrado (spec). No indicador: "Sem áudio". | (item desabilitado) |
| `Unavailable` | `power_prepare` | Item do menu | Suspensão indisponível neste PC (spec). Para Desligar e Reiniciar: Esta ação não está disponível neste PC. | (item desabilitado) |
| `TicketInvalid` | `power_execute` | Diálogo | A confirmação expirou. Escolha a ação de novo. | **Voltar** |
| `Rejected` | `power_execute` | Diálogo | O Windows não aceitou o pedido. Tente de novo. | **Voltar** |
| `AutostartFailed` | `settings_set_autostart` | Linha na configuração | Não foi possível alterar a inicialização com o Windows. Tente de novo. | (item em foco) |
| `UnknownMonitor` | `settings_set_monitor` | Toast | Esse monitor não está mais conectado. Escolha outro. | (lista em foco) |
| `InvalidRequest` | todos | Diálogo | Algo deu errado no SofaLauncher. Tente de novo. (mesmo texto de `Internal`) | **Voltar** |
| `Internal` | todos | Diálogo | Algo deu errado no SofaLauncher. Tente de novo. | **Voltar** |
| Código fora da tabela | qualquer | Diálogo | Algo deu errado no SofaLauncher. Tente de novo. | **Voltar** |

Fora do app (instalador, CA-50): "Não foi possível baixar o WebView2. Conecte o PC à internet e tente de novo." (spec)

Regras de texto:
- Ordem dos botões (RN-34, CA-81, D-47): "Cancelar" ou "Voltar" sempre à esquerda, a ação principal à direita. A ordem nesta tabela é a da tela, da esquerda para a direita. O foco padrão não muda com a posição.
- Erro que o usuário não consegue resolver pelo controle diz o que fazer em outro lugar (Windows, teclado, mouse).
- Se o código trouxer parâmetros (`GameFileMissing{path}`, `SessionAlreadyActive{gameId}`), a UI resolve o nome do jogo pela biblioteca, nunca por texto do núcleo.
- Nome longo no texto de diálogo quebra linha (sem truncar). Nome de jogo que parece HTML aparece literal.

## 3. Textos acrescentados na implementação (validados)

| Onde | Texto final | Observação |
|---|---|---|
| Navegador de pastas, lista cortada (`truncated`) | A lista foi cortada. Mostrando só as primeiras 2.000 entradas desta pasta. | Linha de aviso acima da lista, sem tirar o foco. Aprovado como veio. |
| Configurações > Geral, rótulo | Inicialização | Linha de estado abaixo de "Iniciar com o Windows". |
| Valores de Inicialização | Tarefa agendada do Windows · Entrada de inicialização do seu usuário · Desligada | Ajuste: "Chave Run do seu usuário" é jargão e foge da voz da marca. |
| Configurações > Geral, rótulo | Pasta de logs | Mostra o caminho como texto puro. |
| Configurações > Tela, monitor | <nome> · Principal · Em uso | Sem resolução (o `MonitorDto` não traz e não é necessária). |
| Card "Em andamento" durante Fechando ou fechamento atrasado | A reabre "Fechando <nome>…" ou a oferta de "Forçar fechamento" | Mesmos textos de antes. Mesma trava de 600 ms. |

## 4. Instalador e desinstalador

Fonte do texto: `src-tauri/windows/PortugueseBR.nsh` (um `LangString` por linha). Este catálogo é a versão final; a comparação com o que o dev escreveu está em `revisao-instalador.md`. Voz da marca: frase curta, sem "ops" nem "por favor", diz o que houve e o que fazer.

Regras desta seção:
- O nome visível do produto é sempre "SofaLauncher" (D-49, D-52), uma palavra, sem acento. Dentro de `LangString`, usar `${DISPLAYNAME}` quando o nome vier de variável. Nunca `{{product_name}}` (o bundler não troca no arquivo de idioma). `${PRODUCTNAME}` passa a valer "SofaLauncher" com o `productName` novo, mas `${DISPLAYNAME}` continua sendo a variável do texto.
- A pasta `Program Files\Sofa`, a chave de desinstalação `Sofa`, o binário `sofa.exe` e a pasta de dados `app.sofa.launcher` são identificadores técnicos e não mudam (ADR-0007). Nenhum texto do instalador cita a pasta com o nome do produto.
- Reticências com o caractere "…" (U+2026), como no resto do app.
- Arquivo `.nsh` em UTF-8 **sem BOM** na fonte, de propósito. O nome novo não tem acento, mas as outras strings têm ("já", "está", "não"), então a regra e o guard `NSIS-BOM` ficam. O bundler do Tauri grava o BOM no arquivo gerado, e BOM duplo quebra o `makensis`. O guard `NSIS-BOM` confere fonte sem BOM e gerado com um. (A primeira versão deste catálogo pedia BOM na fonte, o que estava errado.)
- Tamanho (o nome novo tem 5 caracteres a mais que "Sofá"): rádio da página de reinstalação até cerca de 65 caracteres (uma linha); texto do corpo dessa página até cerca de 140; dica da página "Iniciar com o Windows" até cerca de 140; aviso de logon até cerca de 280.
- Termos: Biblioteca, Configurações, Iniciar com o Windows, Manter dados (CA-68).

### Reinstalar e atualizar
| ID | Onde aparece | Texto final |
|---|---|---|
| `addOrReinstall` | Rádio, mesma versão | Reinstalar o SofaLauncher |
| `alreadyInstalled` | Título da página | SofaLauncher já instalado |
| `alreadyInstalledLong` | Corpo, mesma versão | A versão `${VERSION}` já está instalada. Reinstalar mantém a biblioteca e as configurações. Desinstalar pergunta o que fazer com elas. |
| `chooseMaintenanceOption` | Subtítulo, mesma versão | Escolha o que fazer. |
| `chooseHowToInstall` | Subtítulo, versão diferente | Escolha como instalar o SofaLauncher. |
| `uninstallBeforeInstalling` | Rádio 1, versão diferente | Substituir a versão instalada (mantém seus dados) |
| `dontUninstall` | Rádio 2, versão diferente | Instalar por cima, sem remover a versão atual |
| `dontUninstallDowngrade` | Rádio 2 desligado, versão anterior | Instalar por cima (indisponível para versão anterior) |
| `olderOrUnknownVersionInstalled` | Corpo, versão instalada mais antiga ou desconhecida | Uma versão `$R4` do SofaLauncher está instalada. A biblioteca, as configurações e a opção Iniciar com o Windows continuam como estão. |
| `newerVersionInstalled` | Corpo, versão instalada mais nova | Uma versão mais nova do SofaLauncher já está instalada. Não recomendamos instalar uma versão anterior por cima. |
| `older` | Valor de `$R4` | mais antiga |
| `unknown` | Valor de `$R4` | desconhecida |
| `uninstallApp` | Rádio 2, mesma versão | Desinstalar o SofaLauncher |
| `silentDowngrades` | Aviso em instalação sem telas (`/P`, `/S`, `/UPDATE`) | Este modo de instalação não permite instalar uma versão anterior. Use o instalador com telas. (mantém o `$\n` do fim) |
| `unableToUninstall` | Caixa de aviso | Não foi possível remover a versão instalada. Feche o SofaLauncher e tente de novo. Se continuar, desinstale em Aplicativos instalados. |

### Pasta fixa (SEC-BUG-05, ARQ-BUG-16)
O instalador sempre usa Program Files\Sofa (identificador técnico, ADR-0007) e recusa outra pasta ou uma instalação anterior fora dali.

| ID | Onde aparece | Texto final |
|---|---|---|
| `sofaInstallDirFixed` | Caixa de aviso, instalador iniciado com `/D=` para outra pasta | O SofaLauncher só pode ser instalado em Program Files. Rode o instalador sem a opção /D=. |
| `sofaPreviousInstallElsewhere` | Caixa de aviso, instalação anterior fora de Program Files (`$SofaPrevDir` é a pasta) | Há uma instalação do SofaLauncher fora de Program Files (`$SofaPrevDir`). Desinstale-a em Aplicativos instalados e rode este instalador de novo. |
| `sofaUninstallDirFixed` | Caixa de aviso, desinstalador rodando de outra pasta | O desinstalador do SofaLauncher só roda a partir de Program Files. **Desinstale em Aplicativos instalados.** |

### SofaLauncher aberto
| ID | Onde aparece | Texto final |
|---|---|---|
| `appRunning` | Instalação cancelada com o SofaLauncher aberto | O `${DISPLAYNAME}` está aberto. Feche-o e tente de novo. |
| `appRunningOkKill` | Caixa OK/Cancelar | O `${DISPLAYNAME}` está aberto.`$\n`Clique em OK para fechá-lo. |
| `failedToKillApp` | Falha ao fechar | Não foi possível fechar o `${DISPLAYNAME}`. Feche-o você mesmo e tente de novo. |

### Atalho
| ID | Onde aparece | Texto final |
|---|---|---|
| `createDesktop` | Caixa na tela final | Criar atalho na área de trabalho |

### WebView2 (CA-50)
| ID | Onde aparece | Texto final |
|---|---|---|
| `installingWebview2` | Lista de detalhes | Instalando o WebView2… |
| `webview2InstallSuccess` | Lista de detalhes | WebView2 instalado. |
| `webview2InstallError` | Lista de detalhes | A instalação do WebView2 falhou (código `$1`). |
| `webview2AbortError` | Aborta a instalação (spec, não muda) | Não foi possível baixar o WebView2. Conecte o PC à internet e tente de novo. |
| `webview2Downloading`, `webview2DownloadSuccess`, `webview2DownloadError` | Removidos do `.nsh` (o bootstrapper embutido baixa sozinho, DES-10) | não existem mais |

### Página "Iniciar com o Windows" (CA-01, CA-51, RN-19)
| ID | Onde aparece | Texto final |
|---|---|---|
| `sofaOptionsTitle` | Título da página | Iniciar com o Windows |
| `sofaOptionsSubtitle` | Subtítulo | Escolha se o SofaLauncher abre sozinho quando você entra no Windows. |
| `sofaStartupCheckbox` | Caixa, marcada | Iniciar com o Windows |
| `sofaStartupHint` | Sob a caixa | Esta escolha vale como padrão para cada usuário do PC. Cada um pode mudar depois, em Configurações. |
| `sofaNoKeyboardNote` | Aviso na página | Para ligar o PC e jogar sem teclado, configure o logon automático ou o Windows Hello no Windows. O SofaLauncher começa na área de trabalho. O instalador não altera o seu logon. |

### Desinstalador (CA-52, CA-68, D-43)
| ID | Onde aparece | Texto final |
|---|---|---|
| `keepAppData` | Caixa na confirmação, marcada | Manter dados (biblioteca e configurações) |
| `sofaKeepData` | Lista de detalhes | Mantendo a biblioteca e as configurações. |
| `sofaPurgeStart` | Lista de detalhes | Apagando os dados do SofaLauncher do seu usuário… |
| `sofaSidIgnored` | Lista de detalhes | Um perfil do Windows com identificador inválido foi ignorado ao remover a inicialização. |
| `sofaPurgePartial` | Caixa de aviso, seguida da pasta | Não foi possível apagar todos os dados do SofaLauncher. Alguns arquivos podem ter ficado na pasta abaixo (biblioteca, configurações, capas, logs e cache). Você pode apagar a pasta. |
| `sofaPurgeUnread` | Caixa de aviso, seguida da pasta | Não foi possível confirmar a remoção dos dados do SofaLauncher. Alguns arquivos podem ter ficado na pasta abaixo (biblioteca, configurações, capas, logs e cache). Você pode apagar a pasta. |
| `sofaPurgeInvalidTarget` | Caixa de aviso, seguida da pasta | Os dados do SofaLauncher não foram apagados: a pasta não passou na verificação de segurança. Ela está abaixo. Confira o conteúdo antes de apagar. |
| `sofaPurgeAppOpen` | Caixa de aviso, seguida da pasta | O SofaLauncher estava aberto, então os dados não foram apagados. Eles continuam na pasta abaixo, e você pode apagá-la. |
| `sofaPurgeRunning` | Caixa de aviso, seguida da pasta | A remoção dos dados ainda não terminou. Se algo sobrar, apague a pasta abaixo. |
| `sofaPurgeNoProfile` | Caixa de aviso (sem pasta) | Não foi possível saber qual usuário está usando o Windows, então nenhum dado foi apagado. Para removê-los, apague a pasta app.sofa.launcher em AppData\Local do seu usuário. |

### Fora do `PortugueseBR.nsh` (texto visível que o instalador herda)
| Onde | Hoje | Final |
|---|---|---|
| Rodapé de todas as telas (`BrandingText`, vem de `copyright` em `tauri.conf.json`) | Copyright (c) Sofá | Copyright (c) SofaLauncher |
| "Aplicativos instalados", coluna Editor (`publisher` em `tauri.conf.json`) | Sofá | SofaLauncher |
| Atalhos do menu Iniciar e da área de trabalho (`DISPLAYNAME`) | Sofá.lnk | SofaLauncher.lnk (o `Sofá.lnk` antigo é removido pela instalação nova, só se apontar para o `sofa.exe` da pasta fixa) |
| "Aplicativos instalados", coluna Nome (`DisplayName`) | Sofá | SofaLauncher (a chave `Uninstall\Sofa` não muda) |
| Título da janela, `<title>` do `index.html`, propriedades do `sofa.exe` (`FileDescription`, Gerenciador de Tarefas) | Sofá | SofaLauncher |
| Nome do instalador (arquivo) | Sofa_<versão>_x64-setup.exe | SofaLauncher_<versão>_x64-setup.exe |
| Telas padrão do NSIS (boas-vindas, pasta, concluído, confirmação do desinstalador) | texto de fábrica do NSIS em pt-BR | decisão aberta, ver `revisao-instalador.md` seção 5 |
