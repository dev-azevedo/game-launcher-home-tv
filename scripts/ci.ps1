<#
.SYNOPSIS
  CI local do Sofa (D-34): roda tudo que precisa passar antes de commitar ou de gerar release.

.DESCRIPTION
  1. autoteste dos guards (uma violacao plantada de cada regra precisa ser detectada)
  2. guards de arquitetura e seguranca por grep (INV-03/05/08/09, SEC-28, SEC-32)
  3. cargo fmt --check
  4. cargo clippy --workspace --all-targets -D warnings
  5. cargo test --workspace (inclui: lista de comandos x capability, manifesto asInvoker,
     CSP de release sem unsafe-inline/unsafe-eval/origem externa, bindings TypeScript)
  6. bindings TS versionados iguais aos gerados (ui/src/shared/ipc/generated)
  7. cargo deny check (SEC-24, SEC-30: sem cliente HTTP, licencas, advisories)
  8. npm audit dos pacotes de producao da UI (precisa de rede; -SkipAudit pula)
  9. npm run check da UI (typecheck, lint, testes; -SkipUi pula)

  Sai com codigo 0 so se tudo passar. Toolchain: o do rust-toolchain.toml.

.PARAMETER SkipAudit
  Nao roda o npm audit (offline). Registre o motivo no commit/release.
.PARAMETER SkipUi
  Nao roda o npm run check da UI (quando so o back mudou e a UI esta em edicao).
.PARAMETER SelfTestOnly
  Roda so o autoteste dos guards.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/ci.ps1
#>
[CmdletBinding()]
param(
    [switch]$SkipAudit,
    [switch]$SkipUi,
    [switch]$SelfTestOnly
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
. (Join-Path $PSScriptRoot 'ci-guards.ps1')

$script:failed = New-Object System.Collections.Generic.List[string]

function Invoke-Step([string]$Name, [scriptblock]$Body) {
    Write-Host ("`n=== {0}" -f $Name) -ForegroundColor Cyan
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    # PS 5.1 trata stderr de programa nativo (cargo imprime progresso la) como erro com `Stop`.
    $ErrorActionPreference = 'Continue'
    $global:LASTEXITCODE = 0
    try {
        & $Body
        if ($LASTEXITCODE -ne 0) { throw ("codigo de saida {0}" -f $LASTEXITCODE) }
        Write-Host ("OK   {0} ({1:n1}s)" -f $Name, $sw.Elapsed.TotalSeconds) -ForegroundColor Green
    }
    catch {
        Write-Host ("FALHOU {0}: {1}" -f $Name, $_.Exception.Message) -ForegroundColor Red
        $script:failed.Add($Name)
    }
    $ErrorActionPreference = 'Stop'
    $global:LASTEXITCODE = 0
}

Invoke-Step 'autoteste dos guards' {
    $n = Invoke-GuardSelfTest
    Write-Host ("  {0} violacoes plantadas detectadas, arvore limpa sem alarme" -f $n)
}

if (-not $SelfTestOnly) {
    Invoke-Step 'guards de arquitetura e seguranca' {
        $v = @(Invoke-Guards $root)
        if ($v.Count -gt 0) {
            $v | ForEach-Object { Write-Host ("  " + $_) -ForegroundColor Red }
            throw ("{0} violacao(oes)" -f $v.Count)
        }
        Write-Host "  nenhuma violacao"
    }

    Invoke-Step 'cargo fmt --check'  { cargo fmt --all -- --check }
    Invoke-Step 'cargo clippy -D warnings' { cargo clippy --workspace --all-targets -- -D warnings }
    Invoke-Step 'cargo test --workspace'   { cargo test --workspace }

    Invoke-Step 'bindings TypeScript atualizados' {
        # `cargo test` acima regenerou ui/src/shared/ipc/generated; o que esta no git tem que ser igual.
        $dirty = @(git status --porcelain -- ui/src/shared/ipc/generated)
        if ($dirty.Count -gt 0) {
            $dirty | ForEach-Object { Write-Host ("  " + $_) -ForegroundColor Red }
            throw 'bindings gerados diferem do versionado: rode `cargo test -p sofa --test bindings` e faca commit da pasta'
        }
        Write-Host "  generated/ igual ao versionado"
    }

    Invoke-Step 'cargo deny check' { cargo deny --log-level error check --hide-inclusion-graph }

    if ($SkipAudit) {
        Write-Host "`n=== npm audit: PULADO (-SkipAudit)" -ForegroundColor Yellow
    }
    else {
        Invoke-Step 'npm audit (producao, nivel high)' {
            Push-Location (Join-Path $root 'ui')
            try { npm audit --omit=dev --audit-level=high } finally { Pop-Location }
        }
    }

    if ($SkipUi) {
        Write-Host "`n=== npm run check (UI): PULADO (-SkipUi)" -ForegroundColor Yellow
    }
    else {
        Invoke-Step 'npm run check (UI)' {
            Push-Location (Join-Path $root 'ui')
            try { npm run check } finally { Pop-Location }
        }
    }
}

Write-Host ""
if ($script:failed.Count -gt 0) {
    Write-Host ("CI FALHOU em: " + ($script:failed -join ', ')) -ForegroundColor Red
    exit 1
}
Write-Host "CI OK" -ForegroundColor Green
exit 0
