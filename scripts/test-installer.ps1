<#
.SYNOPSIS
  Testes do instalador e do desinstalador do Sofa que NAO exigem instalar nada neste PC
  (BE-37, BE-38, BE-42; ADR-0006 rev. 6, 6a e 6b; SEC-25, SEC-31, SEC-32).

.DESCRIPTION
  Compila com o makensis do Tauri (cache em %LOCALAPPDATA%\tauri\NSIS, baixado pelo primeiro
  `tauri build`) o harness `src-tauri/windows/tests/harness.nsi`, que roda as MESMAS funcoes de
  `src-tauri/windows/uninstall-hooks.nsh` como um instalador comum, e um stub de `sofa.exe`.
  Tudo roda SEM elevacao e em pastas temporarias criadas aqui (dentro de %LOCALAPPDATA%\Temp):

    - SID: tabela de validos e invalidos (SEC-32)
    - decisao por codigo de saida do purge, 0 a 4, "nao lido", "ainda rodando" e codigo desconhecido
    - purge de ponta a ponta com o stub: o filho nasce do Explorer, integridade Medium, mesmo
      usuario, linha de comando exata; caminho alternativo com o CreateProcessW forcado a falhar
    - fallback restrito (D-43): so library.json e settings.json na raiz; raiz junction; arquivos
      como ponto de reanalise e (se houver privilegio) symlink; pasta com o nome; hard link;
      somente leitura; raiz fora do LocalAppData; LocalAppData por junction; junction trocada em laco
    - autostart (BE-42): tarefa de um SID ficticio valido removida; SID malformado ignorado

  NAO toca na pasta real de dados do Sofa, NAO instala, NAO desinstala e NAO pede UAC. Cria e remove
  no maximo uma tarefa agendada de nome `Sofa Autostart (S-1-5-21-1-2-3-4)` (SID ficticio, que nunca
  e o de um usuario), sempre limpa no fim.

  O que so faz sentido ELEVADO ou numa VM fica no QA-17 (docs/qa/roteiro-usuario.md): UAC de outra
  conta, `RunAsUser` e CreateProcessW com o Explorer como pai estando elevado, `HKU\<SID>\Run`
  de outros perfis, ACL da pasta de instalacao, DLL plantada. Para o symlink de verdade num PC sem
  Modo de Desenvolvedor: rode este script num PowerShell elevado (so este teste usa a elevacao).

.PARAMETER Makensis
  Caminho do makensis.exe (padrao: o do cache do Tauri).
.PARAMETER Loops
  Repeticoes do teste de junction trocada em laco (padrao 60).

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/test-installer.ps1
#>
[CmdletBinding()]
param(
    [string]$Makensis = (Join-Path $env:LOCALAPPDATA 'tauri\NSIS\makensis.exe'),
    [int]$Loops = 60
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$winDir = Join-Path $root 'src-tauri\windows'
$testsDir = Join-Path $winDir 'tests'

if (-not (Test-Path -LiteralPath $Makensis)) {
    throw "makensis nao encontrado em $Makensis (rode um `tauri build` antes: ele baixa o NSIS) ou passe -Makensis"
}
$nsisDir = Split-Path -Parent $Makensis
$pluginDir = Join-Path $nsisDir 'Plugins\x86-unicode\additional'
if (-not (Test-Path -LiteralPath (Join-Path $pluginDir 'nsis_tauri_utils.dll'))) {
    $pluginDir = Join-Path $nsisDir 'Plugins\x86-unicode'
}
if (-not (Test-Path -LiteralPath (Join-Path $pluginDir 'nsis_tauri_utils.dll'))) {
    throw "nsis_tauri_utils.dll nao encontrada em $nsisDir\Plugins (rode um `tauri build`)"
}

# --- area de trabalho: SEMPRE dentro de %LOCALAPPDATA%\Temp (caminho longo, sem 8.3)
$work = Join-Path $env:LOCALAPPDATA ('Temp\sofa-installer-tests-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $work | Out-Null
$work = (Get-Item -LiteralPath $work).FullName
$realData = Join-Path $env:LOCALAPPDATA 'app.sofa.launcher'

function Assert-InWork([string]$Path) {
    $full = [System.IO.Path]::GetFullPath($Path)
    if (-not $full.StartsWith($work + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "caminho fora da area de teste: $full"
    }
    if ($full.StartsWith($realData, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "caminho dentro da pasta REAL de dados do Sofa: $full"
    }
}

$script:fail = New-Object System.Collections.Generic.List[string]
$script:passed = 0
$script:skipped = New-Object System.Collections.Generic.List[string]

function Test-Case([string]$Name, [scriptblock]$Body) {
    try {
        & $Body
        $script:passed++
        Write-Host ("  ok    {0}" -f $Name) -ForegroundColor Green
    }
    catch {
        $script:fail.Add($Name)
        Write-Host ("  FALHA {0}: {1}" -f $Name, $_.Exception.Message) -ForegroundColor Red
    }
}
function Skip-Case([string]$Name, [string]$Why) {
    $script:skipped.Add("$Name ($Why)")
    Write-Host ("  pulou {0}: {1}" -f $Name, $Why) -ForegroundColor Yellow
}
function Assert-True($Cond, [string]$Msg) { if (-not $Cond) { throw $Msg } }
# R-11 / ADR-0007: o nome visivel muda, as chaves tecnicas NAO. Com `UNINSTKEY` derivada de ${PRODUCTNAME}, a 0.2.0
# gravaria `Uninstall\SofaLauncher`, nao acharia a 0.1.0 e o PageReinstall perderia a conferencia do SEC-BUG-05.
function Assert-SofaTechnicalKeys([string]$Code, [string]$Where) {
    $lit = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Sofa'
    $m = [regex]::Matches($Code, '(?m)^\s*!define UNINSTKEY\s+"([^"]*)"')
    Assert-Eq $m.Count 1 "definicoes de UNINSTKEY no $Where"
    Assert-Eq $m[0].Groups[1].Value $lit "UNINSTKEY no $Where"
    Assert-True (-not $m[0].Groups[1].Value.Contains('PRODUCTNAME')) "UNINSTKEY no $Where nao pode derivar de PRODUCTNAME"
    foreach ($kv in @(@('MANUKEY', 'Software\${BUNDLEID}'), @('MANUPRODUCTKEY', 'Software\${BUNDLEID}'), @('INSTALLDIRNAME', 'Sofa'))) {
        $mm = [regex]::Matches($Code, ('(?m)^\s*!define ' + $kv[0] + '\s+"([^"]*)"'))
        Assert-Eq $mm.Count 1 "definicoes de $($kv[0]) no $Where"
        Assert-Eq $mm[0].Groups[1].Value $kv[1] "$($kv[0]) no $Where"
    }
}

function Assert-Eq($Got, $Want, [string]$Msg) { if ("$Got" -ne "$Want") { throw ("{0}: obtido '{1}', esperado '{2}'" -f $Msg, $Got, $Want) } }

# --- compilacao
function Invoke-Make([string]$Script, [string[]]$Defines) {
    $args = @('-INPUTCHARSET', 'UTF8', '-V2') + $Defines + @($Script)
    $out = & $Makensis @args 2>&1
    if ($LASTEXITCODE -ne 0) { throw ("makensis falhou em {0}:`n{1}" -f $Script, ($out -join "`n")) }
}
$bin = Join-Path $work 'bin'
New-Item -ItemType Directory -Force -Path $bin | Out-Null
$harness = Join-Path $bin 'harness.exe'
$harnessFail = Join-Path $bin 'harness-failcreate.exe'
$stub = Join-Path $bin 'stub-sofa.exe'
Write-Host "Compilando o harness e o stub com $Makensis" -ForegroundColor Cyan
Invoke-Make (Join-Path $testsDir 'stub.nsi') @("/DOUT_EXE=$stub")
Invoke-Make (Join-Path $testsDir 'harness.nsi') @("/DOUT_EXE=$harness", "/DSOFADIR=$winDir", "/DPLUGINDIR=$pluginDir")
Invoke-Make (Join-Path $testsDir 'harness.nsi') @("/DOUT_EXE=$harnessFail", "/DSOFADIR=$winDir", "/DPLUGINDIR=$pluginDir", '/DSOFA_TEST_FORCE_CREATE_FAIL')

# --- utilitarios de cenario
function Run-Harness([string]$Exe, [string[]]$Opts, [int]$TimeoutSec = 120) {
    $out = Join-Path $work ('out-' + [guid]::NewGuid().ToString('N') + '.txt')
    $arguments = @('/S') + $Opts + @("/OUT=$out")
    $p = Start-Process -FilePath $Exe -ArgumentList $arguments -PassThru -WindowStyle Hidden
    if (-not $p.WaitForExit($TimeoutSec * 1000)) { $p.Kill(); throw "harness nao terminou em $TimeoutSec s" }
    $res = @{}
    if (Test-Path -LiteralPath $out) {
        foreach ($l in Get-Content -LiteralPath $out) { if ($l -match '^([^=]+)=(.*)$') { $res[$Matches[1]] = $Matches[2] } }
    }
    return $res
}

function New-DataRoot([string]$Name) {
    # <work>\<Name>\local\app.sofa.launcher com os dois arquivos e o resto da pasta de dados
    $local = Join-Path $work "$Name\local"
    $r = Join-Path $local 'app.sofa.launcher'
    foreach ($d in @($r, "$r\logs", "$r\cache\covers", "$r\cache\icons", "$r\EBWebView\Default")) {
        New-Item -ItemType Directory -Force -Path $d | Out-Null
    }
    foreach ($f in @('library.json', 'settings.json', 'logs\sofa.log', 'cache\covers\a.jpg', 'cache\icons\a.png', 'EBWebView\Default\x.db', 'library.corrupt-1.json')) {
        Set-Content -LiteralPath (Join-Path $r $f) -Value 'dados'
    }
    return [pscustomobject]@{ Local = $local; Root = $r }
}

function New-Decoy([string]$Name) {
    $d = Join-Path $work "$Name\isca"
    New-Item -ItemType Directory -Force -Path $d | Out-Null
    foreach ($f in @('library.json', 'settings.json', 'outro.txt')) { Set-Content -LiteralPath (Join-Path $d $f) -Value 'ISCA' }
    return $d
}

function Test-DecoyIntact([string]$Decoy) {
    foreach ($f in @('library.json', 'settings.json', 'outro.txt')) {
        $p = Join-Path $Decoy $f
        if (-not (Test-Path -LiteralPath $p)) { throw "isca perdeu $f" }
        if ((Get-Content -LiteralPath $p -Raw).Trim() -ne 'ISCA') { throw "isca alterada em $f" }
    }
}

function New-Junction([string]$Link, [string]$Target) {
    Assert-InWork $Link; Assert-InWork $Target
    $o = cmd /c mklink /J "`"$Link`"" "`"$Target`"" 2>&1
    if ($LASTEXITCODE -ne 0) { throw "mklink /J falhou: $o" }
}

function Remove-Junction([string]$Link) {
    Assert-InWork $Link
    # `rmdir` sem /s remove so o ponto de reanalise
    cmd /c rmdir "`"$Link`"" 2>&1 | Out-Null
}

function Exists([string]$Path) { return (Test-Path -LiteralPath $Path) }

# O SID segue para o harness por arquivo (sem quebra de linha): aspas, espaco e & nao passam pela linha de comando.
function Write-SidFile([string]$Sid) {
    $f = Join-Path $work ('sid-' + [guid]::NewGuid().ToString('N') + '.txt')
    [System.IO.File]::WriteAllText($f, $Sid, (New-Object System.Text.UTF8Encoding($false)))
    return $f
}

function Seeded([string]$Root) {
    return @{
        Lib      = Exists (Join-Path $Root 'library.json')
        Set      = Exists (Join-Path $Root 'settings.json')
        Log      = Exists (Join-Path $Root 'logs\sofa.log')
        Cover    = Exists (Join-Path $Root 'cache\covers\a.jpg')
        Icon     = Exists (Join-Path $Root 'cache\icons\a.png')
        WebView  = Exists (Join-Path $Root 'EBWebView\Default\x.db')
        Corrupt  = Exists (Join-Path $Root 'library.corrupt-1.json')
    }
}

function Assert-OnlyTwoGone([string]$Root) {
    $s = Seeded $Root
    Assert-True (-not $s.Lib -and -not $s.Set) 'library.json e settings.json deviam ter sumido'
    Assert-True ($s.Log -and $s.Cover -and $s.Icon -and $s.WebView -and $s.Corrupt) 'o resto da pasta (logs, capas, icones, EBWebView, *.corrupt-*) devia ficar'
}
function Assert-NothingGone([string]$Root) {
    $s = Seeded $Root
    Assert-True ($s.Lib -and $s.Set -and $s.Log -and $s.Cover -and $s.Icon -and $s.WebView -and $s.Corrupt) 'nada devia ter sido apagado'
}

$sidNow = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value

try {
    # =============================================================== SID (SEC-32)
    Write-Host "`n== SID: ^S-1-5-21(-[0-9]{1,10}){4}$" -ForegroundColor Cyan
    $valid = @('S-1-5-21-1-2-3-4', 'S-1-5-21-1234567890-1234567890-1234567890-1234567890', $sidNow)
    $invalid = @('', 'S-1-5-21-1-2-3', 'S-1-5-21-1-2-3-4-5', 'S-1-5-21-12345678901-2-3-4', 'S-1-5-21--2-3-4', 'S-1-5-21-1-2-3-',
        's-1-5-21-1-2-3-4', 'S-1-5-18', 'S-1-5-21-1-2-3-a', 'S-1-5-21-1-2-3-4 ', 'S-1-5-21-1-2-3-4"/F', 'S-1-5-21-1-2-3-4&calc',
        'S-1-5-21-1-2-3-4\x', 'S-1-5-21-1-2-3-4$(x)', 'S-1-5-21-1-2-3-4-')
    foreach ($s in $valid) {
        Test-Case "SID valido: $s" { $r = Run-Harness $harness @('/SCENARIO=sid', ('/SIDFILE=' + (Write-SidFile $s))); Assert-Eq $r['valid'] '1' 'valid' }
    }
    foreach ($s in $invalid) {
        Test-Case "SID invalido: '$s'" { $r = Run-Harness $harness @('/SCENARIO=sid', ('/SIDFILE=' + (Write-SidFile $s))); Assert-Eq $r['valid'] '0' 'valid' }
    }

    # =============================================================== decisao por codigo (6b)
    Write-Host "`n== Decisao por codigo de saida do purge (ADR-0006 6b)" -ForegroundColor Cyan
    $table = @(
        @{ Name = 'codigo 0';              Opts = @('/READ=1', '/CODE=0');   Outcome = 'ok:codigo-0';                         Fallback = $false; Delete = $false }
        @{ Name = 'codigo 1 (parcial)';    Opts = @('/READ=1', '/CODE=1');   Outcome = 'fallback:codigo-1-parcial:apagou-2';   Fallback = $true;  Delete = $true }
        @{ Name = 'codigo 2 (alvo invalido)'; Opts = @('/READ=1', '/CODE=2'); Outcome = 'nada:codigo-2-alvo-invalido';        Fallback = $false; Delete = $false }
        @{ Name = 'codigo 3 (elevado)';    Opts = @('/READ=1', '/CODE=3');   Outcome = 'fallback:codigo-3-elevado:apagou-2';  Fallback = $true;  Delete = $true }
        @{ Name = 'codigo 4 (Sofa aberto)'; Opts = @('/READ=1', '/CODE=4');  Outcome = 'nada:codigo-4-sofa-aberto';           Fallback = $false; Delete = $false }
        @{ Name = 'codigo desconhecido (7)'; Opts = @('/READ=1', '/CODE=7'); Outcome = 'fallback:codigo-desconhecido-7:apagou-2'; Fallback = $true; Delete = $true }
        @{ Name = 'codigo nao lido';       Opts = @('/READ=0');              Outcome = 'fallback:codigo-nao-lido:apagou-2';    Fallback = $true;  Delete = $true }
        @{ Name = 'purge ainda rodando';   Opts = @('/READ=0', '/RUNNING=1'); Outcome = 'nada:purge-em-andamento';             Fallback = $false; Delete = $false }
        @{ Name = 'rodando vence o codigo lido'; Opts = @('/READ=1', '/CODE=1', '/RUNNING=1'); Outcome = 'nada:purge-em-andamento'; Fallback = $false; Delete = $false }
    )
    $n = 0
    foreach ($row in $table) {
        $n++
        $row = $row
        Test-Case $row.Name {
            $d = New-DataRoot "decide$n"
            $r = Run-Harness $harness (@('/SCENARIO=decide', "/LOCALAPP=$($d.Local)", "/ROOT=$($d.Root)") + $row.Opts)
            Assert-Eq $r['outcome'] $row.Outcome 'resultado'
            if ($row.Delete) { Assert-OnlyTwoGone $d.Root } else { Assert-NothingGone $d.Root }
        }
    }
    Test-Case 'perfil nao resolvido: nao apaga nem no codigo 1' {
        $d = New-DataRoot 'decide-noprofile'
        $r = Run-Harness $harness @('/SCENARIO=decide', '/READ=1', '/CODE=1')   # sem /ROOT nem /LOCALAPP
        Assert-Eq $r['outcome'] 'nada:perfil-nao-resolvido' 'resultado'
        Assert-NothingGone $d.Root
        # sem pasta para mostrar, o texto da caixa nao termina em linhas em branco (BE-56, revisao-instalador.md)
        Assert-Eq $r['tail'] 'io.' 'fim do texto da caixa sem linhas em branco'
    }

    # =============================================================== o usuario interativo
    Write-Host "`n== Usuario interativo (so leitura)" -ForegroundColor Cyan
    Test-Case 'SID do Explorer e da sessao batem com o usuario atual; LocalAppData do perfil' {
        $r = Run-Harness $harness @('/SCENARIO=profile')
        Assert-True ([int]$r['explorer'] -ne 0) 'nao abriu o Explorer da sessao (PROCESS_CREATE_PROCESS)'
        Assert-Eq $r['sid'] $sidNow 'SID resolvido'
        Assert-Eq $r['localapp'] $env:LOCALAPPDATA 'LocalAppData do perfil'
    }

    # =============================================================== purge de ponta a ponta com o stub
    Write-Host "`n== Purge: CreateProcessW com o Explorer como pai (stub no lugar do sofa.exe)" -ForegroundColor Cyan
    foreach ($code in 0..4) {
        Test-Case "stub devolve $code" {
            $inst = Join-Path $work "inst-$code"
            New-Item -ItemType Directory -Force -Path $inst | Out-Null
            Copy-Item -LiteralPath $stub -Destination (Join-Path $inst 'sofa.exe')
            Set-Content -LiteralPath (Join-Path $inst 'exitcode.txt') -Value "$code" -NoNewline
            $d = New-DataRoot "purge$code"
            $r = Run-Harness $harness @('/SCENARIO=purge', "/INSTDIR=$inst", "/LOCALAPP=$($d.Local)")
            Assert-Eq $r['started'] '1' 'CreateProcessW iniciou'
            Assert-Eq $r['read'] '1' 'codigo lido'
            Assert-Eq $r['code'] "$code" 'codigo'
            $log = Get-Content -LiteralPath (Join-Path $inst 'stub.log')
            $args0 = ($log | Where-Object { $_ -like 'args=*' }) -replace '^args=', ''
            Assert-Eq $args0 ('"' + (Join-Path $inst 'sofa.exe') + '" --purge-user-data') 'linha de comando do purge (constante)'
            $integrity = (($log | Where-Object { $_ -like 'integrity=*' }) -replace '^integrity=', '')
            Assert-Eq $integrity '8192' 'integridade do filho (0x2000 = Medium)'
            Assert-Eq (($log | Where-Object { $_ -like 'user=*' }) -replace '^user=', '') $sidNow 'dono do token do filho'
            switch ($code) {
                0 { Assert-NothingGone $d.Root }
                1 { Assert-OnlyTwoGone $d.Root }
                2 { Assert-NothingGone $d.Root }
                3 { Assert-OnlyTwoGone $d.Root }
                4 { Assert-NothingGone $d.Root }
            }
        }
    }
    Test-Case 'o filho do purge nasce com explorer.exe como pai' {
        $inst = Join-Path $work 'inst-parent'
        New-Item -ItemType Directory -Force -Path $inst | Out-Null
        Copy-Item -LiteralPath $stub -Destination (Join-Path $inst 'sofa.exe')
        Set-Content -LiteralPath (Join-Path $inst 'exitcode.txt') -Value '0' -NoNewline
        Set-Content -LiteralPath (Join-Path $inst 'sleep.txt') -Value 'x' -NoNewline
        $d = New-DataRoot 'purge-parent'
        $out = Join-Path $work 'out-parent.txt'
        $p = Start-Process -FilePath $harness -ArgumentList @('/S', '/SCENARIO=purge', "/INSTDIR=$inst", "/LOCALAPP=$($d.Local)", "/OUT=$out") -PassThru -WindowStyle Hidden
        $logPath = Join-Path $inst 'stub.log'
        $childPid = $null
        for ($i = 0; $i -lt 100 -and -not $childPid; $i++) {
            Start-Sleep -Milliseconds 50
            if (Test-Path -LiteralPath $logPath) {
                $l = Get-Content -LiteralPath $logPath -ErrorAction SilentlyContinue | Where-Object { $_ -like 'pid=*' }
                if ($l) { $childPid = [int](($l | Select-Object -First 1) -replace '^pid=', '') }
            }
        }
        Assert-True ($childPid) 'o stub nao registrou o pid'
        $proc = Get-CimInstance Win32_Process -Filter "ProcessId=$childPid"
        Assert-True ($proc) 'o stub ja tinha terminado'
        $parent = Get-CimInstance Win32_Process -Filter "ProcessId=$($proc.ParentProcessId)"
        Assert-Eq $parent.Name 'explorer.exe' 'processo pai do purge'
        Assert-True ($proc.ParentProcessId -ne $p.Id) 'o pai nao pode ser o desinstalador (herdaria o token elevado)'
        Assert-True ($p.WaitForExit(30000)) 'o harness nao terminou'
    }
    Test-Case 'CreateProcessW falha: codigo nao lido, apaga so os dois arquivos' {
        if (Get-Process -Name sofa -ErrorAction SilentlyContinue) { throw 'ha um sofa.exe aberto neste PC; feche-o antes (FindProcess enxerga qualquer um)' }
        $inst = Join-Path $work 'inst-alt'
        New-Item -ItemType Directory -Force -Path $inst | Out-Null
        Copy-Item -LiteralPath $stub -Destination (Join-Path $inst 'sofa.exe')
        Set-Content -LiteralPath (Join-Path $inst 'exitcode.txt') -Value '0' -NoNewline
        $d = New-DataRoot 'purge-alt'
        $r = Run-Harness $harnessFail @('/SCENARIO=purge', "/INSTDIR=$inst", "/LOCALAPP=$($d.Local)")
        Assert-Eq $r['read'] '0' 'codigo lido'
        Assert-Eq $r['outcome'] 'fallback:codigo-nao-lido:apagou-2' 'resultado'
        Assert-OnlyTwoGone $d.Root
        Assert-True (Exists (Join-Path $inst 'stub.log')) 'o RunAsUser devia ter iniciado o stub'
    }

    # =============================================================== fallback restrito (D-43)
    Write-Host "`n== Fallback restrito: so library.json e settings.json, por handle" -ForegroundColor Cyan
    function Run-Fallback($d) { return (Run-Harness $harness @('/SCENARIO=fallback', "/LOCALAPP=$($d.Local)", "/ROOT=$($d.Root)")) }

    Test-Case 'normal: apaga so os dois e nao desce em subpasta' {
        $d = New-DataRoot 'fb-normal'
        # arquivos de mesmo nome em subpastas e nome parecido na raiz: nenhum e tocado
        Set-Content -LiteralPath (Join-Path $d.Root 'cache\library.json') -Value 'sub'
        Set-Content -LiteralPath (Join-Path $d.Root 'settings.json.bak') -Value 'bak'
        $r = Run-Fallback $d
        Assert-Eq $r['deleted'] '2' 'apagados'
        Assert-OnlyTwoGone $d.Root
        Assert-True (Exists (Join-Path $d.Root 'cache\library.json')) 'cache\library.json (subpasta) devia ficar'
        Assert-True (Exists (Join-Path $d.Root 'settings.json.bak')) 'settings.json.bak devia ficar'
    }
    Test-Case 'so um dos dois existe' {
        $d = New-DataRoot 'fb-one'
        Remove-Item -LiteralPath (Join-Path $d.Root 'settings.json')
        $r = Run-Fallback $d
        Assert-Eq $r['deleted'] '1' 'apagados'
    }
    Test-Case 'raiz inexistente: nada a apagar, sem erro' {
        $d = New-DataRoot 'fb-absent'
        Remove-Item -LiteralPath $d.Root -Recurse -Force
        $r = Run-Fallback $d
        Assert-Eq $r['deleted'] '0' 'apagados'
    }
    Test-Case 'raiz e uma junction para a isca: a isca fica intacta' {
        $d = New-DataRoot 'fb-rootjunction'
        $decoy = New-Decoy 'fb-rootjunction'
        Remove-Item -LiteralPath $d.Root -Recurse -Force
        New-Junction $d.Root $decoy
        $r = Run-Fallback $d
        Assert-Eq $r['deleted'] '0' 'apagados'
        Test-DecoyIntact $decoy
        Remove-Junction $d.Root
    }
    Test-Case 'LocalAppData do perfil passa por junction: nada apagado' {
        $d = New-DataRoot 'fb-parentjunction'
        $linkLocal = Join-Path $work 'fb-parentjunction\local-link'
        New-Junction $linkLocal $d.Local
        $r = Run-Harness $harness @('/SCENARIO=fallback', "/LOCALAPP=$linkLocal", "/ROOT=$linkLocal\app.sofa.launcher")
        Assert-Eq $r['deleted'] '0' 'apagados'
        Assert-NothingGone $d.Root
        Remove-Junction $linkLocal
    }
    Test-Case 'raiz que nao e filha direta do LocalAppData: nada apagado' {
        $d = New-DataRoot 'fb-notchild'
        $other = Join-Path $work 'fb-notchild\outra\app.sofa.launcher'
        New-Item -ItemType Directory -Force -Path $other | Out-Null
        Set-Content -LiteralPath (Join-Path $other 'library.json') -Value 'x'
        $r = Run-Harness $harness @('/SCENARIO=fallback', "/LOCALAPP=$($d.Local)", "/ROOT=$other")
        Assert-Eq $r['deleted'] '0' 'apagados'
        Assert-True (Exists (Join-Path $other 'library.json')) 'arquivo fora do LocalAppData foi apagado'
    }
    Test-Case 'library.json e settings.json como PASTAS: intactas' {
        $d = New-DataRoot 'fb-dirs'
        foreach ($n in @('library.json', 'settings.json')) {
            Remove-Item -LiteralPath (Join-Path $d.Root $n)
            New-Item -ItemType Directory -Path (Join-Path $d.Root $n) | Out-Null
            Set-Content -LiteralPath (Join-Path $d.Root "$n\dentro.txt") -Value 'x'
        }
        $r = Run-Fallback $d
        Assert-Eq $r['deleted'] '0' 'apagados'
        foreach ($n in @('library.json', 'settings.json')) { Assert-True (Exists (Join-Path $d.Root "$n\dentro.txt")) "$n (pasta) foi tocada" }
    }
    Test-Case 'library.json e settings.json como JUNCTION para a isca: a isca fica intacta e o link fica' {
        $d = New-DataRoot 'fb-filejunction'
        $decoy = New-Decoy 'fb-filejunction'
        foreach ($n in @('library.json', 'settings.json')) {
            Remove-Item -LiteralPath (Join-Path $d.Root $n)
            New-Junction (Join-Path $d.Root $n) $decoy
        }
        $r = Run-Fallback $d
        Assert-Eq $r['deleted'] '0' 'apagados'
        Test-DecoyIntact $decoy
        foreach ($n in @('library.json', 'settings.json')) {
            Assert-True ((Get-Item -LiteralPath (Join-Path $d.Root $n) -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) "$n deixou de ser o link"
            Remove-Junction (Join-Path $d.Root $n)
        }
    }

    # symlink de verdade (precisa de privilegio) e, sempre, ponto de reanalise de arquivo sem privilegio
    $canSymlink = $true
    try {
        $probe = Join-Path $work 'probe-target.txt'; Set-Content -LiteralPath $probe -Value 'x'
        New-Item -ItemType SymbolicLink -Path (Join-Path $work 'probe-link.txt') -Target $probe -ErrorAction Stop | Out-Null
    } catch { $canSymlink = $false }
    if ($canSymlink) {
        Test-Case 'library.json e settings.json como SYMLINK para a isca (nos dois nomes)' {
            $d = New-DataRoot 'fb-symlink'
            $decoy = New-Decoy 'fb-symlink'
            foreach ($n in @('library.json', 'settings.json')) {
                Remove-Item -LiteralPath (Join-Path $d.Root $n)
                New-Item -ItemType SymbolicLink -Path (Join-Path $d.Root $n) -Target (Join-Path $decoy $n) | Out-Null
            }
            $r = Run-Fallback $d
            Assert-Eq $r['deleted'] '0' 'apagados'
            Test-DecoyIntact $decoy
            foreach ($n in @('library.json', 'settings.json')) {
                Assert-True (Exists (Join-Path $d.Root $n)) "$n (symlink) foi apagado"
            }
        }
    } else {
        Skip-Case 'symlink de arquivo' 'criar symlink exige privilegio (administrador ou Modo de Desenvolvedor); rode este script elevado para este caso. Abaixo, o ponto de reanalise de arquivo cobre a mesma regra'
    }

    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class SofaReparse {
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern SafeFileHandle CreateFileW(string name, uint access, uint share, IntPtr sec, uint disp, uint flags, IntPtr tmpl);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool DeviceIoControl(SafeFileHandle h, uint code, byte[] inBuf, uint inSize, IntPtr outBuf, uint outSize, out uint ret, IntPtr ov);
    // Marca o ARQUIVO como ponto de reanalise (FILE_ATTRIBUTE_REPARSE_POINT) sem privilegio nenhum:
    // para o NTFS e um "link"; abrir sem OPEN_REPARSE_POINT falha. Etiqueta 0x8000FF00 com 4 bytes de
    // dados, em REPARSE_DATA_BUFFER generico (etiquetas de terceiro exigem o formato GUID e o NTFS
    // recusa o que o teste tentou; esta aceita e nenhum filtro a trata).
    public static void MakeReparseFile(string path) {
        var h = CreateFileW(path, 0x40000000 | 0x80000000, 0, IntPtr.Zero, 3, 0x00200000, IntPtr.Zero);
        if (h.IsInvalid) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), "abrir");
        using (h) {
            byte[] b = new byte[8 + 4];
            BitConverter.GetBytes(0x8000FF00u).CopyTo(b, 0);
            BitConverter.GetBytes((ushort)4).CopyTo(b, 4);
            uint ret;
            if (!DeviceIoControl(h, 0x000900A4, b, (uint)b.Length, IntPtr.Zero, 0, out ret, IntPtr.Zero))
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), "FSCTL_SET_REPARSE_POINT");
        }
    }
}
'@
    Test-Case 'library.json e settings.json como PONTO DE REANALISE de arquivo (nos dois nomes): nao apaga' {
        $d = New-DataRoot 'fb-reparsefile'
        foreach ($n in @('library.json', 'settings.json')) {
            $p = Join-Path $d.Root $n
            Remove-Item -LiteralPath $p
            Set-Content -LiteralPath $p -Value 'x'
            [SofaReparse]::MakeReparseFile($p)
            Assert-True ((Get-Item -LiteralPath $p -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) "$n nao virou ponto de reanalise (o teste nao prova nada)"
        }
        $r = Run-Fallback $d
        Assert-Eq $r['deleted'] '0' 'apagados'
        foreach ($n in @('library.json', 'settings.json')) {
            Assert-True (Exists (Join-Path $d.Root $n)) "$n (ponto de reanalise) foi apagado"
        }
        # limpeza: o `Remove-Item -Recurse` do fim lida com o ponto de reanalise sem seguir
    }
    Test-Case 'library.json e hard link para um arquivo de fora: so o nome da raiz some' {
        $d = New-DataRoot 'fb-hardlink'
        $outside = Join-Path $work 'fb-hardlink\fora.txt'
        Set-Content -LiteralPath $outside -Value 'FORA'
        Remove-Item -LiteralPath (Join-Path $d.Root 'library.json')
        $o = cmd /c mklink /H "`"$(Join-Path $d.Root 'library.json')`"" "`"$outside`"" 2>&1
        if ($LASTEXITCODE -ne 0) { throw "mklink /H falhou: $o" }
        $r = Run-Fallback $d
        Assert-Eq $r['deleted'] '2' 'apagados'
        Assert-True (Exists $outside) 'o outro nome do hard link foi apagado'
        Assert-Eq ((Get-Content -LiteralPath $outside -Raw).Trim()) 'FORA' 'conteudo do outro nome'
    }
    Test-Case 'arquivos somente leitura tambem saem' {
        $d = New-DataRoot 'fb-readonly'
        foreach ($n in @('library.json', 'settings.json')) { (Get-Item -LiteralPath (Join-Path $d.Root $n)).IsReadOnly = $true }
        $r = Run-Fallback $d
        Assert-Eq $r['deleted'] '2' 'apagados'
        Assert-OnlyTwoGone $d.Root
    }
    Test-Case 'arquivo aberto por outro processo (sem compartilhar exclusao): nao quebra e o outro arquivo sai' {
        $d = New-DataRoot 'fb-inuse'
        $fs = [System.IO.File]::Open((Join-Path $d.Root 'library.json'), 'Open', 'Read', 'Read')   # sem FILE_SHARE_DELETE
        try {
            $r = Run-Fallback $d
            Assert-Eq $r['deleted'] '1' 'apagados (so o settings.json)'
            Assert-True (Exists (Join-Path $d.Root 'library.json')) 'library.json em uso foi apagado'
        } finally { $fs.Dispose() }
    }

    # junction trocada em laco durante o fallback: a isca nunca perde nada
    Test-Case "junction trocada em laco ($Loops execucoes): a isca fica intacta" {
        $d = New-DataRoot 'fb-race'
        $decoy = New-Decoy 'fb-race'
        $real = Join-Path $work 'fb-race\real'
        $stopFile = Join-Path $work 'fb-race\stop'
        Move-Item -LiteralPath $d.Root -Destination $real
        $swapper = Start-Job -ArgumentList $d.Root, $real, $decoy, $stopFile -ScriptBlock {
            param($root, $real, $decoy, $stop)
            $i = 0
            while (-not (Test-Path -LiteralPath $stop)) {
                $i++
                # estado A: raiz e a pasta de verdade
                cmd /c rmdir "`"$root`"" 2>$null | Out-Null
                if (-not (Test-Path -LiteralPath $root)) { try { New-Item -ItemType Junction -Path $root -Target $real | Out-Null } catch {} }
                Start-Sleep -Milliseconds 1
                # estado B: raiz e uma junction para a isca
                cmd /c rmdir "`"$root`"" 2>$null | Out-Null
                if (-not (Test-Path -LiteralPath $root)) { try { New-Item -ItemType Junction -Path $root -Target $decoy | Out-Null } catch {} }
                Start-Sleep -Milliseconds 1
            }
            $i
        }
        try {
            for ($k = 0; $k -lt $Loops; $k++) {
                [void](Run-Harness $harness @('/SCENARIO=fallback', "/LOCALAPP=$($d.Local)", "/ROOT=$($d.Root)"))
                Test-DecoyIntact $decoy
            }
        }
        finally {
            Set-Content -LiteralPath $stopFile -Value 'x'
            $swaps = Receive-Job -Job $swapper -Wait
            Remove-Job -Job $swapper -Force
            Remove-Junction $d.Root
            Write-Host ("        trocas feitas pelo laco: {0}" -f $swaps)
        }
        Test-DecoyIntact $decoy
    }

    # =============================================================== autostart (BE-42)
    Write-Host "`n== Autostart de todos os perfis (SEC-32): SID ficticio, sem tocar no seu" -ForegroundColor Cyan
    $fakeSid = 'S-1-5-21-1-2-3-4'
    $taskName = "Sofa Autostart ($fakeSid)"
    $badSid = 'S-1-5-21-1-2-3'                      # 3 grupos: invalido
    $badTask = "Sofa Autostart ($badSid)"
    function New-TestTask([string]$Name) {
        $o = cmd /c "`"$env:SystemRoot\System32\schtasks.exe`" /Create /TN `"$Name`" /TR `"$env:SystemRoot\System32\cmd.exe /c exit`" /SC ONCE /ST 23:59 /F 2>&1"
        if ($LASTEXITCODE -ne 0) { throw "schtasks /Create falhou (sem permissao para tarefa do proprio usuario?): $o" }
    }
    function Test-TaskExists([string]$Name) {
        cmd /c "`"$env:SystemRoot\System32\schtasks.exe`" /Query /TN `"$Name`" >nul 2>&1"
        return ($LASTEXITCODE -eq 0)
    }
    function Remove-TestTask([string]$Name) {
        cmd /c "`"$env:SystemRoot\System32\schtasks.exe`" /Delete /TN `"$Name`" /F >nul 2>&1"
    }
    try {
        Assert-True (-not (Test-TaskExists $taskName) -and -not (Test-TaskExists $badTask)) 'ja existe tarefa de teste com este nome'
        Test-Case 'SID valido: a tarefa "Sofa Autostart (<SID>)" e removida' {
            New-TestTask $taskName
            Assert-True (Test-TaskExists $taskName) 'a tarefa de teste nao foi criada'
            # observa os processos filhos do harness: so o schtasks de $SYSDIR pode aparecer
            $p = Start-Process -FilePath $harness -ArgumentList @('/S', '/SCENARIO=autostart', ('/SIDFILE=' + (Write-SidFile $fakeSid)), "/OUT=$(Join-Path $work 'out-as.txt')") -PassThru -WindowStyle Hidden
            $seen = @{}
            while (-not $p.HasExited) {
                Get-CimInstance Win32_Process -Filter "ParentProcessId=$($p.Id)" -ErrorAction SilentlyContinue | ForEach-Object { $seen[$_.ExecutablePath] = $true }
            }
            Assert-True (-not (Test-TaskExists $taskName)) 'a tarefa continua la'
            $paths = @($seen.Keys | Where-Object { $_ })
            Write-Host ("        processos filhos vistos: {0}" -f ($(if ($paths.Count) { $paths -join ', ' } else { '(nenhum capturado)' })))
            foreach ($path in $paths) {
                $okPaths = @("$env:SystemRoot\SysWOW64\schtasks.exe", "$env:SystemRoot\System32\schtasks.exe")
                Assert-True ($okPaths -contains $path) "processo externo inesperado: $path"
            }
        }
        Test-Case 'SID malformado: ignorado, nada executado (a tarefa homonima fica)' {
            New-TestTask $badTask
            $r = Run-Harness $harness @('/SCENARIO=autostart', ('/SIDFILE=' + (Write-SidFile $badSid)))
            Assert-True (Test-TaskExists $badTask) 'o SID invalido foi usado para apagar a tarefa'
        }
    }
    finally {
        Remove-TestTask $taskName
        Remove-TestTask $badTask
    }
    Skip-Case 'HKU\<SID>\...\Run de outros perfis' 'precisa de hive carregado de outro usuario (admin); QA-17'

    # =============================================================== pasta fixa (SEC-BUG-05)
    Write-Host "`n== Pasta de instalacao fixa em Program Files (SEC-BUG-05, ARQ-BUG-16)" -ForegroundColor Cyan
    $fixed = Join-Path $env:ProgramW6432 'Sofa'
    # `/D=` tem de ser o ultimo parametro e sem aspas (regra do NSIS); /OUT= vai antes
    function Run-HarnessDir([string]$DirArg, [string]$Prev) {
        $out = Join-Path $work ('out-' + [guid]::NewGuid().ToString('N') + '.txt')
        $argLine = '/S /SCENARIO=instdir /OUT=' + $out
        if ($Prev) { $argLine += ' /PREV=' + $Prev }
        if ($DirArg) { $argLine += ' /D=' + $DirArg }
        $p = Start-Process -FilePath $harness -ArgumentList $argLine -PassThru -WindowStyle Hidden
        if (-not $p.WaitForExit(60000)) { $p.Kill(); throw 'harness nao terminou' }
        $res = @{}
        if (Test-Path -LiteralPath $out) {
            foreach ($l in Get-Content -LiteralPath $out) { if ($l -match '^([^=]+)=(.*)$') { $res[$Matches[1]] = $Matches[2] } }
        }
        return $res
    }
    Test-Case 'sem /D= e sem instalacao anterior: a pasta e Program Files\Sofa' {
        $r = Run-HarnessDir '' ''
        Assert-Eq $r['result'] '0' 'resultado'
        Assert-Eq $r['instdir'] $fixed 'pasta'
    }
    Test-Case '/D= igual a pasta fixa: aceito' {
        $r = Run-HarnessDir $fixed ''
        Assert-Eq $r['result'] '0' 'resultado'
        Assert-Eq $r['instdir'] $fixed 'pasta'
    }
    foreach ($bad in @('C:\X', 'C:\Sofa', 'D:\Jogos\Sofa', 'C:\Program Files\Outra', "$fixed\sub", ($fixed + 'X'))) {
        $bad = $bad
        Test-Case "/D=${bad}: recusado, o destino nao muda" {
            $r = Run-HarnessDir $bad ''
            Assert-Eq $r['result'] '1' 'resultado (1 = /D= diferente)'
            Assert-True ($r['instdir'] -ne $fixed) 'a pasta nao pode virar a fixa em silencio'
        }
    }
    foreach ($prev in @('C:\Sofa', 'D:\Jogos\Sofa', 'C:\Users\Publico\Sofa')) {
        $prev = $prev
        Test-Case "instalacao anterior em ${prev}: recusada antes de qualquer desinstalador" {
            $r = Run-HarnessDir '' $prev
            Assert-Eq $r['result'] '2' 'resultado (2 = instalacao anterior fora da pasta fixa)'
        }
    }
    Test-Case 'instalacao anterior na pasta fixa: aceita' {
        $r = Run-HarnessDir '' $fixed
        Assert-Eq $r['result'] '0' 'resultado'
        Assert-Eq $r['instdir'] $fixed 'pasta'
    }
    Test-Case '/D= diferente vence a anterior ok: ainda recusado' {
        $r = Run-HarnessDir 'C:\X' $fixed
        Assert-Eq $r['result'] '1' 'resultado'
    }
    Test-Case 'script do instalador (template): sem pagina de pasta, pasta fixada em .onInit e un.onInit, trava antes do ExecWait' {
        $tpl = Join-Path $winDir 'installer.nsi'
        $code = ((Get-Content -LiteralPath $tpl -Encoding UTF8) | Where-Object { $_.TrimStart() -notmatch '^[;#]' }) -join "`n"
        Assert-True ($code -notmatch 'MUI_PAGE_DIRECTORY') 'o instalador nao pode ter a pagina de pasta'
        Assert-True ($code -notmatch 'MUI_UNPAGE_DIRECTORY') 'o desinstalador nao pode ter pagina de pasta'
        $onInit = [regex]::Match($code, '(?s)Function \.onInit(.*?)\nFunctionEnd').Groups[1].Value
        Assert-True ($onInit -match 'Call SofaFixInstallDir') '.onInit precisa chamar SofaFixInstallDir'
        $unInit = [regex]::Match($code, '(?s)Function un\.onInit(.*?)\nFunctionEnd').Groups[1].Value
        Assert-True ($unInit -match 'Call un\.SofaFixInstallDir') 'un.onInit precisa chamar un.SofaFixInstallDir'
        # nenhum ExecWait do desinstalador anterior antes da trava de pasta
        $guardAt = $code.IndexOf('$(sofaPreviousInstallElsewhere)', $code.IndexOf('Function PageLeaveReinstall'))
        $execAt = $code.IndexOf('ExecWait ''"$INSTDIR\uninstall.exe"')
        Assert-True ($guardAt -gt 0 -and $execAt -gt 0 -and $guardAt -lt $execAt) 'a trava de pasta tem de vir antes do ExecWait do desinstalador anterior'
        # o instalador so conhece uma pasta de destino: nada de InstallDir com variavel de usuario
        Assert-True ($code -notmatch 'StrCpy \$INSTDIR\s+"\$LOCALAPPDATA') 'pasta em LOCALAPPDATA proibida'
    }
    Test-Case 'fontes do idioma e do instalador: UTF-8 sem BOM (o bundler grava o proprio; dois BOMs quebram o makensis)' {
        . (Join-Path $PSScriptRoot 'ci-guards.ps1')
        foreach ($n in @('PortugueseBR.nsh', 'installer.nsi')) {
            Assert-Eq (Get-Utf8BomCount (Join-Path $winDir $n)) 0 "BOMs na fonte $n"
        }
    }

    Test-Case 'chaves tecnicas do template: UNINSTKEY fixa em Uninstall\Sofa e as demais iguais as da 0.1.0 (R-11, ADR-0007)' {
        $tpl = Join-Path $winDir 'installer.nsi'
        $code = ((Get-Content -LiteralPath $tpl -Encoding UTF8) | Where-Object { $_.TrimStart() -notmatch '^[;#]' }) -join "`n"
        Assert-SofaTechnicalKeys $code 'template'
    }

    Test-Case 'nome visivel SofaLauncher no template e no harness; atalhos saem de DISPLAYNAME (D-49, BE-62)' {
        foreach ($f in @((Join-Path $winDir 'installer.nsi'), (Join-Path $winDir 'tests\harness.nsi'))) {
            $code = ((Get-Content -LiteralPath $f -Encoding UTF8) | Where-Object { $_.TrimStart() -notmatch '^[;#]' }) -join "`n"
            $m = [regex]::Matches($code, '(?m)^\s*!define DISPLAYNAME\s+"([^"]*)"')
            Assert-Eq $m.Count 1 "definicoes de DISPLAYNAME em $(Split-Path -Leaf $f)"
            Assert-Eq $m[0].Groups[1].Value 'SofaLauncher' "DISPLAYNAME em $(Split-Path -Leaf $f)"
        }
        $tplCode = ((Get-Content -LiteralPath (Join-Path $winDir 'installer.nsi') -Encoding UTF8) | Where-Object { $_.TrimStart() -notmatch '^[;#]' }) -join "`n"
        # o nome do atalho criado e sempre ${DISPLAYNAME}.lnk (SofaLauncher.lnk); nada de nome literal
        foreach ($l in ($tplCode -split "`n" | Where-Object { $_ -match 'CreateShortcut' })) {
            Assert-True ($l -match '\$\{DISPLAYNAME\}\.lnk') "atalho sem DISPLAYNAME: $l"
        }
    }
    # =============================================================== o instalador gerado
    Write-Host "`n== Instalador gerado (se existir)" -ForegroundColor Cyan
    $gen = Get-ChildItem -LiteralPath (Join-Path $root 'target') -Recurse -Filter installer.nsi -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match 'release[\\/]nsis' } | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($gen) {
        . (Join-Path $PSScriptRoot 'ci-guards.ps1')
        Test-Case "guards de NSIS no script gerado ($($gen.Name))" {
            $v = @(Invoke-GeneratedNsisGuard $gen.FullName)
            Assert-True ($v.Count -eq 0) ("violacoes: " + ($v -join '; '))
        }
        Test-Case 'script e idioma gerados: exatamente um BOM e acentos em UTF-8, sem mojibake' {
            $dir = Split-Path -Parent $gen.FullName
            foreach ($n in @('installer.nsi', 'PortugueseBR.nsh')) {
                Assert-Eq (Get-Utf8BomCount (Join-Path $dir $n)) 1 "BOMs em $n gerado"
            }
            # o nome novo e ASCII, mas as outras strings seguem com acento ("ja", "sofa"): "a" agudo em UTF-8 e
            # C3 A1 (nunca C3 83 C2 A1, que e o mojibake de um arquivo lido como ANSI)
            $acute = [byte[]](0xC3, 0xA1)
            $moji = [byte[]](0xC3, 0x83, 0xC2, 0xA1)
            function Find-Bytes([byte[]]$Hay, [byte[]]$Needle) {
                for ($i = 0; $i -le $Hay.Length - $Needle.Length; $i++) {
                    $ok = $true
                    for ($j = 0; $j -lt $Needle.Length; $j++) { if ($Hay[$i + $j] -ne $Needle[$j]) { $ok = $false; break } }
                    if ($ok) { return $true }
                }
                return $false
            }
            foreach ($n in @('installer.nsi', 'PortugueseBR.nsh')) {
                $b = [System.IO.File]::ReadAllBytes((Join-Path $dir $n))
                Assert-True (Find-Bytes $b $acute) "$n gerado sem os acentos em UTF-8"
                Assert-True (-not (Find-Bytes $b $moji)) "$n gerado com acento em mojibake"
            }
            $t = [System.IO.File]::ReadAllText($gen.FullName, [System.Text.Encoding]::UTF8)
            Assert-True ($t.Contains('!define MANUFACTURER "SofaLauncher"')) 'publisher SofaLauncher ausente (BE-61)'
            Assert-True ($t.Contains('!define COPYRIGHT "Copyright (c) SofaLauncher"')) 'copyright SofaLauncher ausente (BE-61)'
            Assert-True ($t.Contains('!define DISPLAYNAME "SofaLauncher"')) 'nome exibido SofaLauncher ausente (BE-62)'
        }
        Test-Case 'script gerado: chaves tecnicas iguais as da 0.1.0 (UNINSTKEY literal Uninstall\Sofa, R-11)' {
            $t = ((Get-Content -LiteralPath $gen.FullName -Encoding UTF8) | Where-Object { $_.TrimStart() -notmatch '^[;#]' }) -join "`n"
            Assert-SofaTechnicalKeys $t 'script gerado'
        }
        Test-Case 'script gerado: sem pagina de pasta e com a pasta fixa' {
            $t = ((Get-Content -LiteralPath $gen.FullName -Encoding UTF8) | Where-Object { $_.TrimStart() -notmatch '^[;#]' }) -join "`n"
            Assert-True ($t -notmatch 'MUI_PAGE_DIRECTORY') 'pagina de pasta no script gerado'
            Assert-True ($t -match 'Call SofaFixInstallDir') 'o .onInit gerado nao fixa a pasta'
            Assert-True ($t -match 'Call un\.SofaFixInstallDir') 'o un.onInit gerado nao fixa a pasta'
            Assert-True ($t -notmatch '\{\{') 'chaves do template sem renderizar no script gerado'
        }
        Test-Case 'script gerado sem ACL, sem HKCU e sem apagar dados' {
            # sem as linhas de comentario (o cabecalho do template cita o que saiu)
            $t = ((Get-Content -LiteralPath $gen.FullName -Encoding UTF8) | Where-Object { $_.TrimStart() -notmatch '^[;#]' }) -join "`n"
            foreach ($bad in @('AccessControl::', 'icacls', 'cacls', 'WriteReg\w+\s+HKCU', 'DeleteReg\w+\s+HKCU', 'RmDir\s+/r(\s|$)')) {
                Assert-True ($t -notmatch $bad) "o script gerado contem $bad"
            }
            Assert-True ($t -match 'perMachine') 'installMode perMachine ausente'
            Assert-True ($t -match '!define BUNDLEID "app\.sofa\.launcher"') 'identificador app.sofa.launcher ausente'
            Assert-True ($t.Contains('!define MANUPRODUCTKEY "Software\${BUNDLEID}"')) 'chave HKLM\Software\app.sofa.launcher ausente'
        }
        # conteudo do instalador (so lista; nao executa nada)
        $setup = Get-ChildItem -LiteralPath (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $gen.FullName))) -Recurse -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($setup) {
            Test-Case "nome SofaLauncher no instalador compilado ($($setup.Name))" {
                # ProductName, FileDescription e LegalCopyright saem do mesmo .nsi (DISPLAYNAME e copyright): se o
                # makensis leu o arquivo como ANSI, as strings viriam em mojibake; o nome tem de ser SofaLauncher (D-49)
                $vi = (Get-Item -LiteralPath $setup.FullName).VersionInfo
                Assert-Eq $vi.ProductName 'SofaLauncher' 'ProductName'
                Assert-Eq $vi.FileDescription 'SofaLauncher' 'FileDescription'
                Assert-Eq $vi.LegalCopyright 'Copyright (c) SofaLauncher' 'LegalCopyright'
            }
        }
        $sevenZip = @("$env:ProgramFiles\7-Zip\7z.exe", "${env:ProgramFiles(x86)}\7-Zip\7z.exe") | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
        if ($setup -and $sevenZip) {
            Test-Case "conteudo do instalador ($($setup.Name))" {
                $list = (& $sevenZip l $setup.FullName) -join "`n"
                foreach ($need in @('sofa.exe', 'licenses\OFL-Figtree.txt', 'licenses\OFL-Sora.txt', 'uninstall.exe', 'MicrosoftEdgeWebview2Setup.exe', 'nsis_tauri_utils.dll')) {
                    Assert-True ($list -match [regex]::Escape($need)) "$need ausente do instalador"
                }
                # carregador do WebView2 e estatico no MSVC (SEC-23): nenhuma DLL do app ao lado do exe; o jogo de teste nao entra
                foreach ($bad in @('WebView2Loader.dll', 'sofa-test-game', 'sofa_test_game')) {
                    Assert-True ($list -notmatch [regex]::Escape($bad)) "$bad nao devia estar no instalador"
                }
            }
        } else {
            Skip-Case 'conteudo do instalador' '7-Zip ou instalador nao encontrado'
        }
    } else {
        Skip-Case 'script gerado' 'rode `tauri build` antes (scripts/ci.ps1 -Release)'
    }
}
finally {
    # pontos de reanalise primeiro (junction: rmdir; arquivo: del), sem seguir o destino
    $ErrorActionPreference = 'Continue'
    Get-ChildItem -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue |
        Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint } | ForEach-Object {
            if ($_.PSIsContainer) { cmd /c "rmdir `"$($_.FullName)`" >nul 2>&1" } else { cmd /c "del /f /q `"$($_.FullName)`" >nul 2>&1" }
        }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host ("{0} passaram, {1} falharam, {2} pulados" -f $script:passed, $script:fail.Count, $script:skipped.Count)
foreach ($s in $script:skipped) { Write-Host ("  pulado: {0}" -f $s) -ForegroundColor Yellow }
if ($script:fail.Count -gt 0) {
    Write-Host ("FALHARAM: " + ($script:fail -join '; ')) -ForegroundColor Red
    exit 1
}
Write-Host 'TESTES DO INSTALADOR OK' -ForegroundColor Green
exit 0
