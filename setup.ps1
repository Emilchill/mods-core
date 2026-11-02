# Build environment for Mods Core (Cinnabar mod) on Windows.
#
#   setup.ps1 install            install everything needed to build
#   setup.ps1 build              build dist\modscore.wasm
#   setup.ps1 uninstall [-Purge] [-Yes]
#                                        remove what `install` added (and only that)
#
# Put this file in the project folder (next to Cargo.toml). Run it from PowerShell:
#   powershell -ExecutionPolicy Bypass -File setup.ps1 install
# It records what it installs in %LOCALAPPDATA%\mods-core-build\installed.txt, so
# `uninstall` never removes a tool you already had.
param(
    [Parameter(Position = 0)]
    [ValidateSet('install', 'build', 'uninstall')]
    [string]$Action = 'install',
    [switch]$Purge,
    [switch]$Yes
)

$ErrorActionPreference = 'Stop'
$Toolchain = '1.99.0'
$Target    = 'wasm32-unknown-unknown'
$StateDir  = Join-Path $env:LOCALAPPDATA 'mods-core-build'
$State     = Join-Path $StateDir 'installed.txt'
# $PSScriptRoot is empty when the text is pasted into a console instead of run as a file.
$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }
if (Test-Path (Join-Path $ScriptDir 'Cargo.toml'))                   { $Root = $ScriptDir }
elseif (Test-Path (Join-Path (Split-Path -Parent $ScriptDir) 'Cargo.toml')) { $Root = Split-Path -Parent $ScriptDir }
else { throw "Cargo.toml not found. Run this from the project folder (the one containing Cargo.toml) or from its scripts folder." }

New-Item -ItemType Directory -Force -Path $StateDir | Out-Null
if (-not (Test-Path $State)) { New-Item -ItemType File -Path $State | Out-Null }

function Log($m)       { Write-Host "==> $m" -ForegroundColor Cyan }
function Record($k)    { if (-not (Recorded $k)) { Add-Content -Path $State -Value $k } }
function Recorded($k)  { (Get-Content $State -ErrorAction SilentlyContinue) -contains $k }
function Has($c)       { [bool](Get-Command $c -ErrorAction SilentlyContinue) }
function Run {
    & $args[0] @($args | Select-Object -Skip 1)
    if ($LASTEXITCODE -ne 0) { throw "command failed ($LASTEXITCODE): $($args -join ' ')" }
}
function Refresh-Path {
    $machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $user    = [Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = "$machine;$user;$env:USERPROFILE\.cargo\bin"
}
Refresh-Path

function Test-Python {
    # The Microsoft Store "python" stub exists but is not a real interpreter.
    if (-not (Has 'python')) { return $false }
    try { & python -c "print(1)" *> $null; return ($LASTEXITCODE -eq 0) } catch { return $false }
}

function Test-Msvc {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { return $false }
    $found = & $vswhere -products '*' -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64
    return [bool]$found
}

function Winget-Install($id, $key, $extra) {
    if (-not (Has 'winget')) { throw "winget not found. Install '$id' manually, then run this again." }
    Log "Installing $id"
    Record $key
    $args = @('install', '--id', $id, '-e', '--silent',
              '--accept-package-agreements', '--accept-source-agreements') + $extra
    Run winget @args
    Refresh-Path
}

function Cmd-Install {
    if (-not (Has 'git'))     { Winget-Install 'Git.Git' 'winget:Git.Git' @() }
    else                      { Log 'git already installed' }

    if (-not (Test-Python))   { Winget-Install 'Python.Python.3.12' 'winget:Python.Python.3.12' @() }
    else                      { Log 'python already installed' }

    if (-not (Test-Msvc)) {
        # The pack tool is built for the host, which needs the MSVC linker (large download).
        Winget-Install 'Microsoft.VisualStudio.2022.BuildTools' 'winget:Microsoft.VisualStudio.2022.BuildTools' `
            @('--override', '--wait --quiet --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended')
    } else { Log 'MSVC build tools already installed' }

    if (-not (Has 'rustup')) {
        Log 'Installing rustup (no PATH changes)'
        Record 'rustup'
        $exe = Join-Path $env:TEMP 'rustup-init.exe'
        Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile $exe
        Run $exe '-y' '--profile' 'minimal' '--default-toolchain' 'none' '--no-modify-path'
        Remove-Item $exe -Force
        Refresh-Path
    } else { Log 'rustup already installed' }

    $have = (& rustup toolchain list) -match "^$([regex]::Escape($Toolchain))"
    if ($have) {
        Log "Rust $Toolchain already installed"
        $targets = & rustup target list --toolchain $Toolchain --installed
        if ($targets -notcontains $Target) {
            Record 'target'
            Run rustup target add --toolchain $Toolchain $Target
        }
    } else {
        Log "Installing Rust $Toolchain with target $Target"
        Record 'toolchain'
        Run rustup toolchain install $Toolchain --profile minimal --target $Target
    }
    Log "Done. Build with: setup.ps1 build"
}

function Cmd-Build {
    if (-not (Has 'rustup')) { throw "Run 'setup.ps1 install' first." }
    Push-Location $Root
    try {
        $env:RUSTUP_TOOLCHAIN = $Toolchain
        Log "Building in $Root"
        Run python scripts\build-mod.py
        Log "Output: $Root\dist\modscore.wasm  (import via Cinnabar: Settings -> Mods -> Import)"
    } finally { Pop-Location }
}

function Cmd-Uninstall {
    $entries = @(Get-Content $State -ErrorAction SilentlyContinue)
    Log 'This will remove what the install step added:'
    if (Recorded 'rustup')    { Write-Host '  - rustup, .cargo and .rustup (installed by this script)' }
    if (Recorded 'toolchain') { Write-Host "  - Rust $Toolchain" }
    if (Recorded 'target')    { Write-Host "  - target $Target" }
    $entries | Where-Object { $_ -like 'winget:*' } | ForEach-Object { Write-Host "  - $($_.Substring(7))" }
    if ($Purge) { Write-Host "  - build output: $Root\target and $Root\dist" }
    if ($entries.Count -eq 0 -and -not $Purge) { Log 'Nothing recorded to remove.'; return }

    if (-not $Yes) {
        $reply = Read-Host 'Continue? [y/N]'
        if ($reply -notin @('y', 'Y')) { throw 'cancelled' }
    }

    if (Has 'rustup') {
        if (Recorded 'rustup') {
            Log 'Removing rustup and everything it manages'
            Run rustup self uninstall -y
        } else {
            if (Recorded 'toolchain') { Log "Removing Rust $Toolchain"; & rustup toolchain uninstall $Toolchain }
            elseif (Recorded 'target') { Log "Removing target $Target"; & rustup target remove --toolchain $Toolchain $Target }
        }
    }

    foreach ($e in $entries | Where-Object { $_ -like 'winget:*' }) {
        $id = $e.Substring(7)
        Log "Removing $id"
        & winget uninstall --id $id -e --silent
    }

    if ($Purge) {
        Log 'Removing build output'
        Remove-Item -Recurse -Force (Join-Path $Root 'target'), (Join-Path $Root 'dist') -ErrorAction SilentlyContinue
    }

    Remove-Item -Recurse -Force $StateDir
    Log 'Uninstalled.'
}

switch ($Action) {
    'install'   { Cmd-Install }
    'build'     { Cmd-Build }
    'uninstall' { Cmd-Uninstall }
}
