# Revisão de design da UI (DES-09)

| Campo | Valor |
|---|---|
| Feature | launcher-tv (Sofá) |
| Branch | `feature/launcher-tv-20261113` |
| Referências | `docs/design/launcher-tv.md` (seções 13 e 16), `microcopy.md`, `mockups/tv.html` e `mockups/prototipo.html`, `docs/brand/sofa/` |
| Veredito | **Aprovado com ajustes.** Nenhum bug alto. Dois médios e cinco baixos. |

Pasta dos screenshots (1920×1080 salvo indicação): `C:\Users\JHONAT~1.AZE\AppData\Local\Temp\claude\17d258b0d4ac8099\f8d80d67-1d19-4574-8bf0-c4b10e3a96b2\scratchpad\des09\`. Nos nomes, `u-` é a UI (Vite, porta 5181) e `m-` é o mockup (`tv.html?scenario=...`).

## 1. Como foi feito

- UI e mockup abertos no Edge headless, dirigidos por DevTools Protocol. Isso permitiu enviar setas, F1 e intenções `input://intent` (View, LB/RB) e chegar a opções do jogo, renomear, ocultar, abrindo, administrador, UAC, troca de jogo, não confirmado, menu rápido, configurações e navegador de pastas.
- Cenários em 1080p: biblioteca cheia, 500 jogos, vazia, carregando (via Atualizar biblioteca), nomes longos e extremos, sem controle, jogo em andamento. Início em 4K e com `prefers-reduced-motion`.
- Fontes, escala e movimento reduzido conferidos por `getComputedStyle` e `document.fonts`. Contraste calculado a partir dos pares de tokens.
- Não foi possível verificar: capas reais (o `cover://` não carrega no navegador de dev, então não vi o fundo desfocado da capa), controle Xbox real, leitura em TV. O devCore não cobre "pasta sem permissão", "pasta de rede" e "forçar sem permissão" por intenção. Para esses estados usei o código e `errors.ts`.
- Armadilha do método: `--screenshot` com `--virtual-time-budget` tirou foto da biblioteca vazia sem anel de foco (`ui-vazia.png`). Com CDP o foco aparece em "Adicionar jogo" e há um único `data-focused="true"` em todas as telas testadas. Foi artefato do headless e não vira bug.

## 2. O que conferi e está de acordo

| Item | Resultado |
|---|---|
| Fontes | Sora 700/800 e Figtree 400/600 carregam do bundle. O corpo usa `Figtree, "Segoe UI"`. Nenhuma fonte externa. |
| Escala 10 pés | `--u` calculado por `100vh/1080`. O 4K é o dobro do 1080p, proporções idênticas (`u-home-4k.png`, `m-4k.png`). Sem rolagem horizontal (`scrollWidth` 1920). |
| Tokens | Cores, raios, sombras e foco vêm de `tokens.css`. O único hex solto é o gradiente do placeholder (`#322a68` a `#1b1540`), previsto no handoff seção 6. Os `rgba` são os tons do próprio token (âmbar, danger, success, scrim). |
| Foco | Anel duplo `--focus-ring`, halo âmbar e escala 1,06 (`matrix(1.06…)`). Blocos âmbar em menus e botões. Lista e grade usam só o anel. Um item em foco por tela. |
| Reduced motion | `transform: none`, sem halo, só o anel. `transition: none`, diálogos e banner sem animação, spinner a 2,4 s, barra de abertura parada. |
| Início, herói e recentes | Layout igual ao mockup: herói fixo, Recentes e Biblioteca, nome em Sora 800, selo de origem, dicas de botão no rodapé (`u-home-right2.png` x `m-home-right2.png`). "Em andamento" no card e no herói idêntico (`u-hero-andamento.png` x `m-andamento.png`). |
| Card cortado (RN-32) | A fileira Biblioteca sangra para fora da borda direita e texto e foco ficam dentro da área segura. "Todos os jogos 500" em foco termina em x≈1817, abaixo do limite de 1824 (`u-muitos-row.png`). Em 4K vale o mesmo. |
| Grade | 7 colunas, 220 de largura, mesmo espaçamento do mockup (`u-all-muitos.png` x `m-all.png`). |
| Estados vazio, carregando, sem controle | Textos iguais ao microcopy. "Adicionar jogo" em foco (CA-23). Faixa "Nenhum controle conectado…" e chip "Sem controle" iguais (`u-noctrl.png` x `m-noctrl.png`). Chip "Atualizando biblioteca…" no cabeçalho. |
| Menu rápido, configurações, navegador de pastas | Praticamente idênticos ao mockup (`u-quick-sess.png`, `u-settings*.png`, `u-folder.png`). Inclui os ajustes pós-implementação do handoff 16 ("Inicialização", "Pasta de logs", "Desligada"). |
| Painel do jogo | Igual ao mockup em estrutura, tipografia e dicas (`ui-panel.png` x `m-session.png`). |
| Aviso antes do UAC | Existe a etapa "Continuar | Voltar" e depois a espera "pede permissão de administrador" com "Voltar" em foco. Texto completo igual ao microcopy. |
| Telas de espera com botão | Abrindo, UAC e Fechando têm um botão em foco. A trava de A por 600 ms está em `WaitScreen`. |
| Contraste (pares de token) | texto sobre `raised` 12,6:1; muted sobre ink 9,5:1; muted sobre surface 8,2:1; muted sobre raised 7,2:1; âmbar sobre ink 10,5:1; tinta sobre âmbar 10,5:1; tinta sobre danger 7,4:1. Tudo acima de 4,5:1 (texto) e 3:1 (anel e ícones). O violeta `--brand-secondary` não é usado como texto. |
| Dados externos | `<img src=x onerror=alert(1)>` aparece literal no card, no rótulo e no herói. |

## 3. Divergências (DES-BUG)

### Média

**DES-BUG-01. Diálogos de erro, aviso e espera invertem a hierarquia**
- Tela: `GameFileMissing`, `NetworkPathBlocked`, administrador (1ª etapa e espera), não confirmado, troca de jogo, Abrindo e Fechando.
- Esperado (mockup): título curto em Sora (nome do jogo, ou "Abrindo <nome>…"), texto da mensagem em corpo abaixo, ícone temático (escudo no administrador) ou capa de 112×168 à esquerda. O microcopy diz para `GameFileMissing`: título `<nome>`, texto "Não encontramos o arquivo do jogo."
- Obtido: a frase inteira vira o título (Sora 800, 48 px, até 4 linhas). O nome do jogo cai numa linha de corpo ou some. Ícone de triângulo em todos. Abrindo, administrador-espera e Fechando não têm ícone nem capa, e faltam os rótulos "Iniciando" e "Esperando o Windows" ao lado do spinner. O texto é o mesmo, mas a leitura a 3 m muda: o título gigante ocupa o diálogo e a pessoa perde a referência de qual jogo é.
- Arquivo provável: `features/session/overlay.ts` (`overlayDialog` monta `title` com a mensagem), `LaunchFlow.tsx`, `shared/ui/ConfirmDialog.tsx` (sem prop de capa), `shared/ui/WaitScreen.tsx`.
- Screenshots: `u-notfound.png` x `m-notfound.png`; `u-admin.png` x `m-admin.png`; `u-admin-wait.png` x `m-admin-wait.png`; `u-launching.png` x `m-launching.png`; `u-unconfirmed.png`; `u-swap.png`.
- Correção sugerida: título curto e mensagem no corpo. `ConfirmDialog` e `WaitScreen` aceitam `cover` (ou ícone por código). A troca de jogo pode manter a frase da spec como título.

**DES-BUG-02. Botão secundário âmbar sem foco nos diálogos de erro**
- Tela: `GameFileMissing` (Voltar | Ocultar) e `NetworkPathBlocked`. O mesmo vale para qualquer confirmação não destrutiva, como "Continuar" com Voltar em foco.
- Esperado: só o item em foco é bloco âmbar (handoff seção 1). No mockup "Ocultar" é neutro (superfície escura, com ícone) e "Voltar" é o âmbar focado.
- Obtido: "Voltar" focado e "Ocultar" também âmbar cheio. Dois blocos iguais lado a lado e o foco só se distingue pelo anel (`.btn.primary` fica âmbar sem foco). A 3 m isso enfraquece o "um item em foco" (CA-54, CA-55).
- Arquivo provável: `shared/ui/ConfirmDialog.tsx` (`destructive ? 'danger' : 'primary'`) e `shared/ui/ui.css` (`.btn.primary`). A confirmação do `LaunchFlow` precisa de uma variante neutra para ações como Ocultar.
- Screenshots: `u-notfound.png`, `u-network.png` x `m-network.png`.

### Baixa

**DES-BUG-03. Opções do jogo sem cabeçalho de jogo e sem dicas**
- Esperado: capa em miniatura, nome, "Steam · Jogado hoje" e dicas "A Escolher · B Voltar" no rodapé do menu.
- Obtido: só o nome como título e nenhuma dica de botão. As dicas existem no menu rápido, no painel e no navegador, então ficou inconsistente.
- Arquivo provável: `features/library/GameOptions.tsx`.
- Screenshots: `u-options.png` x `m-options.png`.

**DES-BUG-04. Ordem dos botões difere do microcopy e do mockup**
- Esperado (microcopy): "Salvar · Cancelar", "Continuar · Voltar", "Restaurar · Cancelar". O mockup coloca a ação principal à esquerda, mas o próprio mockup é inconsistente: em `NetworkPathBlocked` ele põe "Ocultar" à esquerda e "Voltar" à direita.
- Obtido: Cancelar ou Voltar sempre à esquerda, ação à direita (`u-admin.png`, `u-unconfirmed.png`, `u-rename-empty.png` x `m-rename.png`). Não bloqueia nada. Peço decisão de uma regra única (cancelar sempre à esquerda é coerente com RN-14 e com os diálogos de energia). Se mantiver, o microcopy é que muda.
- Arquivo provável: `shared/ui/ConfirmDialog.tsx`, `features/library/RenameDialog.tsx`.

**DES-BUG-05. Aviso de ocultar funde dois parágrafos**
- Esperado (microcopy): dois parágrafos, "O jogo sai da biblioteca e não volta nas atualizações. Nenhum arquivo é apagado." e "Para restaurar, abra Configurações e escolha Itens ocultos."
- Obtido: um parágrafo só (`u-hide.png` x `m-hide.png`).
- Arquivo provável: `features/library/HideConfirmDialog.tsx`.

**DES-BUG-06. Capa placeholder pequena corta o nome**
- Esperado: capa miniatura legível.
- Obtido: no painel do jogo a miniatura de 112 px mostra só "Vale" (de "Vale dos Ventos"). O nome já está ao lado.
- Arquivo provável: `shared/ui/Cover.tsx` e `.cover-ph-name` em `ui.css`. Sugestão: esconder o nome do placeholder em tamanho miniatura.
- Screenshot: `ui-panel.png` x `m-session.png`.

**DES-BUG-07. Renomear sem ícones**
- Esperado: ícone de lápis no cabeçalho do diálogo e check no botão "Salvar".
- Obtido: sem os dois (`u-rename-empty.png` x `m-rename.png`).
- Arquivo provável: `features/library/RenameDialog.tsx`.

## 4. Variações aceitáveis (não são bug)

| Variação | Por que aceitar |
|---|---|
| Cabeçalho da UI sem volume fixo e sem relógio (o mockup mostra "40" e "21:47") | O handoff e a spec só pedem o indicador de volume por 2 s (CA-64). O relógio não está em nenhum documento. O mockup mostra mais do que o produto prevê. Se o produto quiser o relógio, é decisão nova de Produto. |
| "Em uso ✓" na mesma linha em Configurações > Tela | Mais limpo que a quebra em duas linhas do mockup. |
| Bloco "Inicialização" e "Pasta de logs" em Geral, rodapé de versão logo após o conteúdo | Aprovado no handoff 16. |
| Anel de foco em lista e grade passa de 28 px da área segura (`x≈68`) | O mockup tem o mesmo desenho e o item em si fica dentro de 96 px. Handoff seção 7 manda só anel em listas. |
| Painel do jogo aparece sobre a biblioteca, e não sobre uma imagem de jogo | No produto o jogo é outra janela. A imagem do mockup era simulação. |
| Herói com nome longo corta em 2 linhas com reticências | Handoff seção 5: "quebra em 2 linhas no herói". O texto completo está no `aria-label` do card. Observação: com 2 linhas a linha de origem sobe até ≈10 px do logo (`u-longo-r.png`). Não colide, mas é o limite. |
| Selo "Em andamento" e esqueleto continuam pulsando com movimento reduzido | É só opacidade e o handoff não pede para parar. Se quiser, parar o pulso é ajuste de polimento. |
| Texto do `GameFileMissing` sem "em <caminho>" | Já é pendência de spec (handoff 16, item 7). |
| Diferença de capa e placeholder entre UI e mockup | O dev não tem capas reais, a UI usa o placeholder previsto. |

## 5. Pendências para o front e para Produto

- Front: DES-BUG-01 e DES-BUG-02 antes do QA. O resto pode entrar junto com a próxima rodada.
- Produto: decidir a regra de ordem dos botões (DES-BUG-04).
- QA/real: conferir em TV real o fundo desfocado de capa, o halo (custo em GPU, handoff seção 7) e o controle Xbox.

## 6. Tarefas

DES-09: revisão feita. Lista entregue acima. Fechar quando o front resolver DES-BUG-01 e DES-BUG-02 ou registrar a recusa.
