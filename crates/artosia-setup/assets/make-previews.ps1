# Writes the product pictures the setup window embeds (light and dark, one pair for each of
# its ten languages) from the renders in site/img. Only the part of the picture the window shows is
# kept: the top-left 64 by 75 percent, scaled to 656 by 480 pixels (the 328 by 240 point block at
# 200 percent). Run from the repository root: powershell -File crates/artosia-setup/assets/make-previews.ps1
Add-Type -AssemblyName System.Drawing
$out = Join-Path $PSScriptRoot "."
foreach ($lang in "en", "id", "zh-Hans", "zh-Hant", "ko", "ja", "es", "pt-BR", "fr", "vi") {
  foreach ($theme in "light", "dark") {
    $source = [System.Drawing.Image]::FromFile((Resolve-Path "site/img/$lang/hero-0-$theme.png"))
    $crop = New-Object System.Drawing.Rectangle 0, 0, ([int]($source.Width * 328 / 512)), ([int]($source.Height * 240 / 320))
    $bitmap = New-Object System.Drawing.Bitmap 656, 480
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.DrawImage($source, (New-Object System.Drawing.Rectangle 0, 0, 656, 480), $crop, [System.Drawing.GraphicsUnit]::Pixel)
    $path = Join-Path $out "preview-$lang-$theme.png"
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose(); $bitmap.Dispose(); $source.Dispose()
    "{0} {1} bytes" -f $path, (Get-Item $path).Length
  }
}
