# Builds the Mods Core mod into dist\modscore.wasm (Windows).
#
#   powershell -ExecutionPolicy Bypass -File build.ps1            normal build
#   powershell -ExecutionPolicy Bypass -File build.ps1 -Menu      regenerate the screen first (tools\gen_surface.py)
#   powershell -ExecutionPolicy Bypass -File build.ps1 -Clean     delete target\ and dist\ first
#
# Put it in the project folder (next to Cargo.toml) or in its scripts\ folder.
# Run setup.ps1 install once before this.
param(
    [switch]$Menu,
    [switch]$Clean
)

$ErrorActionPreference = 'Stop'
$Toolchain = '1.99.0'
$Target    = 'wasm32-unknown-unknown'

$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }
if     (Test-Path (Join-Path $ScriptDir 'Cargo.toml'))                          { $Root = $ScriptDir }
elseif (Test-Path (Join-Path (Split-Path -Parent $ScriptDir) 'Cargo.toml'))     { $Root = Split-Path -Parent $ScriptDir }
else   { throw 'Cargo.toml not found. Put this file in the project folder or its scripts folder.' }

function Log($m) { Write-Host "==> $m" -ForegroundColor Cyan }
function Run {
    & $args[0] @($args | Select-Object -Skip 1)
    if ($LASTEXITCODE -ne 0) { throw "command failed ($LASTEXITCODE): $($args -join ' ')" }
}

# Pick up tools installed by setup.ps1 in this or an earlier session.
$machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
$user    = [Environment]::GetEnvironmentVariable('Path', 'User')
$env:Path = "$machine;$user;$env:USERPROFILE\.cargo\bin"

foreach ($tool in 'rustup', 'cargo', 'python', 'git') {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "'$tool' not found. Run: powershell -ExecutionPolicy Bypass -File setup.ps1 install (then open a new PowerShell window)."
    }
}
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$hasMsvc = $false
if (Test-Path $vswhere) {
    $hasMsvc = [bool](& $vswhere -products '*' -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64)
}
if (-not $hasMsvc) {
    throw @"
The Microsoft C++ linker (link.exe) is missing. Install the Visual Studio Build Tools with the C++ workload
(run PowerShell as Administrator; it is a large download):

winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

If Build Tools or Visual Studio is already installed, open "Visual Studio Installer" -> Modify -> tick
"Desktop development with C++" -> Modify. Then open a new PowerShell window and run this script again.
"@
}
$toolchains = & rustup toolchain list
if (-not ($toolchains -match "^$([regex]::Escape($Toolchain))")) { throw "Rust $Toolchain is not installed. Run setup.ps1 install." }
$targets = & rustup target list --toolchain $Toolchain --installed
if ($targets -notcontains $Target) { throw "Target $Target is missing for Rust $Toolchain. Run setup.ps1 install." }

Push-Location $Root
try {
    $env:RUSTUP_TOOLCHAIN = $Toolchain

    if ($Clean) {
        Log 'Cleaning target\ and dist\'
        Remove-Item -Recurse -Force (Join-Path $Root 'target'), (Join-Path $Root 'dist') -ErrorAction SilentlyContinue
    }
    if ($Menu) {
        Log 'Regenerating screen'
        Run python tools\gen_surface.py
    }

    # The shipped Cargo.lock can lag behind the version in Cargo.toml, and the build uses --locked.
    # This only syncs the project's own version entries; no dependency is upgraded.
    Log 'Syncing Cargo.lock with Cargo.toml'
    Run cargo update --workspace

    Log 'Building (the first build downloads dependencies and takes a few minutes)'
    $watch = [Diagnostics.Stopwatch]::StartNew()
    Run python scripts\build-mod.py
    $watch.Stop()

    $out = Join-Path $Root 'dist\modscore.wasm'
    if (-not (Test-Path $out)) { throw "Build finished but $out was not created." }
    $size = '{0:N1} MB' -f ((Get-Item $out).Length / 1MB)
    Log ("Built {0} ({1}) in {2:N0}s" -f $out, $size, $watch.Elapsed.TotalSeconds)
    Write-Host 'Import it in Cinnabar: Settings -> Mods -> Import'
    Start-Process explorer.exe "/select,`"$out`""
} finally {
    Pop-Location
}
