# NOTICEEDIT1 plan (pseudocode, written before the code)

SOURCES  editing-mocks E1a to E1d, README D1 to D9, `build/spec.json` (owner-approved 2026-11-14).
         Pattern read: FreeShow `Editbox.svelte` (contenteditable on the slide, one history entry per committed edit)
         and `autosize.ts` (size search between a floor and a ceiling); this repo's `warta::largest` is the same search.
         Rune: `text_input(&edit, style)` (`crates/rune/src/el.rs`) is an editable text with its own caret, selection
         and undo; `.color`, `.caret` and `.absolute` place it over a custom drawing.
BASE     `warta::layout` (artosia-slides): the one description of a slide as boxes (field, rectangle, size, floor, lines).
         The renderer, the editor's overlay, the fit meter and the continuation cut all read it; nothing is measured twice.
BOUNDARY `ui/slide_edit.rs` (screen) -> `App` actions in `edit_slide.rs` -> `apply_card` (the one save) -> store.
         `phone_edit.rs::apply_card_text` and `save_notice` both end in `apply_card`; the revision is a hash of what is
         stored (`edit::revision`), so every write through it moves the revision the phone's conflict check reads.
SCREEN   Announcements tab. The card list folds to a 56 px rail while a card is open (D1); `>` opens the old list.
TREE     header (name, source chip, three buttons) / strip / stage (slide + selection layer + floating bar) /
         sheet (data part) or overflow card (E1d) / bottom bar (Delete, Seconds, Show now, Save).
KIT      `btn`, `Tile`-less chips from `choice_chips`, `SegmentedControl`, `section_label`; new parts: strip, floating bar,
         sheet; each drawn from the mock's numbers (spec.json `resolved`).
RULES    selection: `Option<Field>` plus `editing: bool`. A data field (rows, names, funds, account, verse) is never typed on.
         Fit: title and body search down in whole px from the card's size (or the style's ceiling) to the style's floor;
         a title keeps at most two lines; a body that fails at the floor offers the second slide.
WORDS    48 strings from the mock (spec `copy`, kind new) in `strings.rs` with briefs; English only.
DATA     card form -> `notice_draft` -> `card_slides` (renderer) -> `layout` (boxes, fit) -> overlay; refreshed every frame
         because the form is the source; a window that changes is read by the same paths as today.
STATE    selected, editing: leaf state of the editor (`Notices.sel`); history: `Notices.undo/redo` of whole card
         snapshots, one per committed edit; the caret is the text edit's own; everything else derived.
ACTION   click box: select (outline, handles, bar). Second click or Enter: `editing`, the text field takes focus.
         type: the form's field changes, the slide below redraws and refits in the same frame.
         Esc or click outside: edit ends, one history entry if the words changed. Ctrl Z / Ctrl Shift Z: card history.
         Fit: size set to the largest that fits. Continue: one write (new card after, first trimmed, loop +10 s), one undo.
         Save / Ctrl S / Show now: `apply_card`; refused while a body does not fit and a title is two lines too long (D3).
         Leave tab with a body that does not fit: continue, toast with Undo, save (D4).
ARRIVE   the editor opens on the card in one commit; the overlay needs only the layout, no async.
FLAGS    a data part: no caret, only its sheet. Picture slide: no boxes. A card that reads nothing today: the empty note.
TIMING   outline appears in the commit of the press; fit follows each key in the same commit (a layout is a few ms, measured).
SIZE     strip 38, stage fills, bottom bar 48; the sheet takes 192 of the stage and the slide scales to what is left.
EDGES    empty window, 30 names, title of 3 lines at the floor (Does not fit, Undo only), body at the floor, Indonesian.
GUIDE    n/a: desktop program, keyboard and pointer; every control has a key (Tab, Enter, Esc, Ctrl Z, Ctrl Shift Z, Ctrl S).
