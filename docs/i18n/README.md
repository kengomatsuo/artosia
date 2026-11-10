# Operator window translations

No locale is the source the others are translated from. English is written from the brief like every other locale.

- `brief.json`: one entry per string in `crates/artosia/src/strings.rs`, in file order. Each says the screen, the moment, the role, the job (never the English wording), the character budget, the variables, the register and the product terms used.
- `glossary.md`: the product's fixed things, one word each per language, and the Indonesian words the phone page already chose. Writers get it with the brief.
- A writer for a language gets the brief and the glossary and never sees `strings.rs`. It returns each string with a back-translation.
- Changing a string means changing its brief first, then rewriting that key in every locale from the brief. Never patch one locale to match another.
- Re-run: add or change the entries in `brief.json`, run the check (every `pub const` and `pub fn` outside `#[cfg(test)]` has an entry, no `does` repeats four words of its own string, every budget is a number), then send the changed keys to the writers.
- Budgets are estimates from the rendered window (right column 344 pt wide) and must be re-checked on a render of each locale.

## How languages are carried

The language is a code (`en`, `id`) in the settings, chosen with Look, the search, `--language <code>` or `--indonesian`. A settings file from before the code (`"indonesian": true`) still loads and means `id`.

- English and Indonesian are written in `strings.rs`, because they are the two that were reviewed. A line there is `text!(NAME, "English", "Indonesian")`, and a sentence is a function with an `if indonesian()` branch. Both forms stay as they are.
- Any other language is a table, `docs/i18n/<code>.json`, in the shape of `id.json`: a key to `{"text": ...}`. `text` is a string with named placeholders (`{count}`), or plural forms (`zero`, `one`, `other`), or named cases (`some_lost`) that are each a string or plural forms. Arrays use the key `KEYS_SHOWING_ROWS[4].1`; section names are `section.verse`, `section.chorus` and so on with `section_numbered`; a counted noun inside a sentence is `noun.songs` with its own forms. The cases each sentence function reads are named in the `say(...)` call at the top of that function in `strings.rs`.
- A key the table lacks is English. The plural rule of each language is `plural_category` in `crates/artosia/src/languages.rs`.
- `languages.rs` holds the one list, `LANGUAGES`: code, the language's own name, its table, the words that find it in the search. The Look dialog and the search read the list.
- The phone page has its own tables, `docs/i18n/phone-<code>.json`, with the keys of `STR.en` in `crates/artosia-remote/src/page.html` and optional `labels` (pairs of pattern and replacement for section names). They are listed in `EXTRA_LANGUAGES` in `crates/artosia-remote/src/languages.rs`. A code the page does not know shows English.
- `crates/artosia/testdata/strings-en.txt` and `strings-id.txt` hold what every text printed before the rework. Both must stay byte for byte the same unless a text was changed on purpose; regenerate with `ARTOSIA_WRITE_SNAPSHOT=1 cargo test -p artosia strings_snapshot`.
- `crates/artosia/testdata/pseudo.json` is a test-only language (`xx`, selectable with `--language xx` in a debug build, never listed). It proves a table reaches a text, a plural sentence, an array row and the English fallback.

## No real third language is shipped

A language ships only when a person who can judge it has reviewed it. Until then it stays out of `LANGUAGES`. The steps, from the `blind-translation` skill:

1. Write the brief for the language's keys (`brief.json` and `glossary.md`), with the plural forms and cases the language needs.
2. A writer who never sees `strings.rs` writes the table from the brief.
3. Check the back-translation of each key against the brief; a mismatch fixes the brief, not the string.
4. Render the Look dialog, the key sheet and the search in the language and look for clipped text.
5. A reviewer who reads the language goes through the table and the render.
6. Add a test of the language's known-wrong spellings, as `strings.rs` has for Indonesian.
7. Add the line to `LANGUAGES` (and the phone table to `EXTRA_LANGUAGES`).
