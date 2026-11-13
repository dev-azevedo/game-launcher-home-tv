# Fixtures de formatos de loja (SP-08)

Montadas a partir dos formatos documentados e conhecidos de Steam, Epic e GOG. IDs, hashes e caminhos sao ficticios.
A maquina de dev nao tinha nenhuma loja instalada: nada aqui foi observado em loja real. Ver
`docs/arch/spikes/SP-08.md` (coluna "Observado") e QA-17 para o que ainda precisa de confirmacao.

- `steam/` e `steam/malicious/`: `libraryfolders.vdf`, `appmanifest_*.acf`.
- `epic/Manifests/` e `epic/malicious/Manifests/`: `*.item`.
- `gog/valid/`, `gog/malicious/`: `goggame-*.info`; `gog/registry/games.json` simula
  `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\<id>` (valor -> dado) para um leitor de registro fake.
- Nao versionados, gerados no teste: VDF > 4 MB (10 MB) e VDF com 1.000 niveis (SEC-13); PNG com cabecalho 100000x100000 (SEC-08).
