# artosia-listen

The ears of Artosia: capture an audio input, report a level, and turn phrases into text. The
operator app runs the text through its own search (`SongIndex::search`, `Bible::guess`); this
crate does no matching.

Everything below was fetched on 2026-11-10 from each project's own repository, model card or
paper. Where a number is missing from the primary sources it says so.

## Decision

- **Recogniser: whisper.cpp through `whisper-rs`.** It is the only option that is small enough
  for a Celeron *and* knows Indonesian.
- **First model: `ggml-base-q5_1.bin`** (57 MB on disk, multilingual). `ggml-tiny-q5_1.bin`
  (31 MB) is the fallback for the slowest machines, `ggml-small-q5_1.bin` (181 MB) for anything
  with four real cores.
- **Capture: `cpal`**, not GStreamer.
- **Melody recognition: not built.** There is nothing to match against (see the last section).

### What a Celeron can expect (honest)

- Spoken Bible references and announcements: workable with `base`, readable with `small`.
  Whisper's own paper measures Indonesian read speech at 33.1 % WER (base) and 16.3 % (small) on
  Fleurs, and 51.7 % for tiny. A reference like "Yohanes tiga ayat enam belas" is short and made
  of common words, so it fares better than those averages, but the numbers are the reason to
  prefer base over tiny.
- Sung words over a band or a track: expect poor text, with `base` probably worse than half the
  words wrong. No source measures Indonesian singing with these models, so this is an estimate.
  Treat the output as a few keywords for fuzzy search, never as a transcript, and never show it
  on the projector.
- Speed: the only CPU-class datapoint in the sources is a Celeron N4120 in whisper.cpp's own
  benchmark thread (issue 89, an early-2023 build with BLAS): the encoder took 12.3 s for tiny,
  83 s for small, per full 30 s window. Newer ggml code is faster and this crate encodes only the
  seconds that hold audio (`audio_ctx`), so the real figure is lower, but it was **not measured
  here** (no model on this PC, and the laptop is an i9). Plan for one read of a short phrase on
  tiny/base to take a few seconds on such a CPU, and keep the partial rate low (see the operator
  section).

## Comparison

| | Indonesian | Size | RAM | Streaming | Licence (code / model) | Windows from Rust | Maintenance |
|---|---|---|---|---|---|---|---|
| **whisper.cpp** + whisper-rs | Yes (Whisper is multilingual). Fleurs WER tiny 51.7, base 33.1, small 16.3, medium 10.2, large-v2 7.1 (Whisper paper, Table 13) | tiny 75 MB, base 142 MB, small 466 MB; q5_1: 31, 57, 181 MB (Hugging Face `ggerganov/whisper.cpp`) | README: tiny ~273 MB, base ~388 MB, small ~852 MB (unquantised) | Chunked. The `stream` example re-decodes a sliding window ("a naive example"); this crate does the same per phrase | MIT / MIT | Compiles C++ with CMake; needs Visual Studio C++, CMake and LLVM/libclang (whisper-rs `BUILDING.md`) | whisper.cpp pushed 2026-11-10, v1.9.5 (2026-10-06). whisper-rs 0.16.0 (2026-03-12), Unlicense; its GitHub repo is archived and development moved to Codeberg |
| **Vosk** | **None.** The published model list has no Indonesian | English small 40 MB (WER 9.85 LibriSpeech) | "about 300 MB" for small models | True streaming | Apache-2.0 / mostly Apache-2.0 | `vosk` crate 0.3.1 (Oct 2024) over a prebuilt libvosk DLL | vosk-api pushed 2026-10-15 |
| **sherpa-onnx** | Only through Whisper models, or Qwen3-ASR. No Indonesian-specific or streaming Indonesian model in its pretrained releases | Whisper tarballs (fp32 and int8 inside): tiny 110 MB, base 197 MB, small 609 MB | Not stated in the pages read | Streaming zipformers exist, none for Indonesian | Apache-2.0 / per model | Official `sherpa-onnx` crate 2.0.1 (2026-10-27); build downloads prebuilt static libraries, so no CMake | Very active (pushed 2026-11-10) |
| **Moonshine** | **None.** Speech-to-text list: English, Spanish, German, Mandarin, Japanese, Korean, Vietnamese, Tagalog, Ukrainian, Arabic | Tiny English int8 28 MB in sherpa | Not stated | Yes (current models) | MIT / MIT (legacy non-English models non-commercial) | C API, no Rust crate named | Active (pushed 2026-11-10) |
| **Qwen3-ASR 0.6B** | Yes, and the card lists singing and songs with background music as supported audio. Singing figures exist only for the 1.7B model | 0.9 B parameters, BF16 (about 1.8 GB) | Far above a Celeron's budget | Streaming only through vLLM | Apache-2.0 / Apache-2.0 | Example in sherpa-onnx's Rust examples | Active |

Reading the table:

- **Vosk and Moonshine are out on language alone.** The church sings mostly Indonesian.
- **sherpa-onnx runs the same Whisper weights.** Its advantages are the easy Windows build and
  its Silero VAD. Its Whisper models are ONNX exports; whether their input window can be shortened
  like whisper.cpp's `audio_ctx` was not verified, and that shortening is what keeps a short
  phrase cheap on a slow CPU. It stays the fallback if CMake and LLVM turn out to be too painful
  on a contributor's machine; the `Recogniser` trait in `src/recogniser.rs` is the seam.
- **Qwen3-ASR is the only model whose card claims singing support**, but it is a billion
  parameters. Not for this target.
- **MMS (Meta)** was not considered further: its Indonesian checkpoints are CC-BY-NC, which does
  not fit a GPL app that anyone may redistribute. (Licence from memory, not fetched: check before
  anyone revisits it.)

### Building whisper.cpp for an old CPU

- ggml's CMake defaults to `GGML_NATIVE=ON` unless cross-compiling, so a plain `cargo build` on
  a modern PC makes a binary that can crash with an illegal instruction on an older one.
  whisper-rs passes every `GGML_*` environment variable on to CMake (`build.rs`, checked in the
  0.15.0 `whisper-rs-sys` source).
- With `GGML_NATIVE=OFF` the same CMake file turns SSE4.2, AVX, AVX2, BMI2, FMA and F16C **on**.
  That runs on most Celerons from the Alder Lake-N generation onward but not on older ones. The
  Celeron N4120 from the benchmark thread has no AVX. A release for those machines needs
  `GGML_NATIVE=OFF GGML_AVX=OFF GGML_AVX2=OFF GGML_BMI2=OFF GGML_FMA=OFF GGML_F16C=OFF`, and will
  be slower.
- Which Celerons have AVX or AVX2 (including "Alder Lake-N onward") is from memory, not fetched. Verify with the owner's
  target machines before choosing the shipping baseline (CPU-Z, or `wmic cpu`).

## Sung words over music, from the sources

- The Whisper paper evaluates read and conversational speech. Nothing in it covers singing.
- **LyricWhiz** (ISMIR 2023, arXiv 2306.17103) is the closest primary evidence. It uses
  Whisper large, run several times, with ChatGPT correcting the result. On isolated vocal stems
  of MUSDB18 it gets 26 to 33 % WER; on the full mixture the same method gets 47 to 51 %. On a
  40-song multilingual set it averages 26 %, with Italian at 44 % and Russian at 39 %. Indonesian
  is not in it.
- **Syed et al. 2025** (arXiv 2506.15514) shows that separating the vocals first lowers Whisper's
  WER, in short and long form. A vocal separator is another neural net: not an option on this CPU.
- A Greek thesis found zero-shot Whisper medium at 92.1 % WER on lyrics (secondary report from a
  web search, not read at source).
- Whisper tends to hallucinate or loop over instrumental passages (same secondary brief).
  The segmenter here only passes frames above a noise-adaptive loudness threshold, and
  `clean_text` collapses decoder loops.

The conclusion: with a tiny or base model on a mixed signal, sung lyrics will come out with
most words wrong. The useful case is a **soloist or a song leader on a close microphone** or the
mixing desk's line out with the vocal loud. Plug the listener into a vocal channel, not the room.

## Hear notes and suggest the song (melody)

- **Audio fingerprinting** (Shazam, Wang 2003) identifies a *recording* by matching hashes of
  spectrogram peaks against a database built from the original tracks. Artosia has no recordings
  of the church's songs: the library is OpenLyrics text (`docs/song-library.md`), with no
  notes. Fingerprints of a different performance (this congregation, this key, this band) do
  not match the studio track, and there is no track to fingerprint anyway.
- **Query by singing or humming** matches a sung tune against a database of **monophonic MIDI
  melodies** (MIREX QBSH: 48 and 298 reference MIDI files in its corpora). That needs the tune of
  every song written down as notes. Nobody has done that for 1,335 songs, and the app's own
  data cannot supply it.
- Verdict: **not realistic without melody data, so not built.** If the church ever records its
  songs, or someone enters melodies as MIDI or ABC for the 200 most used songs, a fingerprint or
  a pitch-contour matcher becomes a weekend project. Until then the lyrics path (fuzzy words ->
  `SongIndex::search`) is the only song identifier, and it needs a vocal loud enough to read.

## What was built

```rust
pub struct Device { pub id: String, pub name: String }
pub fn devices() -> Vec<Device>;
pub enum Heard { Level(f32), Partial(String), Final(String), Problem(String) }
pub struct Listener;           // drop = stop, frees the model
impl Listener { pub fn start(device: Option<&str>, model: &Path, language: &str,
                             on_heard: impl Fn(Heard) + Send + 'static) -> Result<Listener, ListenError>; }
```

- `capture.rs`: **cpal 0.18** (WASAPI on Windows, CoreAudio, ALSA; Apache-2.0). GStreamer's
  `wasapi2src` would work too, but only the projector process links GStreamer and the operator
  must not need it; cpal is a plain crate with no DLL to ship and has the same three platforms.
  Device ids are cpal's `host:id` strings, stable across runs.
- `audio.rs`: stereo to mono, level meter (RMS, -60 dBFS to 0 mapped to 0..1, four reports a
  second), windowed-sinc resampler to 16 kHz (tested for length, tone amplitude and alias
  rejection).
- `segment.rs`: loudness-based phrase cutter with an adaptive noise floor. It opens a phrase
  after 100 ms of speech (keeping 300 ms before), closes after 700 ms of quiet, force-closes at
  12 s (singing has no pauses), emits a progress read every 2 s, drops phrases under 0.5 s.
  Also `coalesce` (a stale progress read is dropped when a newer one or a final is queued, so a
  slow CPU never builds a backlog) and `audio_ctx_for`.
- `text.rs`: strips `[BLANK_AUDIO]`, `(music)`, `*applause*`, music notes; collapses a word
  repeated more than three times in a row.
- `recogniser.rs`: the `Recogniser` trait and the whisper implementation, behind the
  **`whisper` cargo feature, off by default**. Greedy decoding, no context between phrases,
  single segment, audio context trimmed to the phrase, threads = cores - 1 (max 4).
- No model file: `Listener::start` still succeeds, levels flow, and one `Heard::Problem`
  says "No speech model installed (looked for ...)". Without the `whisper` feature a model file
  that exists gets "this build has no speech recogniser".

### Not proven here

- **The whisper path has not been compiled or run.** This PC has no CMake and no LLVM, and
  installing them is a machine change that was not approved. The code follows whisper-rs 0.16.0's
  own `basic_use` example and signatures read from the crate source, but treat it as unverified
  until `cargo build -p artosia-listen --features whisper` succeeds once.
- No model was downloaded; none exists on this PC (searched `C:\Users\kenne` and `F:\` for
  `*.gguf`, `ggml-*.bin`, `vosk-model-*`, `*.onnx`).
- No recognition accuracy, delay or CPU figure with a model. Capture-only numbers are below.

## Measured on this PC (i9-13900H laptop, Windows 11)

- Devices listed: Microphone Array (Realtek), Razer Seiren Mini, G435 headset, VB-Audio Cable.
- Capture only (`hear.exe` over 20 s with `tools/ram/measure-cpu.ps1`): **0.01 of a core,
  working set 11 MB (21 MB with the console host), private 2 MB.**
- Resampler plus segmenter on 60 s of 48 kHz audio: 39 ms in a release build, 0.06 % of one core
  on the i9. A Celeron is several times slower and still far below 1 %.
- In a quiet room the meter reads 0.00 (below -60 dBFS) with a 0.17 to 0.19 blip on stream start
  (the same on every run); a tone played through the laptop speakers did not reach the Realtek
  mic array, so no sustained level was seen.

## For the operator app

- **Search `Final` text. Use `Partial` only to show that something is being heard.** A partial is
  re-read from the start of the phrase and often changes; do not move the projector on it.
- A `Final` arrives after about 0.7 s of quiet or at 12 s of continuous sound, plus the
  recognition time. Spoken references: one `Final` per sentence. Singing: one `Final` every
  12 s, so a verse arrives in two or three pieces.
- **Wait for two agreeing suggestions** before highlighting a song: run `SongIndex::search`
  on each `Final` and on the last two `Final`s joined, and only promote a song when it tops both,
  or when it tops one phrase with a clear margin. Never auto-advance a live slide.
- Run `Bible::guess` on every `Final` as well. Whisper may write numbers as words ("tiga",
  "three sixteen") or as digits; convert Indonesian and English number words to digits before
  calling it (`satu` ... `seratus`, `one` ... `hundred`), and map "ayat"/"verse" and
  "pasal"/"chapter" to the separators `Bible::guess` expects.
- Treat all text as unreliable: show it as a suggestion with the matched words, one tap to
  accept.
- Let the person pick the language (`"id"`, `"en"`) per service; `"auto"` costs an extra pass
  and flips on short phrases.
- Show `Heard::Problem` text verbatim; it is already written for a person.
- The model path is an app setting. Ship no model: offer the download (57 MB) once, with consent.
- Call `Listener::start` off the UI thread's critical path (it waits for the device to open, a
  fraction of a second) and drop the `Listener` to stop; dropping waits for a recognition in
  progress, which can take seconds on a slow CPU, so drop it from a background thread.

## Model to approve

`ggml-base-q5_1.bin`, 59,707,625 bytes (57 MB), from
`https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin`, MIT licence
(the repository's licence field reads `mit`; Whisper weights are MIT per the openai/whisper
README). Optional smaller test model: `ggml-tiny-q5_1.bin`, 32,152,673 bytes (31 MB), same host.
