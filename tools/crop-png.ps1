# Cuts a rectangle out of a PNG, given as fractions of its size, and may scale the result.
#   powershell -File tools/crop-png.ps1 -In a.png -Out b.png -X0 0.1 -Y0 0.1 -X1 0.9 -Y1 0.9 [-Width 1200]
param(
  [Parameter(Mandatory)][string]$In,
  [Parameter(Mandatory)][string]$Out,
  [double]$X0 = 0, [double]$Y0 = 0, [double]$X1 = 1, [double]$Y1 = 1,
  [int]$Width = 0
)
Add-Type -AssemblyName System.Drawing
$src = [System.Drawing.Image]::FromFile((Resolve-Path $In))
$x = [int]($src.Width * $X0); $y = [int]($src.Height * $Y0)
$w = [int]($src.Width * ($X1 - $X0)); $h = [int]($src.Height * ($Y1 - $Y0))
$ow = if ($Width -gt 0) { $Width } else { $w }
$oh = [int]($h * $ow / $w)
$bmp = New-Object System.Drawing.Bitmap $ow, $oh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = 'HighQualityBicubic'
$g.PixelOffsetMode = 'HighQuality'
$from = New-Object System.Drawing.Rectangle $x, $y, $w, $h
$to = New-Object System.Drawing.Rectangle 0, 0, $ow, $oh
$g.DrawImage($src, $to, $from, [System.Drawing.GraphicsUnit]::Pixel)
$g.Dispose(); $src.Dispose()
$full = [System.IO.Path]::GetFullPath([System.IO.Path]::Combine((Get-Location).Path, $Out))
$bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
