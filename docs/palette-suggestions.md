# Palette suggestions: a proposal

Status: accepted by the owner on 2026-11-11 ("behavioural, even relationships like after this song usually find this song"). Built; see "Built" at the end.

The owner's question: can the palette be smarter with suggestions, "like google's suggestion logic, maybe based on past calls"?

The short answer is yes, and the tools people already trust do it the same small way. They keep a log of what was picked for what was typed, count the recent picks, and use the count to order results that already matched. None of them lets the habit replace the match. This proposal adds that to Artosia as one new rule at the bottom of the existing cascade, plus a short "suggested" block for the empty box.

## What the code does today

| Part | What it does | File |
|---|---|---|
| Song order | `Facts`, a derived `Ord`: numbered, covered, preferred (in today's plan), together, typos, in_title, exact, infix, then library order | `crates/artosia-core/src/search.rs` |
| Other things | `DocHit` and `compare`: preferred, typos, exact, infix, weight, proximity, position, then document index | `crates/artosia-core/src/palette.rs` |
| Typed query | `query_hits` puts a typed command, a Bible reference, planned songs that hold every word, commands by name, other songs, other finds, then verses | `crates/artosia/src/palette.rs` |
| Empty query | `browse_hits` shows New song, Paste song, then every song in library order | same |
| Storage | One SQLite file. Tables `songs`, `plans`, `announcements`, `people`, `settings`. New columns are added in `Store::prepare` by checking `pragma_table_info` | `crates/artosia-core/src/store.rs` |
| `plans` | One row per plan with the items in a JSON `body`, updated in place by `save_plan` | same |

Nothing in this chain remembers a pick. `preferred` only knows what is in today's plan.

## What the owner's house-kit already has

I read `core/palette` in `kengomatsuo/house-kit` (`palette.ts`, `paletteTrail.ts`, `README.md`).

| Piece | History or learning? |
|---|---|
| `palette.ts`, `SearchOptions.preferred` | A set of keys that "ranked before everything else when it matches, for 'what I picked last time'". It is the first line of the kit's cascade. The kit stores nothing: the caller must build the set. No recency, no count, no typed text. |
| `paletteTrail.ts` | A pure reducer that turns palette events into one row per settled phrase: the phrase, the folded phrase, the picked key, the picked rank, milliseconds to choose, the chips in scope. The caller saves the rows. Nothing reads them back to rank. |
| `react/palette-react` | `preferred` is passed through. No history store. |

So the kit has the two ends (a place to put a habit, a record of picks) and nothing in between. A search of the repository for `frecency` found nothing. Artosia's `preferred` argument was copied from the kit, and the kit's own comment calls it "the only signal here that knows anything about them", which is the gap this proposal fills. `paletteTrail`'s row is a good model for the log below, and its 800 ms rule (a phrase replaced within 800 ms was still being typed) is worth copying.

## What trusted tools do

| Tool | What it records | Formula or rule as published | How old picks fade | Exact match protection |
|---|---|---|---|---|
| Firefox address bar, adaptive history | The typed string, the page picked, a `use_count` | On pick: `use_count * .9 + 1`, "asymptotically approaches a max of 10". Rank per page: `ROUND(MAX(use_count) * (1 + (input = :search_string)), 1)`. It matches every stored string that starts with the input. Ties go to frecency | `use_count` is multiplied by 0.975 on daily idle. Below 0.975^90 (0.1) the entry is removed, "unused for 90 days" | An input equal to the stored string doubles the rank |
| Firefox frecency | Visits | Per visit: `weight * e^(-lambda * age)`, lambda = ln 2 / 30. Total: `sum / samplecount * visitcount`. Stored as `t_ref + ln(score) / lambda` | Half-life 30 days, applied through the stored value | Not stated on the page I read |
| zoxide | A score per directory: 1 on first use, plus 1 per later use | Frecency at query time from the last access: within an hour `score * 4`, a day `* 2`, a week `/ 2`, otherwise `/ 4` | When total score passes `_ZO_MAXAGE`, every score is divided so the total is about 90% of it; scores under 1 are deleted | None. It ranks directories only |
| Slack quick switcher | The typed query, the id picked, a count, the last ten timestamps | `Total Count * Score / Number of Timestamps` (up to 10), where Score sums time-bucket points: 4 h 100, a day 80, 3 days 60, a week 40, a month 20, 90 days 10, beyond 0 | Buckets. Only ten timestamps are kept | Frecency reorders the results the default search already returned; an item scores zero unless a history entry points to it. A pick reached by another query gets half the points, "won't be high enough to override" the usual query |
| VS Code command palette | The command id in an LRU list with a counter | Used commands go before unused ones, newest first. Setting `workbench.commandPalette.history`, default 50, 0 turns it off | The list holds 50, the oldest drops out | History only orders commands the filter already matched. Matches found by tf-idf sort to the bottom |
| VS Code quick open | Recently opened files | History is a separate list placed first. `search.quickOpen.history.filterSortOrder` is `default` (by match relevance) or `recency` | A bounded list | A history file shows above file results, so history does outrank a better match there. There is no score comparison between the two lists |
| Raycast | Picks per typed query | "The more often, and the more recently, you pick a result for a given query, the higher it ranks". Order: exact alias, alias prefix, title fuzzy score, subtitle and keywords, then frecency | Not published | Frecency is the last tier. "Reset Ranking" clears it per command |
| Alfred | Usage, latched to the typed query | "Alfred learns based on the exact typed query", so "c" and "ca" can have different top results | "a 4 week rolling window" | Learning is per exact query. "Clear Knowledge" in Advanced preferences resets it |
| OpenLP | A Song Usage log: date and time, title, copyright, CCLI number, author, displayed or printed | No ranking. It exists for CCLI or similar licensing reports, exported as text between two dates | Records stay until deleted by date | Not applicable. It suggests nothing |
| FreeShow | Its `history` code is the undo and redo stack (`history.ts`, `ActionHistory.svelte`) | No song suggestion found | Not applicable | Not applicable |
| ProPresenter | Quick Search, Library results "in alphabetical order" | No suggestion or usage ranking found in the article | Not applicable | Not applicable |

What these share, and what Artosia takes from it:

1. The unit of learning is the pair (what was typed, what was picked). Firefox, Slack, Raycast and Alfred all key on the typed text.
2. The habit reorders what already matched. Slack and VS Code say so in their own words. Raycast puts frecency last. Firefox ties break to frecency.
3. A short window or a decay removes old habits: 90 days (Firefox, Slack), 4 weeks (Alfred).
4. A reset exists (Raycast, Alfred).
5. Among worship tools, I found OpenLP recording usage for licence reports and no worship tool suggesting songs from history. That is a gap Artosia could fill, and also a reason to assume nothing about what churches want.

## 1. What to record

Two tables in the library's SQLite file. Both are created by `create table if not exists`, so an older library gains them on open, like the existing columns.

```sql
-- What the volunteer picked in the palette.
create table if not exists palette_picks (
    id           integer primary key,
    picked_at    integer not null,           -- unix seconds
    service_date text    not null,           -- local date, 'YYYY-MM-DD'
    weekday      integer not null,           -- local, 0 Monday to 6 Sunday
    plan_id      integer,                    -- plans.id, null with no plan
    kind         text    not null,           -- 'song', 'passage', 'announcement', 'look', 'command'
    target       text    not null,           -- 'song:123', 'command:black', 'look:Forest'
    typed        text    not null default '',-- folded query at the pick, at most 40 characters, '' from the empty list
    shown_rank   integer,                    -- zero-based row the pick sat in
    action       text    not null            -- 'open', 'show_now', 'add_to_plan'
);
create index if not exists palette_picks_typed on palette_picks (typed, target);
create index if not exists palette_picks_when  on palette_picks (picked_at);

-- What was put live, in order: the record "what follows" reads.
create table if not exists service_songs (
    id           integer primary key,
    service_date text    not null,
    plan_id      integer,
    seq          integer not null,           -- 1, 2, 3 within that date
    song_id      integer not null,
    shown_at     integer not null
);
create index if not exists service_songs_song on service_songs (song_id, service_date);
```

Why two: `palette_picks` says what was searched for. A song that arrives in the plan from last week's copy, or from the remote, never passes the palette, and "what follows" needs what was sung. `plans` cannot supply it, because `save_plan` updates a plan row in place, so a past service is lost once the plan is edited.

Must not be recorded:

| Not recorded | Why |
|---|---|
| Lyrics, titles as text, announcement bodies | The id is enough. A title can change on import, and the log must not become a second copy of the church's library |
| The text of `message ...`, `stage ...` and countdown commands | Free text people type to the room. It can hold a name or a plea. `kind` for these is never written |
| Birthday and people names | They sit in `people` and must stay out of any log |
| A query that ended without a pick | Half-typed words are not a choice. `paletteTrail` drops them too (800 ms rule) |
| Every keystroke | Only the settled query at the pick |
| Picks from `--practice` | The practice library is in memory and its picks would teach the real one nothing |
| Anything leaving the PC | No upload, no analytics. The log lives in the library file only |

Two housekeeping rules. Delete rows older than two years at start. When a library is exported or shared, leave these tables out. A "Forget what the search learned" button clears both tables, the same reset Raycast ("Reset Ranking") and Alfred ("Clear Knowledge") offer.

## 2. Where it enters the cascade

One new rule, last, so it only speaks when everything else tied.

Songs, in `Facts`:

```rust
struct Facts {
    numbered, covered, preferred, together, typos, in_title, exact, infix,
    /// Picks of this song in the last 12 weeks for a typed query that starts with this one (0 to 5).
    habit: Reverse<u8>,   // NEW, last line
}
```

Search documents, in `compare`, after `position` and before `doc`:

```rust
.then(a.position.cmp(&b.position))
.then(b.habit.cmp(&a.habit))   // NEW
.then(a.doc.cmp(&b.doc))
```

Why last:

| A row that must stay above a habit | Rule that holds it | Still true with `habit` last? |
|---|---|---|
| A song in today's plan | `preferred`, third line of `Facts` | Yes, nine lines above |
| A title typed exactly | `in_title` and `exact` | Yes. "berkat" puts `Berkat` above a habit for `Diberkati` |
| A number typed | `numbered`, first line | Yes |
| A typed command, a Bible reference | Built before songs in `query_hits` | Yes. `habit` is never read for them |
| A command by name only when typed from its start | `by_name` in `query_hits` | Yes. The habit rule cannot lift a command into that gate |

What it does at the bottom: it orders songs that tie on every other fact. That is the common case for a short prefix. Typed "ka", every song with a word starting "ka" in its title ties on covered, together, typos, in_title, exact and infix, so today they come out in library order. The habit chooses the order among them.

What it cannot do: lift a song that matches only in its lyrics above songs that match in their titles. That is the existing rule and it stays. If testing shows volunteers want it, moving `habit` above `in_title` is a one-line change, to be taken as its own decision (see "Later").

What a row says to explain itself: a chip beside the existing "Plan" chip, in English and Indonesian from `strings.rs` (the brief goes into `docs/i18n/brief.json` first, per the house rule):

> Picked for "ka" 4 times, last 9 Nov

The chip appears only when `habit` decided something, meaning the song had a count above zero. A volunteer can read the row's place from that line alone. The chip is a plain sentence with real numbers, never a score.

What counts as a habit:

| Choice | Value | Reason |
|---|---|---|
| Window | Last 12 weeks | Services are weekly and the song set drifts through the year. 12 weeks is about one season and matches Firefox's and Slack's 90 days. It is my choice and should be tuned on the synthetic history |
| Minimum | 2 picks | One pick is an accident or a one-off |
| Cap | 5 | A volunteer's tenth pick for "ka" should not be unreachable for a new favourite |
| Match on typed text | The stored `typed` starts with the input | The rule Firefox documents. Typing "k" finds picks stored under "ka" and "kasih" |
| Empty `typed` | Never counted for habit | An empty prefix matches everything. These rows feed suggestions only |
| Fading | The window slides. No decay job, no stored count | The log is the truth, the count is a `select count(*)`, so a wrong number can always be recomputed |

The query, run once per keystroke settle, not per row:

```sql
select target, count(*) as picks, max(picked_at) as last
from palette_picks
where typed like ?1 || '%' and typed <> '' and kind = 'song'
  and picked_at >= ?2            -- now minus 84 days
group by target having picks >= 2;
```

The result goes in as a `HashMap<usize, Habit>` next to the `preferred` closure that `search_with` already takes. The index on `(typed, target)` makes the prefix scan cheap at this size.

## 3. Suggestions before typing

Today the empty box shows New song, Paste song and every song alphabetically. Proposed: after those two commands, a block titled "Suggested" with at most five rows, each with a reason. Enter still opens the first row, which stays New song. Suggestions never go to the projector and never add themselves to the plan.

| Candidate | Data it needs | Reason shown | Risk of being wrong |
|---|---|---|---|
| Rest of last service's plan | `service_songs` for the latest earlier `service_date`, minus songs already in today's plan | "In last Sunday's plan" | Low harm. Churches often avoid repeats, and this list could be exactly the songs not wanted. I could not find how this church plans, so the block is off by default until the owner says it helps |
| Sung on this weekday in recent weeks | `service_songs` joined to the same weekday over the last 8 weeks, counted per song, top three not in the plan | "Sung in 3 of the last 8 Sundays" | A song set drifts and seasons change (Advent, Easter). The 8-week window limits it. A song sung every week fills the list forever, so cap the count at 5 services |
| What usually follows the live song | `service_songs` pairs (song at seq n, song at seq n+1) across all services in 26 weeks; shown only while a song is live and a pair occurred at least twice | "After "Song A" in 3 of 4 services" | Small samples. Two services can be a coincidence. A medley that is always sung together is the good case. Show only the best pair, and only with 2 or more |
| Recent picks with no typing | `palette_picks` where `typed = ''`, last 3 weeks | "Picked from the list last week" | Low. It repeats what the volunteer did, and shows it |

Order inside the block: the song after the live one, then the weekday songs, then last week's leftovers. A song already in the plan never appears here, since its row is "Plan" elsewhere. If there is no history, the block is not drawn, so the first week looks exactly like today.

## 4. Learned picks while typing

Stored: the final typed text at the pick, folded the way search folds it (`text::normalize`), at most 40 characters. Prefixes are not stored. One row per pick holds the full typed string, and the match is `typed like input || '%'`. Typing "k", "ka" and "kas" each find a pick stored as "kasih", as Firefox's adaptive history does.

A pick made after typing "ka" and then "kasih" is stored as "kasih", the settled text. The two-keystroke case "ka" followed by Enter is stored as "ka". Both are honest records of what the volunteer typed when they chose.

Fading: the 12-week window. Five picks for "ka" in March, none since June, count zero in September. The song returns to library order within its tie. An 800 ms settle rule (copied from `paletteTrail`) keeps a phrase replaced almost at once from counting.

One risk to test: Indonesian titles that share a first word ("Kasih ..." in many songs). The tie class is large, so the habit does real work there. This matches the church's library of 1,335 songs of which 1,232 are Indonesian only, per `.claude/rules/core.md`.

## 5. What it costs

Estimates from stated assumptions. I have no real log to measure.

| Item | Assumption | Result |
|---|---|---|
| Services a year | 52 weekly, 8 extra | 60 |
| Palette picks per service | about 12 (building and running the plan) | about 700 rows a year |
| Songs shown live per service | about 5 | about 300 `service_songs` rows a year |
| Size | about 100 bytes a row plus two small indexes | about 0.2 MB a year |
| Two-year cap | the housekeeping rule | under 0.5 MB |
| Lookup time | prefix scan over about 1,500 rows once per settle | well under a millisecond; to be measured with the existing 3,000-song speed test as the model |
| Extra RAM | a `HashMap` of a few dozen entries per query | none worth measuring, but `tools/ram/measure.ps1` should be run once after the first step, per the project's first constraint |

Work in days:

| Piece | Days |
|---|---|
| Tables, writes in `pick` and on show, `--practice` off, tests | 1 |
| `habit` in `Facts` and `compare`, the chip, strings in both languages | 1 |
| "Suggested" block with the four candidates | 1.5 |
| Synthetic history and its tests | 1 |
| Forget button, export exclusion, cleanup | 0.5 |
| Total | about 5 |

Testing without a year of history: a seeded generator in `artosia-core` (a test module, no real lyrics). It writes 104 weeks of services against the practice library, with the church's song set drifting: four seasons, each with its own twelve favourites drawn by weight, five songs a service, and volunteers who type the first two to four letters of a title with 20% variation. The tests:

| Test | Passes when |
|---|---|
| A habit never beats a plan song | a song in today's plan stays above any habit for the same query |
| A habit never beats an exact title | typing a whole title puts that song first, whatever the history says |
| A habit orders a tie | with five picks for "ka" in the last 12 weeks, that song leads the "ka" titles |
| A habit fades | with the same picks 13 weeks ago, the order equals library order |
| One pick is not a habit | a single pick changes nothing |
| Season drift | after the generator swaps its favourites, the old favourites drop out of "Suggested" within 8 weeks |
| Practice is silent | `--practice` writes no rows |
| Reset | "Forget" empties both tables and the order returns to library order |

The church's own song titles from `fixtures/church/library` (gitignored) can drive the same generator locally for realistic prefix ties, without any of it entering the repository.

## 6. A first step that fits in a day

Build only this:

1. The two tables, created in `Store::prepare`, with `record_pick` and `habits_for(prefix, now)` and a test for each.
2. `record_pick` called from `Palette::pick` for song picks that have typed text, with `--practice` skipped.
3. `habit` as the last line of `Facts`, fed through `search_with`, and the chip "Picked for "ka" N times, last D Mon".
4. Two tests: a plan song stays above a habit, an exact title stays above a habit.

That gives the owner the feature that is closest to "Google's suggestion" (the song you usually pick for "ka" comes first among the "ka" songs) and begins collecting the data every later piece needs. Collecting starts at once, which matters because history cannot be backfilled.

Left for later:

| Later | Why it waits |
|---|---|
| `service_songs` and the "Suggested" block | Needs the owner's answer on repeats (see Not confirmed) and a few weeks of data |
| Announcements, looks and commands in the habit rule | Songs carry the weight. Commands have the by-name gate |
| Moving `habit` above `in_title` | Only if volunteers miss lyric-only habits. A separate decision |
| Passage picks | Bible references are typed whole, so a habit helps little |
| Weighting an exactly typed query above a prefix (Firefox doubles it) | A cascade has no multipliers. It would be its own earlier line |

## 7. What not to build

| Not built | Reason |
|---|---|
| Cloud learning | The church's library stays on its PC, and the log holds when the church sings what. Nothing here needs a server, and a server would add a place for it to leak |
| Collaborative filtering across churches | It would need every church's picks sent somewhere, the churches' rotations are unrelated, and with one church per library the signal from others is mostly noise. I found no worship tool that does it |
| A blended score | The cascade exists so a volunteer can say why a row is first. Firefox, Slack and zoxide multiply counts and time bonuses into one number. That fits a browser, where nobody has to explain row three. In Artosia a volunteer under pressure must see one reason, "Picked for ka 4 times" |
| A model, embeddings or text prediction | The log of a single church holds about 700 rows a year. That is too little for a model and too much to hide behind one |
| Query completion like Google's | Google's suggestions rest on other people's queries, which Artosia does not have and must not collect. What it has is the volunteer's own picks, which is what this proposal uses |
| Silent auto-add to the plan | A wrong guess must never reach the projector or the plan without a key press |

## Sources

All read 2026-11-11.

- Firefox address bar ranking (frecency, adaptive history): https://firefox-source-docs.mozilla.org/browser/urlbar/ranking.html (the formulas were re-checked against the page's text by `curl`)
- zoxide algorithm: https://github.com/ajeetdsouza/zoxide/wiki/Algorithm (re-checked in the wiki's raw Markdown: https://raw.githubusercontent.com/wiki/ajeetdsouza/zoxide/Algorithm.md)
- Slack quick switcher: https://slack.engineering/a-faster-smarter-quick-switcher/ (re-checked by `curl`)
- VS Code command palette history: https://raw.githubusercontent.com/microsoft/vscode/main/src/vs/platform/quickinput/browser/commandsQuickAccess.ts
- VS Code quick open history: https://raw.githubusercontent.com/microsoft/vscode/main/src/vs/workbench/contrib/search/browser/anythingQuickAccess.ts
- VS Code setting definitions, read through `gh api`: `src/vs/workbench/browser/workbench.contribution.ts` (`workbench.commandPalette.history`, default 50) and `src/vs/workbench/contrib/search/browser/search.contribution.ts` (`search.quickOpen.history.filterSortOrder`, `default` or `recency`) in `microsoft/vscode`
- Raycast search bar: https://manual.raycast.com/search-bar
- Alfred result ordering: https://www.alfredapp.com/help/kb/understanding-result-ordering/
- OpenLP song usage: https://manual.openlp.org/song_usage.html
- FreeShow: https://github.com/ChurchApps/FreeShow, `src/frontend/components/helpers/history.ts`
- ProPresenter quick search: https://support.renewedvision.com/hc/en-us/articles/9808905773715-How-to-Use-Quick-Search-within-ProPresenter
- The owner's palette kit, read through `gh api`: `kengomatsuo/house-kit`, `core/palette/palette.ts`, `paletteTrail.ts`, `README.md`, and `react/palette-react/README.md`
- Artosia code: `docs/operator.md` ("Search is the library"), `crates/artosia-core/src/palette.rs`, `crates/artosia-core/src/search.rs`, `crates/artosia/src/palette.rs`, `crates/artosia-core/src/store.rs`

## Not confirmed

- Firefox's separate Places frecency page (`/browser/places/frecency.html`) returned 404. The frecency formula above is the one on the urlbar ranking page. The Firefox adaptive-history formula was confirmed in that page's text.
- Raycast and VS Code's Quick Open were read through a summarising fetch only, not the raw page. The Raycast quotation is from that summary. Raycast publishes no decay figure that I could find.
- VS Code's default settings page on the docs site did not list the history settings. The values come from the source file.
- Whether FreeShow or ProPresenter suggest songs from usage: I found no sign of it in the pages and files named above. That is an absence in what I read, not a proof. FreeShow's search for `lastUsed` returned 11 code hits that I did not open.
- Whether this church makes a new plan each week or edits one plan in place. `save_plan` updates a row in place, so the design records a `service_date` itself. If a new plan is made weekly, `plan_id` alone would do.
- Whether volunteers want last week's songs suggested or avoided. That is a question for the owner and the worship team, not for research. The proposal ships that block off until answered.
- The window (12 weeks), the minimum (2), the cap (5) and the row counts are my choices and estimates. They need the synthetic history and, later, a real month of data.
- Slack's and Firefox's numbers describe their products at the time of the pages read. I have not checked whether either has changed since.

## Built

Written 2026-11-11, on the branch of the agent that built it.

What exists:

| Piece | Where |
|---|---|
| Tables `palette_picks`, `service_songs`, `service_passages` (created with the other tables, so an older library gains them) | `crates/artosia-core/src/store.rs` |
| Writes, reads, forget with undo, two-year prune | `crates/artosia-core/src/learn.rs` |
| `habit` as the last line of `Facts`, and in `compare` after `position` | `search.rs`, `palette.rs` in `artosia-core` |
| Recording at `pick` and at `show`, groups, reasons, switch, demo history | `crates/artosia/src/learning.rs` |
| Groups and chips on the rows, the sentence in the preview | `crates/artosia/src/ui/palette.rs` |
| "Learn from use" and "Forget what the search learned" | last section of the Look dialog, `ui/sheets.rs` |
| Scenes `learned-browse` and `learned-search` | `main.rs` |

Choices that differ from the proposal above:

- `service_songs` also keeps `weekday`, `typed`, `start_tag` and `from_start`; passages go in `service_passages`, because a passage has no song id. A passage takes a place in the service's order but is skipped when reading what followed a song.
- The side-by-side and nearest-the-start reads in `search_ranked` still come before `habit`, so `habit` orders only songs that tie on everything else. Putting it in `Facts` alone would have placed it above them.
- A song is recorded as going live only when another deck was live before it; a second slide of the same song is not a new song.
- Followers need two services. The weekday group reads eight weeks, counts at most 5 services, and needs 2. The recent-picks group reads three weeks. A song shows in one group only, never when it is in today's plan or live now.
- "Usually started at the chorus": a song that went live at the same section other than its first, three times or more and in 60% of its starts or more, over 26 weeks. It shows as a chip and in the preview; Enter opens the song as before.
- Not built, as the proposal says: last service's plan (off until the owner answers), habit for announcements, looks, commands and passages. `compare` carries the field and a test; nothing feeds it yet.
- Recording needs a real library folder, a real window, no `--practice`, and the switch on. Tests turn `learn.recording` on by hand.

Verified: `cargo test -p artosia -p artosia-core` (store and cascade tests on a made-up clock and offset: window, fade, cap, follower pairs, weekday, recent, start habit, forget and undo, prune; app tests: habit never beats a plan song, an exact title or a number, nothing recorded in practice, no message text kept), `cargo clippy` clean, and `--snap` renders of both scenes in light, dark and Indonesian plus the Look dialog scrolled to the end, and a script that steps over a heading with Down.

Not verified: a real window on the second screen, the RAM measure, the speed of the habit query on a library with two years of rows, and the Indonesian text (English stands in both languages until it is written).

## Suggestions in the window

Written 2026-11-11, after the owner's three moments of panic: a medley song nobody put in the plan, a priest who starts a song unannounced after the sermon, and a volunteer who wants to be told what usually comes next. The pieces already existed and hid: the listener's phrases (`heard_final`) showed only in the Listen dialog and F4, and the learned followers only in the empty search. `crates/artosia/src/suggest.rs` is the model and `ui/suggest.rs` the strip; [operator.md](operator.md) has the table of when each offer shows.

How a heard phrase is read. A sung phrase runs over several lines and a library of 1,335 songs holds every common word somewhere, so the old test (`complete`: every word found anywhere in the song) is too weak to name a song. The phrase is cut into chunks of six words, stepping three, and each chunk is searched alone. A chunk names a song only when a single title or lyric line holds at least 70% of its words and at least three (`SongHit::together`, added to `artosia-core`). The song with the most agreeing chunks comes first, and the matching line gives the section.

How sure "the words being sung are not on the screen" can be. Not sure, and it is built so that it stays silent when it cannot be. The listener reports no confidence, only text, and `crates/artosia-listen/README.md` estimates that base on a mixed signal gets more than half the sung words wrong. A phrase that matches nothing can therefore mean a song outside the library or a recogniser that cannot read this room, and the two cannot be told apart from one phrase. The line shows only when (a) a song is live, (b) some phrase matched a song in the last four minutes, which shows the listener can read this room, and (c) the next two phrases of four words or more within 90 seconds both match nothing, the newest less than 30 seconds old. It disappears when a phrase matches, when the singing stops, and when a person presses it. It is untested against a real model: with a base model and a loud band it may never show, which is the safe failure, and with a close vocal microphone it may fire on a song that really is missing. What it claims is only "the words were not found", never why.

Choices the owner may want different: the 60-second life of a heard offer; two misses before the line; 70% of a chunk; a part of the live song is "far" at two slides; followers are shown only with an empty Preview and nothing after the live item; the chorus a song usually starts at (`learn.starts`) is not used by the strip, so a follower opens at its first section.

Not verified: a real window (the shell has not placed the strip yet), a real microphone, a speech model, Indonesian text (English stands in both languages until it is written), and the cost of up to six searches per heard phrase on a slow PC.
