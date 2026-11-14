# Guards de arquitetura e seguranca por grep (BE-12, D-34). Compativel com Windows PowerShell 5.1.
#
# Cada regra diz: padrao proibido, arquivos "donos" onde ele pode aparecer, e o INV/SEC que protege.
# Varre `crates/**/src/**/*.rs` e `src-tauri/src/**/*.rs` (nao varre `tests/`), pulando linhas de
# comentario e o modulo `#[cfg(test)]` do fim do arquivo. Manifestos (`.manifest`, `.xml`, `.rc`) e scripts NSIS (`.nsi`, `.nsh`) tem regras
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

# SEC-32: executavel externo do instalador so por $SYSDIR\
$script:NsisPattern = '\b(nsExec::\w+|ExecWait|ExecShell\w*|Exec)\b'

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

# Devolve uma lista de textos "REGRA arquivo:linha padrao". Lista vazia = limpo.
function Invoke-Guards([string]$Root) {
    $violations = New-Object System.Collections.Generic.List[string]

    foreach ($f in @(Get-SourceFiles $Root @('*.rs'))) {
        $rel = ConvertTo-Rel $Root $f.FullName
        $lines = @(Get-Content -LiteralPath $f.FullName -Encoding UTF8)
        for ($i = 0; $i -lt $lines.Count; $i++) {
            # Convencao do repo: o modulo `#[cfg(test)]` fica no fim do arquivo e nao e codigo de producao.
            if ($lines[$i] -match '^\s*#\[cfg\(test\)\]') { break }
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
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if (Test-CommentLine $lines[$i] 'nsis') { continue }
            if ($lines[$i] -match $script:NsisPattern -and $lines[$i] -notmatch '\$SYSDIR[\\/]') {
                $violations.Add(('SEC-32 {0}:{1} executavel externo sem $SYSDIR\ ({2})' -f $rel, ($i + 1), $Matches[0]))
            }
        }
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
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/ok.nsh') -Value @('; ExecWait sem SYSDIR em comentario', 'nsExec::Exec ''"$SYSDIR\schtasks.exe" /?''')
        $clean = @(Invoke-Guards $tmp)
        if ($clean.Count -ne 0) { throw ("autoteste: arvore limpa acusou violacao: " + ($clean -join '; ')) }

        # Plantadas: uma por regra (o nome do token entra no texto da violacao).
        $planted = @(
            'XInputGetState', 'SetForegroundWindow', 'SendInput', 'AttachThreadInput', 'SetWinEventHook',
            'WINEVENT_INCONTEXT', 'SPI_SETFOREGROUNDLOCKTIMEOUT', 'EWX_FORCEIFHUNG', 'SeDebugPrivilege',
            'PROCESS_ALL_ACCESS', 'shutdown.exe', 'SetWindowsHookExW', 'CreateRemoteThread',
            'ReadProcessMemory', 'WriteProcessMemory', 'LoadLibraryW', 'remove_file', 'remove_dir_all',
            'DeleteFileW', 'fs::rename', 'MoveFileExW',
            'RegisterTaskDefinition', 'CurrentVersion\Run', 'StartupApproved', 'TASK_RUNLEVEL_HIGHEST',
            'TASK_LOGON_S4U', 'schtasks', 'ExitWindowsEx', 'SetSuspendState', 'AdjustTokenPrivileges'
        )
        Set-Content -LiteralPath (Join-Path $win 'rogue.rs') -Value ($planted | ForEach-Object { "fn x() { $_(); }" })
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/bad.manifest') -Value '<requestedExecutionLevel level="asInvoker" uiAccess="true"/>'
        Set-Content -LiteralPath (Join-Path $tmp 'src-tauri/bad.nsh') -Value @('nsExec::Exec "schtasks.exe /delete"', 'ExecWait "cmd /c del x"')
        $found = @(Invoke-Guards $tmp)
        $text = $found -join "`n"
        foreach ($token in $planted) {
            if ($text -notmatch [regex]::Escape($token)) { throw "autoteste: violacao plantada nao detectada: $token" }
        }
        foreach ($token in @('uiAccess="true"', 'nsExec::Exec', 'ExecWait')) {
            if ($text -notmatch [regex]::Escape($token)) { throw "autoteste: violacao plantada nao detectada: $token" }
        }
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
