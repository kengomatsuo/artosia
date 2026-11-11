# Operator window translations

No locale is the source the others are translated from. English is written from the briefs like every other locale, and Indonesian is a table like any other (`docs/i18n/id.json`), written from the same briefs by a writer who never sees English.

The rule behind all of it: **a writer never sees the source string and is never handed a ready-made word.** The briefs describe the screen and the job of each text; the writer chooses the words of the language. An earlier round hid the English but still handed over a glossary of ready-made Indonesian words, and the briefs described the English sentence instead of the screen. The result was a button that read as a transliteration. Both faults are removed: `glossary.md` and `id-choices.md` are deleted, and no file in this folder carries a word of any target language.

## What is in this folder

- `brief.json`: one entry for every text the operator window's table must hold, in the order of `strings.rs`. Fields: `key`, `kind`, `screen` (which window and where the text sits), `moment` (what the person just did), `role`, `does` (the job, never the English wording), `budget` (characters that really fit), `variables` (each with a made-up example), `cases`, `counts`, `register`, `concepts`, `notes`.
- `brief-phone.json`: the same for the phone page (`STR` in `crates/artosia-remote/src/page.html`) and its section labels.
- `concepts.md`: the things the product is made of, described in plain English and never named in any language.
- `id.json`: the Indonesian table, in the shape every table has. It is rewritten from the briefs in the next round; until then it is the table the program ships.
- `../../tools/check-brief.py`: the check that every key the code reads has an entry.

## Kinds of entry

| kind | what it is |
|---|---|
| `const` | a `text!` line of `strings.rs` |
| `sentence` | a function that builds a sentence: the entry names its key, each variable with an example, each named case, and what is counted |
| `row` | a row of an array: `KEYS_SHOWING_ROWS[n].1` and the three other key-sheet groups (the second column only), and `ANNOUNCEMENT_TEMPLATES[n].0` and `.1` |
| `section` | `section.verse`, `section.chorus` and the other section names, and `section_numbered` |
| `noun` | `noun.songs` and the other counted nouns that sentences insert |
| `months` | `MONTHS[0]` to `MONTHS[11]` |
| `phone` | a text of the phone page, or a section label of it (`label.verse` and so on) |

The count of entries is printed by the check. Entries for texts that other sessions add later (a start mark for medleys, a backdrop browser) are added when those texts land.

## Left out on purpose

Keys are drawn as the keyboard prints them and are never translated, and proper nouns stay as they are:

- `APP_NAME`, a proper noun.
- `KEY_SEARCH`, `KEY_ENTER`, `KEY_SHOW_NOW`, `KEY_ADD`, `KEY_SAVE`, `ctrl_digit`, and the key column of the four key-sheet groups (`KEYS_*_KEYS`, the first column of the rows): keys.
- `LANGUAGE_ENGLISH` and `LANGUAGE_INDONESIAN`: each language's own name for itself.
- `STREAM_ADDRESS_HINT`: a sample web address.
- `BIBLES_SOURCE_EBIBLE` and `BIBLES_SOURCE_BEBLIA`: the names of two websites.
- The short chips on slides (V1, C, B and the others) are drawn by `artosia-core` from the kind of section and are not in `strings.rs`.

`tools/check-brief.py` holds the same list with the reasons.

## A round

1. **Briefs.** Add or change entries in `brief.json` (and `brief-phone.json`, `concepts.md`) first, then run `python -I tools/check-brief.py`. It lists every key the code reads and fails on a missing or extra entry, a missing field, a budget that is not a number up to 170, a concept the concepts file does not describe, or a `does` that shares four words in a row with the English text of its key. Measure a budget on a render: `cargo run -p artosia -- --snap out.png --scene <name>`, `tools/snap.sh <folder>` for every scene.
2. **The writer's word list.** One writer per language, given only the briefs, `concepts.md`, `human-prose` and the project's copy rules, and told that no source string exists and that none may be asked for. Before writing a single string the writer reads `concepts.md` and chooses one word or short phrase for each thing in it, and returns that list. The orchestrator checks it: one word for one thing, no word that means something else to a volunteer (a verse of a song against a verse of the Bible), no transliteration where the language has a word its church volunteers already say.
3. **The writer's table.** The same writer writes the whole table from the briefs and the word list, in the shape of `id.json` (`text`, plus `back` and `why` for the review). Each string comes back with a literal back-translation into English and one line on the register or grammar choice.
4. **A reader who sees only rendered screens.** A second person, who has not read the briefs and does not know the English, is given PNGs of every scene rendered in the language (`--language <code>`, `--indonesian` for Indonesian) and says what each control does. A control the reader cannot name is rewritten from its brief, never patched toward the English.
5. **The fit check.** Render every scene in the language and look for clipped text. A text over its budget goes back to its writer with the budget, not to an editor with scissors.
6. **Check meaning.** The back-translation of each key is compared with its brief. A mismatch fixes the brief and re-runs that key.
7. **Pin it.** A test for the language's known-wrong spellings, as `strings.rs` has for Indonesian.

Changing a text means changing its brief first and rewriting that key in every language from the brief. Never patch one language to match another.

## How languages are carried

The language is a code (`en`, `id`) in the settings, chosen in Settings, in the search, `--language <code>` or `--indonesian`. A settings file from before the code (`"indonesian": true`) still loads and means `id`.

- English and Indonesian are written in `strings.rs` today, as `text!(NAME, "English", "Indonesian")` and sentence functions with an `if indonesian()` branch. Indonesian moves to a table, `docs/i18n/id.json`, when the new table exists; a separate change swaps `strings.rs` over.
- Any other language is a table, `docs/i18n/<code>.json`: a key to `{"text": ...}`. `text` is a string with named placeholders (`{count}`), or plural forms (`zero`, `one`, `other`), or named cases (`some_lost`) that are each a string or plural forms. Arrays use the key `KEYS_SHOWING_ROWS[4].1`; section names are `section.verse`, `section.chorus` and so on with `section_numbered`; a counted noun inside a sentence is `noun.songs` with its own forms. The cases each sentence reads are the `cases` of its brief entry.
- Some sentence functions do not call `say(...)` yet (the habit, start-section and Bible-download sentences, `birthdays_cover_week`, `birthdays_cover_month`, `names_today`). Their brief entries name the variables the function will pass; the change that swaps the tables adds the `say` calls with exactly those names.
- A key the table lacks is English. The plural rule of each language is `plural_category` in `crates/artosia/src/languages.rs`.
- `languages.rs` holds the one list, `LANGUAGES`: code, the language's own name, its table, the words that find it in the search.
- The phone page has its own tables, `docs/i18n/phone-<code>.json`, with the keys of `STR.en` and optional `labels` (pairs of pattern and replacement for section names). They are listed in `EXTRA_LANGUAGES` in `crates/artosia-remote/src/languages.rs`. A code the page does not know shows English.
- `crates/artosia/testdata/strings-en.txt` and `strings-id.txt` hold what every text printed before the rework; regenerate with `ARTOSIA_WRITE_SNAPSHOT=1 cargo test -p artosia strings_snapshot`.
- `crates/artosia/testdata/pseudo.json` is a test-only language (`xx`, selectable with `--language xx` in a debug build, never listed).

## No real language ships unreviewed

A language ships only when a person who can judge it has reviewed it. Until then it stays out of `LANGUAGES`. Add the line to `LANGUAGES` (and the phone table to `EXTRA_LANGUAGES`) after step 7.
