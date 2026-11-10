# Presses real keys and drags the real mouse on one running program's window, and saves a
# PNG after each step, for checking in a real window what a headless script cannot: keys
# as Windows sends them, a modifier being let go, a drag.
#
#   powershell -File tools/drive-window.ps1 -ProcessId 1234 -Out shots -Steps "key:SPACE shot:started down:SHIFT key:DOWN shot:held up:SHIFT shot:let-go drag:130,195,0,90 shot:dragged"
#
# Steps: key:<name>  down:<name>  up:<name>  drag:<x>,<y>,<dx>,<dy> (points inside the window)
#        shot:<name>  wait:<milliseconds>
# Input goes to whatever window is in front, so every step first checks that this program's
# window is the one in front and stops when it is not.
param([int]$ProcessId, [string]$Out, [string]$Steps)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Drive {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint mapType);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
}
"@
[Drive]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null
$keys = @{ SPACE = 0x20; ENTER = 0x0D; ESC = 0x1B; TAB = 0x09; LEFT = 0x25; UP = 0x26; RIGHT = 0x27; DOWN = 0x28; SHIFT = 0x10; CTRL = 0x11; ALT = 0x12; F1 = 0x70; F2 = 0x71; F3 = 0x72; F4 = 0x73; F5 = 0x74; HOME = 0x24; END = 0x23 }
# The arrow keys and their neighbours are "extended" keys; without the flag Windows may
# report them as number-pad keys.
$extended = @(0x25, 0x26, 0x27, 0x28, 0x24, 0x23)
$p = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
if (-not $p -or $p.MainWindowHandle -eq 0) { "no window for process $ProcessId"; exit 1 }
$h = $p.MainWindowHandle
New-Item -ItemType Directory -Force -Path $Out | Out-Null

function InFront {
    if ([Drive]::GetForegroundWindow() -eq $h) { return $true }
    [Drive]::ShowWindow($h, 9) | Out-Null
    # Windows lets a program take the front only after input; an Alt tap counts as input.
    [Drive]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [Drive]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
    [Drive]::SetForegroundWindow($h) | Out-Null
    Start-Sleep -Milliseconds 300
    return ([Drive]::GetForegroundWindow() -eq $h)
}
function Press([string]$name, [bool]$down, [bool]$up) {
    $vk = $keys[$name.ToUpper()]
    if ($null -eq $vk) { throw "unknown key $name" }
    $flag = if ($extended -contains $vk) { 1 } else { 0 }
    # The scan code a keyboard would send with it; programs that read the physical key need it.
    $scan = [byte][Drive]::MapVirtualKey([uint32]$vk, 0)
    if ($down) { [Drive]::keybd_event([byte]$vk, $scan, $flag, [UIntPtr]::Zero) }
    if ($up) { [Drive]::keybd_event([byte]$vk, $scan, ($flag -bor 2), [UIntPtr]::Zero) }
}
function Shot([string]$name) {
    $r = New-Object Drive+RECT
    [Drive]::GetWindowRect($h, [ref]$r) | Out-Null
    $bmp = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc()
    [Drive]::PrintWindow($h, $dc, 2) | Out-Null
    $g.ReleaseHdc($dc); $g.Dispose()
    $bmp.Save((Join-Path $Out "$name.png")); $bmp.Dispose()
}

$held = @()
$all = $Steps -split "\s+" | Where-Object { $_ }
try {
    foreach ($step in $all) {
        $what, $arg = $step -split ":", 2
        if ($what -ne "shot" -and $what -ne "wait" -and -not (InFront)) { "stopped at '$step': the window is not in front"; exit 2 }
        switch ($what) {
            "key" { Press $arg $true $true }
            "down" { Press $arg $true $false; $held += $arg }
            "up" { Press $arg $false $true; $held = $held | Where-Object { $_ -ne $arg } }
            "wait" { Start-Sleep -Milliseconds ([int]$arg) }
            "shot" { Start-Sleep -Milliseconds 350; Shot $arg }
            "drag" {
                $x, $y, $dx, $dy = $arg -split "," | ForEach-Object { [double]$_ }
                $scale = [Drive]::GetDpiForWindow($h) / 96.0
                $o = New-Object Drive+POINT
                [Drive]::ClientToScreen($h, [ref]$o) | Out-Null
                $fx = $o.X + $x * $scale; $fy = $o.Y + $y * $scale
                [Drive]::SetCursorPos([int]$fx, [int]$fy) | Out-Null
                Start-Sleep -Milliseconds 120
                [Drive]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
                foreach ($i in 1..12) {
                    [Drive]::SetCursorPos([int]($fx + $dx * $scale * $i / 12), [int]($fy + $dy * $scale * $i / 12)) | Out-Null
                    [Drive]::mouse_event(1, 0, 0, 0, [UIntPtr]::Zero)
                    Start-Sleep -Milliseconds 30
                }
                Start-Sleep -Milliseconds 150
                [Drive]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
            }
            default { throw "unknown step $step" }
        }
        Start-Sleep -Milliseconds 180
    }
    "done: $($all.Count) steps"
} finally {
    # Never leave a key held down for the next program.
    foreach ($name in $held) { Press $name $false $true }
}
