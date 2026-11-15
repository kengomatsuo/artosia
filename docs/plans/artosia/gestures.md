# Gestures across the window (GESTURES1)

The owner asked on 2026-11-15 for drag from the plan to Preview or Live, a double-click that
goes straight to Live (and whether that is risky for people who double-click everything), file
dropping, sorting, movement and resizable sections with limits.

Research: `~/Shared/inspo/artosia-gestures/notes.md` (eight worship apps, Microsoft, Apple,
NN/g), proposal picture `~/Shared/inspo/artosia-gestures/proposal.png`.

## What others do

| Question | Finding | Source |
|---|---|---|
| Double-click a plan item | Goes live in EasiSlides, FreeShow, OpenLP's service list, EasyWorship's preview, MediaShout's presenter view; not in Quelea, Proclaim or ProPresenter | notes.md, Q1 |
| Guard against accidents | A separate Live or On Air switch (EasyWorship, Proclaim), a lock (FreeShow Ctrl+L), Black and Clear keys | notes.md, Q3 |
| Drag to Preview or Live | Quelea's preview and live panels take a dropped song; FreeShow drops a slide on an output; EasyWorship drops media on the preview | notes.md, Q2 |
| Resizing | FreeShow: 290 px default, 2.2 times at most, snaps near the default, a click never closes a pane, sizes saved; OpenLP and Quelea save splitters | notes.md, Q6 |
| Double-click risk | NN/g: double-click "causes novice users great difficulties"; Microsoft gives the primary action to one click; Apple asks for an alternative to every drag | notes.md, risk section |

## What Artosia already has

| Gesture | Where |
|---|---|
| Drag to reorder plan rows | `ui/plan.rs` `draggable` |
| One click opens a plan item in Preview, never on the projector | `ui/plan.rs` (owner rule: Preview is the safe haven) |
| Double-click a calendar day for a new event, drag events | `ui/calendar.rs` |
| A dropped file becomes the background, or a list of people on Members | `ui/mod.rs` `on_drop_files`, first file only |

## Proposal (waiting on the owner's yes)

| # | Gesture | Does | Guard |
|---|---|---|---|
| 1 | Double-click a plan item | Show: fills Live, as EasiSlides does | The projector shows it only while the Live switch is on; a setting turns it to Preview only |
| 2 | Drag a plan item onto Preview | Opens it there, as one click | none needed |
| 3 | Drag a plan item onto Live | Show, with the pane ringed and named while it hovers | the Live switch, as above |
| 4 | Drag the plan column's edge | 260 to 520 points, double-click resets to 320, saved per PC | never collapses, so a volunteer cannot lose the plan |
| 5 | Drag the Preview and Live divider | 35 to 65 percent, double-click resets to half | buttons below keep their measured minimum |
| 6 | Drop files | By kind and place: pictures and videos onto the plan become items at the drop place, onto Design the background; PowerPoint opens its gallery; song files import; Bible modules install; lists onto Members import; every file of a drop, a refused one said why | Rune's `on_drop_rejected_hover` draws "cannot drop here" |
| 7 | Drag to sort | Song parts in the editor, announcement slides, plans in the list, the order strip | Undo in the toast |
| 8 | Movement | Rows step aside while one is carried (as the plan does now), a drop target lights, panes spring to their limit | reduced motion settles at once |

Rune supports each: `on_double_press` and `Drag::clicks`, `on_drag`, `on_drop_files` with
`on_drop_hover`, `on_drop_move` and `on_drop_rejected`, the kit's `Reorder` and `PaneState`
with `range(min, max)` and double-click reset (`rune-kit/src/shell/pane.rs`, `reorder.rs` at
the pinned rev).
