param([string]$Output = "dist/windows")
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$binary = Join-Path $root 'target/release/matrix-screensaver-scr.exe'
$consoleBinary = Join-Path $root 'target/release/matrix-screensaver.exe'
if (-not (Test-Path -LiteralPath $binary)) { throw 'Build the screensaver host with cargo build --release --locked --bin matrix-screensaver-scr --features windows-saver-host.' }
if (-not (Test-Path -LiteralPath $consoleBinary)) { throw 'Build the standalone binary with cargo build --release --locked --bin matrix-screensaver.' }

function Get-PESubsystem([string]$Path) {
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 256) { throw "$Path is too small to be a PE executable." }
    $peOffset = [BitConverter]::ToInt32($bytes, 0x3c)
    $subsystemOffset = $peOffset + 24 + 68
    if ($peOffset -lt 0 -or $subsystemOffset + 2 -gt $bytes.Length) { throw "$Path has an invalid PE header." }
    return [BitConverter]::ToUInt16($bytes, $subsystemOffset)
}

if ((Get-PESubsystem $binary) -ne 2) { throw 'The screensaver host must use the Windows GUI subsystem.' }
if ((Get-PESubsystem $consoleBinary) -ne 3) { throw 'The standalone binary must retain the Windows console subsystem.' }
New-Item -ItemType Directory -Force -Path $Output | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $Output 'Matrix Screensaver.scr')
Copy-Item -LiteralPath (Join-Path $root 'assets/matrix-screensaver.png') -Destination $Output
Copy-Item -LiteralPath (Join-Path $root 'LICENSE'), (Join-Path $root 'docs/third-party-notices.md'), (Join-Path $root 'docs/windows-install.md') -Destination $Output
