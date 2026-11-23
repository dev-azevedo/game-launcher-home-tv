---
status: accepted
date: 2026-11-23
feature: launcher-tv
---

# Atualização pelo próprio app: tauri-plugin-updater só no Rust, assinatura minisign e GitHub Releases

A spec deixava "Atualização automática do launcher" fora do MVP e a segurança proibia cliente HTTP e o plugin `updater` (SEC-24, SEC-30). Para quem já usa o app, atualizar exigia baixar o instalador do GitHub à mão, com teclado e mouse. O pedido agora é: ao abrir o SofaLauncher com versão nova publicada, o próprio app avisa, baixa e atualiza.

## Decisão

- **Plugin oficial, usado só pelo processo Rust.** `tauri-plugin-updater` 2.14 com `default-features = false` e `native-tls` (schannel do Windows; sem `rustls` e sem `zip`). A UI chama dois comandos novos sem argumento, `update_check` (#31) e `update_install` (#32), e ouve `update://progress`. A capability `main` não ganha permissão `updater:` e a CSP continua sem origem externa: a webview nunca fala com a rede nem escolhe URL ou arquivo.
- **Origem fixa:** `https://github.com/dev-azevedo/game-launcher-home-tv/releases/latest/download/latest.json`, em `tauri.conf.json`. O `latest.json` é gerado pelo `ci-release.ps1` (versão, notas, data, URL do instalador e assinatura).
- **Autenticidade pela chave do projeto, não pelo GitHub.** O instalador só roda se a assinatura minisign bater com a chave pública da configuração. A chave privada fica fora do repositório (`%USERPROFILE%\.tauri\sofalauncher.key` ou `TAURI_SIGNING_PRIVATE_KEY`). Com isso, uma conta do GitHub comprometida não basta para distribuir instalador: falta a chave. O plugin também recusa versão menor ou igual à instalada.
- **Instalação passiva com UAC.** O instalador NSIS roda com `/P /UPDATE /R`: barra de progresso sem perguntas, o desinstalador anterior em `/UPDATE` (dados e autostart ficam, CA-53) e o launcher reaberto sem elevação (`RunAsUser`). Continua `perMachine` (RN-26), então o Windows pede UAC, que exige teclado ou mouse (R-04). O aviso diz isso antes de começar. UAC recusado → `UpdateCancelled`, e o launcher continua aberto.
- **Pergunta antes, nunca sozinho.** A UI procura versão nova 8 s depois de abrir. Se houver, e nenhum jogo estiver em sessão, mostra "Nova versão disponível" com "Atualizar" e "Agora não", uma vez por abertura. Se houver jogo aberto, espera a sessão voltar a `Idle`. Sem internet, a procura automática falha em silêncio. Em Configurações > Geral, "Procurar atualizações" faz a procura manual e mostra o resultado. `update_install` recusa com jogo em sessão (`InvalidState`).

## Alternativas

- **Cliente próprio (WinHTTP + `minisign-verify`)**, mantendo o `deny.toml` sem `reqwest`: rejeitada por ora. É mais código de rede e de instalação para manter e testar, sem ganho de segurança sobre o plugin (a verificação de assinatura é a mesma). Fica como plano B se o `reqwest` virar problema de advisory.
- **Permissão `updater:` na capability e chamada direto pelo JS:** rejeitada. Daria à webview acesso ao plugin inteiro e mudaria a regra "a UI só chama a lista fechada" (INV-02, SEC-04).
- **Instalação por usuário (`%LOCALAPPDATA%`, sem UAC):** rejeitada. Muda a RN-26, exige migrar quem já tem a 0.2.0 em Program Files e deixa o binário gravável pelo próprio usuário.
- **Só o SHA-256 do `SHA256SUMS.txt`:** rejeitada. Vem da mesma origem do instalador e não prova quem publicou.

## Consequências

- `deny.toml` passa a aceitar `reqwest` só via `tauri-plugin-updater` (`wrappers`) e `hyper` só via a pilha do `reqwest`. Qualquer outro crate que puxe cliente HTTP continua barrado.
- A spec deixa de listar a atualização no "fora do MVP" (RN-36, CA-83 a CA-86). SEC-24 e SEC-30 ganham a exceção, e o SEC-33 descreve os requisitos.
- O release precisa sair como **Latest** no GitHub (não pre-release, que `releases/latest` ignora), com o instalador, o `.sig` e o `latest.json`.
- Quem tem a 0.2.0 ou mais antiga instala a próxima versão à mão uma última vez. A partir dela, as versões chegam pelo app.
- Perder a chave privada impede publicar atualização para quem já está instalado. Sem a chave, só dá para trocar a pública numa versão instalada à mão.
