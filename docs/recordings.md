# Recordings: what the listener heard, kept so it can be learned from

Asked by the owner on 2026-11-11: "make the app collect telemetry audio and the attempts for us
to learn. But how without a database?"

## The answer: a folder is the database

A database would need a server, a key inside a free program, and someone to run it. None of
that is needed to learn from a service. Artosia writes plain files into a folder on the
computer it runs on, and the folder travels to the people who build Artosia in one of two ways:

| Way | What the church does |
|---|---|
| One file | Listen, Recordings, **Save as one file** packs the folder into a zip to send by chat, e-mail or a memory stick |
| A drive that copies itself | **Move folder** puts the folder inside OneDrive or Google Drive, and the files arrive without anyone doing anything |

Nothing is sent by Artosia itself. The owner chose this over a storage bucket (asked through a
question, 2026-11-11). A bucket needs an account and a bill, and its key would sit inside a
free program where a stranger can read it.

## What is kept, and what is not

Keeping is **off until a person switches it on** (Listen, Recordings, "Keep recordings"), and
the choice is remembered. While it keeps, a filled mark reading "Recording" shows in the bar at
the foot of the window, and a press on it opens the switch.

One folder is made each time the listener is switched on, named by its date and time:

| File | What it holds |
|---|---|
| `about.json` | Artosia's version, when it began, the kind of source (default microphone, a microphone, or system audio: never the device's name), the language listened for, whether words are read and by which model's file, how many songs the library has |
| `phrase-00001.wav` ... | Each phrase the listener cut, exactly as the recogniser was given it: 16 kHz, one channel, 16 bits, cut at a pause or after 12 seconds. About 115 MB for an hour of sound |
| `attempts.jsonl` | One JSON object a line, in the order things happened, each with `t`, the seconds since the folder began |

The lines of `attempts.jsonl`:

| `kind` | When | Fields |
|---|---|---|
| `phrase` | a phrase was written | `n` (the file's number), `seconds` |
| `read` | the recogniser finished phrase `n` | `heard` (the words, or null), `near` (the three nearest songs, each with `together`, the words in one line), `named` (the songs that phrase named by itself), `passage` |
| `live` | what is on the projector changed | `what` (song, passage, notice or null), `title`, `slide`, `section`, `in_the_room` (false while Live is off, the screen is black or the words are hidden) |
| `offered` | the strip of offers changed | `offers`, each with `why` and `title` |
| `taken` | a person took an offer into Preview | `why`, `title` |
| `tempo` | the tempo read changed | `bpm`, `sure` (0 to 1), `count` (`half`, `as heard` or `double`), `shown`; `bpm` null when the pulse is lost |
| `count` | a person pressed Half, As heard or Double | `to`, `wanted` (the tempo the press said it is) |

**What is on the projector is the answer.** A volunteer shows the right song, so every phrase
heard while it was shown is a labelled example: the sound, what the model read in it, what
Artosia guessed, and what it really was. Nobody has to mark anything.

Not kept: the name of any device, the words of any song in the library, the title of a notice
(a notice may hold people's names, so only that a notice was shown is written), and anything at
all in practice or in a headless run.

A phrase is whatever the microphone heard loudly enough: singing, preaching, prayers, people
talking near it. That is why it is off by default, why the mark is lit while it keeps, and why
**Delete all** is one press. Delete all moves every folder aside and offers Undo in the toast;
when the toast leaves, they are deleted for good. Closing Artosia while the toast is still up
leaves them set aside, so the next start finishes the delete at its first poll
(`Recordings::at_start`).

## The limit

The folder holds at most 1 GB (`recordings.rs` `LIMIT`). When a sitting begins, and again during
a long one, the oldest sittings are deleted until it fits. The sitting being written is never
deleted; if it alone reaches the limit, about nine hours of sound, a new sitting begins and
that one goes as the oldest. A folder that cannot be written (a
full disk, a drive pulled out) stops the keeping and says so beside the buttons, where Move
folder is.

## How it is built

| Part | Where |
|---|---|
| Writing each phrase | `crates/artosia-listen/src/lib.rs`: `Listener::start_keeping`, `Keeper`, `Heard::Kept` and `Heard::Read`, on the listening thread, never the window's. It cuts phrases for the folder even when no speech model is there, so the tempo can be studied from them too |
| The folder, the log, the limit, delete, move, zip | `crates/artosia/src/recordings.rs`; the zip is `artosia_core::pack::zip_folder` on a thread of its own |
| What Artosia made of a phrase | `suggest.rs` `attempt_of`, the same functions the offers use |
| The switch, the four buttons, the mark | `ui/sheets.rs` `recordings_part`, `ui/mod.rs` `statusbar` |
| Reading it back | `python tools/learn-from.py <folder or zip>`: song by song, how many phrases had words, how often the song shown was among the nearest three, was named, or another was named, and the tempo read. It prints titles, never the words |

The Listen dialog became two parts, Listening and Recordings, as Design has three: with the
recordings under the rest it was taller than a laptop's screen. The first part alone was
already too tall for one once words were heard: six sources, the tempo, the heard line and
three songs came to about 800 points with the listener on. The parts and the Listen switch
now stay, and the rows under them scroll with a fade when the window is lower than the
dialog (`ui/sheets.rs` `listen`, scene `listen-heard`, rendered at 1366x768 and 1280x720 in
both languages).

The toast after Delete all prints Ctrl Z beside Undo, and the key did nothing while the
dialog was open: the key handler returned before it for every open dialog. Ctrl Z now takes
back what a toast offers over a dialog too (`main.rs` `on_key`); a text field still uses the
key for its own typing first.

## Checked

2026-11-11, on this PC, in a real window maximized at 125%, with the speech build on a copy of
the church's library (1333 songs), System audio as the source and Indonesian as the language.
A worship recording played quietly from the web in a browser of its own, and the song playing
was the song on the projector.

| What | Result |
|---|---|
| Switching on | Real clicks on the Recordings part, on "Keep recordings", then on Listen. The folder of the sitting appeared within 20 seconds, with `about.json`, `attempts.jsonl` and the first phrase |
| The mark | "Recording" lit in the bar while the listener was on, and left when Listen was switched off |
| The sound | 11 phrases in 2 minutes 12 seconds, each 12 seconds and 384,044 bytes |
| The attempts | 6 of the 11 phrases had words. The song shown was among the nearest three in 5 of those 6, no single phrase named it, and no other song was named |
| The tempo | 71 BPM read after a minute, lost at 80 seconds, back at 84. A real click on Double wrote a `count` line and the tempo shown became 142 |
| Reading it back | `tools/learn-from.py` on the folder, and on the zip Artosia made, printed the same figures and which phrases to listen to |
| Delete all | A real click: the toast with Undo came up, the amount read "Nothing kept yet", and when the toast left the files were gone from the disk |
| Save, Move, Delete by script | On a copy, with the file windows answered by the script: a zip of 13 files (3.9 MB from 4.2 MB), the folder moved into `Artosia recordings` inside the one picked and the old one removed, the sittings set aside by Delete all, and brought back by Ctrl Z over the open dialog |
| A delete cut short | The script's run ended with the toast still up; the next start removed what was set aside |
| Ctrl Z in a real window | Before the change the real key did nothing over the dialog and the files went when the toast left. After it, Delete all then the real Ctrl Z read "5 MB kept" again and the sitting was back on the disk |
| The system's own Save and folder windows | In the released 0.1.11, on a library holding a sitting made from a made-up tone: Save as one file opened the Windows save window, its file-name field was given a path and Enter pressed, the zip was written and the toast read "Saved as second.zip". Move folder opened the folder window; one Enter went into the picked folder and a second picked it, the row then read `...\picked\Artosia recordings` and the sitting was there, with the old folder gone. Answered with `tools/answer-dialog.ps1` |
| A second sitting, nothing on the projector | 8 phrases in 96 seconds with the build that rounds its figures (`71.4`, `0.3`, `as heard`). The song playing was the nearest in three of the four phrases with words and was offered after 25 seconds; `learn-from.py` says of such a sitting that no song was shown to measure against |

The first log wrote 32-bit figures as they stood (`71.4000015258789`); they are rounded now, and
a test holds that.

Not tried: a whole service, the 1 GB limit on a real folder (a test holds it with small
numbers), a microphone in a room, and a folder inside a drive that really copies itself.
