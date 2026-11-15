<#
  Release do Sofa (BE-39, D-34, SEC-24, SEC-26): chamado por `scripts/ci.ps1 -Release`.

  O release sai em MSVC (ADR-0001, P-08 b): e o unico alvo em que o loader do WebView2 e estatico
  (SEC-23) e o `sofa.exe` tem exatamente um RT_MANIFEST (ARQ-BUG-08). O CI de dev segue no `-gnu`.

  Passos, nesta ordem (qualquer falha para tudo e nao deixa instalador na pasta de releases):
    1. toolchain MSVC na versao fixada em rust-toolchain.toml (SEC-24)
    2. cargo test no MSVC (pula com -SkipMsvcTests; -FullMsvcTests inclui os testes do jogo de teste, que o Defender barra)
    3. relatorios: cargo deny e npm audit de producao (anexados ao release)
    4. `tauri build --target x86_64-pc-windows-msvc --bundles nsis` (NSIS perMachine, bootstrapper do WebView2)
    5. TRAVA ARQ-BUG-08: o sofa.exe MSVC com exatamente um RT_MANIFEST valido (tests/manifest_resource.rs)
    6. guards de NSIS no installer.nsi gerado
    7. testes do instalador e do desinstalador sem instalar (scripts/test-installer.ps1)
    8. copia o instalador para <releases>\v<versao>\, gera SHA256SUMS.txt, README.txt e BUILD-INFO.txt
       e confere o hash recalculando
#>

function Get-PinnedRustChannel([string]$Root) {
    $toml = Get-Content -LiteralPath (Join-Path $Root 'rust-toolchain.toml') -Raw
    if ($toml -notmatch 'channel\s*=\s*"([^"]+)"') { throw 'rust-toolchain.toml sem channel' }
    return $Matches[1]
}

# Nome da toolchain MSVC que respeita o pin. Usa `<pin>-x86_64-pc-windows-msvc` se instalada; senao a
# `stable-x86_64-pc-windows-msvc` SE ela for exatamente a versao do pin; senao falha com instrucao.
function Resolve-MsvcToolchain([string]$Root) {
    $pin = Get-PinnedRustChannel $Root
    $installed = @(rustup toolchain list | ForEach-Object { ($_ -split '\s+')[0] })
    $exact = "$pin-x86_64-pc-windows-msvc"
    if ($installed -contains $exact) { return $exact }
    $stable = 'stable-x86_64-pc-windows-msvc'
    if ($installed -contains $stable) {
        $v = (& rustc "+$stable" --version) -replace '^rustc (\S+).*$', '$1'
        if ($v -eq $pin) { return $stable }
        throw "a toolchain $stable e a $v e o pin e $pin (SEC-24). Rode: rustup toolchain install $exact --profile minimal"
    }
    throw "toolchain MSVC ausente. Rode: rustup toolchain install $exact --profile minimal"
}

function Get-ReleaseVersion([string]$Root) {
    $conf = Get-Content -LiteralPath (Join-Path $Root 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
    return $conf.version
}

function Invoke-Release {
    param(
        [Parameter(Mandatory)][string]$Root,
        [string]$ReleasesDir,
        [switch]$SkipMsvcTests,
        [switch]$FullMsvcTests,
        [switch]$SkipAudit,
        [switch]$AllowUntagged,
        [Parameter(Mandatory)][scriptblock]$Step
    )
    $target = 'x86_64-pc-windows-msvc'
    $version = Get-ReleaseVersion $Root
    if (-not $ReleasesDir) { $ReleasesDir = Join-Path (Split-Path -Parent $Root) 'sofa-releases' }
    $outDir = Join-Path $ReleasesDir ("v{0}" -f $version)
    $reports = Join-Path $outDir 'relatorios'
    $targetDir = Join-Path $Root "target\$target\release"
    $exe = Join-Path $targetDir 'sofa.exe'
    $bundleDir = Join-Path $targetDir 'bundle\nsis'
    $installerName = "Sofa_${version}_x64-setup.exe"

    # build a partir de tag (RN-30): HEAD exatamente numa tag v<versao> e arvore limpa.
    # via cmd: no PowerShell 5.1 o stderr de um nativo vira erro terminante com `Stop`, e "sem tag" e um caso normal
    $tag = (cmd /c "git describe --exact-match --tags HEAD 2>nul")
    $global:LASTEXITCODE = 0
    # so conta o que entra no binario e no instalador (docs e planilhas de tarefas nao mudam o hash)
    $dirty = @(git status --porcelain --untracked-files=no -- crates src-tauri ui scripts Cargo.toml Cargo.lock rust-toolchain.toml deny.toml)
    $commit = (git rev-parse HEAD).Trim()
    if (-not $tag -or $tag.Trim() -ne "v$version") {
        if (-not $AllowUntagged) { throw "release a partir de tag: HEAD nao esta na tag v$version. Crie a tag ou passe -AllowUntagged (build de teste, marcado como tal em BUILD-INFO.txt)" }
        Write-Host "AVISO: build SEM tag v$version (-AllowUntagged); fica registrado em BUILD-INFO.txt" -ForegroundColor Yellow
    }
    if ($dirty.Count -gt 0 -and -not $AllowUntagged) { throw "arvore com alteracoes nao commitadas; release so de arvore limpa" }

    $tc = Resolve-MsvcToolchain $Root
    Write-Host ("Toolchain do release: {0}" -f $tc) -ForegroundColor Cyan
    $oldTc = $env:RUSTUP_TOOLCHAIN
    $env:RUSTUP_TOOLCHAIN = $tc
    try {
        # a pasta so existe se tudo passar: monta num temporario e move no fim
        $stage = Join-Path $ReleasesDir (".stage-v{0}-{1}" -f $version, [guid]::NewGuid().ToString('N').Substring(0, 8))
        New-Item -ItemType Directory -Force -Path (Join-Path $stage 'relatorios') | Out-Null

        if (-not $SkipMsvcTests) {
            if ($FullMsvcTests) {
                & $Step 'cargo test --workspace (MSVC, completo)' { cargo test --workspace --target $target }
            }
            else {
                # Padrao: sem os testes que sobem o `sofa-test-game.exe` (session_e2e e session_runtime). O Defender
                # classifica esse helper de teste, compilado no MSVC, como `Trojan:Win32/Bearfoos.A!ml` (falso positivo
                # de aprendizado de maquina) e bloqueia a execucao (erro 225). O instalador e o sofa.exe de release
                # passam no Defender (conferido com `MpCmdRun -Scan -File`). `-FullMsvcTests` roda tudo.
                & $Step 'cargo test (MSVC): launcher-core, sofa e launcher-win sem os testes do jogo de teste' {
                    cargo test -p launcher-core -p sofa --target $target
                    if ($LASTEXITCODE -ne 0) { return }
                    cargo test -p launcher-win --target $target --lib --test library_integration --test qa_robustez
                }
            }
        }

        & $Step 'relatorio: cargo deny' {
            $txt = Join-Path $stage 'relatorios\cargo-deny.txt'
            cargo deny --log-level warn check --hide-inclusion-graph 2>&1 | Tee-Object -FilePath $txt | Out-Host
            if ($LASTEXITCODE -ne 0) { throw 'cargo deny reprovou (veja o relatorio)' }
        }
        if ($SkipAudit) {
            Write-Host 'npm audit: PULADO (-SkipAudit); registre o motivo' -ForegroundColor Yellow
            Set-Content -LiteralPath (Join-Path $stage 'relatorios\npm-audit.txt') -Value 'npm audit pulado (-SkipAudit) neste build.'
        } else {
            & $Step 'relatorio: npm audit (producao)' {
                Push-Location (Join-Path $Root 'ui')
                try {
                    $txt = Join-Path $stage 'relatorios\npm-audit.txt'
                    npm audit --omit=dev --audit-level=high 2>&1 | Tee-Object -FilePath $txt | Out-Host
                    if ($LASTEXITCODE -ne 0) { throw 'npm audit com achado alto ou critico (veja o relatorio)' }
                } finally { Pop-Location }
            }
        }

        & $Step 'tauri build (MSVC, NSIS)' {
            Push-Location $Root
            try { & (Join-Path $Root 'ui\node_modules\.bin\tauri.cmd') build --target $target --bundles nsis } finally { Pop-Location }
        }

        & $Step 'trava ARQ-BUG-08: exatamente um RT_MANIFEST no sofa.exe MSVC' {
            if (-not (Test-Path -LiteralPath $exe)) { throw "sofa.exe do release nao encontrado em $exe" }
            $env:SOFA_MANIFEST_EXE = $exe
            try { cargo test --target $target -p sofa --test manifest_resource } finally { Remove-Item Env:\SOFA_MANIFEST_EXE -ErrorAction SilentlyContinue }
        }

        & $Step 'guards de NSIS no installer.nsi gerado' {
            $gen = Join-Path $targetDir 'nsis\x64\installer.nsi'
            if (-not (Test-Path -LiteralPath $gen)) { throw "installer.nsi gerado nao encontrado em $gen" }
            $v = @(Invoke-GeneratedNsisGuard $gen)
            if ($v.Count -gt 0) { $v | ForEach-Object { Write-Host ("  " + $_) -ForegroundColor Red }; throw ("{0} violacao(oes)" -f $v.Count) }
            Write-Host '  nenhuma violacao'
        }

        & $Step 'testes do instalador e do desinstalador (sem instalar)' {
            & (Join-Path $PSScriptRoot 'test-installer.ps1')
            if ($LASTEXITCODE -ne 0) { throw 'test-installer.ps1 reprovou' }
        }

        & $Step 'SHA256SUMS e pasta de release' {
            $built = Join-Path $bundleDir $installerName
            if (-not (Test-Path -LiteralPath $built)) { throw "instalador nao encontrado: $built" }
            Copy-Item -LiteralPath $built -Destination (Join-Path $stage $installerName)
            $hash = (Get-FileHash -LiteralPath (Join-Path $stage $installerName) -Algorithm SHA256).Hash.ToLowerInvariant()
            # formato do `sha256sum -c` (hash, dois espacos, nome), LF
            [System.IO.File]::WriteAllText((Join-Path $stage 'SHA256SUMS.txt'), ("{0}  {1}`n" -f $hash, $installerName), (New-Object System.Text.UTF8Encoding($false)))

            $readme = @(
                "Sofa $version - instalador para Windows 10 e 11 (64 bits)",
                "",
                "Arquivo:  $installerName",
                "SHA-256:  $hash",
                "",
                "O instalador NAO tem assinatura de codigo (RN-30). O Windows pode mostrar o aviso do SmartScreen.",
                "Antes de instalar, confira o hash no PowerShell, na pasta do arquivo:",
                "",
                "    Get-FileHash .\$installerName -Algorithm SHA256",
                "",
                "O valor tem que ser igual ao de SHA256SUMS.txt (e ao SHA-256 acima). Se for diferente, nao instale.",
                "",
                "Instalar pede permissao de administrador (UAC): o Sofa vai para C:\Program Files\Sofa para todo o PC.",
                "A pasta e fixa: nao ha escolha de pasta e a opcao /D= de outra pasta e recusada.",
                "Sem o WebView2 (comum so no Windows 10), o instalador baixa da Microsoft e precisa de internet.",
                "Desinstalar: Configuracoes > Aplicativos instalados > Sofa. A caixa 'Manter dados' vem marcada.",
                "",
                "Relatorios desta versao: pasta 'relatorios' (cargo deny e npm audit). Detalhes do build: BUILD-INFO.txt."
            )
            [System.IO.File]::WriteAllText((Join-Path $stage 'README.txt'), (($readme -join "`r`n") + "`r`n"), (New-Object System.Text.UTF8Encoding($false)))

            $nsisVer = ((& (Join-Path $env:LOCALAPPDATA 'tauri\NSIS\makensis.exe') /VERSION) -join ' ')
            $info = @(
                "versao: $version",
                "commit: $commit",
                ("tag: " + $(if ($tag) { $tag.Trim() } else { '(nenhuma: build de teste, -AllowUntagged)' })),
                ("arvore: " + $(if ($dirty.Count -gt 0) { 'COM alteracoes nao commitadas' } else { 'limpa' })),
                "alvo: $target",
                ("data: " + (Get-Date -Format 'yyyy-MM-dd HH:mm:ss')),
                "toolchain: $tc",
                ((& rustc "+$tc" -vV) -join "`n"),
                ("tauri-cli: " + ((& (Join-Path $Root 'ui\node_modules\.bin\tauri.cmd') --version) -join ' ')),
                "nsis: $nsisVer",
                "sha256: $hash",
                ("tamanho: " + (Get-Item -LiteralPath (Join-Path $stage $installerName)).Length + ' bytes')
            )
            [System.IO.File]::WriteAllText((Join-Path $stage 'BUILD-INFO.txt'), (($info -join "`r`n") + "`r`n"), (New-Object System.Text.UTF8Encoding($false)))

            # confere: o hash publicado bate com o arquivo
            $line = @(Get-Content -LiteralPath (Join-Path $stage 'SHA256SUMS.txt'))[0]
            $pub, $name = $line -split '  ', 2
            $again = (Get-FileHash -LiteralPath (Join-Path $stage $name) -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($pub -ne $again) { throw "SHA256SUMS.txt nao confere com $name" }

            if (Test-Path -LiteralPath $outDir) { Remove-Item -LiteralPath $outDir -Recurse -Force }
            Move-Item -LiteralPath $stage -Destination $outDir
            Write-Host ("  pasta de release: {0}" -f $outDir)
            Write-Host ("  {0}  {1}" -f $pub, $name)
        }
    }
    finally {
        if ($null -eq $oldTc) { Remove-Item Env:\RUSTUP_TOOLCHAIN -ErrorAction SilentlyContinue } else { $env:RUSTUP_TOOLCHAIN = $oldTc }
        if ($stage -and (Test-Path -LiteralPath $stage)) { Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue }
    }
}
