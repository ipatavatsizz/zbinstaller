param(
    [string]$Executable = '',
    [int]$Port = 19225
)

$ErrorActionPreference = 'Stop'
if (![System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) { throw 'This acceptance runner requires Windows.' }
$projectDirectory = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$configuration = Get-Content -LiteralPath (Join-Path $projectDirectory 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
if (!$Executable) { $Executable = Join-Path $projectDirectory "target/release/$($configuration.mainBinaryName).exe" }
$Executable = (Resolve-Path -LiteralPath $Executable).Path
$mainConfiguration = $configuration.app.windows | Where-Object label -eq 'main'

# Refuse to attach to a different application's debugging endpoint.
$portProbe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
try { $portProbe.Start() } finally { $portProbe.Stop() }

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class InstallerUiWindows {
    public delegate bool EnumProc(IntPtr handle, IntPtr data);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr data);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr handle, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr handle, StringBuilder text, int length);
    [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr handle, int index);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr handle);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr handle);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr handle, out Rect rect);
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    public static IntPtr Find(uint processId, string title) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((handle, data) => {
            uint pid; GetWindowThreadProcessId(handle, out pid);
            var text = new StringBuilder(512); GetWindowText(handle, text, text.Capacity);
            if (pid == processId && text.ToString() == title && IsWindowVisible(handle)) { found = handle; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
'@

$testProfile = Join-Path ([System.IO.Path]::GetTempPath()) ('zbinstaller-ui-' + [guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $testProfile)
$oldBrowserArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$oldProfile = $env:WEBVIEW2_USER_DATA_FOLDER
$testProcess = $null
try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
    $env:WEBVIEW2_USER_DATA_FOLDER = $testProfile
    try { $testProcess = Start-Process -FilePath $Executable -WindowStyle Hidden -PassThru }
    finally {
        $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $oldBrowserArguments
        $env:WEBVIEW2_USER_DATA_FOLDER = $oldProfile
    }
    $handle = [IntPtr]::Zero
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        if ($testProcess.HasExited) { throw 'Installer exited before creating the main window.' }
        $handle = [InstallerUiWindows]::Find($testProcess.Id, $mainConfiguration.title)
        if ($handle -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 200
    }
    if ($handle -eq [IntPtr]::Zero) { throw 'Visible main window was not found.' }
    $style = [InstallerUiWindows]::GetWindowLong($handle, -16)
    if (($style -band 0x00040000) -or ($style -band 0x00010000) -or ($style -band 0x00020000)) { throw 'Resize/maximize/minimize window styles are still enabled.' }
    $rect = [InstallerUiWindows+Rect]::new()
    if (![InstallerUiWindows]::GetClientRect($handle, [ref]$rect)) { throw 'Could not measure the native client area.' }
    $dpi = [InstallerUiWindows]::GetDpiForWindow($handle)
    $width = ($rect.Right - $rect.Left) * 96 / $dpi
    $height = ($rect.Bottom - $rect.Top) * 96 / $dpi
    if ([Math]::Abs($width - 680) -gt 1 -or [Math]::Abs($height - 540) -gt 1) { throw "Unexpected native client size: $width x $height" }
    if ([InstallerUiWindows]::GetForegroundWindow() -ne $handle) { throw 'Main window did not receive foreground focus on startup.' }
    Write-Output "Native window: $width x $height logical pixels, DPI=$dpi; resize/maximize/minimize disabled; startup focus verified."

    & node (Join-Path $PSScriptRoot 'windows-ui.mjs') $Port
    if ($LASTEXITCODE -ne 0) { throw 'Native WebView keyboard acceptance failed.' }
    if (!$testProcess.WaitForExit(5000)) { throw 'Escape did not close the main window.' }
    Write-Output 'PASS: final Escape closed the application. No installation, uninstall, Workshop or patch actions were invoked.'
}
finally {
    if ($testProcess -and !$testProcess.HasExited) {
        [void]$testProcess.CloseMainWindow()
        if (!$testProcess.WaitForExit(3000)) { Stop-Process -Id $testProcess.Id }
    }
    # Only remove the isolated profile created above, never the real app's WebView profile.
    $resolvedProfile = [System.IO.Path]::GetFullPath($testProfile)
    $resolvedTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    if ($resolvedProfile.StartsWith($resolvedTemp, [StringComparison]::OrdinalIgnoreCase) -and [System.IO.Path]::GetFileName($resolvedProfile).StartsWith('zbinstaller-ui-')) {
        Remove-Item -LiteralPath $resolvedProfile -Recurse -Force -ErrorAction SilentlyContinue
    }
}
