<#
.SYNOPSIS
  Junta os logs do Sofa e dados tecnicos do PC num zip para o QA analisar (QA-16/QA-17).

.DESCRIPTION
  Le (nunca altera) a pasta de dados do Sofa em %LOCALAPPDATA%\app.sofa.launcher e monta um zip
  com o minimo necessario. Nada sai do PC sozinho: o zip fica na area de trabalho e voce o envia.

  Entra no zip:
    - logs\*            logs do Sofa (SEC-18: so IDs, nomes e caminhos de jogo, sem dado pessoal)
    - settings.json     configuracoes, incluindo a secao "advanced" (degraus 2 e 3, nivel do log)
    - sistema.txt       versao e build do Windows, processador, memoria, placa de video, telas
                        (resolucao e escala), versao do WebView2, `powercfg /a` (se existe Suspender)
    - sofa.txt          onde o Sofa esta instalado, se roda elevado (integridade), tarefa de inicio
                        automatico (SID mascarado) e valor Run
    - LEIA-ME.txt       lista do que entrou e do que ficou de fora

  Fica de fora: library.json (lista de jogos), cache\ (capas e icones), EBWebView\, nome de usuario,
  nome do computador, enderecos de rede e qualquer arquivo seu. No texto coletado, o nome do usuario
  e do computador viram <usuario> e <pc>. Use -IncluirBiblioteca so se o QA pedir.

.PARAMETER Destino
  Pasta onde o zip e criado. Padrao: area de trabalho.
.PARAMETER IncluirBiblioteca
  Inclui library.json (nomes e caminhos dos jogos cadastrados e dos ocultos).

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File .\coletar-logs.ps1
#>
[CmdletBinding()]
param(
    [string]$Destino = [Environment]::GetFolderPath('Desktop'),
    [switch]$IncluirBiblioteca
)

$ErrorActionPreference = 'Continue'
$dados = Join-Path $env:LOCALAPPDATA 'app.sofa.launcher'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("sofa-coleta-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null

$usuario = $env:USERNAME
$pc = $env:COMPUTERNAME

function Mascarar([string]$texto) {
    if ([string]::IsNullOrEmpty($texto)) { return $texto }
    if ($usuario) { $texto = $texto -replace [regex]::Escape($usuario), '<usuario>' }
    if ($pc) { $texto = $texto -replace [regex]::Escape($pc), '<pc>' }
    # SID completo vira S-1-5-21-<...>-<RID>; o RID (500+) basta para saber se e conta comum.
    $texto = $texto -replace 'S-1-5-21-\d+-\d+-\d+-(\d+)', 'S-1-5-21-<...>-$1'
    # Enderecos IPv4 e MAC nao tem motivo para estar aqui.
    $texto = $texto -replace '\b\d{1,3}(\.\d{1,3}){3}\b', '<ip>'
    $texto = $texto -replace '\b([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b', '<mac>'
    return $texto
}

function Copiar-Texto([string]$origem, [string]$destinoRelativo) {
    $alvo = Join-Path $tmp $destinoRelativo
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $alvo) | Out-Null
    $bytes = $null
    try {
        # Abre com compartilhamento total: o Sofa pode estar com o log aberto.
        $fs = [IO.File]::Open($origem, 'Open', 'Read', 'ReadWrite,Delete')
        try { $ms = New-Object IO.MemoryStream; $fs.CopyTo($ms); $bytes = $ms.ToArray() } finally { $fs.Dispose() }
    } catch {
        "(nao foi possivel ler: $($_.Exception.Message))" | Set-Content -Encoding UTF8 -LiteralPath $alvo
        return
    }
    $texto = [Text.Encoding]::UTF8.GetString($bytes)
    [IO.File]::WriteAllText($alvo, (Mascarar $texto), (New-Object Text.UTF8Encoding($false)))
}

$incluidos = New-Object System.Collections.Generic.List[string]
$ausentes = New-Object System.Collections.Generic.List[string]

# ------------------------------------------------------------------ logs e configuracao
if (Test-Path -LiteralPath $dados) {
    $logs = Join-Path $dados 'logs'
    if (Test-Path -LiteralPath $logs) {
        # So arquivos comuns, ate 20 MB cada; links e pastas dentro de logs\ sao ignorados.
        Get-ChildItem -LiteralPath $logs -File -Force -ErrorAction SilentlyContinue |
            Where-Object { -not ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -and $_.Length -le 20MB } |
            ForEach-Object { Copiar-Texto $_.FullName (Join-Path 'logs' $_.Name); $incluidos.Add("logs\$($_.Name)") }
    } else { $ausentes.Add('logs\ (pasta nao existe)') }

    $settings = Join-Path $dados 'settings.json'
    if (Test-Path -LiteralPath $settings) { Copiar-Texto $settings 'settings.json'; $incluidos.Add('settings.json') }
    else { $ausentes.Add('settings.json') }

    if ($IncluirBiblioteca) {
        $lib = Join-Path $dados 'library.json'
        if (Test-Path -LiteralPath $lib) { Copiar-Texto $lib 'library.json'; $incluidos.Add('library.json (a pedido)') }
    }
} else {
    $ausentes.Add("pasta de dados $dados (o Sofa ainda nao foi aberto neste usuario)")
}

# ------------------------------------------------------------------ sistema
$sis = New-Object System.Collections.Generic.List[string]
try {
    $os = Get-CimInstance Win32_OperatingSystem
    $sis.Add("Windows: $($os.Caption) versao $($os.Version) build $($os.BuildNumber) ($($os.OSArchitecture))")
    $disp = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion' -ErrorAction SilentlyContinue).DisplayVersion
    if ($disp) { $sis.Add("Versao de recursos: $disp") }
} catch { $sis.Add("Windows: erro ao ler ($($_.Exception.Message))") }
try {
    $cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
    $sis.Add("Processador: $($cpu.Name.Trim()) ($($cpu.NumberOfLogicalProcessors) logicos)")
    $mem = Get-CimInstance Win32_ComputerSystem
    $sis.Add("Memoria: {0:N1} GB" -f ($mem.TotalPhysicalMemory / 1GB))
    Get-CimInstance Win32_VideoController | ForEach-Object {
        $sis.Add("Video: $($_.Name) | driver $($_.DriverVersion) | $($_.CurrentHorizontalResolution)x$($_.CurrentVerticalResolution)")
    }
} catch { $sis.Add("Hardware: erro ao ler ($($_.Exception.Message))") }
try {
    Add-Type -AssemblyName System.Windows.Forms -ErrorAction Stop
    $sis.Add('Telas:')
    foreach ($t in [System.Windows.Forms.Screen]::AllScreens) {
        $sis.Add(("  {0} | {1}x{2} em ({3},{4}) | principal: {5}" -f ($t.DeviceName -replace '^\\\\\.\\', ''), $t.Bounds.Width, $t.Bounds.Height, $t.Bounds.X, $t.Bounds.Y, $t.Primary))
    }
    $dpi = (Get-ItemProperty 'HKCU:\Control Panel\Desktop\WindowMetrics' -ErrorAction SilentlyContinue).AppliedDPI
    if ($dpi) { $sis.Add(("Escala do Windows: {0}% (DPI {1})" -f [math]::Round($dpi / 96 * 100), $dpi)) }
} catch { $sis.Add("Telas: erro ao ler ($($_.Exception.Message))") }
try {
    $wv = $null
    foreach ($k in 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
                   'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
                   'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}') {
        $v = (Get-ItemProperty $k -ErrorAction SilentlyContinue).pv
        if ($v) { $wv = $v; break }
    }
    $sis.Add("WebView2 Runtime: " + $(if ($wv) { $wv } else { 'nao encontrado' }))
} catch { }
$sis.Add('')
$sis.Add('--- powercfg /a (estados de energia; mostra se Suspender existe) ---')
try { $sis.Add((& "$env:SystemRoot\System32\powercfg.exe" /a 2>&1 | Out-String)) } catch { $sis.Add("powercfg: erro ($($_.Exception.Message))") }
$sis.Add('--- controles Xbox/XInput (dispositivos HID ligados) ---')
try {
    Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue |
        Where-Object { $_.FriendlyName -match 'Xbox|XINPUT|Gamepad|Game Controller|controle para jogos|Wireless Controller' } |
        ForEach-Object { $sis.Add("  $($_.Class) | $($_.FriendlyName)") }
} catch { }
$sis.Add('--- audio (dispositivos ativos, entrada e saida) ---')
try {
    Get-PnpDevice -Class AudioEndpoint -Status OK -ErrorAction SilentlyContinue |
        ForEach-Object { $sis.Add("  $($_.FriendlyName)") }
} catch { }
[IO.File]::WriteAllText((Join-Path $tmp 'sistema.txt'), (Mascarar ($sis -join "`r`n")), (New-Object Text.UTF8Encoding($false)))
$incluidos.Add('sistema.txt')

# ------------------------------------------------------------------ Sofa instalado e integridade
$sofa = New-Object System.Collections.Generic.List[string]
$pf = ${env:ProgramFiles}
$exes = @()
$candidato = Join-Path $pf ('Sofa\sofa.exe')
if (Test-Path -LiteralPath $candidato) { $exes += $candidato }
try {
    $inst = (Get-ItemProperty 'HKLM:\Software\app.sofa.launcher' -ErrorAction SilentlyContinue)
    if ($inst) {
        $sofa.Add("HKLM\Software\app.sofa.launcher: InstallLocation=$($inst.InstallLocation) InstallDefaults=$($inst.InstallDefaults)")
    } else { $sofa.Add('HKLM\Software\app.sofa.launcher: ausente') }
} catch { }
foreach ($e in $exes) {
    $vi = (Get-Item -LiteralPath $e).VersionInfo
    $sofa.Add("Executavel: $e | versao $($vi.FileVersion) | $((Get-Item -LiteralPath $e).Length) bytes")
    $sofa.Add("SHA-256: $((Get-FileHash -LiteralPath $e -Algorithm SHA256).Hash)")
    $sofa.Add("ACL da pasta de instalacao:")
    $sofa.Add((& "$env:SystemRoot\System32\icacls.exe" (Split-Path -Parent $e) 2>&1 | Out-String))
}
if (-not $exes) { $sofa.Add('Sofa nao encontrado em Program Files (build de teste fora do instalador?)') }

$integridade = @'
using System;
using System.Runtime.InteropServices;
public static class SofaTok {
  [DllImport("advapi32.dll", SetLastError=true)] static extern bool OpenProcessToken(IntPtr h, uint a, out IntPtr t);
  [DllImport("advapi32.dll", SetLastError=true)] static extern bool GetTokenInformation(IntPtr t, int c, IntPtr b, int n, out int r);
  [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
  public static string Nivel(IntPtr proc) {
    IntPtr t; if (!OpenProcessToken(proc, 0x8, out t)) return "sem acesso";
    try {
      int len; GetTokenInformation(t, 25, IntPtr.Zero, 0, out len);
      IntPtr b = Marshal.AllocHGlobal(len);
      try {
        if (!GetTokenInformation(t, 25, b, len, out len)) return "erro";
        IntPtr sid = Marshal.ReadIntPtr(b);
        int n = Marshal.ReadByte(sid, 1);
        int rid = Marshal.ReadInt32(sid, 8 + (n - 1) * 4);
        string nome = rid >= 0x4000 ? "System" : rid >= 0x3000 ? "High" : rid >= 0x2000 ? "Medium" : "Low";
        int el; int r2;
        IntPtr b2 = Marshal.AllocHGlobal(4);
        try { GetTokenInformation(t, 20, b2, 4, out r2); el = Marshal.ReadInt32(b2); } finally { Marshal.FreeHGlobal(b2); }
        return nome + " (rid 0x" + rid.ToString("X") + "), elevado=" + (el != 0);
      } finally { Marshal.FreeHGlobal(b); }
    } finally { CloseHandle(t); }
  }
}
'@
try {
    Add-Type -TypeDefinition $integridade -ErrorAction Stop
    $procs = Get-Process -Name sofa -ErrorAction SilentlyContinue
    if (-not $procs) { $sofa.Add('Processo sofa.exe: nao esta rodando agora (abra o Sofa e rode de novo para registrar a integridade)') }
    foreach ($p in $procs) {
        $sofa.Add("Processo sofa.exe PID $($p.Id): integridade $([SofaTok]::Nivel($p.Handle)) | memoria privada $([math]::Round($p.PrivateMemorySize64/1MB)) MB | iniciado $($p.StartTime.ToString('s'))")
    }
} catch { $sofa.Add("Integridade: erro ($($_.Exception.Message))") }

$sofa.Add('')
$sofa.Add('--- tarefa agendada de inicio automatico (do usuario atual) ---')
try {
    $tarefas = Get-ScheduledTask -ErrorAction SilentlyContinue | Where-Object { $_.TaskName -like 'Sofa Autostart*' }
    if (-not $tarefas) { $sofa.Add('nenhuma tarefa "Sofa Autostart (...)"') }
    foreach ($t in $tarefas) {
        $sofa.Add("Nome: $($t.TaskName) | estado $($t.State)")
        $sofa.Add("Principal: UserId=$($t.Principal.UserId) LogonType=$($t.Principal.LogonType) RunLevel=$($t.Principal.RunLevel)")
        foreach ($a in $t.Actions) { $sofa.Add("Acao: $($a.Execute) $($a.Arguments)") }
        $info = $t | Get-ScheduledTaskInfo -ErrorAction SilentlyContinue
        if ($info) { $sofa.Add(("Ultima execucao: {0} | resultado 0x{1:X}" -f $info.LastRunTime, $info.LastTaskResult)) }
        $sofa.Add((Export-ScheduledTask -TaskName $t.TaskName -TaskPath $t.TaskPath 2>&1 | Out-String))
    }
} catch { $sofa.Add("tarefa: erro ($($_.Exception.Message))") }
$sofa.Add('--- HKCU Run e StartupApproved (so entradas do Sofa) ---')
try {
    $run = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue
    $achou = $false
    foreach ($n in $run.PSObject.Properties) { if ($n.Name -like 'Sofa*' -or "$($n.Value)" -like '*sofa.exe*') { $sofa.Add("Run: $($n.Name) = $($n.Value)"); $achou = $true } }
    if (-not $achou) { $sofa.Add('Run: nenhuma entrada do Sofa') }
    $ap = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run' -ErrorAction SilentlyContinue
    foreach ($n in $ap.PSObject.Properties) { if ($n.Name -like 'Sofa*') { $sofa.Add("StartupApproved: $($n.Name) = " + (($n.Value | ForEach-Object { $_.ToString('X2') }) -join ' ')) } }
} catch { }
[IO.File]::WriteAllText((Join-Path $tmp 'sofa.txt'), (Mascarar ($sofa -join "`r`n")), (New-Object Text.UTF8Encoding($false)))
$incluidos.Add('sofa.txt')

# ------------------------------------------------------------------ LEIA-ME e zip
$leia = @()
$leia += "Coleta do Sofa em $(Get-Date -Format 's')"
$leia += ''
$leia += 'Entrou:'
$incluidos | ForEach-Object { $leia += "  - $_" }
if ($ausentes.Count) { $leia += ''; $leia += 'Nao encontrado:'; $ausentes | ForEach-Object { $leia += "  - $_" } }
$leia += ''
$leia += 'Ficou de fora de proposito: library.json (a menos que pedido), cache\ (capas e icones), EBWebView\,'
$leia += 'nome de usuario e do computador (trocados por <usuario> e <pc>), IPs e MACs (trocados por <ip> e <mac>).'
$leia += 'Confira o conteudo antes de enviar. Nada e enviado por este script.'
[IO.File]::WriteAllText((Join-Path $tmp 'LEIA-ME.txt'), ($leia -join "`r`n"), (New-Object Text.UTF8Encoding($false)))

New-Item -ItemType Directory -Force -Path $Destino | Out-Null
$zip = Join-Path $Destino "sofa-logs-$stamp.zip"
Compress-Archive -Path (Join-Path $tmp '*') -DestinationPath $zip -Force
Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "Pronto: $zip"
Write-Host "Arquivos no zip: $($incluidos.Count + 1)"
Write-Host 'Abra o zip, confira e envie para o QA.'
