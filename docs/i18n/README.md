# Operator window translations

No locale is the source the others are translated from. English is written from the briefs like every other locale, and Indonesian is a table like any other (`docs/i18n/id.json`), written from the same briefs by a writer who never sees English.

The rule behind all of it: **a writer never sees the source string and is never handed a ready-made word.** The briefs describe the screen and the job of each text; the writer chooses the words of the language. An earlier round hid the English but still handed over a glossary of ready-made Indonesian words, and the briefs described the English sentence instead of the screen. The result was a button that read as a transliteration. Both faults are removed: `glossary.md` and `id-choices.md` are deleted, and no file in this folder carries a word of any target language.

## What is in this folder

- `brief.json`: one entry for every text the operator window's table must hold, in the order of `strings.rs`. Fields: `key`, `kind`, `screen` (which window and where the text sits), `moment` (what the person just did), `role`, `does` (the job, never the English wording), `budget` (characters that really fit), `variables` (each with a made-up example), `cases`, `counts`, `register`, `concepts`, `notes`.
- `brief-phone.json`: the same for the phone page (`STR` in `crates/artosia-remote/src/page.html`) and its section labels.
- `concepts.md`: the things the product is made of, described in plain English and never named in any language.
- `id.json`: the Indonesian table, in the shape every table has. It is rewritten from the briefs in the next round; until then it is the table the program ships.
- `<code>.json`: the operator table of each further language (`zh-Hans`, `zh-Hant`, `ko`, `ja`, `es`, `pt-BR`, `fr`, `vi`), in the shape of `id.json`.
- `phone-<code>.json`: the phone page's table of each language. The setup's tables are `crates/artosia-setup/strings-<code>.json` and the homepage's `site/strings/<code>.json`, each with its own brief.
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

1. **Briefs.** Add or change entries in `brief.json` (and `brief-phone.json`, `concepts.md`) first, then run `python -I tools/check-brief.py`. It lists every key the code reads and fails on a missing or extra entry, a missing field, a budget that is not a number up to 240 (the longest text is the phone page's paragraph of keys), a concept the concepts file does not describe, or a `does` that shares four words in a row with the English text of its key. Measure a budget on a render: `cargo run -p artosia -- --snap out.png --scene <name>`, `tools/snap.sh <folder>` for every scene.
2. **The writer's word list.** One writer per language, given only the briefs, `concepts.md`, `human-prose` and the project's copy rules, and told that no source string exists and that none may be asked for. Before writing a single string the writer reads `concepts.md` and chooses one word or short phrase for each thing in it, and returns that list. The orchestrator checks it: one word for one thing, no word that means something else to a volunteer (a verse of a song against a verse of the Bible), no transliteration where the language has a word its church volunteers already say.
3. **The writer's table.** The same writer writes the whole table from the briefs and the word list, in the shape of `id.json` (`text`, plus `back` and `why` for the review). Each string comes back with a literal back-translation into English and one line on the register or grammar choice.
4. **A reader who sees only rendered screens.** A second person, who has not read the briefs and does not know the English, is given PNGs of every scene rendered in the language (`--language <code>`, `--indonesian` for Indonesian) and says what each control does. A control the reader cannot name is rewritten from its brief, never patched toward the English.
   What the reader marks goes back three ways, never as an edit of a string (second round, 2026-11-11, about 70 marks on 71 screens). One thing under two names, or two things under one, becomes a rule in the first section of `concepts.md`, "Words that must stay apart", and the writer revises the word list and every text that used the old word. A text the reader could not understand gets a fuller brief. And what words cannot mend is code: a text left in English because it never had a key (`why.rs` now gives every reason for a failed download as a string, where a library's own error text used to show), a name stored as data (looks are stored under fixed English names and shown through `look_label`), a row cut off by its box.
5. **The fit check.** Render every scene in the language and look for clipped text. A text over its budget goes back to its writer with the budget, not to an editor with scissors.
6. **Check meaning.** The back-translation of each key is compared with its brief. A mismatch fixes the brief and re-runs that key.
7. **Pin it.** A test for the language's known-wrong spellings, as `strings.rs` has for Indonesian. Rows that are briefed by position are pinned too: `every_key_row_has_the_brief_of_its_own_keys` fails when a row of the keys sheet moves in the code without its brief, which once put "take it out of the plan" beside Ctrl M.

Changing a text means changing its brief first and rewriting that key in every language from the brief. Never patch one language to match another.

## How languages are carried

Ten languages are listed: English, Indonesian, Simplified Chinese (`zh-Hans`), Traditional Chinese (`zh-Hant`), Korean, Japanese, Spanish, Brazilian Portuguese (`pt-BR`), French and Vietnamese. The operator window, the phone page, the setup and the homepage speak all ten.

The language is a code (`en`, `id`, `zh-Hans` and so on) in the settings, chosen in Settings, in the search, `--language <code>` or `--indonesian`. A settings file from before the code (`"indonesian": true`) still loads and means `id`.

- English and Indonesian are written in `strings.rs` today, as `text!(NAME, "English", "Indonesian")` and sentence functions with an `if indonesian()` branch. Indonesian moves to a table, `docs/i18n/id.json`, when the new table exists; a separate change swaps `strings.rs` over.
- Any other language is a table, `docs/i18n/<code>.json`: a key to `{"text": ...}`. `text` is a string with named placeholders (`{count}`), or plural forms (`zero`, `one`, `other`), or named cases (`some_lost`) that are each a string or plural forms. Arrays use the key `KEYS_SHOWING_ROWS[4].1`; section names are `section.verse`, `section.chorus` and so on with `section_numbered`; a counted noun inside a sentence is `noun.songs` with its own forms. The cases each sentence reads are the `cases` of its brief entry.
- Some sentence functions do not call `say(...)` yet (the habit, start-section and Bible-download sentences, `birthdays_cover_week`, `birthdays_cover_month`, `names_today`). Their brief entries name the variables the function will pass; the change that swaps the tables adds the `say` calls with exactly those names.
- A key the table lacks is English. The plural rule of each language is `plural_category` in `crates/artosia/src/languages.rs`.
- `languages.rs` holds the one list, `LANGUAGES`: code, the language's own name, its table, the words that find it in the search.
- The phone page has its own tables, `docs/i18n/phone-<code>.json`, listed in `EXTRA_LANGUAGES` in `crates/artosia-remote/src/languages.rs`. A phone table has the shape of an operator table (the keys of `STR.en`, and `label.verse`, `label.numbered` and their neighbours for the names of a song's parts); the server flattens it when it serves the page (`flat`). A code the page does not know shows English.
- The setup lists its tables in `crates/artosia-setup/src/strings.rs` and the homepage in `LANGUAGES` of `tools/build-site.py`.
- One rule fits a computer's or a browser's language tag to a language: `crates/artosia-locale` (`pick`, `pick_first`, `languages`, `region`). The same code wins; Chinese goes by script (a tag that says Hant, or names Taiwan, Hong Kong or Macau, is Traditional); then the same first part. The setup uses it, the phone page repeats it in JavaScript (`fit` in `page.html`) and the homepage's first page repeats it in its redirect (`REDIRECT` in `tools/build-site.py`).
- The song reader learns every language's section names at start (`languages::teach_markers`), so `[Estrofa 2]` is read whatever language the window speaks. A marker is written in the window's words only when it reads back as the same section.
- `crates/artosia/testdata/strings-en.txt` and `strings-id.txt` hold what every text printed before the rework; regenerate with `ARTOSIA_WRITE_SNAPSHOT=1 cargo test -p artosia strings_snapshot`.
- `crates/artosia/testdata/pseudo.json` is a test-only language (`xx`, selectable with `--language xx` in a debug build, never listed).

## Writers compose every language, native speakers correct after (owner, 2026-11-12)

A language is listed once its writer has composed it from the briefs and the tests pass; a second reader checks rendered screens, and the texts are then opened for native speakers to correct. This replaces the earlier rule that a language waits for a human reviewer before it is listed. Each of the eight newest was written by its own writer who never saw another language.

## Adding a further language

1. Write the four tables from their briefs, one writer, no other language in view: `docs/i18n/<code>.json`, `docs/i18n/phone-<code>.json`, `crates/artosia-setup/strings-<code>.json`, `site/strings/<code>.json`.
2. List them: a line in `LANGUAGES` (`crates/artosia/src/languages.rs`), in `EXTRA_LANGUAGES` (`crates/artosia-remote/src/languages.rs`), in the setup's strings and in `LANGUAGES` of `tools/build-site.py`; add its plural rule to `plural_category`.
3. Run the tests that hold every table to the briefs' widths and variables: `cargo test -p artosia languages`, `cargo test -p artosia-remote`, `cargo test -p artosia-setup`.
4. Render the screens in it (steps 4 and 5 of "A round"; `tools/site-shots.sh` for the homepage) and give them to a second reader.

## A round for new texts (the overhaul's way, 2026-11-14)

1. Briefs: add entries with `python -I tools/add-briefs.py . <entries.json>` (or by hand), then `python -I tools/check-brief.py` to 0 problems.
2. Pack the briefs no table answers yet: `python -I tools/i18n-round.py pack . <round>/briefs.md <keys...>`.
3. Three writer agents in parallel (Indonesian, Vietnamese and French; the two Chinese, Japanese and Korean; Spanish and Brazilian Portuguese), each given `docs/i18n/writer-rules.md`, the pack and the round folder, and nothing else.
4. Read every back-translation against its brief; send a wrong key back with a fuller brief.
5. `python -I tools/i18n-round.py merge . <round>`, then `ARTOSIA_WRITE_SNAPSHOT=1 cargo test -p artosia strings_snapshot` when English or Indonesian changed, then the tests, a fresh `cargo build` and renders in two or three languages.
