# SofaLauncher: guia de marca (v1.1)

Nome exibido: **SofaLauncher** (uma palavra, S e L maiúsculos, sem acento). Sigla: **SL**. Identificadores técnicos, arquivos e pastas seguem `sofa`.
Decisão D-49: o produto deixa de se chamar Sofá e o símbolo passa a ser um controle de videogame. Paleta, tipografia e voz não mudaram. A versão anterior (v1.0, Sofá) está em `_v0.1.0/`.
Tagline: "Do sofá, direto pro jogo."
Escolha do usuário: direção A (Palco), em `direcoes.html`.

Arquivos: `brand-book.html` (e `.pdf`), `tokens.css`, `tokens.json`, `assets/`.

## Estratégia

- **Propósito:** tirar o teclado e o mouse do caminho entre ligar o PC e jogar.
- **Posicionamento:** para quem joga no PC ligado à TV, com controle Xbox, e quer ligar e jogar sem levantar, o SofaLauncher é um launcher de tela cheia que reúne jogos da Steam, da Epic, do GOG e executáveis soltos, com volume e energia do PC no mesmo lugar.
- **Diferença:** abre direto nos jogos do próprio dono, com um único item em foco o tempo todo. Sem conta, loja nem rede.
- **Arquétipo:** o Cotidiano (confortável, sem cerimônia).
- **História:** o PC e a TV já estavam na sala; faltava um caminho curto do sofá até o jogo. O jogador é o herói, o SofaLauncher só abre a porta.
- **Público:** uma pessoa, o dono do PC. Sem peças de marketing.

## Conceito

Valores: **Clareza** (um item em foco, legível a 3 m), **Calma** (fundo escuro, pouca informação), **Pessoal** (os jogos do dono em primeiro plano), **Controle** (tudo com o controle na mão).
Personalidade: mais descontraído que sério, mais acessível que técnico, mais contido que exuberante, direto.

## Logo

- Controle de videogame genérico, âmbar, sob um cone de luz, em selo violeta escuro. Desenho próprio (corpo com duas empunhaduras, cruzeta e dois botões), sem copiar a silhueta de controle de fabricante nem marca de terceiros.
- **SL:** não entra no símbolo. Com as letras dentro do corpo do controle, o ícone deixa de ler em 16 px (cada letra teria cerca de 4 px). O controle sozinho lê bem em 16 px. Nos tamanhos maiores o nome aparece por extenso (wordmark e lockup). "SL" é sigla de texto: Sora 800, para título de janela estreita, tooltip e identificadores curtos.
- Versões: lockup horizontal (escuro, claro/ink e mono branco), símbolo, wordmark, ícone do app, ícone pequeno (16 a 32 px, sem o cone, controle maior).
- Área de proteção: meia largura do selo em todos os lados.
- Mínimos: ícone 16 px, lockup 160 px de largura, wordmark 96 px.
- Proibido: distorcer, girar, trocar as cores, separar o nome ("Sofa Launcher"), mudar a caixa, colocar letras dentro do controle.
- Wordmark "SofaLauncher" em Sora 700, convertido em curvas nos SVG (não depende da fonte instalada).

## Cores

| Papel | Token | HEX | Uso |
|---|---|---|---|
| Tinta | `--brand-ink` | #14102B | Fundo, ~60% |
| Superfície | `--brand-surface` | #221B47 | Barras e cards, ~25% |
| Superfície elevada | `--brand-surface-raised` | #2C2459 | Menus e diálogos |
| Holofote | `--brand-primary` | #FFB547 | Foco e ação, ~8% |
| Violeta de apoio | `--brand-secondary` | #8F7DFF | Detalhes, ~3% |
| Texto | `--brand-text` | #F5F1FF | Corpo |
| Texto secundário | `--brand-text-muted` | #BDB4E0 | Metadados e dicas |
| Alerta | `--brand-danger` | #FF7A85 | Ações destrutivas, erro |
| Confirmação | `--brand-success` | #5FE3A1 | Sucesso |

Todos os pares de uso passam em WCAG AA (4,5:1 para texto, 3:1 para anel de foco e ícones). A tabela com as razões está em `tokens.json` (`contrast`) e no brand book. Não usar o violeta de apoio como texto sobre superfície nem o âmbar como texto corrido.

## Tipografia

- Display: **Sora** 600, 700, 800 (OFL 1.1). Texto: **Figtree** 400, 600, 700 (OFL 1.1). Fallback: Segoe UI, system-ui.
- Arquivos woff2 em `assets/fonts/` (latim, cobre Á, ã, ç), embutidos no app. Fonte dos arquivos: Fontsource, a partir do Google Fonts. Licenças em `OFL-Sora.txt` e `OFL-Figtree.txt`.
- Escala a 1080p (px): legenda 24 (mínimo), corpo 28, rótulo 32, título de card 40, título de tela 48, destaque 72, herói 96. Em 4K, o dobro.
- No código: `calc(N * var(--u))`, com `--u: calc(100vh / 1080)`. Área segura: `--safe-x: 5vw`, `--safe-y: 5vh`.

## Voz e tom

Frases curtas, verbo no botão, sem tom de atendente. Diz o que houve e o que fazer.
Preferir: jogo, biblioteca, controle, abrir, fechar, voltar. Evitar: "ops", "oba", "experiência", "solução", "por favor", exclamação.
Exemplos: "Nenhum controle conectado. Ligue o controle ou conecte o cabo." / "Não foi possível salvar. Tente de novo."

## Elementos

- **Foco:** anel duplo (6 px de tinta + 14 px de âmbar) e halo de holofote (`--focus-ring`, `--glow-spotlight`). Card em foco com escala 1,06; os demais a 72% de opacidade. Sempre exatamente um item em foco.
- **Movimento:** 160 ms na troca de foco, 200 ms em diálogos, easing `cubic-bezier(.22,1,.36,1)`, sem bounce. Com redução de movimento: sem escala nem brilho animado, só o anel.
- **Capa sem imagem:** gradiente escuro com o nome do jogo em Sora 700.
- **Fotografia e ilustração:** não se aplicam; a arte são as capas dos jogos.
- **Ações destrutivas:** diálogo com "Cancelar" em foco por padrão.

## Aplicações

Tela inicial, diálogo de confirmação, tela de abertura (`assets/splash.svg`, 1920x1080), ícone na barra de tarefas, atalho e janela. Não há materiais de marketing.

## Uso no app (Tauri 2, React e Vite)

- Importar `tokens.css`; ajustar o caminho do `url()` das fontes para onde os woff2 ficarem no bundle.
- CSP local: `font-src 'self'`. Sem Google Fonts em produção.
- Ícones do instalador: `assets/icon.ico` já traz 16, 24, 32, 48, 64, 128 e 256 px. Para gerar o conjunto completo do Tauri: `npx tauri icon assets/png/icon-512.png`.

## Ícones do app que precisam ser regenerados (pelos devs)

Origem: `assets/png/icon-512.png` (versão completa). Tudo em `src-tauri/icons` deve vir daí:
- `npx tauri icon docs/brand/sofa/assets/png/icon-512.png` gera `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png`, `icon.icns`, `icon.ico` e os `Square*Logo.png`/`StoreLogo.png`.
- Depois, sobrescrever `src-tauri/icons/icon.ico` com `assets/icon.ico` (traz o desenho pequeno, sem cone, em 16, 24 e 32 px; o gerado pelo Tauri usa a versão completa em todos os tamanhos).
- Favicon e janela da UI: `assets/icon.svg` (ou `icon-small.svg` abaixo de 32 px). Splash: `assets/splash.svg` e `assets/png/splash-1920x1080.png`. Nome na UI e no `tauri.conf.json` (`productName`, título da janela): SofaLauncher.
