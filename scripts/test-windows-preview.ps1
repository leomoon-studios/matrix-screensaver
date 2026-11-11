param([string]$AppDataDirectory)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class PreviewWin32 { [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hwnd); }'
$scr = Join-Path (Split-Path -Parent $PSScriptRoot) 'dist/windows/Matrix Screensaver.scr'
if (-not (Test-Path -LiteralPath $scr)) { throw 'Package the Windows screensaver first.' }
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Matrix preview host test'
$form.ClientSize = New-Object System.Drawing.Size(640,360)
$form.StartPosition = 'CenterScreen'
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 5000
$timer.Add_Tick({
    $timer.Stop()
    $script:previewAliveBeforeHostClose = $script:previewProcess -and -not $script:previewProcess.HasExited
    $bmp = New-Object System.Drawing.Bitmap(640,360)
    $graphics = [System.Drawing.Graphics]::FromImage($bmp)
    $graphics.CopyFromScreen($form.PointToScreen([System.Drawing.Point]::Empty), [System.Drawing.Point]::Empty, $form.ClientSize)
    $bmp.Save((Join-Path (Split-Path -Parent $PSScriptRoot) 'dist/windows/preview.png'))
    $graphics.Dispose()
    $bmp.Dispose()
    $form.Close()
})
$form.Add_Shown({
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $scr
    $info.Arguments = "/p $($form.Handle.ToInt64())"
    $info.UseShellExecute = $false
    if ($AppDataDirectory) { $info.EnvironmentVariables['APPDATA'] = $AppDataDirectory }
    $script:previewProcess = [System.Diagnostics.Process]::Start($info)
    $timer.Start()
})
$form.ShowDialog() | Out-Null
$hostHandle = $form.Handle
$form.Dispose()
Write-Output "Host window alive after dispose: $([PreviewWin32]::IsWindow($hostHandle))"
if (-not $script:previewAliveBeforeHostClose) {
    throw 'Preview process exited before its host window closed.'
}
if ($script:previewProcess -and -not $script:previewProcess.WaitForExit(5000)) {
    $script:previewProcess.Kill()
    throw 'Preview process did not exit when its host window closed.'
}
