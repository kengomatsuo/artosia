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

### Measured on real recordings (2026-11-11)

Played from YouTube through system audio, 100 s each from the first half-minute, with the base
model and the church's 1,333 songs as the library. The test is `names_its_song` in
`crates/artosia/src/heard_test.rs`; it prints titles and word counts, never the words.

| Played | What the model made of it | Named |
|---|---|---|
| An English hymn sung by a choir, in the library | 5 to 15 words a phrase | the right song from about a minute in, alone |
| Four Indonesian worship songs with a band, in the library | 5 to 9 words in a 12 s phrase, two to four of them together in a line of the right song | one of the four, from 36 to 50 s, in each of three passes; the other three never |
| A sermon in Indonesian | a phrase every five seconds | nothing |
| The news, and a pop song that is not in the library | | nothing |

- The right song was among a phrase's three nearest in about half the phrases on a good pass
  and in one phrase of five on a bad pass of the same recording. A pass differs from the next
  because the 12 s cuts fall elsewhere.
- The matcher that 0.1.7 ships offered **five songs in 100 s of that sermon** and a wrong song
  for one of the Indonesian recordings: a tail of three or four words sitting in a line was
  enough. What replaced it is in `docs/palette-suggestions.md`, "How a heard phrase is read".
- The hymn's line is sung again and again, and the same word five times once put eight "words
  together" in a song that was not being sung. Each word of a phrase now counts once.
- So the answer to "which song is this" is: a clear English hymn, yes, in about a minute; an
  Indonesian song over a band, sometimes. The model is the limit, as the papers above say, and
  `small` was not tried.

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

## Sources: microphones and the computer's own sound

- `devices()` lists the inputs of the default host; each has cpal's `host:id` string and a name.
  The operator saves the **id**. Its first version saved the name and handed it to
  `Listener::start`, which looked it up as an id, so every microphone but the default failed
  with "device ... is not connected". `find_input` now takes either: the id first, then the name,
  which also reads what older settings hold.
- **`SYSTEM_AUDIO` ("system-audio") is what the computer plays.** On Windows it is WASAPI
  loopback of the default output device. cpal 0.18.2 says so in its own source: "If you use a
  WASAPI output device as an input device it will transparently enable loopback mode"
  (`host/wasapi/mod.rs`), and `device.rs` sets `AUDCLNT_STREAMFLAGS_LOOPBACK` when a render
  device is opened for capture. `system_audio_available()` is true on Windows and macOS; ALSA
  has nothing like it.
- **Loopback delivers nothing while nothing plays**, so the listener feeds the meter and the
  tracker silence on a timer; otherwise the level would freeze at its last value.
- The operator lists the sources as the computer's own microphone, every microphone by name,
  then "System audio".
- `cargo run -p artosia-listen --example sources -- list | mic <name-or-id> | tone | tempo <bpm> |
  hear <seconds> [keep <file>] | read <file>...` opens each source through the listener's own
  code and prints what arrived. It keeps no sound.

Opened on this PC, 2026-11-11: the default microphone and a USB microphone by name (48 kHz,
one channel), the laptop's microphone array (48 kHz, two channels), a virtual cable (opens,
silent), a wireless headset's microphone by id and by name (48 kHz, one channel, frames
arriving but every one of them zero, as with the headset switched off), and system audio (a
440 Hz tone played at low volume came back at -20.7 dBFS peak).
Not tried: the headset with someone speaking into it, a device at 44.1 or 16 kHz, a device
unplugged while it runs.

## Tempo and the beat

`TempoTracker` (`src/tempo.rs`) is fed the same mono blocks as the level meter and reports, twice
a second, `Heard::Tempo(Option<Tempo>)`: beats per minute (50 to 132), a confidence from 0 to 1,
the beat period, and how many seconds ago the last beat fell. `None` means no steady pulse.
Nothing is added to `Cargo.toml`: the FFT is 40 lines of plain Rust.

### Sources (fetched 2026-11-11)

- **Scheirer 1998**, "Tempo and beat analysis of acoustic musical signals", JASA 103(1)
  (`ee.columbia.edu/~dpwe/papers/Schei98-beats.pdf`). Read for the idea: envelopes of a few
  bands (six in the paper), their rectified difference, then a bank of comb filters, one per
  candidate tempo, run causally so the beat can be predicted. Not copied: the resonator bank per
  band, which costs more than a plain autocorrelation of one envelope.
- **Ellis 2007**, "Beat Tracking by Dynamic Programming"
  (`ee.columbia.edu/~dpwe/pubs/Ellis07-beattrack.pdf`). Onset envelope at 8 kHz, 32 ms windows,
  4 ms hop, 40 mel bands in dB, positive differences summed across bands; tempo from the
  autocorrelation of that envelope weighted by a Gaussian on a log axis centred on 120 BPM, width
  1.4 octaves (0.9 octaves when two metrical levels are scored together). Used: the envelope
  recipe and the log-Gaussian window at 0.9 octaves. Not used: the dynamic-programming pass. The
  paper says the abrupt tempo change needs "a more radical modification" and that searching over
  tempos in DP is "considerably more computationally expensive".
- **Böck and Widmer 2013**, "Maximum Filter Vibrato Suppression for Onset Detection", DAFx-13
  (SuperFlux; the abstract was read, the parameters come from the authors' reference code,
  `github.com/CPJKU/SuperFlux`: `max_bins = 3`, `log10(mul * spec + add)`, `mul = add = 1`). Used:
  the flux is taken against a maximum filter over the neighbouring bands of an earlier frame, so
  a sung note gliding across a band is not an onset. That matters for a congregation. Böck and
  Schedl's spectral-flux work is the base of it and was not read separately.
- **aubio** (`src/tempo/tempo.c` and `beattracking.c`, master): `specflux` onsets, a window of
  about 6 s, a Rayleigh weight centred on 120 BPM, and the autocorrelation summed at one to four
  multiples of each lag with weights `1/(2a-1)`. Used: the 6 s window and the harmonic sum (three
  harmonics, those weights). The source cites no paper. It is GPL; nothing was copied.
- **librosa** `feature.rhythm.tempo` (`rhythm.py`, main): `start_bpm = 120`, `std_bpm = 1.0`
  octaves, `ac_size = 8 s`. Confirms the 120 BPM log-normal prior.

### Method

1. Average to about 8 kHz (a box filter; aliasing does no harm to an onset signal).
2. 256-point FFT every 5 ms, 24 mel-spaced bands from 60 Hz to 3.8 kHz, `log10(1 + 100 * magnitude)`.
3. SuperFlux onset strength: the last three frames of band levels are averaged (a held chord's
   partials beat against each other at 50 Hz and more, which an unaveraged flux turns into a
   steady false pulse), then each band's rise over the largest of the three neighbouring bands
   four frames earlier is counted above twice that band's running mean rise, and summed.
4. Every 0.5 s, on the last 8 s of that envelope (mean removed): autocorrelation at the lags for
   50 to 132 BPM, summed with its values at two, three and four times the lag (weights 1, 0.1,
   0.15, 1: the bar matters more than the pair), times the log-normal prior around 128 BPM and 1.2
   octaves wide. The same is done for each of the 24 bands' own rises (an FFT per band) and
   averaged with weight peak squared; the larger of the two autocorrelations is used, which finds
   a pulse that lives in a few bands under hiss and ripple in the rest. The score of every period
   is blended with the previous reports' (0.7 kept) so a kick and snare that alternate do not flip
   between 60 and 120. The peak is refined by a parabola. Confidence is the geometric mean of the
   normalised autocorrelation at one period and the larger of that at two periods and 0.6 times
   that at four (a syncopated bar need not repeat after two beats), less 0.08. A pulse that is
   only half regular has a strong first peak and a weak second, which is what plain speech does.
5. Beat phase: the offset at which a pulse train of that period collects the most envelope,
   recent beats counting more.
6. A report needs confidence 0.28 twice in a row at periods within 4 %. Once reporting it carries
   on down to 0.15, and it goes quiet after four half-seconds without a usable estimate.

### Why it fits this room

- A slow CPU: no model, one 256-point FFT per 5 ms, and about 1.7 million multiply-adds a second
  for the autocorrelation. A live microphone: it is causal and keeps 8 s of envelope.
- A reverberant hall: the envelope is a rise in energy, so a tail that decays smoothly adds no
  onsets. The test uses a Schroeder reverberator with a tail near 1.5 s.
- A band or a single keyboard: the method needs attacks. A pad with no attack, or slow held
  chords, gives no pulse and the tracker says nothing instead of guessing.
- Congregational singing: voices have soft onsets, so the pulse comes from drums, guitar and
  clapping. Over singing alone it will usually report nothing.
- The 128 BPM prior, the harmonic sum with the bar and the blend are what keep it off half and double speed.

### Measured on made-up signals (`cargo test -p artosia-listen`)

The tests synthesise a drum-like thump plus a noise burst, never a recording.

- Click tracks at 60, 90, 120 and 130 BPM: every report from 8 s on is within 0.1 BPM of the
  truth (the test allows 2). The first correct report arrives within 5 s.
- The reported last-beat time falls within 20 ms of a real hit (the test allows 30).
- Hits in noise at the hits' own loudness, and in a reverberant tail (wet signal 1.5 times the
  dry): within 2 BPM from 10 s on. The lowest raw confidence seen was 0.22 to 0.36.
- A change from 90 to 130 BPM is followed: read correctly by 10 s after the change (the window is
  8 s long).
- Silence, steady noise and a very quiet hiss report nothing. Six seeds of speech-like noise
  (syllables 120 to 250 ms, gaps 30 to 300 ms) report nothing for 40 s; their raw confidence
  peaks between 0.08 and 0.20, under the 0.28 that starts a report. The gap is narrow: music with
  weak, irregular accents in a very noisy room can also fall under 0.28 and report nothing.
- Half and double time: hits that alternate loud and soft, at 80, 100, 140 and 170 BPM with the
  soft hit at half and at a quarter strength, always read as the true tempo, half of it or double
  it, never another number; the test allows those three. Real music is measured below.
- Cost: 60 s of 48 kHz audio takes 290 ms in a release build on the i9 laptop, 0.5 % of one core
  (`cargo test -p artosia-listen --release --test cost -- --ignored --nocapture`). A Celeron is
  several times slower and should stay near 1 %; not run on one.
- It also works at 44.1 kHz.
- Five rhythms (four on the floor, kick and snare with hats, a syncopated bar, a soft-attack
  guitar strum, a sustained four-note pad with a quiet pulse under it) at ten tempos from 60 to
  176 BPM, with room noise, built in `tests/tempo_accuracy.rs`, three noise seeds each: every row
  up to 128 BPM reads within 2 BPM of the truth from 8.5 s on, with no reading at half or double.
  At 140, 160 and 176 BPM, above the cap, the steady rhythms read half the tempo, the strum
  takes 8.5 to 9 s to do so, and the syncopated bar, which does not repeat at half speed, reads
  nothing; the test allows half or nothing there, never another number. 60 BPM takes
  5.5 to 8 s to settle, 72 and 84 BPM 4 to 5 s and the rest 3.5 to 4.5 s.

### Measured on real music (2026-11-11, owner: "try using music from youtube")

Thirty-five tracks whose uploader states the tempo in the title (backing tracks, drum loops and
worship pads, 50 to 160 BPM) were played from YouTube at low volume and read through system
audio, 30 s each from half a minute in. Twenty-two were looked at while the tracker was
changed; thirteen were played afterwards and never used for a decision.

| Stated tempo | Tracks | Read |
|---|---|---|
| 67 to 132 BPM | 23 | 22 within 0.2 BPM of the title (one title says 76 and the track plays at 78); a pad with no drums and no attacks gave no reading |
| 60 BPM, four beats in a bar | 3 | 2 right (one only after 23 s, and unsteady); a strummed ballad read 120 |
| 50 and 60 BPM, six eighth notes in a bar | 3 | 2 read 99.9 and 120.1, the eighth notes counted two to the stated beat; one drum loop gave no reading |
| 60 BPM, twelve eighth notes in a bar | 2 | 1 right after 15 s; 1 read 52.7 twice in 30 s |
| 140 and 160 BPM | 4 | half the tempo, steadily (70, 70, 80, 80): see the cap below |

The first reading comes 3.5 to 5.5 s in for most tracks and 15 to 24 s in for the slowest.

**What was wrong, and the one change that came of it.** Before this the tracker reported up to
200 BPM, and of the first twenty-two tracks it read 15 right, 3 slow ones at double speed (60 as
120, 68 as 136, 70 as 140) and 2 fast ones at half. A slow ballad whose eighth notes are as
strong as its beats looks, to an autocorrelation, exactly like a song twice as fast. Nothing in
the signal separates the two: the autocorrelation at half the beat, at the beat, at two beats
and at the bar was printed for every track (`TEMPO_EXPLAIN=1 ... sources read`), and the slow
tracks that doubled sat inside the range of the fast tracks that were right on every figure.
Essentia's Percival estimator settles the same question with three figures, one of them the
tempo itself, and weights learned from other music (`percivalbpmestimator.cpp`, read 2026-11-11).

So `MAX_BPM` is 132: the double of any song from 67 BPM up is no longer on offer. A church sings
far more songs between 60 and 80 than above 132, and a fast song read at half speed still
pulses on its first and third beats. After the change every track from 67 to 132 BPM that reads at
all reads right, more steadily than before, and the four fast tracks read a steady half where
three of them used to flip between the two.

Tried and put back, each scored on the kept tracks and on the made-up table:

| Change | Real tracks | Made-up table |
|---|---|---|
| Count the pair of beats as much as the bar (weights 1, 1, 0.15, 1) | the two halved fast tracks came right, the three doubled slow ones stayed | 33 of 150 rows failed, slow rows read double |
| The same with the pair at 0.5 | the same | 18 rows failed |
| A prior centred lower (118, 110, 100 BPM) with those weights | more fast tracks halved | not run |
| A prior centred lower (100, 92, 91 BPM) with the cap | 120 BPM read as 60 | not run |

Since the sound cannot settle it, a person can: Half, As heard and Double sit beside the number
in the Listen dialog and hold the tempo for the song (`docs/operator.md`, "Listening and
streaming").

Still wrong: a ballad at 60 to 66 BPM with strong eighth notes can read double, because its
double is under the cap; music in six or twelve eighth notes to the bar reads at the eighth
note, slowly, or not at all; a pad with no attacks reads nothing, as designed.

**How to score a change.** `sources hear 30 keep <file>` writes the tracker's own 5 ms steps of
whatever is playing (the rise in each of 24 bands and the energy: not sound, and it cannot be
played back), and `sources read <files>` runs them through the tracker again in a moment. Name a
file `<stated tempo>-<anything>` and `TEMPO_EXPLAIN=1` also prints the figures at that tempo,
half and double. The files stay out of the repository: the thirty-five live in
`fixtures/tempo-steps/first` and `held-out`, which git ignores, and
`python tools/score-tempo.py` reads them all and counts right, double, half and unread.

### What tempo is for, and what is not built

The owner's "next detection" is knowing when the lines of a slide are about to end, so the next
slide can be ready. Tempo is the first piece. What the rest would take:

1. **The beat and the bar.** The tracker gives the beat. A bar is usually four beats, and the bar
   line is the beat with the strongest onset (the kick, the first chord). Not built: finding the
   downbeat needs the accent pattern, which the autocorrelation here does not keep.
2. **How long a line is.** A line of a song usually lasts a whole number of bars, most often two
   or four. The slide knows its line count, but nothing records how many bars a line takes. That
   would be a number per song: entered once by a volunteer, or learned the first time through.
3. **Where in the line the singing is.** Beats say how time passes, not where the words are.
   `segment.rs` knows when singing starts and stops, so a line start could be anchored to the
   first loud frame after a pause. Matching sung words to the slide's lines needs the speech
   recogniser on a vocal channel.
4. **Then** "this slide ends in about N beats" is lines times bars per line times beats per bar,
   less the beats already sung. That estimate will be wrong often enough that it should only ever
   prepare something (light the Next button), never move the projector.

None of this is built: no downbeat, no bars per line, no matching, no estimate. The tempo and the
time of the last beat are all that exist, and the operator uses them for the number and the dot
in the Listen dialog and for the optional pulse of the Live label.

## What was built

```rust
pub struct Device { pub id: String, pub name: String }
pub fn devices() -> Vec<Device>;
pub enum Heard { Level(f32), Tempo(Option<Tempo>), Partial(String), Final(String), Problem(String) }
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
- No model file: `Listener::start` still succeeds, levels and tempo flow, and one
  `Heard::Problem(Problem::NoModel)` says so. Without the `whisper` feature a model file that
  exists gets `Problem::NoRecogniser`.
- **A problem is a kind, never a sentence.** `Problem` (`NoModel`, `NoRecogniser`, `Model`,
  `Input`, `Reading`) and `ListenError` (`NoDevice`, `Stream`, `Thread`) carry the library's own
  text only for logs. The operator words each kind in the window's language
  (`crates/artosia/src/why.rs` `listen_error`, `listen_problem`): the first version put
  "No speech model installed (looked for C:\...)" on the Listen dialog, in English, with a path.

- `tempo.rs`: the beat tracker, below. Needs no model and no feature flag.

### Building the recogniser on Windows (2026-11-11)

It compiles and runs. The build needs two tools, both found by `tools/speech-env.sh`:

- **CMake**, the copy inside Visual Studio 2022 Community
  (`Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin`).
- **libclang**, for `bindgen`: `libclang.dll` and clang's own headers unpacked from LLVM's
  23.1.5 release into `~/.artosia-tools/llvm-23.1.5/bin`, named by `LIBCLANG_PATH`.

```
. tools/speech-env.sh        # exports PATH and LIBCLANG_PATH, says what is missing
cargo build --release -p artosia --features speech
cargo test --release -p artosia-listen --features whisper
```

- **The trap: `WHISPER_DONT_GENERATE_BINDINGS` does not work on Windows.** `whisper-rs-sys` then
  uses the bindings bundled in the crate, which were generated on Linux; they fail a struct size
  check on Windows. Real libclang is the only way.
- whisper, ggml and ggml-cpu link **statically** (`whisper-rs-sys` `build.rs`: `BUILD_SHARED_LIBS=OFF`,
  OpenMP off), so the release carries no extra DLL. The model is not shipped: `set_listening`
  looks for `<library folder>\models\ggml-base-q5_1.bin`, and when it is missing the Listen
  dialog offers the download in a row of its own (`crates/artosia/src/speech_model.rs`).
- A release-profile build of the operator with the feature takes about 6.5 minutes from a warm
  cache; the speech-enabled `artosia.exe` is 38 MB.
- Without the feature nothing changes: `cargo build` needs neither tool.

### Proven here (2026-11-11, i9-13900H, base q5_1)

Windows' own synthesiser (`tools/make-speech-sample.ps1`, voice Microsoft David) spoke made-up
sentences; `hear_samples` ran them through the same resampler, segmenter and recogniser as the
microphone path (`tests/speech.rs`, ignored: it needs the model and `fixtures/speech`).

| Spoken | Heard | Audio | Time, model load included |
|---|---|---|---|
| The blue lantern waits beside the wooden gate. | The blue lantern waits beside the wooden gate. | 3.11 s | 0.23 to 0.28 s |

- Peak working set of the test process with the model loaded and one phrase read: **133 MB**
  (5 MB before).
- **No Indonesian voice is installed on this PC** (voices: David and Zira en-US, Huihui zh-CN), so
  Indonesian recognition is still unheard. The test skips itself and says so.
- Synthesised speech is clean and slow, so this proves the path works, not accuracy with a real
  room, accent or singing. The i9 says nothing about a Celeron.
- `tests/speech.rs` and `crates/artosia/src/heard_test.rs` never open a microphone.

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
