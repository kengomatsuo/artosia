<#
Makes an upload-ready update folder from a built release folder.

  powershell -File tools/release.ps1 -Dir target\release-folder -Version 0.2.0 -Out dist\0.2.0 -Notes "What changed"

The private key is read from -KeyFile (a file OUTSIDE the repo) or from the
ARTOSIA_UPDATE_KEY environment variable. It is never printed and never put on a command line.
Nothing is uploaded: copy the contents of -Out to the host yourself.
Bash twin (tools/release.sh) would run the same two commands: cargo build, then artosia-release build.
#>
param(
    [Parameter(Mandatory)][string]$Dir,
    [Parameter(Mandatory)][string]$Version,
    [Parameter(Mandatory)][string]$Out,
    [string]$Notes = "",
    [string]$NotesFile = "",
    [string]$KeyFile = "",
    [string]$Platform = ""
)
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
if (-not $KeyFile -and -not $env:ARTOSIA_UPDATE_KEY) { throw "Give -KeyFile or set ARTOSIA_UPDATE_KEY" }
if ($KeyFile -and ((Resolve-Path $KeyFile).Path.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase))) {
    throw "The private key must live outside the repository"
}
Push-Location $repo
try {
    cargo build --release -p artosia-update --bin artosia-release --target-dir target/release-tool
    if ($LASTEXITCODE -ne 0) { throw "Could not build the release tool" }
} finally { Pop-Location }
$tool = Join-Path $repo "target/release-tool/release/artosia-release.exe"
$toolArgs = @("build", "--dir", $Dir, "--version", $Version, "--out", $Out)
if ($NotesFile) { $toolArgs += @("--notes-file", $NotesFile) } elseif ($Notes) { $toolArgs += @("--notes", $Notes) }
if ($KeyFile) { $toolArgs += @("--key-file", $KeyFile) }
if ($Platform) { $toolArgs += @("--platform", $Platform) }
& $tool @toolArgs
if ($LASTEXITCODE -ne 0) { throw "The release was not made" }
Write-Host "Upload everything in $Out to the update host (manifest.json last)."
