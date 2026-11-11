$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class SaverSettingsUi { [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h,int id); [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h,uint message,IntPtr w,IntPtr l); }'
$root = Split-Path -Parent $PSScriptRoot
$scr = Join-Path $root 'dist/windows/Matrix Screensaver.scr'
$isolatedAppData = Join-Path $root 'dist/settings-test-appdata'
New-Item -ItemType Directory -Force -Path $isolatedAppData | Out-Null
$info = New-Object System.Diagnostics.ProcessStartInfo
$info.FileName = $scr
$info.Arguments = '/c'
$info.UseShellExecute = $false
$info.EnvironmentVariables['APPDATA'] = $isolatedAppData
$process = [System.Diagnostics.Process]::Start($info)
try {
    Start-Sleep -Seconds 1
    $process.Refresh()
    if ($process.HasExited -or $process.MainWindowTitle -ne 'Matrix Screensaver Settings') { throw 'Settings dialog did not open.' }
    $hwnd = $process.MainWindowHandle
    foreach ($pair in @(@(10,1), @(11,1), @(12,59), @(13,10))) {
        $combo = [SaverSettingsUi]::GetDlgItem($hwnd, $pair[0])
        if ($combo -eq [IntPtr]::Zero) { throw "Missing settings control $($pair[0])." }
        [void][SaverSettingsUi]::SendMessage($combo,334,[IntPtr]$pair[1],[IntPtr]::Zero)
    }
    $save = [SaverSettingsUi]::GetDlgItem($hwnd,1)
    [void][SaverSettingsUi]::SendMessage($save,245,[IntPtr]::Zero,[IntPtr]::Zero)
    if (-not $process.WaitForExit(3000)) { throw 'Settings dialog did not close after Save.' }
    $actual = Get-Content -LiteralPath (Join-Path $isolatedAppData 'Matrix Screensaver/settings.txt')
    $expected = @('--type','classic','--color','blue','--fps','60','--render-scale','0.75')
    if (($actual -join '|') -ne ($expected -join '|')) { throw "Saved settings differ: $($actual -join ', ')" }
} finally {
    if (-not $process.HasExited) { $process.Kill() }
}
