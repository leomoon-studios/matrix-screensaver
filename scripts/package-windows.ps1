param([string]$Output = "dist/windows")
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$binary = Join-Path $root 'target/release/matrix-screensaver.exe'
if (-not (Test-Path -LiteralPath $binary)) { throw 'Build the release binary first with cargo build --release --locked.' }
New-Item -ItemType Directory -Force -Path $Output | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $Output 'Matrix Screensaver.scr')
Copy-Item -LiteralPath (Join-Path $root 'assets/matrix-screensaver.png') -Destination $Output
Copy-Item -LiteralPath (Join-Path $root 'LICENSE'), (Join-Path $root 'docs/third-party-notices.md'), (Join-Path $root 'docs/windows-install.md') -Destination $Output
