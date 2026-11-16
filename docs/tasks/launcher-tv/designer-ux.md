# Tarefas — designer-ux — launcher-tv

A marca está pronta em `docs/brand/sofa/` (tokens, fontes, ícones, brand book, guia). Os mockups são HTML em `docs/design/launcher-tv/`, importam `docs/brand/sofa/tokens.css` sem copiar e seguem a interface de 10 pés: área segura de 5%, texto ≥ 24 px a 1080p, um item em foco, "Cancelar" em foco nos diálogos destrutivos.

Os mockups ficam em `docs/design/launcher-tv/mockups/` (os nomes `home.html`, `sessao.html` etc. da tabela valem dentro dessa pasta; entrada: `index.html`). Handoff: `docs/design/launcher-tv.md`.

Usar os termos da spec (seção 4): "Ocultar" e "Itens ocultos". A arquitetura cita "Remover" e "Jogos ocultos" na seção 3.1; vale a spec.

O front só começa as telas depois da aprovação do usuário (DES-08).

| ID | Tarefa | Depende de | Refs | Pronto quando | Status |
|----|--------|------------|------|---------------|--------|
| DES-01 | Fundamentos de 10 pés e componentes base: área segura, escala, `Focusable` (foco, desabilitado), `MenuList`, `ConfirmDialog` ("Cancelar" em foco), faixa de aviso, `VolumeHud`, `OriginBadge`, `InProgressBadge`, capa e placeholder, truncamento de nome | — | CA-03, CA-24, CA-54, CA-55, CA-60, RN-14, E-13; reusa: docs/brand/sofa/tokens.css, docs/brand/sofa/assets/ | `docs/design/launcher-tv/componentes.html` usa os tokens sem cópia; texto ≥ 24 px a 1080p; contraste ≥ 4,5:1; variante com `prefers-reduced-motion` | feito |
| DES-02 | Mockup Home e Todos os jogos: herói, recentes, grade, foco, estado vazio com "Adicionar jogo" em foco, faixa "Nenhum controle conectado", aviso de desconexão, "Em andamento", indicador de volume | DES-01 | CA-06, CA-13, CA-14, CA-15, CA-22, CA-23, CA-24, CA-36, CA-60, CA-64, RN-05, E-13 | `docs/design/launcher-tv/home.html` em 1080p e 4K, com biblioteca cheia, vazia e com nome longo | feito |
| DES-03 | Mockup opções do jogo: Renomear (campo de teclado físico, Enter e Esc), Ocultar com confirmação, Restaurar nome original | DES-01 | CA-21, CA-61, CA-62, RN-04, RN-14, RN-22 | `docs/design/launcher-tv/opcoes-jogo.html` com todos os estados, inclusive nome vazio recusado | feito |
| DES-04 | Mockup fluxo de jogo: "Abrindo <nome>…", arquivo não encontrado, pasta de rede, não confirmado, pede administrador, UAC recusado, troca de jogo, painel do jogo em andamento, fechando e forçar fechamento | DES-01 | CA-26, CA-28, CA-29, CA-31, CA-32, CA-34, CA-35, CA-36, CA-63, CA-71, CA-73, E-10 | `docs/design/launcher-tv/sessao.html` com cada estado e a opção em foco por padrão marcada | feito |
| DES-05 | Mockup menu rápido: volume (←/→), Mudo, sem dispositivo, Desligar/Reiniciar/Suspender (suspensão indisponível), confirmações com aviso de jogo aberto, Configurações, Sair do launcher | DES-01 | CA-05, CA-37, CA-38, CA-39, CA-42, CA-43, CA-47, CA-48, RN-14 | `docs/design/launcher-tv/menu-rapido.html` com cada diálogo e "Cancelar" em foco | feito |
| DES-06 | Mockup Configurações (Tela, Biblioteca, Geral, Itens ocultos) e navegador de pastas: lista de monitores, Atualizar biblioteca, Adicionar jogo, "Iniciar com o Windows", aviso de execução elevada, lista de ocultos e "Nenhum item oculto", erros de duplicata e exe inválido | DES-01 | CA-04, CA-19, CA-20, CA-66, CA-70, CA-71, CA-72, SEC-20 | `docs/design/launcher-tv/configuracoes.html` e `navegador-pastas.html` com estados vazio, erro e lista longa | feito |
| DES-07 | Catálogo de microcopy pt-BR na voz da marca: todos os textos de tela e um texto por código de `ErrorDto` do contrato | DES-02, DES-03, DES-04, DES-05, DES-06, BE-07 | CA-13, CA-20, CA-28, CA-29, CA-31, CA-35, CA-42, CA-47, CA-48, CA-50, CA-71, CA-72, CA-73, RN-19 | `docs/design/launcher-tv/microcopy.md` cobre 100% dos códigos do contrato e usa as frases literais da spec | feito: 26 de 26 códigos do contrato rev. 2 |
| DES-08 | Protótipo navegável por teclado ligando os mockups e rodada de aprovação com o usuário | DES-07 | CA-09, CA-10, CA-55 | `docs/design/launcher-tv/index.html` navegável com setas, Enter e Esc; aprovação do usuário registrada no handoff; ajustes pedidos aplicados | feito: usuário aprovou os mockups |
| DES-09 | Revisão de design da UI implementada contra os mockups aprovados | FE-05, FE-07, FE-08, FE-09, FE-10, FE-11, FE-12, FE-13 | CA-54, CA-55, CA-60, CA-77, CA-78, CA-79, RN-31, RN-32, RN-33, E-13 | Lista de divergências entregue ao dev-frontend e resolvida, ou registro de "sem divergências" | feito (revisão em docs/design/launcher-tv/revisao-ui.md: rodada 2 fechou DES-BUG-01 a 07, sem bug aberto) |

## Versão 0.2.0 (pós-0.1.0)

DES-10 (revisão do texto do instalador, `docs/design/launcher-tv/revisao-instalador.md`) aparece no painel, mas não tem linha neste arquivo. A marca nova (guia v1.1, logo do controle) já está em `docs/brand/sofa/`. O front da 0.2.0 só começa depois da aprovação do usuário (DES-12).

| ID | Tarefa | Depende de | Refs | Pronto quando | Status |
|----|--------|------------|------|---------------|--------|
| DES-11 | Atualizar `docs/design/launcher-tv.md`, a microcopy e os mockups para a 0.2.0: nome SofaLauncher e logo do controle (abertura, cabeçalho), "Fechar o launcher" no fim do grupo de energia do menu rápido, sem "Sair do launcher", `ExitDialog` com os dois textos e o aviso de como reabrir, textos do instalador com o nome novo e o `shortDescription` do `tauri.conf.json` | — | D-49, D-50, D-52, D-53, D-55, CA-05, CA-82, RN-34, RN-35, R-10; reusa: docs/brand/sofa/assets | `menu-rapido.html` e `boot.html` com os estados novos; microcopy com o texto final de cada string visível (UI e instalador); a variante sem "Iniciar com o Windows" registrada como D-58 | feito |
| DES-12 | Aprovação do usuário dos mockups e da microcopy da 0.2.0 | DES-11 | D-55, D-58, CA-82 | Aprovação registrada no handoff; D-58 decidida; ajustes pedidos aplicados | feito |
| DES-13 | Revisão de design da UI implementada da 0.2.0 contra os mockups aprovados (marca, menu rápido, `ExitDialog`) | FE-20, FE-21, FE-22, FE-23 | CA-05, CA-82, RN-34, D-49, D-52 | Lista de divergências entregue ao dev-frontend e resolvida, ou registro de "sem divergências" em `docs/design/launcher-tv/revisao-ui.md` | a fazer |
