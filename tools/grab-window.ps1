# Saves a PNG of one running program's main window, drawn by the window itself, so a window
# that sits under another one is still captured as it looks.
# Usage: powershell -File tools/grab-window.ps1 -Name artosia-output -Out shot.png
# For proving that a real window shows video and text together; headless --snap cannot show video.
param([string]$Name, [string]$Out)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Grab {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
}
"@
# Per-monitor awareness (-4), so a window on a screen with another zoom reports its real pixels.
# With system awareness a 1920x1080 window on a 150% screen came back as 1600x900 and cropped.
if (-not [Grab]::SetProcessDpiAwarenessContext([IntPtr](-4))) { [Grab]::SetProcessDPIAware() | Out-Null }
$p = Get-Process -Name $Name -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { "no window for $Name"; exit 1 }
$r = New-Object Grab+RECT
[Grab]::GetWindowRect($p.MainWindowHandle, [ref]$r) | Out-Null
$w = $r.R - $r.L; $h = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
# 2 is PW_RENDERFULLCONTENT, the flag that captured a GPU-drawn Rune window here on 2026-11-10.
[Grab]::PrintWindow($p.MainWindowHandle, $dc, 2) | Out-Null
$g.ReleaseHdc($dc)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
"saved {0}x{1} to {2}" -f $w, $h, $Out
