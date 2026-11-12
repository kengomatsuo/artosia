# The manifest's three pictures, scaled from the app icon (assets/brand/artosia-icon-512.png, the
# white mark on a black plate): StoreLogo 50, Square44x44Logo 44, Square150x150Logo 150.
#   powershell -File tools/store/make-images.ps1 -Out <folder>
param([Parameter(Mandatory)][string]$Out)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$source = [System.Drawing.Image]::FromFile((Join-Path $repo "assets\brand\artosia-icon-512.png"))
New-Item -ItemType Directory -Force $Out | Out-Null
foreach ($picture in @(@("StoreLogo", 50), @("Square44x44Logo", 44), @("Square150x150Logo", 150))) {
    $size = $picture[1]
    $bitmap = New-Object System.Drawing.Bitmap $size, $size
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.InterpolationMode = 'HighQualityBicubic'
    $graphics.PixelOffsetMode = 'HighQuality'
    $graphics.SmoothingMode = 'HighQuality'
    # A mark with room around it, so the tile's edge never touches it.
    $graphics.DrawImage($source, 0, 0, $size, $size)
    $graphics.Dispose()
    $bitmap.Save((Join-Path (Resolve-Path $Out) "$($picture[0]).png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}
$source.Dispose()
