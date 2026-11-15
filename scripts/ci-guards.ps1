# Guards de arquitetura e seguranca por grep (BE-12, D-34). Compativel com Windows PowerShell 5.1.
#
# Cada regra diz: padrao proibido, arquivos "donos" onde ele pode aparecer, e o INV/SEC que protege.
# Varre `crates/**/src/**/*.rs` e `src-tauri/src/**/*.rs` (nao varre `tests/`), pulando linhas de
# comentario e os itens marcados com `#[cfg(test)]` (so o item, nao o resto do arquivo). Manifestos (`.manifest`, `.xml`, `.rc`) e scripts NSIS (`.nsi`, `.nsh`) tem regras
# proprias. Uso: `. scripts/ci-guards.ps1; Invoke-Guards -Root <raiz>`.

Set-StrictMode -Version 2.0

# Donos (caminhos relativos a raiz, com `/`). Lista vazia = proibido em qualquer arquivo.
$script:GuardRules = @(
    # INV-03: um dono por recurso
    @{ Id = 'INV-03'; Pattern = '\bXInputGetState(Ex)?\b';          Allow = @('crates/launcher-win/src/xinput.rs') },
    @{ Id = 'INV-03'; Pattern = '\bSetForegroundWindow\b';          Allow = @('crates/launcher-win/src/foreground.rs') },
    @{ Id = 'INV-03'; Pattern = '\bSendInput\b';                    Allow = @('crates/launcher-win/src/foreground.rs') },
    @{ Id = 'INV-03'; Pattern = '\bAttachThreadInput\b';            Allow = @('crates/launcher-win/src/foreground.rs') },
    # SEC-28: so ForegroundBroker e SystemEvents usam o hook de evento, e sempre fora de contexto
    @{ Id = 'SEC-28'; Pattern = '\bSetWinEventHook\b';              Allow = @('crates/launcher-win/src/foreground.rs', 'crates/launcher-win/src/system_events.rs') },
    @{ Id = 'SEC-28'; Pattern = '\bWINEVENT_INCONTEXT\b';           Allow = @() },
    @{ Id = 'SEC-28'; Pattern = '\bSPI_SETFOREGROUNDLOCKTIMEOUT\b'; Allow = @() },
    # INV-05: nada forcado sem 2a confirmacao
    @{ Id = 'INV-05'; Pattern = '\bEWX_FORCE\w*';                   Allow = @() },
    @{ Id = 'INV-05'; Pattern = '\bSeDebugPrivilege\b';             Allow = @() },
    @{ Id = 'INV-05'; Pattern = '\bPROCESS_ALL_ACCESS\b';           Allow = @() },
    @{ Id = 'INV-05'; Pattern = 'shutdown\.exe';                    Allow = @() },
    # SEC-BUG-01: o Job so rastreia; nunca se encerra o Job inteiro (so membros, por handle)
    @{ Id = 'INV-05'; Pattern = '\bTerminateJobObject\b';        Allow = @() },
    # INV-08: sem hook global, injecao ou acesso a memoria de outro processo
    @{ Id = 'INV-08'; Pattern = '\bSetWindowsHookEx\w*';            Allow = @() },
    @{ Id = 'INV-08'; Pattern = '\bCreateRemoteThread\w*';          Allow = @() },
    @{ Id = 'INV-08'; Pattern = '\bReadProcessMemory\b';            Allow = @() },
    @{ Id = 'INV-08'; Pattern = '\bWriteProcessMemory\b';           Allow = @() },
    @{ Id = 'INV-08'; Pattern = '\bLoadLibrary(Ex)?[AW]?\b';        Allow = @() },
    # INV-09: escrita/remocao em disco so nos donos (repositorios, logs, cache de capas, purge)
    @{ Id = 'INV-09'; Pattern = '\bremove_file\b';                  Allow = @('crates/launcher-win/src/persistence/json.rs', 'crates/launcher-win/src/logging.rs', 'crates/launcher-win/src/cover_cache.rs', 'crates/launcher-win/src/purge.rs') },
    @{ Id = 'INV-09'; Pattern = '\bremove_dir(_all)?\b';            Allow = @('crates/launcher-win/src/cover_cache.rs', 'crates/launcher-win/src/purge.rs') },
    @{ Id = 'INV-09'; Pattern = '\bDeleteFileW\b';                  Allow = @('crates/launcher-win/src/purge.rs') },
    @{ Id = 'INV-09'; Pattern = '\bfs::rename\b';                   Allow = @('crates/launcher-win/src/persistence/json.rs', 'crates/launcher-win/src/logging.rs', 'crates/launcher-win/src/cover_cache.rs') },
    @{ Id = 'INV-09'; Pattern = '\bMoveFileExW\b';                  Allow = @('crates/launcher-win/src/persistence/json.rs') },
    # INV-09 (ARQ-BUG-06): criar e gravar tambem so nos donos
    @{ Id = 'INV-09'; Pattern = '\bfs::write\b';                    Allow = @('crates/launcher-win/src/bin/sofa_test_game.rs', 'crates/launcher-win/src/persistence/json.rs', 'crates/launcher-win/src/logging.rs', 'crates/launcher-win/src/cover_cache.rs', 'crates/launcher-win/src/purge.rs') },
    @{ Id = 'INV-09'; Pattern = '\bFile::create(_new)?\b';          Allow = @('crates/launcher-win/src/persistence/json.rs', 'crates/launcher-win/src/logging.rs', 'crates/launcher-win/src/cover_cache.rs', 'crates/launcher-win/src/purge.rs') },
    @{ Id = 'INV-09'; Pattern = '\bOpenOptions\b';                  Allow = @('crates/launcher-win/src/persistence/json.rs', 'crates/launcher-win/src/logging.rs', 'crates/launcher-win/src/cover_cache.rs', 'crates/launcher-win/src/purge.rs') },
    @{ Id = 'INV-09'; Pattern = '\bcreate_dir(_all)?\b';            Allow = @('crates/launcher-win/src/persistence/json.rs', 'crates/launcher-win/src/logging.rs', 'crates/launcher-win/src/cover_cache.rs', 'crates/launcher-win/src/purge.rs') },
    @{ Id = 'INV-09'; Pattern = '\bfs::copy\b';                     Allow = @('crates/launcher-win/src/persistence/json.rs', 'crates/launcher-win/src/cover_cache.rs') },
    @{ Id = 'INV-09'; Pattern = '\bCreateFileW\b';                  Allow = @('crates/launcher-win/src/purge.rs') },
    @{ Id = 'INV-09'; Pattern = '\bSetFileInformationByHandle\b';   Allow = @('crates/launcher-win/src/purge.rs') },
    # INV-05 / SEC-15: encerrar processo so no dono (por handle e tempo de criacao)
    @{ Id = 'INV-05'; Pattern = '\bTerminateProcess\b';             Allow = @('crates/launcher-win/src/process_probe.rs') },
    # SEC-10 / AF-01: iniciar processo so no GameStarter; o verbo runas nunca existe
    @{ Id = 'SEC-10'; Pattern = '\bCreateProcess(W|A|AsUserW|WithLogonW|WithTokenW)\b'; Allow = @('crates/launcher-win/src/game_starter.rs') },
    @{ Id = 'SEC-10'; Pattern = '\bShellExecute(Ex)?[AW]\b';        Allow = @('crates/launcher-win/src/game_starter.rs') },
    @{ Id = 'SEC-10'; Pattern = '"runas"|w!\("runas"\)';            Allow = @() },
    # O `sofa_test_game` (binario so dos testes, fora do instalador) pode gravar arquivo e iniciar o filho de teste.
    @{ Id = 'SEC-10'; Pattern = '\bstd::process::Command\b|\bCommand::new\b'; Allow = @('crates/launcher-win/src/bin/sofa_test_game.rs') },
    # SEC-29: escrita no registro so no autostart (Run e StartupApproved)
    @{ Id = 'SEC-29'; Pattern = '\bReg(SetValueEx|DeleteValue|CreateKeyEx|DeleteKey(Ex)?|SetKeyValue|DeleteKeyValue)[AW]\b'; Allow = @('crates/launcher-win/src/autostart/run_key.rs') },
    # INV-03 / SEC-22: so o AutostartRegistry registra tarefa e toca Run e StartupApproved; nunca elevada,
    # com senha, em nome de outra conta nem por schtasks.exe
    @{ Id = 'INV-03'; Pattern = '\bRegisterTaskDefinition\b';       Allow = @('crates/launcher-win/src/autostart/task_scheduler.rs') },
    @{ Id = 'INV-03'; Pattern = 'CurrentVersion\\{1,2}Run\b';       Allow = @('crates/launcher-win/src/autostart/run_key.rs') },
    @{ Id = 'INV-03'; Pattern = '\bStartupApproved\b';              Allow = @('crates/launcher-win/src/autostart/run_key.rs') },
    @{ Id = 'SEC-22'; Pattern = '\bTASK_RUNLEVEL_HIGHEST\b';        Allow = @() },
    @{ Id = 'SEC-22'; Pattern = '\bTASK_LOGON_(S4U|PASSWORD|SERVICE_ACCOUNT|GROUP)\b'; Allow = @() },
    @{ Id = 'SEC-22'; Pattern = 'schtasks';                         Allow = @() },
    # SEC-27 / AF-04: energia e privilegio de desligar so no adapter de energia
    @{ Id = 'SEC-27'; Pattern = '\bExitWindowsEx\b';                Allow = @('crates/launcher-win/src/power.rs') },
    @{ Id = 'SEC-27'; Pattern = '\bSetSuspendState\b';              Allow = @('crates/launcher-win/src/power.rs') },
    @{ Id = 'SEC-27'; Pattern = '\bAdjustTokenPrivileges\b';        Allow = @('crates/launcher-win/src/power.rs') }
)

# uiAccess="true" nunca, em manifesto nenhum (SEC-20, SEC-28)
$script:ManifestRule = @{ Id = 'SEC-28'; Pattern = 'uiAccess\s*=\s*["'']true["'']' }

# SEC-32: executavel externo do instalador so por caminho absoluto de pasta que o instalador
# controla: `$SYSDIR\` (sistema), `$PLUGINSDIR\` (pasta temporaria exclusiva do instalador, onde vai
# o bootstrapper do WebView2) ou `$INSTDIR\` (o desinstalador da versao anterior). Nome relativo,
# `$TEMP\` e executavel lido de variavel continuam proibidos. O caminho tem que ser o proprio
# executavel (logo depois do comando, com aspas opcionais), nao um argumento qualquer da linha.
$script:NsisPattern = '\b(nsExec::\w+|ExecWait|ExecShell\w*|Exec)\b'
$script:NsisGoodPattern = '\b(nsExec::\w+|ExecWait|ExecShell\w*|Exec)\s+(/\w+(=\S+)?\s+)*[''"`]{0,3}\$(SYSDIR|PLUGINSDIR|INSTDIR)[\\/]'

# SEC-31 / SEC-32 (DR-06, SG-09): o plugin `System` chama a API do Windows direto e escapa do padrao
# acima. Iniciar processo, abrir shell, apagar e mover arquivo por `System::Call` so no `.nsh` do
# desinstalador revisado pela seguranca (BE-38); em qualquer outro arquivo, nunca. O padrao pega
# tambem `dll::Funcao` fora de um `System::Call` (por exemplo num `!define`).
$script:NsisApis = 'CreateProcess\w*|ShellExecute\w*|DeleteFile\w*|RemoveDirectory\w*|SHFileOperation\w*|MoveFile\w*'
$script:NsisSystemCallPattern = '(\bSystem::Call\b.*?|::\s*)\b(' + $script:NsisApis + ')\b'
$script:NsisSystemCallOwners = @('src-tauri/windows/uninstall-hooks.nsh')

# SEC-25 / SEC-31: `Delete` e `RMDir` do NSIS nunca sobre a pasta de dados do usuario nem sob
# `$LOCALAPPDATA` / `$APPDATA` (nem pela variavel que a guarde): quem apaga os dados e o `--purge-user-data`
# (token do usuario) ou o fallback por handle do ADR-0006 6b. Vale tambem para o dono acima.
$script:NsisDataDeletePattern = '^\s*(Delete|RMDir)\b.*(\$\{?\w*(DATA|PROFILE)\w*\}?|%(LOCAL)?APPDATA%|%USERPROFILE%|app\.sofa\.launcher)'

function ConvertTo-Rel([string]$Root, [string]$Path) {
    $full = (Resolve-Path -LiteralPath $Path).Path
    $base = (Resolve-Path -LiteralPath $Root).Path.TrimEnd('\')
    return $full.Substring($base.Length + 1).Replace('\', '/')
}

function Get-SourceFiles([string]$Root, [string[]]$Include) {
    $dirs = @('crates', 'src-tauri') | ForEach-Object { Join-Path $Root $_ } | Where-Object { Test-Path -LiteralPath $_ }
    foreach ($d in $dirs) {
        # `-Include` com `-LiteralPath` e ignorado no Windows PowerShell 5.1 (devolvia todos os arquivos
        # para todas as regras); o filtro por nome abaixo garante que `*.rs` so olhe `.rs`, etc.
        Get-ChildItem -LiteralPath $d -Recurse -File -Include $Include -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -notmatch '[\\/](target|node_modules|gen|tests|fixtures)[\\/]' } |
            Where-Object { $name = $_.Name; @($Include | Where-Object { $name -like $_ }).Count -gt 0 }
    }
}

function Test-CommentLine([string]$Line, [string]$Kind) {
    $t = $Line.TrimStart()
    if ($Kind -eq 'rs') { return $t.StartsWith('//') }
    if ($Kind -eq 'nsis') { return ($t.StartsWith(';') -or $t.StartsWith('#')) }
    return $false
}

# Indice da ultima linha do item marcado por `#[cfg(test)]` na linha $Start (indentacao $Indent), ou -1
# se nao achar. Item de uma linha (`...;` ou `{ ... }`) termina nela; item com corpo termina na
# primeira linha que e so `}` na mesma indentacao do marcador (rustfmt garante isso).
function Get-CfgTestItemEnd([string[]]$Lines, [int]$Start, [string]$Indent) {
    for ($j = $Start + 1; $j -lt $Lines.Count; $j++) {
        $l = $Lines[$j]
        if ($l -match '^\s*#\[') { continue }          # outros atributos do mesmo item
        if ($l.Trim().Length -eq 0) { continue }
        $open = ([regex]::Matches($l, '\{')).Count
        $close = ([regex]::Matches($l, '\}')).Count
        if ($open -eq 0 -and $l -match ';\s*$') { return $j }
        if ($open -gt 0 -and $open -eq $close) { return $j }
        if ($open -gt 0) {
            for ($k = $j + 1; $k -lt $Lines.Count; $k++) {
                if ($Lines[$k] -ceq ($Indent + '}')) { return $k }
            }
            return -1
        }
    }
    return -1
}

# Regras de NSIS (SEC-31, SEC-32) sobre as linhas de UM arquivo. `$Rel` e o caminho relativo ao repo
# (a lista de donos do System::Call usa esse caminho). Vale para os `.nsi`/`.nsh` do repo e para o
# `installer.nsi` gerado pelo Tauri (BE-39: scripts/ci.ps1 -Release).
function Get-NsisViolations([string]$Rel, [string[]]$Lines) {
    $out = New-Object System.Collections.Generic.List[string]
    for ($i = 0; $i -lt $Lines.Count; $i++) {
        if (Test-CommentLine $Lines[$i] 'nsis') { continue }
        if ($Lines[$i] -match $script:NsisPattern -and $Lines[$i] -notmatch $script:NsisGoodPattern) {
            $out.Add(('SEC-32 {0}:{1} executavel externo sem caminho absoluto ($SYSDIR\, $PLUGINSDIR\ ou $INSTDIR\) ({2})' -f $Rel, ($i + 1), $Matches[0]))
        }
        if ($Lines[$i] -match $script:NsisSystemCallPattern -and ($script:NsisSystemCallOwners -notcontains $Rel)) {
            $out.Add(('SEC-32 {0}:{1} System::Call com `{2}` fora do .nsh do desinstalador revisado' -f $Rel, ($i + 1), $Matches[2]))
        }
        if ($Lines[$i] -match $script:NsisDataDeletePattern) {
            $out.Add(('SEC-31 {0}:{1} `{2}` do NSIS sobre a pasta de dados do usuario (use o purge do usuario)' -f $Rel, ($i + 1), $Matches[1]))
        }
    }
    return $out.ToArray()
}

# Script NSIS gerado pelo Tauri (target/<triple>/release/nsis/<arch>/installer.nsi): mesmas regras.
function Invoke-GeneratedNsisGuard([string]$Path) {
    $lines = @(Get-Content -LiteralPath $Path -Encoding UTF8)
    return @(Get-NsisViolations ('generated/' + (Split-Path -Leaf $Path)) $lines)
}

# Devolve uma lista de textos "REGRA arquivo:linha padrao". Lista vazia = limpo.
function Invoke-Guards([string]$Root) {
    $violations = New-Object System.Collections.Generic.List[string]

    foreach ($f in @(Get-SourceFiles $Root @('*.rs'))) {
        $rel = ConvertTo-Rel $Root $f.FullName
        $lines = @(Get-Content -LiteralPath $f.FullName -Encoding UTF8)
        $skipUntil = -1
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($i -le $skipUntil) { continue }
            # `#[cfg(test)]` marca so o item que vem a seguir (ARQ-BUG-06): pula esse item e segue
            # varrendo o resto, para codigo de producao depois dele nao ficar fora do grep.
            if ($lines[$i] -match '^(\s*)#\[cfg\(test\)\]') {
                $end = Get-CfgTestItemEnd $lines $i $Matches[1]
                if ($end -lt 0) {
                    $violations.Add(('CFG-TEST {0}:{1} fim do item #[cfg(test)] nao encontrado; arquivo varrido por inteiro' -f $rel, ($i + 1)))
                } else {
                    $skipUntil = $end
                }
                continue
            }
            if (Test-CommentLine $lines[$i] 'rs') { continue }
            foreach ($rule in $script:GuardRules) {
                if ($lines[$i] -match $rule.Pattern -and ($rule.Allow -notcontains $rel)) {
                    $violations.Add(('{0} {1}:{2} `{3}` fora do arquivo dono' -f $rule.Id, $rel, ($i + 1), $Matches[0]))
                }
            }
        }
    }

    foreach ($f in @(Get-SourceFiles $Root @('*.manifest', '*.xml', '*.rc'))) {
        $rel = ConvertTo-Rel $Root $f.FullName
        $text = Get-Content -LiteralPath $f.FullName -Raw -Encoding UTF8
        if ($text -match $script:ManifestRule.Pattern) {
            $violations.Add(('{0} {1} uiAccess="true" e proibido' -f $script:ManifestRule.Id, $rel))
        }
    }

    foreach ($f in @(Get-SourceFiles $Root @('*.nsi', '*.nsh'))) {
        $rel = ConvertTo-Rel $Root $f.FullName
        $lines = @(Get-Content -LiteralPath $f.FullName -Encoding UTF8)
        foreach ($v in @(Get-NsisViolations $rel $lines)) { $violations.Add($v) }
    }
    # Sem a virgula: a lista e desenrolada na saida e o chamador usa @(...) para contar.
    return $violations.ToArray()
}

# Autoteste (D-34): arvore com uma violacao plantada de cada regra deve falhar; arvore limpa, passar.
function Invoke-GuardSelfTest {
    $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("sofa-guards-" + [guid]::NewGuid().ToString('N'))
    try {
        $win = Join-Path $tmp 'crates/launcher-win/src'
        New-Item -ItemType Directory -Force -Path $win | Out-Null
        New-Item -ItemType Directory -Force -Path (Join-Path $tmp 'src-tauri') | Out-Null

        # Limpo: donos podem usar o que e deles; comentarios e Allow* nao contam.
        Set-Content -LiteralPath (Join-Path $win 'xinput.rs') -Value 'fn a() { XInputGetState(0, &mut s); }'
        Set-Content -LiteralPath (Join-Path $win 'foreground.rs') -Value @('// SendInput so aqui', 'fn b() { SetForegroundWindow(h); AllowSetForegroundWindow(0); }')
        Set-Content -LiteralPath (Join-Path $win 'clean.rs') -Value @('// EWX_FORCE nao e usado', 'fn c() { EWX_POWEROFF; }', '#[cfg(test)]', 'mod tests { fn t() { remove_dir_all(p); } }')
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/ok.manifest') -Value '<requestedExecutionLevel level="asInvoker" uiAccess="false"/>'
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/ok.nsh') -Value @('; ExecWait sem SYSDIR em comentario', 'nsExec::Exec ''"$SYSDIR\schtasks.exe" /?''',
            'ExecWait ''"$PLUGINSDIR\web.exe" /silent /install'' $1',
            'ExecWait ''"$INSTDIR\uninstall.exe" /UPDATE _?=$INSTDIR'' $0')
        # NSIS: o dono revisado pode chamar API pelo System; Delete e RMDir fora da pasta de dados
        # e comentario sobre a pasta de dados passam em qualquer arquivo.
        New-Item -ItemType Directory -Force -Path (Join-Path $tmp 'src-tauri/windows') | Out-Null
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/windows/uninstall-hooks.nsh') -Value @(
            'System::Call ''kernel32::CreateProcessW(w r1, w r2, p 0, p 0, i 0, i 0, p 0, p 0, p r3, p r4) i .r0''',
            'System::Call ''kernel32::WaitForSingleObject(p r1, i 60000)''',
            'Delete "$INSTDIR\sofa.exe"',
            'RMDir /r "$INSTDIR"'
        )
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/ok-system.nsh') -Value @(
            '; System::Call kernel32::DeleteFileW so em comentario; RMDir /r "$LOCALAPPDATA\app.sofa.launcher" idem',
            'System::Call ''kernel32::GetExitCodeProcess(p r1, *i .r2)''',
            'Delete "$SMPROGRAMS\Sofa.lnk"',
            'rmdir "$INSTDIR\bin"'
        )
        $clean = @(Invoke-Guards $tmp)
        if ($clean.Count -ne 0) { throw ("autoteste: arvore limpa acusou violacao: " + ($clean -join '; ')) }

        # Plantadas: uma por regra (o nome do token entra no texto da violacao).
        $planted = @(
            'XInputGetState', 'SetForegroundWindow', 'SendInput', 'AttachThreadInput', 'SetWinEventHook',
            'WINEVENT_INCONTEXT', 'SPI_SETFOREGROUNDLOCKTIMEOUT', 'EWX_FORCEIFHUNG', 'SeDebugPrivilege',
            'PROCESS_ALL_ACCESS', 'shutdown.exe', 'TerminateJobObject', 'SetWindowsHookExW', 'CreateRemoteThread',
            'ReadProcessMemory', 'WriteProcessMemory', 'LoadLibraryW', 'remove_file', 'remove_dir_all',
            'DeleteFileW', 'fs::rename', 'MoveFileExW',
            'RegisterTaskDefinition', 'CurrentVersion\Run', 'StartupApproved', 'TASK_RUNLEVEL_HIGHEST',
            'TASK_LOGON_S4U', 'schtasks', 'ExitWindowsEx', 'SetSuspendState', 'AdjustTokenPrivileges',
            'fs::write', 'File::create', 'OpenOptions', 'create_dir_all', 'fs::copy', 'CreateFileW',
            'SetFileInformationByHandle', 'TerminateProcess', 'CreateProcessW', 'ShellExecuteExW',
            '"runas"', 'Command::new', 'RegSetValueExW', 'RegDeleteValueW'
        )
        # ARQ-BUG-06: `#[cfg(test)]` no meio do arquivo pula so aquele item; o codigo de producao
        # depois dele continua varrido. O corpo do modulo de teste nao conta.
        Set-Content -LiteralPath (Join-Path $win 'midtest.rs') -Value @(
            'fn before() {}',
            '#[cfg(test)]',
            'pub(crate) mod fake {',
            '    fn t() { remove_dir_all(p); }',
            '}',
            '',
            'fn after() { DeleteFileW(p); }',
            '#[cfg(test)]',
            'mod tests {',
            '    fn t2() { remove_file(p); }',
            '}'
        )
        Set-Content -LiteralPath (Join-Path $win 'rogue.rs') -Value ($planted | ForEach-Object { "fn x() { $_(); }" })
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/bad.manifest') -Value '<requestedExecutionLevel level="asInvoker" uiAccess="true"/>'
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/bad.nsh') -Value @('nsExec::Exec "schtasks.exe /delete"', 'ExecWait "cmd /c del x"', 'nsExec::Exec ''"cmd.exe" /c $SYSDIR\x.exe''', 'ExecWait ''"$TEMP\web.exe" /install'' $1', 'ExecWait ''"$R1" /x'' $0')
        # NSIS: System::Call fora do dono (uma linha por API) e Delete/RMDir sobre a pasta de dados.
        $nsisApis = @('CreateProcessW', 'ShellExecuteW', 'DeleteFileW', 'RemoveDirectoryW', 'SHFileOperationW', 'MoveFileExW', 'DeleteFileA')
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/bad-system.nsh') -Value @(
            'System::Call ''kernel32::CreateProcessW(w r1, w r2, p 0, p 0, i 0, i 0, p 0, p 0, p r3, p r4) i .r0''',
            'System::Call "shell32::ShellExecuteW(p 0, t ''open'', t ''x'', t '''', t '''', i 1)"',
            'System::Call ''kernel32::DeleteFileW(w "$INSTDIR\x")''',
            'System::Call ''kernel32::RemoveDirectoryW(w "$INSTDIR\x")''',
            'system::call ''shell32::SHFileOperationW(p r1)''',
            'System::Call ''kernel32::MoveFileExW(w "a", w "b", i 4)''',
            '!define FN "kernel32::DeleteFileA"'
        )
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/bad-data.nsh') -Value @(
            'RMDir /r "$LOCALAPPDATA\app.sofa.launcher"',
            'Delete "$APPDATA\app.sofa.launcher\library.json"',
            'RMDir /r $DataDir',
            'rmdir /r "%LOCALAPPDATA%\app.sofa.launcher"'
        )
        # O dono revisado nao pode apagar a pasta de dados pelo NSIS (a regra vale para ele tambem).
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/windows/uninstall-hooks.nsh') -Value @(
            'System::Call ''kernel32::CreateProcessW(w r1, w r2, p 0, p 0, i 0, i 0, p 0, p 0, p r3, p r4) i .r0''',
            'RMDir /r "$LOCALAPPDATA\app.sofa.launcher"'
        )
        $found = @(Invoke-Guards $tmp)
        $text = $found -join "`n"
        foreach ($token in $planted) {
            if ($text -notmatch [regex]::Escape($token)) { throw "autoteste: violacao plantada nao detectada: $token" }
        }
        foreach ($token in @('uiAccess="true"', 'nsExec::Exec', 'ExecWait')) {
            if ($text -notmatch [regex]::Escape($token)) { throw "autoteste: violacao plantada nao detectada: $token" }
        }
        if ($text -notmatch 'midtest\.rs:7 `DeleteFileW`') { throw 'autoteste: codigo de producao depois de #[cfg(test)] no meio do arquivo nao foi varrido' }
        if ($text -match 'midtest\.rs:(4|10) ') { throw 'autoteste: item #[cfg(test)] foi varrido (deveria ser pulado)' }
        if ($text -notmatch 'bad\.nsh:3 ') { throw 'autoteste: $SYSDIR como argumento (nao como executavel) nao foi acusado' }
        if ($text -notmatch 'bad\.nsh:4 ') { throw 'autoteste: executavel em $TEMP nao foi acusado' }
        if ($text -notmatch 'bad\.nsh:5 ') { throw 'autoteste: executavel lido de variavel nao foi acusado' }
        foreach ($api in $nsisApis) {
            if ($text -notmatch ('bad-system\.nsh:\d+ System::Call com `' + $api + '`')) { throw "autoteste: System::Call com $api fora do dono nao foi acusado" }
        }
        if ($text -notmatch 'bad-system\.nsh:7 System::Call com `DeleteFileA`') { throw 'autoteste: dll::Funcao fora de System::Call nao foi acusada' }
        foreach ($n in 1..4) {
            if ($text -notmatch ('SEC-31 src-tauri/bad-data\.nsh:' + $n + ' ')) { throw "autoteste: Delete/RMDir sobre a pasta de dados (linha $n) nao foi acusado" }
        }
        if ($text -notmatch 'SEC-31 src-tauri/windows/uninstall-hooks\.nsh:2 ') { throw 'autoteste: o dono do System::Call tambem nao pode apagar a pasta de dados pelo NSIS' }
        if ($text -match 'uninstall-hooks\.nsh:1 ') { throw 'autoteste: dono do System::Call acusado indevidamente' }
        if ($text -match 'ok-system\.nsh') { throw 'autoteste: arquivo NSIS limpo acusado' }
        if ($text -match 'ok\.nsh') { throw 'autoteste: $PLUGINSDIR e $INSTDIR como executavel foram acusados indevidamente' }
        # Os donos nao podem aparecer entre as violacoes.
        foreach ($owner in @('xinput.rs', 'foreground.rs')) {
            if ($text -match ($owner + ':\d+ `(XInputGetState|SetForegroundWindow)`')) { throw "autoteste: dono acusado indevidamente: $owner" }
        }
        return $found.Count
    }
    finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}
