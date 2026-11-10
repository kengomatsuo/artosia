# Glossary: the product's fixed things

Each thing below gets ONE word (or fixed phrase) per language, used on every screen. The brief's `vocabulary` field names these by the id in backticks. The "already chosen" column is what the repo already says in Indonesian: the phone page (`crates/artosia-remote/src/page.html`, table `STR.id` and `LABELS_ID`), the command aliases in `crates/artosia/src/palette.rs`, and the section aliases in the same file. A new Indonesian writer must agree with those, or change them all in one commit.

This file is the one place a concrete word is allowed.

## The things

| id | What it is, where it appears | Already chosen in the repo (Indonesian) |
|---|---|---|
| `plan` | The list of songs, passages and announcements prepared for today's service, in order. The left column; dividers inside it group items. | Phone page: tab "Acara", empty text "Susunan acara masih kosong", add text "ke acara". Palette aliases: "susunan baru", "rencana", "ibadah baru". Dominant: "Susunan acara" (the order of service) or "Acara". |
| `slide` | One screenful of words on the projector. The middle column shows all slides of the open item; the right column counts them. | Phone page: "slide" (loanword), "{0} dari {1}", jump heading "Loncat ke bagian". |
| `section` | A named part of a song. Umbrella for the five below. | Palette aliases: "bagian". Phone page: "bagian". |
| `verse` | The numbered parts of a song, sung once each (part 1, part 2, ...). | Phone: "Bait 1", "Bait $1". Palette alias: "bait", "ayat". Use "Bait". Never "ayat" for a song part (it means a Bible verse). |
| `chorus` | The part repeated after each numbered part. Button in the editor, chip over slides, order letter C. | Phone: "Reff"; keys text "C atau R: reff". Palette aliases: "reff", "ref", "refrain", "koor". Use "Reff". |
| `bridge` | The part sung once, near the end, different in tune from the rest. Order letter B. | Phone: "Bridge" kept as is. Palette aliases: none beyond English. |
| `pre-chorus` | The lead-in sung before the repeated part. | Phone: "Pra-reff". Palette aliases: "pre reff", "pre", "prechorus". |
| `ending` | The closing part of a song. | Phone: "Penutup". Palette aliases: "penutup", "akhir", "coda". Intro, if ever needed: "Intro" (phone). |
| `projector` | The big screen the congregation reads, which the program draws on a second monitor or projector. Used in the top bar ("Screens"), the status bar, the dialogs and many details. | Phone page avoids the noun and says "Tayang" for what is on it. Palette aliases: "layar", "proyektor". Choose one noun for the whole window and say so in the glossary entry of the phone page. Candidates already in the repo: "layar" (as in "Layar hitam"), "proyektor". |
| `stage-screen` | The second picture facing the musicians (next slide, notices for the band). | Palette aliases: "stage", "monitor". No phone-page word. Writer decides one word and records it here. |
| `live-preview` | The picture of the congregation's screen inside the operator window, marked with a red "Live" pill and the state pill ("On screen" / "Nothing"). Also the plan row and slide thumbnail markers. | Phone: "Tayang" for live; "Belum ada yang tayang" for nothing live; "Logo sedang tayang". Use "Tayang". |
| `clear-text` | Hide the words but keep the background picture or video (key F1). | Phone: "Sembunyikan teks"; state "Teks disembunyikan"; palette aliases "sembunyikan teks", "kosongkan teks", "hapus teks". Use "Sembunyikan teks". |
| `black` | Turn the congregation's screen fully dark (key F2). | Phone: "Layar hitam"; state "Layar sedang hitam". Palette aliases: "hitam", "gelap", "layar hitam", "matikan layar". Use "Layar hitam". |
| `logo` | The church's emblem shown alone on the congregation's screen (key F3). Picture chosen in the Look dialog. | Phone: "Logo"; state "Logo sedang tayang"; hint "Pasang gambar logo di Artosia dulu". Use "Logo". |
| `look` | The set of colours, background picture or video, text size, text position and Bible versions for the congregation's screen; also the dialog and the top-bar button. | Palette aliases: "tampilan", "tema", "latar", "ukuran teks". Use "Tampilan". |
| `search-box` | The one box that finds songs, Bible passages and commands. Top bar, opens as a panel. | Phone: tab "Cari", placeholder "Cari lagu atau ayat", "Mencari". Use "Cari". |
| `announcements` | Notices shown on the projector before the service, looping. The first row of the plan and an editor. | Palette aliases: "pengumuman", "warta". Use "Pengumuman". |
| `phone-remote` | The phone that changes slides over the same wifi; dialog and top-bar button. | Phone page title: "Remote Artosia", connect "Sambungkan", PIN "PIN". Palette aliases: "hp", "ponsel", "telepon", "remote hp". Use "Remote" for the feature and "HP" or "Telepon" for the device (writer decides, record it here). |
| `countdown` | A timer counting down to the start, shown on the projector. Block in the right column. | Phone: "Hitung mundur", "Menit", "Mulai", "Berhenti". Palette aliases: "hitung mundur", "waktu mundur". Use "Hitung mundur". |
| `room-message` | A short notice shown to the congregation over the slide ("to the room"). | Phone: "Pesan untuk jemaat", "Tulis pesan", buttons "Tampilkan" and "Sembunyikan", "Tampil: {0}". Use "Pesan untuk jemaat". |
| `stage-message` | A short notice shown only to the musicians on the stage screen. | No phone word. Suggest "Pesan untuk pemusik" or "Pesan untuk panggung"; writer decides and records it here. |
| `wall-picture` | The physical picture as the congregation sees it on the wall: squaring its corners, brightness, contrast, colour intensity. Section of the Screens dialog. | None. Writer decides. |
| `library` | The church's stored songs and Bibles. | Song library doc (`docs/song-library.md`) says "Indonesian first"; no fixed word. Suggest "Pustaka" or "Perpustakaan lagu". Writer decides. |
| `song`, `bible` | A song; the Bible and its versions. | Phone: "lagu", "ayat" (a Bible verse), "Alkitab" (palette aliases), "terjemahan", "versi". |
| `listen` | The microphone helper that proposes a song or verse from what it hears; it never shows anything by itself. | Palette aliases: "dengar", "dengarkan", "mikrofon", "suara". |
| `ring` | The white outline on the slide Enter will show, in the Preview pane. Moved with Shift and an arrow. | "bingkai" ("bingkai putih" where it fits). Chosen 2026-11-10: a selection outline on a thumbnail is called this; "cincin" reads oddly. |
| `preview-pane` | The middle pane of the main window: the slides of what comes next. | "Pratinjau" as the pane's title, "pratinjau" inside a sentence. "Bawa ... ke pratinjau" for putting something there. |
| `plan-item` | One entry of the plan: a song, a passage or the announcements. | "item" ("item acara"). |
| `learned` | What the search remembers of past use on this PC: which song was picked for what was typed, and what followed what. | "ingat" and "dilupakan" ("Ingat pilihan di PC ini", "Lupakan pilihan yang diingat"); a past pick is "dipilih ... kali", coming right after another song is "menyusul". "Lupakan", never "Hapus", which would read as deleting songs. |

## Phone page words the operator window must agree with

| English action or state | Indonesian on the phone page |
|---|---|
| Next (action) | "Lanjut" |
| Next (caption: what follows) | "Berikutnya"; "Setelah ini" for the next item |
| Back | "Kembali" |
| Show (an item or a message) | "Tayangkan"; for a message "Tampilkan" |
| Hide | "Sembunyikan" |
| Live / on screen | "Tayang" |
| Nothing open / nothing live | "Belum ada yang dibuka" / "Belum ada yang tayang" |
| Plan | "Acara" (tab), "Susunan acara" |
| Add | "Tambah"; added "Masuk" |
| Search | "Cari" |
| Countdown, minutes, start, stop | "Hitung mundur", "Menit", "Mulai", "Berhenti" |
| Chorus, verse, pre-chorus, bridge, ending | "Reff", "Bait", "Pra-reff", "Bridge", "Penutup" |
| Clear text, black, logo | "Sembunyikan teks", "Layar hitam", "Logo" |
| Keys (heading) | "Tombol" |
| Spacebar, left and right arrow, "1 to 9" | "Spasi", "panah kiri", "panah kanan", "1 sampai 9" |
| Language | "Bahasa" |
| Lost connection, trying again | "Koneksi terputus", "Mencoba lagi" |
| Message to the room | "Pesan untuk jemaat" |
| Slide count | "{0} slide" (loanword) |

## Open choices the first writer settles (and records here)

The noun for `projector`, the word for `stage-screen`, `stage-message`, `wall-picture`, `library`, the device word for `phone-remote`. Everything else above is already fixed.
