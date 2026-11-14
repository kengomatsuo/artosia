# Puts the three overhaul directions side by side, one sheet per page, from target/overhaul.
#   powershell -File tools/overhaul-sheets.ps1 -Dir target/overhaul -Out target/overhaul/sheets
param([string]$Dir, [string]$Out)
# One sheet per page: directions A, B and C side by side, each labelled, 900 px wide each.
Add-Type -AssemblyName System.Drawing
$names = @{ 'a' = 'A  Inspector (ProPresenter 7)'; 'b' = 'B  Data windows (Planning Center)'; 'c' = 'C  Drawer and forms (FreeShow)' }
$pages = @{ 1 = 'Announcement editor'; 2 = 'Colour choice'; 3 = 'Weekly schedule'; 4 = 'Birthdays list'; 5 = 'Money with total'; 6 = 'Template gallery'; 7 = 'Rundown: video, audio, ending scene' }
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$w = 900; $gap = 24; $top = 110
foreach ($p in 1..7) {
    $first = [System.Drawing.Image]::FromFile((Join-Path $Dir "a-p$p.png"))
    $h = [int]($first.Height * $w / $first.Width); $first.Dispose()
    $sheet = New-Object System.Drawing.Bitmap ((3 * $w + 4 * $gap), ($top + $h + 70))
    $g = [System.Drawing.Graphics]::FromImage($sheet)
    $g.InterpolationMode = 'HighQualityBicubic'; $g.TextRenderingHint = 'AntiAlias'
    $g.Clear([System.Drawing.Color]::FromArgb(24, 24, 27))
    $big = New-Object System.Drawing.Font('Segoe UI Semibold', 30)
    $small = New-Object System.Drawing.Font('Segoe UI', 20)
    $white = [System.Drawing.Brushes]::White
    $grey = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(170, 170, 180))
    $g.DrawString("P$p  $($pages[$p])", $big, $white, $gap, 28)
    $i = 0
    foreach ($d in 'a', 'b', 'c') {
        $img = [System.Drawing.Image]::FromFile((Join-Path $Dir "$d-p$p.png"))
        $x = $gap + $i * ($w + $gap)
        $g.DrawImage($img, $x, $top, $w, $h)
        $g.DrawString($names[$d], $small, $grey, $x, $top + $h + 16)
        $img.Dispose(); $i++
    }
    $sheet.Save((Join-Path $Out "sheet-p$p.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $sheet.Dispose()
}
Write-Output "sheets written"
