---
status: accepted (fechado pelo SP-07 técnico; instalador elevado, RunAsUser e DLL no QA-17)
date: 2026-11-13
revised: 2026-11-13 (P-01, P-03, P-07 decididos; SEC-21/22/23/25 e AF-05/AF-10 incorporados; rev. 3: SEC-31, SEC-32, DR-03; rev. 4: D-31, D-32, D-35; rev. 5: resultado do SP-07); 2026-11-14 (rev. 6: DR-06 e D-43, fallback restrito a dois arquivos; código de saída do purge; SEC-BUG-03); 2026-11-15 (rev. 7: implementação do instalador, template próprio, pasta fixa, SEC-32 com $PLUGINSDIR e $INSTDIR, schtasks por SID)
feature: launcher-tv
---

# Instalador NSIS por máquina; o Sofá é o único dono do "Iniciar com o Windows", feito por tarefa de logon do próprio usuário sem elevação (fallback HKCU Run)

A instalação é para todo o PC, com UAC (RN-26), e o autostart é por usuário, sem administrador (RN-17). Dois problemas guiam este ADR. Primeiro (E-19): quando o UAC é aprovado com credenciais de **outra** conta administradora, o instalador roda como essa conta, e `HKCU` e `%LOCALAPPDATA%` apontam para o perfil errado. Segundo (RA-03): o Explorer atrasa a execução das entradas `Run` no logon, o que ameaça M-02.

## Decisão

### 1. Bundler

NSIS do Tauri 2 (`bundle.windows.nsis`) com:
- `installMode: "perMachine"`, pasta em `%ProgramFiles%\Sofa` (nome ASCII conforme o guia da marca; `productName` "Sofá" só no atalho e em "Aplicativos instalados").
- `webviewInstallMode: embedBootstrapper` (P-01 a).
- `installerIcon` = `docs/brand/sofa/assets/icon.ico`.
- Hooks NSIS (`NSIS_HOOK_POSTINSTALL`, `NSIS_HOOK_PREUNINSTALL`) para os passos abaixo.

MSI (WiX) fica de fora: não traz ganho para uso pessoal e dificulta a pergunta customizada do desinstalador (CA-68).

**Implementação (rev. 7, BE-37 a BE-39 e BE-42, aceita no ARQ-05 rodada 3):**
- **Template NSIS próprio** (`src-tauri/windows/installer.nsi`, fork do template do Tauri 2.12.4, escolhido em `bundle.windows.nsis.template`), com `hooks.nsh`, `install-hooks.nsh`, `uninstall-hooks.nsh` e `PortugueseBR.nsh`. Os hooks sozinhos não bastavam, porque o template original apaga dados com `RmDir /r` elevado e grava em `HKCU`. Saíram: a remoção recursiva de dados, a gravação em `HKCU`, a migração de WiX, o WebView2 por download ou offline e a atualização do runtime. Custo: a cada atualização do Tauri, as diferenças listadas no cabeçalho do fork têm de ser reaplicadas (tarefa explícita, igual ao `windows-rs`).
- **Pasta de instalação fixa**: `$PROGRAMFILES64\Sofa`. O SEC-20 e a exceção de `$INSTDIR` abaixo dependem de a pasta herdar as ACLs de `Program Files`. Escolher outra pasta, pela página de diretório ou por `/D=`, **não** é permitido (ARQ-BUG-16).
- **Executável externo (SEC-32)**: só por `$SYSDIR\`, `$PLUGINSDIR\` (o bootstrapper do WebView2, na pasta temporária privada do NSIS da conta que instala) ou `$INSTDIR\` (o desinstalador anterior, e só se estiver no `$INSTDIR` atual). Continuam proibidos `$TEMP\`, nome relativo e caminho vindo de variável ou do registro.
- **§6a: remoção do autostart de todos os perfis** por `"$SYSDIR\schtasks.exe" /Delete /TN "Sofa Autostart (<SID>)" /F`, com SID validado caractere a caractere em `^S-1-5-21(-[0-9]{1,10}){4}$`. A enumeração por COM com prefixo foi descartada: exigiria `VARIANT` por valor pelo plugin `System`, sem segurança no NSIS. Junto vai a remoção do valor `Sofa` em `HKU\<SID>\...\Run` e em `StartupApproved\Run` dos hives carregados. Hive descarregado deixa entrada órfã, que o Windows ignora porque o exe não existe mais.
- **§6b: usuário interativo**: o SID vem de duas fontes, o token do Explorer da sessão e o usuário da sessão (`WTSQuerySessionInformationW`). Divergência ou ausência das duas = nada apagado. O `LocalAppData` é o `ProfileImagePath` da `ProfileList` + `\AppData\Local`, e o fallback exige que o caminho final seja o mesmo texto, sem junção no caminho. As duas regras são mais restritivas que o texto anterior.

### 2. Autostart: dono único, criado no contexto do usuário

- O instalador **não** cria autostart no perfil de ninguém. Ele grava a escolha da caixa "Iniciar com o Windows" (marcada por padrão, CA-51) em `HKLM\Software\app.sofa.launcher\InstallDefaults` e, na página final, abre o Sofá **como o usuário interativo, sem elevação**.
- Na primeira execução de cada usuário (`settings.json` ausente ou sem `installDefaultsApplied`), o próprio Sofá, rodando como esse usuário, lê o padrão de `HKLM` e cria ou não o autostart. É o mesmo módulo (`AutostartRegistry`) usado pela opção nas configurações (CA-04).
- **Mecanismo preferido (P-03 b)**: tarefa agendada criada pelo Sofá via COM `ITaskService` (sem `schtasks.exe` nem shell, SEC-10), registrada no contexto do usuário:
  - nome `Sofa Autostart (<SID do usuário>)`, na raiz do Agendador;
  - gatilho `LogonTrigger` só para o próprio usuário (`UserId` = SID atual);
  - `Principal`: `LogonType = InteractiveToken`, **`RunLevel = LeastPrivilege`** (nunca "Executar com privilégios mais altos"), `UserId` = o próprio usuário;
  - ação: `"C:\Program Files\Sofa\sofa.exe"` com argumento `--autostart` (caminho absoluto entre aspas, SEC-22);
  - configurações: `ExecutionTimeLimit = PT0S` (o padrão de 72 h encerraria o Sofá, RA-07), `MultipleInstancesPolicy = IgnoreNew`, `DisallowStartIfOnBatteries = false`, `StopIfGoingOnBatteries = false`, sem atraso no gatilho.
- **Fallback automático (P-03 a)**, sem spike e sem pergunta ao usuário (rev. 4). O `AutostartRegistry` passa para o Run quando (i) `RegisterTaskDefinition` falha (ex.: `E_ACCESSDENIED`) ou (ii) na abertura seguinte a tarefa existe mas `IRegisteredTask::LastTaskResult` indica falha. Em ambos os casos remove a tarefa e registra o motivo no log: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, valor `Sofa` = `"C:\Program Files\Sofa\sofa.exe" --autostart`, respeitando o `StartupApproved` (desligado pelo Gerenciador de Tarefas). Nesse caso M-02 passa a contar a partir do disparo da inicialização automática.
- **Sem volta automática Run → tarefa (SEC-22 d, ARQ-05)**: depois do fallback, o Sofá só volta a tentar a tarefa quando o usuário religa a opção. A nova tentativa "numa atualização do app, uma vez", que SEC-22 (d) permite, **não** é feita no MVP: é o lado mais restritivo do requisito, e o ganho é pequeno. Volta a ser avaliada se o QF-06 mostrar o Run estourando M-02.
- **Um mecanismo ativo por vez**: ligar um remove o outro, o que também cobre a migração entre versões.
- **Proibidos (AF-05, SEC-22)**: tarefa elevada, tarefa em nome de outra conta, serviço Windows, `HKLM\...\Run`.

### 3. SEC-21: abrir o app sem elevação

A página final do template NSIS do Tauri abre o app com a rotina `RunAsUser` do plugin `nsis_tauri_utils`. Ela usa o token do shell do usuário interativo (Explorer), não o token elevado do instalador, então o app abre com integridade Medium e como o usuário que está na sessão, mesmo com UAC de outra conta. O dev verifica com uma conta padrão de teste (ST-03); o QA final confirma com conta padrão + credencial de outro admin (D-31, QF-05). Se falhar, a opção "Executar ao terminar" é removida e o autostart é aplicado na primeira abertura pelo menu Iniciar.

### 4. SEC-23: DLL segura

- **Instalador**: NSIS 3.x (o Tauri baixa uma versão 3.x fixa) chama `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)` no stub antes de carregar DLLs de sistema. Plugins NSIS são extraídos para `$PLUGINSDIR`, uma pasta temporária aleatória, e não para a pasta de onde o instalador roda.
- **App**: `sofa.exe` chama `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_APPLICATION_DIR)` como primeira instrução do `main`. O loader do WebView2 é estático (sem `WebView2Loader.dll` ao lado do exe). ASLR, DEP e CFG ficam nos padrões do toolchain MSVC.
- **Verificação (ST-03 do dev e QA final)**: Process Monitor com `version.dll`, `uxtheme.dll`, `userenv.dll` e `cryptbase.dll` plantadas ao lado do instalador em Downloads. Nenhuma pode ser carregada.

### 5. Atualização (CA-53)

O NSIS do Tauri detecta a versão instalada e substitui os binários. Dados, tarefa e entrada `Run` não são tocados, porque o caminho do exe não muda.

### 6. Desinstalação (CA-52, CA-68, SEC-25, SEC-31, SEC-32, AF-10, DR-03)

O processo elevado **só decide**; ele nunca percorre nem apaga a árvore de dados do usuário (SEC-31: a checagem de reparse point seguida de remoção por caminho, feita como admin, é uma corrida TOCTOU que vira "apagar qualquer arquivo como admin").

**a) Autostart (sempre, todos os perfis; DR-03 permite porque o binário deixa de existir)**
- Tarefas (SEC-32): para cada SID listado em `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList` que passe na regex `^S-1-5-21(-[0-9]{1,10}){4}$`, o desinstalador executa `"$SYSDIR\schtasks.exe" /Delete /TN "Sofa Autostart (<SID>)" /F` via `nsExec`. O caminho é absoluto, não há `cmd`, e o nome da tarefa é montado só a partir de uma constante + o SID validado. A `ProfileList` cobre também usuários sem hive carregado. Alternativa preferida pela segurança, se o dev mostrar no ST-03 que é viável no NSIS sem plugin novo: enumerar a raiz do Agendador via COM pelo prefixo `Sofa Autostart (` e remover.
- `Run`: remove o valor `Sofa` em `HKU\<SID>\...\Run` de cada SID validado com hive carregado. Hives não carregados ficam com uma entrada órfã que aponta para um exe inexistente, que o Windows ignora (resto documentado).
- Remove `HKLM\Software\app.sofa.launcher`.

**b) Dados: só do usuário interativo (DR-03), sem elevação (SEC-31)**
- A caixa "Manter dados" vem marcada (CA-68). Desmarcada, o desinstalador, **antes** de apagar os binários:
  1. **fecha o Sofá** do usuário interativo (SEC-BUG-03). Usa a verificação de app em execução do template NSIS do Tauri, que pede para fechar. Se o Sofá não fechar, o purge não roda: a pasta é mostrada e nada é apagado;
  2. inicia `"$INSTDIR\sofa.exe" --purge-user-data` como o usuário interativo, sem elevação (integridade Medium);
  3. **obtém o código de saída** como descrito em "Código de saída do purge", abaixo.
- O modo `--purge-user-data` do `sofa.exe` (doc de arquitetura §2.1, "Modo de remoção de dados"):
  1. é tratado no `main` **antes** de criar o Tauri, o WebView2 e o plugin de instância única. Não abre janela, não é comando IPC e não é repassado por segunda instância (SEC-19). Só é aceito se for o **único** argumento;
  2. recusa rodar elevado (`TokenElevation` verdadeiro → sai com código 3, sem apagar nada);
  2a. **recusa rodar com o Sofá aberto (rev. 6, SEC-BUG-03)**: antes de apagar, tenta `OpenMutexW(SYNCHRONIZE, FALSE, "app.sofa.launcher-sim")` (o mutex da instância única, no namespace da sessão). Se o mutex existir, sai com **código 4** ("Sofá aberto"), sem apagar nada. Com o Sofá aberto, a remoção ficaria parcial e o Sofá regravaria `library.json` e `settings.json`;
  3. calcula o alvo sozinho: `SHGetKnownFolderPath(FOLDERID_LocalAppData)` + o identifier **fixo no build**. Não recebe caminho de fora;
  4. confere SEC-25: o alvo está exatamente um nível abaixo de `%LOCALAPPDATA%`, não é ponto de reanálise e contém `library.json` ou `settings.json`. Se falhar, sai com código 2, sem apagar nada;
  5. apaga **por handle**: abre cada entrada com `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`; se for ponto de reanálise, apaga o próprio ponto (`FileDispositionInfoEx` no handle) sem descer; se for pasta comum, enumera pelo handle (`GetFileInformationByHandleEx`) e desce; arquivos são apagados pelo handle. Como roda com os direitos do próprio usuário, qualquer corrida restante não ganha privilégio;
  6. devolve o código 0 em sucesso; 1 em remoção parcial; 2 com alvo inválido; 3 rodando elevado; 4 com o Sofá aberto. Log em `%TEMP%\sofa-purge.log`.

**Código de saída do purge (rev. 6, DR-06).** Com `nsis_tauri_utils::FindProcess` em laço, o desinstalador sabe só que o processo sumiu, não como ele terminou. O `RunAsUser` do plugin também não devolve handle. Proposta, em ordem:
- **Preferido: iniciar o purge pelo plugin `System` do NSIS** (já vem com o NSIS, não é plugin novo), com o shell do usuário interativo como processo pai:
  1. `GetShellWindow` → `GetWindowThreadProcessId` → `OpenProcess(PROCESS_CREATE_PROCESS)` no Explorer da sessão;
  2. `InitializeProcThreadAttributeList` + `UpdateProcThreadAttribute(PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, <handle do Explorer>)`;
  3. `CreateProcessW("$INSTDIR\sofa.exe", "\"$INSTDIR\sofa.exe\" --purge-user-data", ..., EXTENDED_STARTUPINFO_PRESENT, ...)`. O filho herda o token do pai (o usuário interativo, Medium), e o desinstalador recebe o `hProcess` em `PROCESS_INFORMATION`;
  4. `WaitForSingleObject(hProcess, 60000)` e, se terminou, `GetExitCodeProcess`. Fecha os handles.
  
  O dev confirma no BE-38 (e o QF-05 com UAC de outra conta) três pontos: a integridade Medium do filho; que o filho roda como o usuário interativo e não como a conta do UAC; e que `OpenProcess` no Explorer funciona quando o UAC foi aprovado por outra conta. Para isso **não** se habilita `SeDebugPrivilege` no instalador. Se não funcionar, vale o caminho seguinte.
- **Alternativa: `RunAsUser` + `FindProcess` em laço (60 s)**, como antes. O código é tratado como **não lido**.
- **Código não lido = falha.** Entra aqui todo caso em que não há um `GetExitCodeProcess` válido: handle não obtido, `CreateProcessW` falhou, timeout de 60 s, ou o caminho alternativo. Nesse caso o desinstalador **não apaga além de `library.json` e `settings.json`** (fallback abaixo). Se, passados os 60 s, um `sofa.exe` ainda estiver rodando, o fallback também não roda, porque o purge pode estar no meio: nada é apagado e a pasta é mostrada.

Decisão por código lido:

| Código | O que o desinstalador faz |
|---|---|
| 0 | Nada mais a apagar |
| 1 (parcial) | Fallback restrito; mostra a pasta |
| 2 (alvo inválido, SEC-25) | **Não** apaga nada; mostra a pasta. O alvo reprovou na conferência; o fallback não pode passar por cima dela |
| 3 (rodou elevado) | Fallback restrito; mostra a pasta. Indica que o caminho sem elevação falhou |
| 4 (Sofá aberto) | **Não** apaga nada; mostra a pasta e pede para fechar o Sofá e desinstalar de novo, ou apagar a pasta à mão |
| não lido, ou outro valor | Falha: fallback restrito, salvo se o purge ainda estiver rodando; mostra a pasta |

- **Fallback restrito (rev. 6, DR-06 e D-43 do usuário).** Roda elevado e apaga **no máximo dois arquivos**, pelo nome exato, na raiz da pasta de dados do usuário interativo: `library.json` e `settings.json`. Sem curinga, sem subpasta e sem recursão. Capas, ícones, logs, `cache\`, `*.corrupt-*.json` e `EBWebView\` ficam, e o desinstalador **mostra a pasta** ao usuário. Se o perfil interativo não for resolvido, nada é apagado e a pasta é informada (SEC-25). Passos, pelo plugin `System` (sem plugin novo):
  1. abre a **raiz** (`%LOCALAPPDATA%\app.sofa.launcher` do usuário interativo) com `CreateFileW(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)`, **sem `FILE_SHARE_DELETE`**: enquanto o handle está aberto, a raiz não pode ser renomeada nem trocada por uma junction;
  2. lê `FileAttributeTagInfo` pelo handle: se a raiz for ponto de reanálise, para sem apagar nada. Guarda o caminho final da raiz (`GetFinalPathNameByHandleW`) e confere que ele é exatamente um nível abaixo do `%LOCALAPPDATA%` do usuário interativo;
  3. para cada um dos dois nomes, abre `<raiz>\<nome>` com `FILE_FLAG_OPEN_REPARSE_POINT` e acesso `DELETE | FILE_READ_ATTRIBUTES`. O arquivo ausente é ignorado;
  4. pelo handle do arquivo, confere três coisas: (a) `FileAttributeTagInfo` sem `FILE_ATTRIBUTE_REPARSE_POINT` e sem `FILE_ATTRIBUTE_DIRECTORY`, ou seja, **não é link** nem pasta; (b) o caminho final (`GetFinalPathNameByHandleW`) é exatamente `<caminho final da raiz>\<nome>`; (c) o caminho final da raiz, relido agora, não mudou. Qualquer divergência: fecha o handle e não apaga esse arquivo;
  5. apaga **pelo handle** (`SetFileInformationByHandle` com `FileDispositionInfo`), nunca por caminho (`Delete` do NSIS é proibido aqui);
  6. fecha os handles e mostra a pasta.
  
  Um hard link com um desses nomes não é problema: a exclusão por handle remove só o nome na pasta do usuário, e o outro nome continua intacto.
- **Verificação (ST-03 do dev, BE-38 e QF-05 no QA final)**:
  - junction-isca dentro da pasta de dados: desinstalar sem manter dados preserva a isca;
  - fallback com a raiz trocada por junction (estática e em laço durante o fallback, como no `purge_integration`), e com `library.json` e `settings.json` como symlink ou junction para uma isca: a isca fica intacta;
  - purge com o Sofá aberto: código 4, nada apagado (BE-35, SEC-BUG-03);
  - código não lido (simular falha do `CreateProcessW`): apaga no máximo os dois arquivos;
  - `--purge-user-data` rodado elevado recusa; com argumentos extras é ignorado (abre o app normal ou sai sem efeito);
  - dados de outro usuário logado (troca rápida) ficam intactos (DR-03).
- Jogos nunca são tocados.

### 7. SEC-26

O build publica `SHA256SUMS.txt` ao lado do instalador, já que ele não é assinado (D-11, R-06, RR-02).

## Resultado do SP-07 (`docs/arch/spikes/SP-07.md`, executado no host como conta comum; sem VM)

**Autostart: decidido.** O mecanismo padrão é a tarefa de logon do próprio usuário, com fallback automático para HKCU Run.
- **Tarefa criada sem administrador**: `RegisterTaskDefinition(TASK_CREATE_OR_UPDATE, UserId = SID atual, TASK_LOGON_INTERACTIVE_TOKEN)`, `RunLevel = TASK_RUNLEVEL_LUA`. Leva 19 ms, sem UAC. O processo iniciado pela tarefa roda com token comum, integridade Medium, com `argv = ["--autostart"]`.
- **Gatilho do fallback confirmado**: `E_ACCESSDENIED (0x80070005)` em `RegisterTaskDefinition`. O sistema também nega tarefa elevada e tarefa em nome de outro SID à conta comum, o que reforça AF-05.
- **`LastTaskResult = 0x41303` (`SCHED_S_TASK_HAS_NOT_RUN`) não é falha.** O fallback (ii) só vale para códigos de erro reais depois de a tarefa ter rodado.
- **Conferir que a tarefa é do Sofá** (BE-36):
  - comparar `UserId` por **SID** (o `IPrincipal.UserId` devolve nome de conta: resolver com `LookupAccountNameW`, ou ler o XML);
  - conferir `IPrincipal.RunLevel == TASK_RUNLEVEL_LUA` e a **ausência** de `<RunLevel>HighestAvailable</RunLevel>` (o XML omite `<RunLevel>` quando é LeastPrivilege);
  - comparar o `Path` da ação normalizando as aspas.
- Proxy de M-02: `IRegisteredTask::Run` → processo iniciado em 100 a 450 ms. O tempo de logon real fica para o QF-06.

**Remoção de tarefas (SEC-32).** Enumerar a raiz do Agendador por COM pelo prefixo `Sofa Autostart (` e chamar `DeleteTask` **funciona do lado do Windows**. Falta provar no NSIS (BE-42):
- se for viável sem plugin novo, é o caminho padrão;
- senão, `"$SYSDIR\schtasks.exe" /Delete /TN "Sofa Autostart (<SID>)" /F` com SID validado.

**Purge sem elevação (SEC-31).** O algoritmo de §6b foi medido como conta comum: handle aberto com `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`, atributos lidos do handle (`FileAttributeTagInfo`), caminho final do filho conferido dentro do caminho final do pai (`GetFinalPathNameByHandle`), exclusão por `FileDispositionInfoEx(DELETE | POSIX_SEMANTICS | IGNORE_READONLY)` com fallback para `FileDispositionInfo`.
- Junction estática: removida sem descer, isca 50/50 intacta.
- Corrida: 57 trocas por junction durante a remoção, 0 arquivos da isca perdidos.
- O BE-41 reaproveita o algoritmo e repete a corrida em `win-integration`.

**Instância única (SEC-19): manter o `tauri-plugin-single-instance`.** O mutex do plugin (v2.5.5) é `"{identifier}-sim"`, **sem SID e sem prefixo**. Nome sem prefixo cai no namespace da sessão, então usuários em sessões diferentes (troca rápida) não colidem. A segunda instância envia `WM_COPYDATA` com cwd + `argv`. Decisão:
- manter o plugin (que também chama `AllowSetForegroundWindow` para a 1ª instância, o que ajuda a trazer a janela);
- **o callback ignora `argv` e cwd por completo**. O `--autostart` só é lido pela própria instância que inicia, nunca pelo repasse;
- o `--purge-user-data` é tratado no `main` antes do plugin, então nunca passa por ele.

**Aceito pela segurança** (SEC-19 emendado na rev. 5): o mutex sem SID é aceito com a condição de ignorar `argv`. O namespace da sessão isola outros usuários, e o restante é RR-01. Não há adapter próprio.

**Toolchain (SEC-23).** Na máquina de dev não há MSVC Build Tools (instalar exige admin). O dev compila com Rust `x86_64-pc-windows-gnu` e `windows-rs` 0.58 fixado. Nesse alvo o `WebView2Loader.dll` fica **ao lado do exe** (carregador dinâmico), o que não atende SEC-23. **O build de release, o instalador e o teste de DLL plantada usam obrigatoriamente o alvo `x86_64-pc-windows-msvc`** (carregador estático, CFG). Eles rodam nesta máquina de dev depois que o usuário instalar o VS Build Tools (P-08 b). Registrado no ADR-0001.

**Pendente para o QA-17 (QF-05, QF-06)**: `RunAsUser` com UAC de outra conta; DLL plantada ao lado do instalador elevado e do `sofa.exe` (só vale no build MSVC); desinstalador completo (autostart de todos os perfis, purge via `RunAsUser`, `EBWebView\` em uso, dados de outro perfil); logon → navegável; Windows 10 22H2 sem WebView2.

## Alvo principal Windows 10 e validação (rev. 4)

- **Win10 22H2 é o alvo principal (D-32).** Tudo aqui funciona no Win10:
  - Agendador de Tarefas 2.0 (`ITaskService`);
  - `RunAsUser`, NSIS 3.x e `SetDefaultDllDirectories` (Win8+);
  - bootstrapper do WebView2 (para Win10 sem o runtime);
  - `FILE_DISPOSITION_FLAG_POSIX_SEMANTICS` no purge (Win10 1709+), com fallback para exclusão comum.
- O dev testa em VM Win10 22H2 sem WebView2 (ST-05).
- **Validar no QA final (D-31, QF-05/QF-06)**: instalação com conta padrão + UAC de outro admin; app aberto como o usuário; mecanismo de autostart ativo (o log diz qual e por quê); tempo logon → navegável (M-02); desinstalar mantendo e apagando dados.
- **Para a segurança reavaliar (SEC-22)**: a escolha entre tarefa e Run passou a ser automática, em runtime. Os dois mecanismos já são permitidos pelo SEC-22 emendado e a regra de caminho (DR-04) vale para ambos.

## Opções consideradas

- **Instalador grava o autostart direto**: falha em E-19 (iria para a conta admin do UAC) e viola SEC-21. Descartada.
- **Active Setup**: atinge todo usuário que entrar no PC e só age no próximo logon. Descartada; fica como plano B se o RunAsUser falhar.
- **Só HKCU Run**: aprovado pela segurança, mas sofre o atraso do Explorer (RA-03). Mantido como fallback.
- **Tarefa elevada, serviço, HKLM Run**: rejeitados pela segurança (AF-05).

## Consequências

- SEC-22 e SEC-29 já foram emendados pela segurança (rev. 2) para incluir a tarefa do próprio usuário com `LeastPrivilege`.
- Pela emenda de SEC-22 (DR-04), o `AutostartRegistry` recusa registrar autostart se o exe em execução não estiver sob `%ProgramFiles%` (ex.: uma cópia portátil numa pasta gravável).
- O `sofa.exe` ganha um segundo modo de entrada (`--purge-user-data`), que existe só para o desinstalador e fica fora da IPC.
- Remoção elevada recursiva genérica (a primeira versão deste ADR) foi descartada por SEC-31 (DR-01). A remoção elevada por curinga em subpastas conhecidas (rev. 1 a 5) foi descartada por DR-06: checar e depois apagar por caminho, como admin, é a mesma corrida. O usuário manteve um fallback mínimo (D-43, AF-11): dois arquivos na raiz, por handle.
- Quando o purge sem elevação falha, capas, ícones, logs e `EBWebView\` ficam na pasta, que é mostrada ao usuário. É o custo aceito de não apagar como admin.
- Se o usuário fechar o instalador sem abrir o Sofá, o autostart só é aplicado na primeira abertura manual. Aceitável, porque o atalho do menu Iniciar existe.
