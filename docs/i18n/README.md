# Operator window translations

No locale is the source the others are translated from. English is written from the brief like every other locale.

- `brief.json`: one entry per string in `crates/artosia/src/strings.rs`, in file order. Each says the screen, the moment, the role, the job (never the English wording), the character budget, the variables, the register and the product terms used.
- `glossary.md`: the product's fixed things, one word each per language, and the Indonesian words the phone page already chose. Writers get it with the brief.
- A writer for a language gets the brief and the glossary and never sees `strings.rs`. It returns each string with a back-translation.
- Changing a string means changing its brief first, then rewriting that key in every locale from the brief. Never patch one locale to match another.
- Re-run: add or change the entries in `brief.json`, run the check (every `pub const` and `pub fn` outside `#[cfg(test)]` has an entry, no `does` repeats four words of its own string, every budget is a number), then send the changed keys to the writers.
- Budgets are estimates from the rendered window (right column 344 pt wide) and must be re-checked on a render of each locale.
