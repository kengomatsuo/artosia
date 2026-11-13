# Makes spoken test samples with Windows' own speech synthesiser, so recognition can be tried
# without a microphone. The sentences are made up; no real lyric or verse is spoken.
#
#   powershell -File tools/make-speech-sample.ps1 -Out fixtures/speech            # writes the waves
#   powershell -File tools/make-speech-sample.ps1 -List                           # lists the voices
#
# Writes <Out>/<name>.wav for each sentence whose language has an installed voice, and says
# which were skipped.
param(
    [string]$Out = "fixtures/speech",
    [switch]$List
)
Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
if ($List) {
    $synth.GetInstalledVoices() | ForEach-Object { "{0}  {1}  enabled={2}" -f $_.VoiceInfo.Name, $_.VoiceInfo.Culture, $_.Enabled }
    return
}
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path $Out).Path

# name, culture prefix, sentence
$samples = @(
    @("english-sentence", "en", "The blue lantern waits beside the wooden gate."),
    @("english-title", "en", "Hill of Morning Stars"),
    @("indonesian-sentence", "id", "Lampu biru menunggu di samping pintu kayu.")
)
foreach ($s in $samples) {
    $voice = $synth.GetInstalledVoices() | Where-Object { $_.Enabled -and $_.VoiceInfo.Culture.Name -like ($s[1] + "*") } | Select-Object -First 1
    if (-not $voice) { "skipped {0}: no {1} voice installed" -f $s[0], $s[1]; continue }
    $synth.SelectVoice($voice.VoiceInfo.Name)
    $path = Join-Path $Out ($s[0] + ".wav")
    $synth.SetOutputToWaveFile($path)
    $synth.Speak($s[2])
    $synth.SetOutputToNull()
    "wrote {0} with {1}" -f $path, $voice.VoiceInfo.Name
}
