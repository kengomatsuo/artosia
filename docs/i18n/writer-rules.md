# Writing Artosia's interface texts in your languages

Artosia is a worship presentation program for Windows, run by church volunteers at the console during a service.

## Rules

- No source string exists. Each text is written from its brief: the screen, the moment, the job. Never ask for an English or other-language version, and never open `crates/`, `strings.rs`, `brief.json`, `site/` or another language's table.
- Write what a church volunteer in that language would read naturally at that moment. Compose in the language; never shape a sentence after English clause order.
- Keep the product's own words. Before writing, read your language's existing table, `F:\Repositories\artosia\docs\i18n\<code>.json` (`id.json` for Indonesian), and reuse the word it already uses for each thing the briefs name (plan, announcement, design, window, key names and so on). Read `F:\Repositories\artosia\docs\i18n\concepts.md` for what each concept id means, including the rules in "Words that must stay apart". A concept your table has no word for yet: choose one word and use it in every text of this round.
- Stay inside each budget, counted in characters, where a Chinese, Japanese or Korean character counts as two.
- Never use an em dash, an en dash, a middle dot or an exclamation mark (also not the full-width one). No full stop at the end of a button, tab, label or fragment.
- Key names (Ctrl, Shift, PgUp, PgDn, Enter, Tab, F1) are written as the keyboard prints them, never translated.
- Copy rules: read `C:\Users\kenne\.claude\skills\human-prose\SKILL.md` Part 1 for your language's tells, and `C:\Users\kenne\.claude\skills\ui-copy\SKILL.md`.
- A variable in braces, such as `{name}`, stays exactly as written and every variable a brief lists appears in the text.

## What to return

For each language, write `<code>.json` into the round's folder, an object keyed by the brief's key:

```json
{"TAB_PLAN": {"text": "...", "back": "a literal back-translation into English", "why": "one line on the word or register choice"}}
```

Then reply with one line per language: its code and the number of texts written, and any brief you found thin.

## Sentences, cases and counts
- A brief of kind `sentence` lists its variables (keep each `{name}` exactly) and may list `cases` and `counts`.
- With cases, `text` is an object keyed by case name: `{"text": {"two_months": "...", "two_years": "..."}}`.
- With a count, `text` holds plural forms keyed `zero`, `one`, `other` (and `few`, `many` where the language has them; Chinese, Japanese, Korean, Indonesian and Vietnamese need only `other`, plus `zero` when the brief asks), each using `{count}`. A case can itself hold plural forms.
- Look at entries of the same shape in your language's table for the form it already uses.
