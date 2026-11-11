# Answers a system file window (Save, Open, pick a folder) that one running program opened,
# for checking in a real window what a headless script answers by itself.
#
#   powershell -File tools/answer-dialog.ps1 -ProcessId 1234 -Text "C:\some\folder\file.zip" -Enter
#   powershell -File tools/answer-dialog.ps1 -ProcessId 1234 -Text "C:\some\folder" -Enter -Again
#
# It sets the text of the focused field, reads it back, and presses Enter. A folder window
# goes into the folder on the first Enter and picks it on the second, so pass -Again there.
# The text is set, never typed: typed keys lost a character in a long path on 2026-11-11.
# It stops when the window in front belongs to another program or is the program's main
# window, or when the focused control is not a text field.
param([int]$ProcessId, [string]$Text, [switch]$Enter, [switch]$Again)
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class Field {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct GUITHREADINFO { public int cbSize; public int flags; public IntPtr hwndActive, hwndFocus, hwndCapture, hwndMenuOwner, hwndMoveSize, hwndCaret; public RECT rcCaret; }
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool GetGUIThreadInfo(uint thread, ref GUITHREADINFO info);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder text, int max);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, uint message, IntPtr w, string l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, uint message, IntPtr w, StringBuilder l);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
}
"@
$p = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
if (-not $p) { "no process $ProcessId"; exit 1 }
$h = [Field]::GetForegroundWindow(); $owner = 0
$thread = [Field]::GetWindowThreadProcessId($h, [ref]$owner)
if ($owner -ne $ProcessId -or $h -eq $p.MainWindowHandle) { "the window in front is not a dialog of process $ProcessId"; exit 1 }
$info = New-Object Field+GUITHREADINFO; $info.cbSize = [System.Runtime.InteropServices.Marshal]::SizeOf($info)
[Field]::GetGUIThreadInfo($thread, [ref]$info) | Out-Null
$class = New-Object System.Text.StringBuilder 100; [Field]::GetClassName($info.hwndFocus, $class, 100) | Out-Null
if ($class.ToString() -ne "Edit") { "the focused control is '$class', not a text field"; exit 1 }
[Field]::SendMessage($info.hwndFocus, 0x000C, [IntPtr]::Zero, $Text) | Out-Null
$back = New-Object System.Text.StringBuilder 600; [Field]::SendMessage($info.hwndFocus, 0x000D, [IntPtr]600, $back) | Out-Null
if ($back.ToString() -ne $Text) { "the field reads something else: $($back.Length) characters"; exit 1 }
"the field holds the text ($($back.Length) characters)"
function PressEnter { [Field]::keybd_event(0x0D, 0, 0, [UIntPtr]::Zero); [Field]::keybd_event(0x0D, 0, 2, [UIntPtr]::Zero) }
if ($Enter) { PressEnter; "Enter pressed" }
if ($Enter -and $Again) {
    Start-Sleep -Milliseconds 1500
    if ([Field]::GetForegroundWindow() -eq $h) { PressEnter; "Enter pressed again" } else { "the window had closed after the first Enter" }
}
