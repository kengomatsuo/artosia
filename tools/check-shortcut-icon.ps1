# Shows, in a real Explorer window, that a shortcut takes a new icon after an update.
#
#   powershell -File tools/check-shortcut-icon.ps1 -Old <artosia.exe with the old icon> -New <artosia.exe with the new one> -Out <folder for the pictures>
#
# A scratch folder under target\ holds a copy of the old program and a shortcut to it, written
# as the setup writes it. An Explorer window shows the shortcut. The program is swapped the way
# the updater swaps it (the old file renamed aside, the new one renamed into its place), and then
# Artosia's own code gives the shortcut the icon file (the ignored test
# `a_prepared_shortcut_takes_the_shipped_icon`, which calls what a start of Artosia calls).
# A picture of the window is saved after each step: 1 the old icon, 2 still the old icon after
# the swap (what Windows does by itself), 3 the new icon. Neither program is ever started, and
# nothing outside the scratch folder is touched.
param([Parameter(Mandatory)][string]$Old, [Parameter(Mandatory)][string]$New, [Parameter(Mandatory)][string]$Out)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Look {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
}
"@
[Look]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null
$repo = Split-Path -Parent $PSScriptRoot
$root = Join-Path $repo 'target\shortcut-icon-check'
$app = Join-Path $root 'app'
$links = Join-Path $root 'links'
Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $app, $links, $Out | Out-Null
$exe = Join-Path $app 'artosia.exe'
Copy-Item $Old $exe
Copy-Item $New (Join-Path $app 'artosia.exe.new')
$wsh = New-Object -ComObject WScript.Shell
$lnk = $wsh.CreateShortcut((Join-Path $links 'Artosia.lnk'))
$lnk.TargetPath = $exe; $lnk.WorkingDirectory = $app; $lnk.IconLocation = "$exe,0"; $lnk.Save()

$shell = New-Object -ComObject Shell.Application
$shell.Open($links)
$window = $null
foreach ($try in 1..40) {
  Start-Sleep -Milliseconds 250
  $window = $shell.Windows() | Where-Object { try { $_.Document.Folder.Self.Path -eq $links } catch { $false } } | Select-Object -First 1
  if ($window) { break }
}
if (-not $window) { 'no Explorer window opened'; exit 1 }
$window.Width = 560; $window.Height = 380
try { $window.Document.CurrentViewMode = 1; $window.Document.IconSize = 48 } catch {}

function Shot([string]$name) {
  Start-Sleep -Milliseconds 2500
  $handle = [IntPtr]$window.HWND
  $r = New-Object Look+RECT
  [Look]::GetWindowRect($handle, [ref]$r) | Out-Null
  $bmp = New-Object System.Drawing.Bitmap ($r.R - $r.L), ($r.B - $r.T)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $dc = $g.GetHdc()
  # 2 is PW_RENDERFULLCONTENT, as in tools/grab-window.ps1.
  [Look]::PrintWindow($handle, $dc, 2) | Out-Null
  $g.ReleaseHdc($dc); $g.Dispose()
  # Only the file pane is kept: the folder list beside it is the person's own.
  $pane = New-Object System.Drawing.Rectangle ([int]($bmp.Width * 0.40)), ([int]($bmp.Height * 0.44)), ([int]($bmp.Width * 0.58)), ([int]($bmp.Height * 0.44))
  $cut = $bmp.Clone($pane, $bmp.PixelFormat)
  $cut.Save((Join-Path $Out "$name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
  $cut.Dispose(); $bmp.Dispose()
  "shot $name"
}

Shot '1-old-program'
Rename-Item $exe 'artosia.exe.artosia-old'
Rename-Item (Join-Path $app 'artosia.exe.new') 'artosia.exe'
Shot '2-after-the-update-swapped-it'
$env:ARTOSIA_ICON_TEST = $root
Push-Location $repo
cargo test -q -p artosia a_prepared_shortcut -- --ignored 2>&1 | Select-String 'test result' | ForEach-Object { $_.Line.Trim() }
Pop-Location
Shot '3-after-artosia-pointed-the-shortcut-at-its-icon-file'
$window.Quit()
Start-Sleep -Milliseconds 800
Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
'closed and removed: {0}' -f (-not (Test-Path $root))
