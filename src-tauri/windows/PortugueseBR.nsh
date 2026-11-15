; Texto das telas do instalador e do desinstalador do Sofá (pt-BR). Substitui o PortugueseBR.nsh do
; Tauri (`bundle.windows.nsis.customLanguageFiles`): traz as chaves que o template usa.
; Catálogo final: docs/design/launcher-tv/microcopy.md, seção 4. Regras de texto da marca: frase
; curta, sem jargão, o que fazer em seguida.
; O nome exibido vem de ${DISPLAYNAME} ("Sofá"), definido em installer.nsi antes deste arquivo; o
; bundler não renderiza este arquivo, então nada de chaves duplas do template aqui.
; ENCODING: UTF-8 SEM BOM nesta fonte. O bundler do Tauri grava o próprio BOM UTF-8 no arquivo que
; copia para o build (e no installer.nsi renderizado), e é esse BOM que faz o NSIS Unicode ler os
; acentos certo. Com BOM aqui o gerado fica com dois e o makensis para na linha 1. O guard
; scripts/ci-guards.ps1 (NSIS-BOM) e o test-installer.ps1 conferem fonte sem BOM e gerado com um.

; --- reinstalar e atualizar
LangString addOrReinstall ${LANG_PORTUGUESEBR} "Reinstalar o Sofá"
LangString alreadyInstalled ${LANG_PORTUGUESEBR} "Sofá já instalado"
LangString alreadyInstalledLong ${LANG_PORTUGUESEBR} "A versão ${VERSION} já está instalada. Reinstalar mantém a biblioteca e as configurações. Desinstalar pergunta o que fazer com elas."
LangString chooseMaintenanceOption ${LANG_PORTUGUESEBR} "Escolha o que fazer."
LangString chooseHowToInstall ${LANG_PORTUGUESEBR} "Escolha como instalar o Sofá."
LangString dontUninstall ${LANG_PORTUGUESEBR} "Instalar por cima, sem remover a versão atual"
LangString dontUninstallDowngrade ${LANG_PORTUGUESEBR} "Instalar por cima (indisponível para versão anterior)"
LangString newerVersionInstalled ${LANG_PORTUGUESEBR} "Uma versão mais nova do Sofá já está instalada. Não recomendamos instalar uma versão anterior por cima."
LangString older ${LANG_PORTUGUESEBR} "mais antiga"
LangString olderOrUnknownVersionInstalled ${LANG_PORTUGUESEBR} "Uma versão $R4 do Sofá está instalada. A biblioteca, as configurações e a opção Iniciar com o Windows continuam como estão."
LangString silentDowngrades ${LANG_PORTUGUESEBR} "Este modo de instalação não permite instalar uma versão anterior. Use o instalador com telas.$\n"
LangString unableToUninstall ${LANG_PORTUGUESEBR} "Não foi possível remover a versão instalada. Feche o Sofá e tente de novo. Se continuar, desinstale em Aplicativos instalados."
LangString uninstallApp ${LANG_PORTUGUESEBR} "Desinstalar o Sofá"
LangString uninstallBeforeInstalling ${LANG_PORTUGUESEBR} "Substituir a versão instalada (mantém seus dados)"
LangString unknown ${LANG_PORTUGUESEBR} "desconhecida"

; --- pasta fixa (SEC-BUG-05, ARQ-BUG-16; install-dir.nsh). Textos acrescentados na leva 6.
LangString sofaInstallDirFixed ${LANG_PORTUGUESEBR} "O Sofá só pode ser instalado em Program Files. Rode o instalador sem a opção /D=."
LangString sofaPreviousInstallElsewhere ${LANG_PORTUGUESEBR} "Há uma instalação do Sofá fora de Program Files ($SofaPrevDir). Desinstale-a em Aplicativos instalados e rode este instalador de novo."
LangString sofaUninstallDirFixed ${LANG_PORTUGUESEBR} "O desinstalador do Sofá só roda a partir de Program Files."

; --- Sofá aberto
LangString appRunning ${LANG_PORTUGUESEBR} "O ${DISPLAYNAME} está aberto. Feche-o e tente de novo."
LangString appRunningOkKill ${LANG_PORTUGUESEBR} "O ${DISPLAYNAME} está aberto.$\nClique em OK para fechá-lo."
LangString failedToKillApp ${LANG_PORTUGUESEBR} "Não foi possível fechar o ${DISPLAYNAME}. Feche-o você mesmo e tente de novo."

; --- atalho
LangString createDesktop ${LANG_PORTUGUESEBR} "Criar atalho na área de trabalho"

; --- WebView2 (CA-50). A mensagem de falha é a da spec, sem alteração. O bootstrapper embutido baixa
; sozinho: as strings webview2Download* do template não são usadas e saíram.
LangString installingWebview2 ${LANG_PORTUGUESEBR} "Instalando o WebView2…"
LangString webview2AbortError ${LANG_PORTUGUESEBR} "Não foi possível baixar o WebView2. Conecte o PC à internet e tente de novo."
LangString webview2InstallError ${LANG_PORTUGUESEBR} "A instalação do WebView2 falhou (código $1)."
LangString webview2InstallSuccess ${LANG_PORTUGUESEBR} "WebView2 instalado."

; --- página "Iniciar com o Windows" (CA-51, RN-19)
LangString sofaOptionsTitle ${LANG_PORTUGUESEBR} "Iniciar com o Windows"
LangString sofaOptionsSubtitle ${LANG_PORTUGUESEBR} "Escolha se o Sofá abre sozinho quando você entra no Windows."
LangString sofaStartupCheckbox ${LANG_PORTUGUESEBR} "Iniciar com o Windows"
LangString sofaStartupHint ${LANG_PORTUGUESEBR} "Esta escolha vale como padrão para cada usuário do PC. Cada um pode mudar depois, em Configurações."
LangString sofaNoKeyboardNote ${LANG_PORTUGUESEBR} "Para ligar o PC e jogar sem teclado, configure o logon automático ou o Windows Hello no Windows. O Sofá começa na área de trabalho. O instalador não altera o seu logon."

; --- desinstalador (CA-52, CA-68)
LangString keepAppData ${LANG_PORTUGUESEBR} "Manter dados (biblioteca e configurações)"
LangString sofaKeepData ${LANG_PORTUGUESEBR} "Mantendo a biblioteca e as configurações."
LangString sofaPurgeStart ${LANG_PORTUGUESEBR} "Apagando os dados do Sofá do seu usuário…"
LangString sofaSidIgnored ${LANG_PORTUGUESEBR} "Um perfil do Windows com identificador inválido foi ignorado ao remover a inicialização."
LangString sofaPurgePartial ${LANG_PORTUGUESEBR} "Não foi possível apagar todos os dados do Sofá. Alguns arquivos podem ter ficado na pasta abaixo (biblioteca, configurações, capas, logs e cache). Você pode apagar a pasta."
LangString sofaPurgeUnread ${LANG_PORTUGUESEBR} "Não foi possível confirmar a remoção dos dados do Sofá. Alguns arquivos podem ter ficado na pasta abaixo (biblioteca, configurações, capas, logs e cache). Você pode apagar a pasta."
LangString sofaPurgeInvalidTarget ${LANG_PORTUGUESEBR} "Os dados do Sofá não foram apagados: a pasta não passou na verificação de segurança. Ela está abaixo. Confira o conteúdo antes de apagar."
LangString sofaPurgeAppOpen ${LANG_PORTUGUESEBR} "O Sofá estava aberto, então os dados não foram apagados. Eles continuam na pasta abaixo, e você pode apagá-la."
LangString sofaPurgeRunning ${LANG_PORTUGUESEBR} "A remoção dos dados ainda não terminou. Se algo sobrar, apague a pasta abaixo."
LangString sofaPurgeNoProfile ${LANG_PORTUGUESEBR} "Não foi possível saber qual usuário está usando o Windows, então nenhum dado foi apagado. Para removê-los, apague a pasta app.sofa.launcher em AppData\Local do seu usuário."
