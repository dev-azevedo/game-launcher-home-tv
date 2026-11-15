; Texto das telas do instalador e do desinstalador do Sofa (pt-BR). Substitui o PortugueseBR.nsh do
; Tauri (`bundle.windows.nsis.customLanguageFiles`): traz todas as chaves que o template usa.
; Regras de texto da marca: frase curta, sem jargao, o que fazer em seguida.
; `{{product_name}}` e trocado em tempo de execucao pelo nome exibido (DISPLAYNAME).

; --- reinstalar e atualizar
LangString addOrReinstall ${LANG_PORTUGUESEBR} "Reinstalar o Sofá"
LangString alreadyInstalled ${LANG_PORTUGUESEBR} "Sofá já instalado"
LangString alreadyInstalledLong ${LANG_PORTUGUESEBR} "O Sofá ${VERSION} já está instalado. Escolha o que fazer e clique em Avançar."
LangString chooseMaintenanceOption ${LANG_PORTUGUESEBR} "Escolha o que fazer."
LangString chooseHowToInstall ${LANG_PORTUGUESEBR} "Escolha como instalar o Sofá."
LangString dontUninstall ${LANG_PORTUGUESEBR} "Manter a versão instalada e só atualizar os arquivos"
LangString dontUninstallDowngrade ${LANG_PORTUGUESEBR} "Manter a versão instalada (instalar uma versão anterior por cima está desligado)"
LangString newerVersionInstalled ${LANG_PORTUGUESEBR} "Uma versão mais nova do Sofá já está instalada. Não recomendamos instalar uma versão anterior por cima. Escolha o que fazer e clique em Avançar."
LangString older ${LANG_PORTUGUESEBR} "mais antiga"
LangString olderOrUnknownVersionInstalled ${LANG_PORTUGUESEBR} "Uma versão $R4 do Sofá está instalada. Atualizar mantém a biblioteca, as configurações e o Iniciar com o Windows. Escolha o que fazer e clique em Avançar."
LangString silentDowngrades ${LANG_PORTUGUESEBR} "Instalar uma versão anterior está desligado neste instalador. Use a instalação com telas.$\n"
LangString unableToUninstall ${LANG_PORTUGUESEBR} "Não foi possível remover a versão instalada. Feche o Sofá e tente de novo."
LangString uninstallApp ${LANG_PORTUGUESEBR} "Desinstalar o Sofá"
LangString uninstallBeforeInstalling ${LANG_PORTUGUESEBR} "Atualizar a versão instalada"
LangString unknown ${LANG_PORTUGUESEBR} "desconhecida"

; --- pasta fixa (SEC-BUG-05, ARQ-BUG-16; install-dir.nsh). Textos acrescentados na leva 6.
LangString sofaInstallDirFixed ${LANG_PORTUGUESEBR} "O Sofá só pode ser instalado em Program Files. Rode o instalador sem a opção /D=."
LangString sofaPreviousInstallElsewhere ${LANG_PORTUGUESEBR} "Há uma instalação do Sofá fora de Program Files ($SofaPrevDir). Desinstale-a em Aplicativos instalados e rode este instalador de novo."
LangString sofaUninstallDirFixed ${LANG_PORTUGUESEBR} "O desinstalador do Sofá só roda a partir de Program Files."

; --- Sofá aberto
LangString appRunning ${LANG_PORTUGUESEBR} "O {{product_name}} está aberto. Feche o Sofá e tente de novo."
LangString appRunningOkKill ${LANG_PORTUGUESEBR} "O {{product_name}} está aberto.$\nClique em OK para fechá-lo."
LangString failedToKillApp ${LANG_PORTUGUESEBR} "Não foi possível fechar o {{product_name}}. Feche o Sofá e tente de novo."

; --- atalho
LangString createDesktop ${LANG_PORTUGUESEBR} "Criar atalho na área de trabalho"

; --- WebView2 (CA-50). A mensagem de falha e a da spec, sem alteracao.
LangString installingWebview2 ${LANG_PORTUGUESEBR} "Instalando o WebView2..."
LangString webview2AbortError ${LANG_PORTUGUESEBR} "Não foi possível baixar o WebView2. Conecte o PC à internet e tente de novo."
LangString webview2DownloadError ${LANG_PORTUGUESEBR} "Não foi possível baixar o WebView2: $0"
LangString webview2DownloadSuccess ${LANG_PORTUGUESEBR} "WebView2 baixado."
LangString webview2Downloading ${LANG_PORTUGUESEBR} "Baixando o WebView2..."
LangString webview2InstallError ${LANG_PORTUGUESEBR} "A instalação do WebView2 falhou (código $1)."
LangString webview2InstallSuccess ${LANG_PORTUGUESEBR} "WebView2 instalado."

; --- pagina "Iniciar com o Windows" (CA-51, RN-19)
LangString sofaOptionsTitle ${LANG_PORTUGUESEBR} "Iniciar com o Windows"
LangString sofaOptionsSubtitle ${LANG_PORTUGUESEBR} "Escolha se o Sofá abre sozinho quando você entra no Windows."
LangString sofaStartupCheckbox ${LANG_PORTUGUESEBR} "Iniciar com o Windows"
LangString sofaStartupHint ${LANG_PORTUGUESEBR} "Vale para cada pessoa que usar o Sofá neste PC. Dá para mudar depois, em Configurações."
LangString sofaNoKeyboardNote ${LANG_PORTUGUESEBR} "Para usar o Sofá só com o controle, deixe o Windows entrar sozinho na área de trabalho: entrada automática, PIN ou Windows Hello. Isso é uma configuração do Windows, fora do Sofá, e o instalador não altera o seu login."

; --- desinstalador (CA-52, CA-68)
LangString keepAppData ${LANG_PORTUGUESEBR} "Manter dados (biblioteca e configurações)"
LangString sofaKeepData ${LANG_PORTUGUESEBR} "Mantendo a biblioteca e as configurações."
LangString sofaPurgeStart ${LANG_PORTUGUESEBR} "Apagando a biblioteca e as configurações do seu usuário..."
LangString sofaSidIgnored ${LANG_PORTUGUESEBR} "Um perfil com identificador fora do padrão foi ignorado."
LangString sofaPurgePartial ${LANG_PORTUGUESEBR} "Não foi possível apagar todos os dados do Sofá. A biblioteca e as configurações foram removidas. O que sobrou (capas, registros e cache) está na pasta abaixo, e você pode apagá-la."
LangString sofaPurgeUnread ${LANG_PORTUGUESEBR} "Não foi possível confirmar a remoção dos dados do Sofá. A biblioteca e as configurações foram removidas. O que sobrou (capas, registros e cache) está na pasta abaixo, e você pode apagá-la."
LangString sofaPurgeInvalidTarget ${LANG_PORTUGUESEBR} "Os dados do Sofá não foram apagados: a pasta não passou na conferência de segurança. Ela está abaixo, e você pode apagá-la se quiser."
LangString sofaPurgeAppOpen ${LANG_PORTUGUESEBR} "O Sofá estava aberto, então os dados não foram apagados. Feche o Sofá e desinstale de novo, ou apague a pasta abaixo."
LangString sofaPurgeRunning ${LANG_PORTUGUESEBR} "A remoção dos dados ainda está em andamento e nada mais foi apagado. A pasta está abaixo."
LangString sofaPurgeNoProfile ${LANG_PORTUGUESEBR} "Não foi possível saber qual usuário está usando o Windows, então nenhum dado foi apagado. Apague você mesmo a pasta app.sofa.launcher dentro de AppData\Local do seu usuário."
