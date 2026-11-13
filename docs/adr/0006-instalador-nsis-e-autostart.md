---
status: accepted (fallback automático de autostart; validar no QA final)
date: 2026-11-13
revised: 2026-11-13 (P-01, P-03, P-07 decididos; SEC-21/22/23/25 e AF-05/AF-10 incorporados; rev. 3: SEC-31, SEC-32, DR-03; rev. 4: D-31, D-32, D-35)
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
- A caixa "Manter dados" vem marcada (CA-68). Desmarcada, o desinstalador, **antes** de apagar os binários, inicia `"$INSTDIR\sofa.exe" --purge-user-data` com `RunAsUser` (token do shell do usuário interativo, integridade Medium) e espera o processo terminar (`nsis_tauri_utils::FindProcess` em laço, com timeout de 60 s, caso o `RunAsUser` não devolva handle; o dev confirma no ST-03).
- O modo `--purge-user-data` do `sofa.exe` (doc de arquitetura §2.1, "Modo de remoção de dados"):
  1. é tratado no `main` **antes** de criar o Tauri, o WebView2 e o plugin de instância única. Não abre janela, não é comando IPC e não é repassado por segunda instância (SEC-19). Só é aceito se for o **único** argumento;
  2. recusa rodar elevado (`TokenElevation` verdadeiro → sai com código 3, sem apagar nada);
  3. calcula o alvo sozinho: `SHGetKnownFolderPath(FOLDERID_LocalAppData)` + o identifier **fixo no build**. Não recebe caminho de fora;
  4. confere SEC-25: o alvo está exatamente um nível abaixo de `%LOCALAPPDATA%`, não é ponto de reanálise e contém `library.json` ou `settings.json`. Se falhar, sai com código 2, sem apagar nada;
  5. apaga **por handle**: abre cada entrada com `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`; se for ponto de reanálise, apaga o próprio ponto (`FileDispositionInfoEx` no handle) sem descer; se for pasta comum, enumera pelo handle (`GetFileInformationByHandleEx`) e desce; arquivos são apagados pelo handle. Como roda com os direitos do próprio usuário, qualquer corrida restante não ganha privilégio;
  6. devolve o código 0 em sucesso; 1 em remoção parcial, com log em `%TEMP%\sofa-purge.log`.
- **Fallback** (o `RunAsUser` falhou, o perfil interativo não foi resolvido ou o código de saída ≠ 0): o desinstalador elevado apaga **só arquivos de nome conhecido**, sem recursão genérica: `library.json`, `settings.json`, `*.corrupt-*.json`, `logs\*.log*`, `cache\scan.json`, `cache\covers\*`, `cache\icons\*`. Cada subpasta conhecida é verificada como não-reparse antes. `EBWebView\` fica como resto documentado, e o desinstalador informa a pasta ao usuário. Se nem o perfil interativo for resolvido, não apaga nada e informa (SEC-25).
- **Verificação (ST-03 do dev e QF-05 no QA final)**: junction-isca dentro da pasta de dados; desinstalar sem manter dados preserva a isca. `--purge-user-data` rodado elevado recusa. `--purge-user-data` com argumentos extras é ignorado (abre o app normal ou sai sem efeito). Dados de outro usuário logado (troca rápida) ficam intactos (DR-03).
- Jogos nunca são tocados.

### 7. SEC-26

O build publica `SHA256SUMS.txt` ao lado do instalador, já que ele não é assinado (D-11, R-06, RR-02).

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
- Remoção elevada recursiva genérica (a primeira versão deste ADR) foi descartada por SEC-31 (DR-01).
- Se o usuário fechar o instalador sem abrir o Sofá, o autostart só é aplicado na primeira abertura manual. Aceitável, porque o atalho do menu Iniciar existe.
