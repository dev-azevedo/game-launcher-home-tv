# Sofá: guia de marca (v1.0)

Nome exibido: **Sofá** (com acento). Identificadores técnicos, arquivos e pastas: `sofa`.
Tagline: "Do sofá, direto pro jogo."
Escolha do usuário: direção A (Palco), em `direcoes.html`.

Arquivos: `brand-book.html` (e `.pdf`), `tokens.css`, `tokens.json`, `assets/`.

## Estratégia

- **Propósito:** tirar o teclado e o mouse do caminho entre ligar o PC e jogar.
- **Posicionamento:** para quem joga no PC ligado à TV, com controle Xbox, e quer ligar e jogar sem levantar, o Sofá é um launcher de tela cheia que reúne jogos da Steam, da Epic, do GOG e executáveis soltos, com volume e energia do PC no mesmo lugar.
- **Diferença:** abre direto nos jogos do próprio dono, com um único item em foco o tempo todo. Sem conta, loja nem rede.
- **Arquétipo:** o Cotidiano (confortável, sem cerimônia).
- **História:** o PC e a TV já estavam na sala; faltava um caminho curto do sofá até o jogo. O jogador é o herói, o Sofá só abre a porta.
- **Público:** uma pessoa, o dono do PC. Sem peças de marketing.

## Conceito

Valores: **Clareza** (um item em foco, legível a 3 m), **Calma** (fundo escuro, pouca informação), **Pessoal** (os jogos do dono em primeiro plano), **Controle** (tudo com o controle na mão).
Personalidade: mais descontraído que sério, mais acessível que técnico, mais contido que exuberante, direto.

## Logo

- Cartaz vertical âmbar sob um cone de luz, em selo violeta escuro. O triângulo de play diz o que o botão A faz.
- Versões: lockup horizontal (escuro e claro), símbolo, wordmark, ícone do app, ícone pequeno (16 a 32 px, sem o cone).
- Área de proteção: meia largura do selo em todos os lados.
- Mínimos: ícone 16 px, lockup 120 px de largura, wordmark 64 px.
- Proibido: distorcer, girar, trocar as cores, escrever "Sofa" sem acento.
- Wordmark em Sora 700, convertido em curvas nos SVG (não depende da fonte instalada).

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
