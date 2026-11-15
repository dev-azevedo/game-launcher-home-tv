# Revisão do texto do instalador e do desinstalador (DES-10)

| Campo | Valor |
|---|---|
| Feature | launcher-tv (Sofá) |
| Arquivos lidos | `src-tauri/windows/PortugueseBR.nsh`, `installer.nsi`, `install-hooks.nsh`, `uninstall-hooks.nsh`, `hooks.nsh`, `src-tauri/tauri.conf.json` |
| Referências | `microcopy.md` (seções 1 a 3), `docs/brand/sofa/guia.md`, spec CA-01, CA-50, CA-53, CA-66 a CA-68, RN-18, RN-19, D-43 |
| Catálogo final | `microcopy.md`, seção 4 |
| Veredito | Textos bons na maior parte. 23 das 41 strings mudam, 2 por bug de comportamento visível e 1 problema de encoding que estraga todos os acentos. Nenhum arquivo `.nsh` ou `.nsi` foi alterado. |

## 1. Encoding: problema real (corrigir primeiro)

- `installer.nsi` declara `Unicode true` (linha 23). Está certo.
- `PortugueseBR.nsh` está em **UTF-8 sem BOM** (conferi os bytes: `Sofá` é `53 6f 66 c3 a1`, e o arquivo começa em `3b 20`, sem `ef bb bf`). O mesmo vale para `installer.nsi`, que tem `Sofá` na linha 93 (`DISPLAYNAME`).
- O NSIS 3 em modo Unicode lê arquivo sem BOM como ANSI (página de código da máquina que compila). Cada acento vira dois caracteres: "SofÃ¡", "jÃ¡ estÃ¡ instalado". Isso atinge o título da janela, o atalho (`Sofá.lnk`), o nome em "Aplicativos instalados" e todas as mensagens.
- Correção: salvar `PortugueseBR.nsh` e `installer.nsi` como **UTF-8 com BOM**. `install-hooks.nsh`, `uninstall-hooks.nsh` e `hooks.nsh` só têm ASCII e podem ficar como estão (o guard de CI não precisa mudar para eles).
- O `installer.nsi` final é gerado pelo bundler a partir do template. Não consegui confirmar se o bundler grava o BOM ao renderizar. Depois de aplicar, gerar o instalador e conferir na tela, no atalho e em "Aplicativos instalados" que aparece "Sofá".
- Não consegui rodar `makensis` aqui (não está instalado nesta máquina). Nada abaixo foi compilado. Os limites de tamanho são estimativa do tamanho das caixas do `nsDialogs`; conferir na tela.

Sugestão de guard: um teste que lê os 2 arquivos e falha se faltar `EF BB BF` no início.

## 2. Marca no texto que o arquivo não controla

| Onde | Hoje | Proposto | Quem aplica |
|---|---|---|---|
| `{{product_name}}` em `appRunning`, `appRunningOkKill`, `failedToKillApp` | O bundler não renderiza o arquivo de idioma (o original do Tauri usa `${PRODUCTNAME}`). A mensagem sairia com as chaves visíveis. E se rendesse, `productName` é "Sofa", sem acento (guia: proibido) | `${DISPLAYNAME}` ("Sofá", definido na linha 93, antes de o arquivo de idioma ser incluído) | dev, no `.nsh` (já na tabela abaixo) |
| `BrandingText "${COPYRIGHT}"`, rodapé de toda tela | "Copyright (c) Sofa" | "Copyright (c) Sofá" | dev, `copyright` em `tauri.conf.json` |
| `Publisher` em "Aplicativos instalados" | "Sofa" | "Sofá" | dev, `publisher` em `tauri.conf.json` |

Não trocar `productName` para "Sofá": ele entra em `UNINSTKEY` e em outras chaves de registro. Só `copyright` e `publisher`.

## 3. Strings alteradas (23 de 41)

Ordem do arquivo. "Motivo" usa as regras: frase curta, diz o que fazer, sem repetir o que a tela já mostra, termos do `microcopy.md`.

| string-id | Texto atual | Texto proposto | Motivo |
|---|---|---|---|
| `alreadyInstalledLong` | O Sofá ${VERSION} já está instalado. Escolha o que fazer e clique em Avançar. | A versão ${VERSION} já está instalada. Reinstalar mantém a biblioteca e as configurações. Desinstalar pergunta o que fazer com elas. | O título já diz "Sofá já instalado" e o subtítulo já diz "Escolha o que fazer". O corpo passa a dizer o que cada rádio faz com os dados (CA-53, CA-68). |
| `dontUninstall` | Manter a versão instalada e só atualizar os arquivos | Instalar por cima, sem remover a versão atual | "Manter a versão instalada" contradiz o caso de atualização (a versão muda). O rádio só pula o desinstalador. |
| `dontUninstallDowngrade` | Manter a versão instalada (instalar uma versão anterior por cima está desligado) | Instalar por cima (indisponível para versão anterior) | Cabe em uma linha de rádio. Só aparece com `allowDowngrades` falso (o padrão do Tauri é verdadeiro e `tauri.conf.json` não define). |
| `olderOrUnknownVersionInstalled` | Uma versão $R4 do Sofá está instalada. Atualizar mantém a biblioteca, as configurações e o Iniciar com o Windows. Escolha o que fazer e clique em Avançar. | Uma versão $R4 do Sofá está instalada. A biblioteca, as configurações e a opção Iniciar com o Windows continuam como estão. | "o Iniciar com o Windows" soa torto. Vale para os dois rádios (os dois mantêm os dados), não só para "Atualizar". Sem repetir a instrução do subtítulo. |
| `newerVersionInstalled` | Uma versão mais nova do Sofá já está instalada. Não recomendamos instalar uma versão anterior por cima. Escolha o que fazer e clique em Avançar. | Uma versão mais nova do Sofá já está instalada. Não recomendamos instalar uma versão anterior por cima. | Tira só a instrução repetida do subtítulo, igual às outras duas páginas. |
| `silentDowngrades` | Instalar uma versão anterior está desligado neste instalador. Use a instalação com telas.$\n | Este modo de instalação não permite instalar uma versão anterior. Use o instalador com telas.$\n | Aparece em `/S`, `/P` e `/UPDATE`, não só no silencioso. "Desligado" fica para opção de tela. Manter o `$\n`. |
| `unableToUninstall` | Não foi possível remover a versão instalada. Feche o Sofá e tente de novo. | Não foi possível remover a versão instalada. Feche o Sofá e tente de novo. Se continuar, desinstale em Aplicativos instalados. | Dá um próximo passo quando fechar o Sofá não resolve (a mesma caixa aparece quando a pasta anterior é outra, `installer.nsi` linhas 361 a 364). |
| `uninstallBeforeInstalling` | Atualizar a versão instalada | Substituir a versão instalada (mantém seus dados) | A string é usada também no caso de versão anterior (linha 262), onde "Atualizar" é falso. "Substituir" vale nos dois e diferencia do rádio 2. |
| `appRunning` | O {{product_name}} está aberto. Feche o Sofá e tente de novo. | O ${DISPLAYNAME} está aberto. Feche-o e tente de novo. | Placeholder que não é trocado (seção 2) e nome repetido na mesma frase. |
| `appRunningOkKill` | O {{product_name}} está aberto.$\nClique em OK para fechá-lo. | O ${DISPLAYNAME} está aberto.$\nClique em OK para fechá-lo. | Só o placeholder. |
| `failedToKillApp` | Não foi possível fechar o {{product_name}}. Feche o Sofá e tente de novo. | Não foi possível fechar o ${DISPLAYNAME}. Feche-o você mesmo e tente de novo. | Placeholder e nome repetido. |
| `installingWebview2` | Instalando o WebView2... | Instalando o WebView2… | Reticências como no resto do app ("Carregando biblioteca…"). Exige o BOM. |
| `webview2Downloading` | Baixando o WebView2... | Baixando o WebView2… | Idem. A string não é usada hoje (ver seção 4). |
| `sofaStartupHint` | Vale para cada pessoa que usar o Sofá neste PC. Dá para mudar depois, em Configurações. | Esta escolha vale como padrão para cada usuário do PC. Cada um pode mudar depois, em Configurações. | A caixa grava o padrão (`InstallDefaults`); cada usuário aplica na 1ª abertura e pode mudar. O texto antigo soa como regra fixa. "Usuário" é o termo de Configurações. |
| `sofaNoKeyboardNote` | Para usar o Sofá só com o controle, deixe o Windows entrar sozinho na área de trabalho: entrada automática, PIN ou Windows Hello. Isso é uma configuração do Windows, fora do Sofá, e o instalador não altera o seu login. | Para ligar o PC e jogar sem teclado, configure o logon automático ou o Windows Hello no Windows. O Sofá começa na área de trabalho. O instalador não altera o seu logon. | Mesma frase do aviso de logon em Configurações (`microcopy.md`, RN-19), para o usuário ler a mesma coisa nos dois lugares. "Entrada automática" não é o nome que o Windows usa. Ver dúvida 2 sobre o PIN. |
| `sofaPurgeStart` | Apagando a biblioteca e as configurações do seu usuário... | Apagando os dados do Sofá do seu usuário… | O purge apaga também capas, logs e cache (CA-68). Reticências com "…". |
| `sofaSidIgnored` | Um perfil com identificador fora do padrão foi ignorado. | Um perfil do Windows com identificador inválido foi ignorado ao remover a inicialização. | Diz de que perfil se trata e o que deixou de ser feito. "Fora do padrão" não diz nada. |
| `sofaPurgePartial` | Não foi possível apagar todos os dados do Sofá. A biblioteca e as configurações foram removidas. O que sobrou (capas, registros e cache) está na pasta abaixo, e você pode apagá-la. | Não foi possível apagar todos os dados do Sofá. Alguns arquivos podem ter ficado na pasta abaixo (biblioteca, configurações, capas, logs e cache). Você pode apagar a pasta. | A mensagem escolhe o texto antes de saber se o fallback apagou algo (`SofaPurgeDecide`, `$SofaDeleted` não é lido). D-43 garante a tentativa, não o resultado. Dizer "foram removidas" pode ser falso, e o usuário que pediu para apagar confia nisso. "Logs" é o termo de Configurações ("Pasta de logs"). |
| `sofaPurgeUnread` | Não foi possível confirmar a remoção dos dados do Sofá. A biblioteca e as configurações foram removidas. O que sobrou (capas, registros e cache) está na pasta abaixo, e você pode apagá-la. | Não foi possível confirmar a remoção dos dados do Sofá. Alguns arquivos podem ter ficado na pasta abaixo (biblioteca, configurações, capas, logs e cache). Você pode apagar a pasta. | Mesmo problema da anterior. |
| `sofaPurgeInvalidTarget` | Os dados do Sofá não foram apagados: a pasta não passou na conferência de segurança. Ela está abaixo, e você pode apagá-la se quiser. | Os dados do Sofá não foram apagados: a pasta não passou na verificação de segurança. Ela está abaixo. Confira o conteúdo antes de apagar. | A trava falha quando a pasta é junction ou symlink. Mandar apagar sem olhar é conselho arriscado nesse caso. |
| `sofaPurgeAppOpen` | O Sofá estava aberto, então os dados não foram apagados. Feche o Sofá e desinstale de novo, ou apague a pasta abaixo. | O Sofá estava aberto, então os dados não foram apagados. Eles continuam na pasta abaixo, e você pode apagá-la. | **Bug de comportamento.** O hook roda dentro de `Section Uninstall` e a desinstalação segue depois dele: `sofa.exe`, `uninstall.exe` e a chave de registro são removidos (`installer.nsi` linhas 753 a 827). "Desinstale de novo" é impossível. |
| `sofaPurgeRunning` | A remoção dos dados ainda está em andamento e nada mais foi apagado. A pasta está abaixo. | A remoção dos dados ainda não terminou. Se algo sobrar, apague a pasta abaixo. | "Nada mais foi apagado" afirma o que o código não sabe (o purge pode continuar rodando). Diz o que fazer. |
| `sofaPurgeNoProfile` | Não foi possível saber qual usuário está usando o Windows, então nenhum dado foi apagado. Apague você mesmo a pasta app.sofa.launcher dentro de AppData\Local do seu usuário. | Não foi possível saber qual usuário está usando o Windows, então nenhum dado foi apagado. Para removê-los, apague a pasta app.sofa.launcher em AppData\Local do seu usuário. | Ajuste de tom: deixa claro que é opcional. Caminho e identificador intactos. |

## 4. Strings mantidas (18) e observações

Mantidas como o dev escreveu: `addOrReinstall`, `alreadyInstalled`, `chooseMaintenanceOption`, `chooseHowToInstall`, `older`, `unknown`, `uninstallApp`, `createDesktop`, `webview2AbortError` (literal da spec CA-50, confere caractere a caractere), `webview2DownloadError`, `webview2DownloadSuccess`, `webview2InstallError`, `webview2InstallSuccess`, `sofaOptionsTitle`, `sofaOptionsSubtitle`, `sofaStartupCheckbox`, `keepAppData` (CA-68: "Manter dados" marcada), `sofaKeepData`.

Observações para o dev:
- `webview2Downloading`, `webview2DownloadSuccess` e `webview2DownloadError` não são usadas pelo `installer.nsi` (o bootstrapper embutido baixa sozinho). O comentário do arquivo diz "traz todas as chaves que o template usa". Se o `makensis` reclamar de `LangString` sem uso, apagar as três. Não afeta o texto.
- Quando `SofaRoot` está vazio (`sofaPurgeNoProfile`), `SofaPurgeDecide` ainda acrescenta `$\r$\n$\r$\n` + nada, e a caixa sai com duas linhas em branco no fim. Acrescentar o caminho e as quebras só quando `$SofaRoot` não for vazio.
- `${VERSION}` e `$R4` em `LangString` funcionam porque o define vem antes do `!include` do idioma e `$R4` é preenchido por `PageReinstall`. Não mudei nada disso.
- Os textos de `sofaPurge*` aparecem em `MessageBox` e a pasta vai colada embaixo. Com `/SD IDOK` a caixa some sozinha em modo silencioso; nesse modo o usuário não vê o aviso. É o comportamento do dev e está certo para `/S`.

## 5. Telas do NSIS que não passam pelo `PortugueseBR.nsh`

Boas-vindas, escolha de pasta, tela final e confirmação do desinstalador usam o texto de fábrica do NSIS em pt-BR (vem do `MUI_LANGUAGE "PortugueseBR"`). Não consegui ler esses textos aqui (sem `makensis`). O de boas-vindas de fábrica fala em "assistente" e em fechar outros aplicativos para evitar reinício, o que não se aplica ao Sofá. Proposta opcional, a decidir (não entra na contagem das 23):

| Define em `installer.nsi` | Texto proposto |
|---|---|
| `MUI_WELCOMEPAGE_TITLE` | Instalar o Sofá |
| `MUI_WELCOMEPAGE_TEXT` | Do sofá, direto pro jogo. Clique em Avançar para instalar o Sofá neste PC. |
| `MUI_FINISHPAGE_TITLE` | Sofá instalado |
| `MUI_FINISHPAGE_TEXT` | Clique em Concluir para sair do instalador. |

Os defines ficam antes de `!insertmacro MUI_PAGE_WELCOME` e `MUI_PAGE_FINISH`; precisam do BOM como o resto. Ver dúvida 3.

## 6. Dúvidas para Produto

1. **WebView2 (CA-50).** `webview2AbortError` diz "Não foi possível baixar" para qualquer falha do bootstrapper (`$1` diferente de 0), mesmo as que não são rede. A frase é literal da spec e ficou como está. O código aparece na lista de detalhes (`webview2InstallError`). Aceitar assim, ou a spec ganha uma segunda frase para falha que não seja de conexão?
2. **PIN no aviso de logon (RN-19).** A spec cita "logon automático, Windows Hello, PIN"; o aviso de Configurações no `microcopy.md` cita só logon automático e Windows Hello. Mantive os dois iguais ao microcopy porque o PIN pede digitar. Se o PIN deve constar, acrescento nos dois textos.
3. **Telas padrão do NSIS.** Trocar o texto de boas-vindas e final (seção 5) ou aceitar o de fábrica?
4. **Versão anterior.** Com `allowDowngrades` verdadeiro (padrão do Tauri, `tauri.conf.json` não define), instalar versão anterior é permitido nas telas e `silentDowngrades` nunca aparece. Foi intencional? O ADR-0006 pode dizer o contrário; não li o ADR.

## 7. Resumo para o dev

1. Salvar `PortugueseBR.nsh` e `installer.nsi` em UTF-8 com BOM.
2. Trocar `{{product_name}}` por `${DISPLAYNAME}` (3 linhas).
3. Aplicar a tabela da seção 3 (23 strings), mantendo `$\n`, `$R4`, `$1`, `${VERSION}`.
4. `tauri.conf.json`: `publisher` e `copyright` com "Sofá".
5. Corrigir as linhas em branco de `sofaPurgeNoProfile` (seção 4).
6. Gerar o instalador e conferir "Sofá" na janela, no atalho e em "Aplicativos instalados".

## 7. Rodada 2: textos da pasta fixa e conferência do DES-10 (commit 4120287)

### Conferência das 23 strings

Comparei, por script, cada texto proposto na seção 3 com o valor atual em `src-tauri/windows/PortugueseBR.nsh`. Resultado: **22 de 22 iguais, caractere a caractere**, incluindo `${DISPLAYNAME}`, `…` e o `$\n` de `silentDowngrades`. A 23ª, `webview2Downloading`, foi removida junto com as outras `webview2Download*`, como combinado (não são usadas). `copyright` e `publisher` com acento e `allowDowngrades false` entraram no mesmo commit. Nada a corrigir.

Sobre o encoding: a seção 1 desta revisão mandava salvar o `.nsh` com BOM. Estava errada para este bundler. A fonte fica sem BOM, o Tauri grava o próprio, e BOM duplo quebra o `makensis`. O guard `NSIS-BOM` e `test-installer.ps1` conferem fonte sem BOM e gerado com um. O `microcopy.md` seção 4 foi corrigido.

### Os 3 textos novos

| string-id | Texto no `.nsh` | Veredito |
|---|---|---|
| `sofaInstallDirFixed` | O Sofá só pode ser instalado em Program Files. Rode o instalador sem a opção /D=. | Aprovado. Diz o que houve e o que fazer. `/D=` é jargão, mas quem passa `/D=` é quem instala por linha de comando. |
| `sofaPreviousInstallElsewhere` | Há uma instalação do Sofá fora de Program Files ($SofaPrevDir). Desinstale-a em Aplicativos instalados e rode este instalador de novo. | Aprovado. Mostra a pasta, diz o caminho de saída e usa o termo "Aplicativos instalados" já usado em `unableToUninstall`. |
| `sofaUninstallDirFixed` | O desinstalador do Sofá só roda a partir de Program Files. | **Troca de uma frase.** Só informa a regra e deixa a pessoa sem saída. |

### Troca para o dev aplicar

`sofaUninstallDirFixed`:
- Atual: `O desinstalador do Sofá só roda a partir de Program Files.`
- Novo: `O desinstalador do Sofá só roda a partir de Program Files. Desinstale em Aplicativos instalados.`

O `microcopy.md` seção 4 já registra o texto novo, marcado como troca pendente. Quando o dev aplicar, o aviso "troca proposta" daquela linha sai.

### Veredito

**Aprovado.** Dois textos ficam como estão e um recebe uma frase a mais. Não editei o `.nsh`.
