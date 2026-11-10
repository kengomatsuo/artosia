# Indonesian: word choices and doubts

Written from `brief.json` and `glossary.md` only. The check script found 402 entries, no missing or extra keys, no unknown or repeated `{variables}`, and no dash, middot, emoji or exclamation mark.

## Words chosen for the open glossary things

| Thing | Word | Why |
|---|---|---|
| `projector` (the congregation's screen) | "layar"; "layar jemaat" where the operator's own screen could be meant (`SCREEN_PROJECTOR`, `MESSAGE_DETAIL`) | The phone page already says "Layar hitam", and volunteers say "layar" for what the congregation reads. "Proyektor" names the device and is not used in any string. |
| `stage-screen` | "monitor panggung"; "Panggung" alone in the three-way screen choice | Church sound and multimedia teams say "monitor" for what faces the band. "Panggung" is the head noun, used only where 16 characters do not fit. |
| `stage-message` | "Pesan untuk pemusik" | Mirrors "Pesan untuk jemaat" on the phone page. |
| `wall-picture` | "Gambar proyeksi" | Describes the picture on the wall without borrowing "layar", which already means the congregation's screen. |
| `library` | "pustaka" | One short noun for songs plus Bibles. "Perpustakaan lagu" would leave the Bibles out. |
| `phone-remote` | feature "Remote", device "HP", button and command "Remote HP" | "HP" is what volunteers say aloud. "Remote" matches the phone page title. |

Other fixed choices used everywhere: plan = "acara" ("Acara hari ini", "ke acara"); service = "ibadah"; file = "berkas"; import = "impor"; divider in the plan = "pemisah"; the Look = "Tampilan" (never "tema", which the palette keeps as an alias); preview = "Pratinjau" (so "Tampilan" is never used for a preview); undo = "Urungkan"; edit = "Edit".

## The ten lines I am least sure of

1. `FIELD_KEY` = "Nada" (back: "Pitch/key"). The budget is 8, so "Nada dasar" (10), the term musicians use, does not fit.
2. `KEY_EDIT` "Ctrl E", `KEY_SAVE` "Ctrl S", `KEYS_EDITING_ROWS[0].0` "Ctrl N", `KEYS_EDITING_ROWS[5].0` "Ctrl Z". The brief gives no letters, so these are guesses. Check them against the real key bindings.
3. `CMD_IMPORT_BIRTHDAYS_DETAIL` = "Tabel nama dan tanggal lahir di Word" (back: "A table of names and birth dates in Word"). The brief also names PowerPoint, which did not fit in 44.
4. `SCREEN_STAGE` = "Panggung" (back: "Stage"). It is the short form of "monitor panggung" and breaks the one-word rule for the stage screen.
5. `live_caption` = "{title}, {label}, {index} dari {count}" (back: "{title}, {label}, {index} of {count}"). When the part name is empty the program must drop "{label}, ", or a double comma shows. `suggest_web` has the same problem with "{artist}, ".
6. `BTN_CLEAR_TEXT` = "Sembunyikan teks" (back: "Hide the text"). It is 16 characters against a budget of 12. I kept the phone page word instead of inventing a shorter one. `BTN_BLACK` (11 of 10) and `BTN_HIDE` (11 of 10) are over by one for the same reason.
7. `PLAN_REMOVE` = "Keluarkan" (back: "Take out"). I avoided "Hapus" because the song stays in the library, but "Keluarkan" is less common as a menu word.
8. `LISTEN_LANGUAGE` = "Bahasa lisan" (back: "Spoken language"). Clear on paper, but a volunteer may not say it that way.
9. `PLAN_NEW` = "Acara baru" (back: "New plan"). It reads like "add an item" more than "empty the plan". `CMD_NEW_PLAN_DETAIL` carries the meaning.
10. `KEYS_PLAN_ROWS[2].0` = "Alt panah atas, bawah" (back: "Alt up arrow, down"). It is 21 characters against 14. The brief asks for the two arrow words after Alt, and the phone page writes arrows as words.

Also over budget and flagged in `id.json`: `KEYS_PLAN_ROWS[0].0` (16 of 14), `KEYS_PLAN_ROWS[1].0` (15 of 14), `KEYS_EDITING_ROWS[3].0` (15 of 14).

## Notes for the program side

- `screen_title` and `screen_detail`: the `{this}` flag only selects a case, so it is not written into the text. The cases are `without_this` and `with_this`, `normal_zoom` and `with_zoom`.
- Cases named in `id.json`: `phones` (zero, one, many), `birthdays_on_file` (zero, one, many), `announcements` (zero, one, many), `import_done` (songs_only, songs_and_bibles), `import_summary` (nothing_changed, only_added, only_updated, kept_some), `pptx_imported` (without_lost, with_lost), `birthdays_imported` (none_skipped, some_skipped). In Indonesian "one" and "many" are the same line.
- `LYRICS_PLACEHOLDER` keeps `[1]` and `[chorus]` literally, as the brief says.

## Added with the streaming dialog

Broadcast = "siaran" (never "tayang", which keeps meaning live on the congregation screen). Streaming address and key use the Indonesian YouTube Studio terms "URL streaming" and "Kunci streaming". Least sure: `BTN_LISTEN` "Dengar" (bare verb, 6 characters), `LOOK_WINDOW` "Warna jendela Artosia", `stream_live` "gambar hilang" for lost frames. The editor shortcut letters (Ctrl E, S, N, Z) and the empty-label and empty-artist commas were confirmed by the coordinator.
