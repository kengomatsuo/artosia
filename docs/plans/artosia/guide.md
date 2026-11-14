# The guide: tutorials, help at each feature, what's new (GUIDE1, 2026-11-14)

Owner's requirements [card: GUIDE1] [card: HELPNEAR1] [card: WHATSNEW1]:
1. Interactive tutorials written as goals ("open the calendar"); when several controls do it, the
   step uses the one a person would think of first.
2. Every step checks itself: it shows only when it can be done, finishes when the goal is reached,
   and recovers when the person goes elsewhere.
3. Every explanation carries an animation of the real program.
4. Help sits at each feature, findable again in or beside the feature.
5. New features are announced inside the program, with an animation.

Research: `~/Shared/research/artosia-guide-framework/` (prior art, JetBrains IDE Features Trainer
source saved there) and `~/Shared/inspo/artosia-guide/` (help marks, coach marks, lesson lists,
what's new, recovery; Mobbin and rivals). Read both READMEs before task G3.

## The model, after JetBrains' lesson engine

JetBrains (`jb-TaskContext.kt`, saved): a lesson is tasks; a task has `before` (prepare),
`trigger(actionId)` (wait for the person's real action), `stateCheck` (the goal holds),
`restoreState`/`restoreByUi` (when the needed UI is gone, return to the step that brings it back),
`triggerUI` (find the component by predicate and highlight it), and `test { }` (a scripted run that
proves the lesson can be passed, run in CI).

Artosia already names every action: each is a `Command` in `crates/artosia/src/palette.rs`, also
reachable from search [rule: .claude/rules/operator.md, "every menu action is also a command"]. So:

| Part | What it is |
|---|---|
| `Goal` | a `Command` to run, or a state to reach (a predicate on `App`) |
| `Anchor` | a control that fires a command: element key, the command, and its place (top bar, panel, menu, search) |
| Ranking | among anchors of one command: on screen and enabled now, then in the open panel, then a named button before a menu row, then fewest presses to reach it (a tab or dialog to open first becomes a step of its own); search is the last way and is always there |
| `Step` | goal, text, media, `before` (prepare state), `ready` (precondition: an anchor is found, visible and enabled, via the layout's rectangles), `done` (goal check), `restore` (the step to go back to when `ready` fails mid-way) |
| `Lesson` | steps, a name, the feature it belongs to, and a scripted test |
| Runner | a state machine with no window: shows a step when `ready`, advances when `done`, steps back when the anchor vanishes, never sends anything to the projector |
| Media | frames recorded from the real program by Rune's headless `--script` and `--strip` (`tools/record-guide.sh`), played in the bubble as frames, because Rune's video player runs on macOS only [rule: .claude/rules/rune.md] |

## Tasks

| Id | Task | Done when | Verify |
|---|---|---|---|
| G1 | `crates/artosia/src/guide/` step model and runner (pure logic) | a lesson advances, waits, steps back and ends in tests | `cargo test -p artosia guide` |
| G2 | Anchor registry: every control that fires a command registers its key; ranking as above | the ranking picks the visible button over a menu row, and search last | unit tests on the ranking; a snap with the chosen anchor ringed |
| G3 | Spotlight and bubble UI from the research, replacing `ui/tour.rs`; the five tour stops become the first lesson | the first lesson runs in a real window and recovers after a click elsewhere | `--script` run of the lesson with a wrong click; `tools/drive-window.ps1` |
| G4 | Media: `tools/record-guide.sh` records each step's move as frames; the bubble plays them | every step of every lesson has its frames | the recorder's file count matches the steps; a strip of a bubble playing |
| G5 | Help mark: one widget placed in or beside every feature, opening its explanation, its animation and "show me" (the lesson) | every panel and dialog has one | a static check lists every panel without a help mark; snaps |
| G6 | Lessons, one per feature: plan, preview and live, songs, Bible, announcements, calendar, people, offerings, design and colours, screens, phone, stream, listen, PowerPoint import, songs from the web | each lesson's scripted test passes | `cargo test -p artosia lessons` runs every lesson headless |
| G7 | What's new: release notes per version carry feature entries with media and a lesson link; shown once after an update and from Help | an update to a version with entries opens the panel once | a snap after a simulated update; the entries come from the release's notes file |
| G8 | Every string in ten languages from briefs | `tools/check-brief.py` passes | blind writers per `blind-translation` |

Order: G1, G2, G3, G4, G5, G6, G7, G8 (strings ride along with each task).

## Where it stands (2026-11-14)

| Task | State |
|---|---|
| G1 to G4, G7 | Built: engine, anchors, bubble, clips, what's new |
| G5 | Help marks on the seven tabs; help rows in Screens, Phone, Listen, Stream, songs from the internet and the PowerPoint import. Owed: a static check that lists a panel without one; the song editor and the Bible search |
| G6 | Thirteen lessons and the tour, each walked to its end by `tools/check-lessons.py` (14 pass). Owed: songs (the editor) and Bible |
| G8 | Every guide text in ten languages from its brief |
