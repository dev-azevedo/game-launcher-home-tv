# Microcopy do Sofá (pt-BR)

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
| Herói com jogo em andamento | Em andamento. A volta ao jogo. |

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
| Renomear, botões | Salvar · Cancelar |
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
| Painel, jogo minimizado | O Windows não deixou o Sofá passar na frente. O jogo foi minimizado. Voltar ao jogo restaura a janela. |
| Painel, jogo como administrador | Este jogo roda como administrador. A combinação de retorno pode falhar nele. |
| Fechar jogo, título | Fechar <nome>? |
| Fechar jogo, texto | O Sofá pede ao jogo para fechar. / O progresso não salvo pode ser perdido. |
| Fechar jogo, botões | Cancelar (em foco) · Fechar jogo |
| Fechando | Fechando <nome>… · Esperando o jogo fechar. Pode levar alguns segundos. · botão: Ir para a biblioteca |
| Forçar, título | <nome> não fechou |
| Forçar, texto | O jogo não respondeu ao pedido para fechar. / O progresso não salvo será perdido. (spec: "Forçar fechamento (o progresso não salvo será perdido)") |
| Forçar, botões | Cancelar (em foco) · Forçar fechamento |
| Jogo fechou | <nome> fechado. |

### Menu rápido
| Elemento | Texto |
|---|---|
| Itens | Volume · Mudo · Desligar · Reiniciar · Suspender · Configurações · Sair do launcher |
| Dica de gatilhos | Na biblioteca, LT diminui e RT aumenta o volume. |
| Sem áudio (spec) | Nenhum dispositivo de áudio encontrado |
| Sem suspensão (spec) | Suspensão indisponível neste PC |
| Dicas | D-pad Ajustar volume · A Escolher · B Fechar |
| Desligar | Desligar o PC? / O Windows desliga do jeito normal. Se outro app tiver trabalho não salvo, o Windows avisa. / Cancelar (em foco) · Desligar |
| Reiniciar | Reiniciar o PC? / O Windows reinicia do jeito normal. (com Iniciar com o Windows ligado: " O Sofá abre de novo quando você entrar.") / Cancelar · Reiniciar |
| Suspender | Suspender o PC? / O PC entra em suspensão. Ao acordar, o Sofá volta para esta tela. / Cancelar · Suspender |
| Com jogo aberto (spec) | <nome> está aberto. O progresso não salvo pode ser perdido. |
| Em execução | Desligando… / Reiniciando… / Suspendendo… · "O Windows está encerrando." / "…reiniciando." / "O PC está entrando em suspensão." · botão Voltar |
| Sair, título e texto | Sair do Sofá? / O Sofá fecha e o Windows mostra a área de trabalho. |
| Sair com jogo aberto | <nome> continuará aberto. |
| Sair, botões | Cancelar (em foco) · Sair do Sofá |

### Configurações
| Elemento | Texto |
|---|---|
| Título e abas | Configurações · Tela · Biblioteca · Geral · Itens ocultos (com contagem) |
| Dicas | Abas: A Entrar · B Voltar. Painel: A Escolher · B Abas |
| Tela | Monitor do launcher / O Sofá abre em tela cheia no monitor escolhido. / <monitor> · <resolução> · Principal / Em uso |
| Um monitor | Só há um monitor conectado. |
| Monitor ausente | O monitor escolhido não está conectado. O Sofá está no monitor principal e volta ao monitor escolhido quando ele reconectar. |
| Monitor trocado | Sofá movido para <monitor>. |
| Biblioteca | Resumo: "N jogos: 6 Steam, 3 Epic…" · Atualizar biblioteca ("Procura jogos novos na Steam, na Epic e no GOG.") · Adicionar jogo ("Escolha um arquivo .exe em um disco deste PC.") |
| Iniciar com o Windows | Iniciar com o Windows / Abre o Sofá quando você entra no Windows. Vale só para o seu usuário. |
| Aviso de logon (RN-19) | Para ligar o PC e jogar sem teclado, configure o logon automático ou o Windows Hello no Windows. O Sofá começa na área de trabalho. |
| Aviso de elevação | O Sofá nunca roda como administrador. Jogos que exigem administrador pedem confirmação no Windows, com teclado ou mouse. |
| Rodapé | Sofá 1.0.0 · Fontes Sora e Figtree sob licença OFL |
| Itens ocultos | Escolha um item para voltar à biblioteca. |
| Itens ocultos vazio (spec) | Nenhum item oculto / Os jogos que você ocultar aparecem aqui. |
| Restaurar | Restaurar <nome>? / O jogo volta para a biblioteca com o nome e a data de último jogo que tinha. / Restaurar (em foco) · Cancelar |
| Restaurado | <nome> voltou para a biblioteca. |

### Navegador de pastas
| Elemento | Texto |
|---|---|
| Título e apoio | Adicionar jogo / Escolha o arquivo .exe do jogo. Só aparecem discos deste PC. |
| Raiz | Este computador |
| Linha de arquivo | Adicionar |
| Dicas | A Abrir ou escolher · B Pasta anterior (na raiz: Voltar) |
| Pasta vazia | Nenhuma pasta ou .exe aqui / Esta pasta não tem pastas nem arquivos .exe. / Voltar |
| Sem permissão | Sem permissão para abrir esta pasta / O Windows não deixa o Sofá ver o que há aqui. / Voltar |
| Pasta não existe | Não encontramos esta pasta / Ela pode ter sido removida, ou o disco foi desconectado. / Voltar |
| Confirmar | Adicionar este jogo? / O jogo entra na biblioteca como <nome>. Dá para renomear depois. / <caminho> / Adicionar (em foco) · Voltar |
| Adicionado | <nome> adicionado. |

## 2. Erros e avisos por código

A UI traduz o `code` do `ErrorDto`. Nenhuma mensagem mostra detalhe técnico, caminho do sistema do Sofá nem texto vindo do núcleo (SEC-06). O contrato final (`docs/api/`, BE-07) ainda não existe; a lista abaixo parte dos comandos da arquitetura (seção 4.1). Código que o contrato trouxer a mais cai em "Desconhecido" até entrar aqui.

| Código | Comando | Onde aparece | Texto | Botões (foco em negrito) |
|---|---|---|---|---|
| `Busy` | `library_refresh`, `session_launch` | Diálogo | Um momento. O Sofá ainda está terminando outra ação. Tente de novo em instantes. | **Voltar** |
| `Duplicate` | `library_add_manual` | Diálogo | Este jogo já está na biblioteca (spec). Apoio: Não criamos uma segunda cópia. | **Voltar** |
| `NotAnExecutable` | `library_add_manual` | Diálogo | Este arquivo não é um executável válido (spec). Apoio: O arquivo tem a extensão .exe, mas o Windows não o reconhece como programa. Escolha outro arquivo. | **Voltar** |
| `PathNotAllowed` (cadastro e navegador) | `library_add_manual`, `fs_list_dir` | Diálogo | Este local não é aceito. O Sofá só abre jogos de discos deste PC. | **Voltar** |
| `PathNotAllowed` (ao iniciar) | `session_launch` | Diálogo | Jogos em pastas de rede não são aceitos. Copie o jogo para um disco deste PC. (spec, CA-71) | Ocultar · **Voltar** |
| `NotFound` (pasta) | `fs_list_dir` | Estado da lista | Não encontramos esta pasta. Ela pode ter sido removida, ou o disco foi desconectado. | **Voltar** |
| `AccessDenied` (pasta) | `fs_list_dir` | Estado da lista | Sem permissão para abrir esta pasta. O Windows não deixa o Sofá ver o que há aqui. | **Voltar** |
| `EmptyName` | `game_rename` | Linha no diálogo | O nome não pode ficar vazio. | (campo em foco) |
| `UnknownGame` | `game_rename`, `game_hide`, `game_unhide`, `game_restore_name` | Diálogo | Este jogo não está mais na biblioteca. | **Voltar** |
| `SessionAlreadyActive` | `session_launch` | Diálogo | <atual> está aberto. Fechar e abrir <novo>? (spec) | **Cancelar** · Fechar e abrir |
| `GameFileMissing` | `session_launch` | Diálogo | Não encontramos o arquivo do jogo em <caminho> (spec) | Ocultar · **Voltar** |
| (estado) sem processo em 60 s | `SessionDto` | Diálogo | Não foi possível confirmar que <nome> abriu (spec) | **Tentar de novo** · Voltar |
| `StoreNotInstalled` | `session_launch` | Diálogo | Não encontramos a <loja> neste PC. Instale a loja para abrir este jogo. | **Voltar** |
| `NeedsElevation` | `session_launch` | Diálogo, 1ª etapa | <nome> precisa de permissão de administrador para abrir. O Windows vai pedir a sua confirmação. | **Continuar** · Voltar |
| (estado) esperando o UAC | `session_launch_with_uac` | Diálogo | <nome> pede permissão de administrador. Confirme no Windows com teclado ou mouse. A combinação de retorno pode não funcionar neste jogo. (spec) | **Voltar** |
| (estado) UAC recusado | `session_launch_with_uac` | Diálogo | O Windows não deu permissão para abrir <nome> (spec) | **Voltar** |
| `InvalidState` | vários | Diálogo | Isso não pode ser feito agora. Volte e tente de novo. | **Voltar** |
| `NoActiveSession` | `session_return_to_game`, `session_close` | Diálogo | Nenhum jogo está aberto. | **Voltar** (leva à biblioteca) |
| `ForegroundDenied` | `session_return_to_game` | Diálogo | O Windows não deixou voltar ao jogo. Tente de novo. | **Tentar de novo** · Ir para a biblioteca |
| `NotOverdue` | `session_force_close` | Diálogo | O jogo ainda está fechando. Espere alguns segundos. | **Voltar** |
| `AccessDenied` (forçar) | `session_force_close` | Diálogo | Não foi possível fechar <nome>. O jogo roda como administrador. Feche pelo menu do próprio jogo. | **Voltar** |
| `NoAudioDevice` | `volume_step`, `volume_toggle_mute` | Indicador e menu | Nenhum dispositivo de áudio encontrado (spec). No indicador: "Sem áudio". | (item desabilitado) |
| `Unavailable` | `power_prepare` | Item do menu | Suspensão indisponível neste PC (spec). Para Desligar e Reiniciar: Esta ação não está disponível neste PC. | (item desabilitado) |
| `TicketInvalid` | `power_execute` | Diálogo | A confirmação expirou. Escolha a ação de novo. | **Voltar** |
| `Rejected` | `power_execute` | Diálogo | O Windows não aceitou o pedido. Tente de novo. | **Voltar** |
| `AutostartFailed` | `settings_set_autostart` | Linha na configuração | Não foi possível alterar a inicialização com o Windows. Tente de novo. | (item em foco) |
| `UnknownMonitor` | `settings_set_monitor` | Toast | Esse monitor não está mais conectado. Escolha outro. | (lista em foco) |
| Desconhecido ou interno | qualquer | Diálogo | Algo deu errado no Sofá. Tente de novo. | **Voltar** |

Fora do app (instalador, CA-50): "Não foi possível baixar o WebView2. Conecte o PC à internet e tente de novo." (spec)

Regras de texto:
- Erro que o usuário não consegue resolver pelo controle diz o que fazer em outro lugar (Windows, teclado, mouse).
- Se o código trouxer parâmetros (`GameFileMissing{path}`, `SessionAlreadyActive{gameId}`), a UI resolve o nome do jogo pela biblioteca, nunca por texto do núcleo.
- Nome longo no texto de diálogo quebra linha (sem truncar). Nome de jogo que parece HTML aparece literal.
