$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$scr = Join-Path $root 'dist/windows/Matrix Screensaver.scr'
$screens = [System.Windows.Forms.Screen]::AllScreens
if ($screens.Count -lt 1) { throw 'No interactive Windows display.' }
$process = Start-Process -FilePath $scr -ArgumentList '/s' -PassThru -WindowStyle Normal
try {
    Start-Sleep -Seconds 4
    if ($process.HasExited) { throw "Screensaver exited during startup: $($process.ExitCode)" }
    $screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = New-Object System.Drawing.Bitmap($screen.Width, $screen.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($screen.Location, [System.Drawing.Point]::Empty, $screen.Size)
        $bitmap.Save((Join-Path $root 'dist/windows/fullscreen.png'))
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
    [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    if (-not $process.WaitForExit(3000)) { throw 'Escape did not dismiss the screensaver.' }
    if ($process.ExitCode -ne 0) { throw "Screensaver exited with code $($process.ExitCode)." }
} finally {
    if (-not $process.HasExited) { $process.Kill() }
}
