# Exports every slide of a PowerPoint file as a PNG through the installed PowerPoint (COM).
#   powershell -File tools/pptx-slides.ps1 -Pptx <file.pptx> -Out <folder>
# Used to compare a church's own slides with what Artosia's import makes of them.
param([string]$Pptx, [string]$Out)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$app = New-Object -ComObject PowerPoint.Application
try {
    # Open read-only, untitled, without a window.
    $deck = $app.Presentations.Open($Pptx, $true, $true, $false)
    $i = 0
    foreach ($slide in $deck.Slides) {
        $i++
        $slide.Export((Join-Path $Out ("slide{0}.png" -f $i)), 'PNG', 1600, 900)
    }
    $deck.Close()
    Write-Output "exported $i"
} finally {
    $app.Quit()
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($app) | Out-Null
}
