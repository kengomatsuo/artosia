# PHONEEDIT1 plan (pseudocode, written before the code)

SOURCES  editing-mocks P1a to P1e and README D1 to D9 (owner-approved 2026-11-14); research phone-editing README 1 to 10.
         Pattern read: this repo's `RemoteState` + long-poll `version`, `hits` check in `parse_command`.
BASE     `edit.rs` in artosia-remote: the editable rows (card, person, event), their revision, the access level, the
         edit command and its refusals; the server only checks and relays, the operator applies.
BOUNDARY phone page -> server (`POST /api/command`, role + revision + limits checked in `parse_command`)
         -> `RemoteCommand::Edit` -> channel -> `App::remote_command` -> `phone_edit.rs` apply functions.
         The operator owns the truth; the server holds a copy (`RemoteState`) republished each tick.
SCREEN   phone: Edit pill on the live card (card slides only) and on each announcement row of Plan; list screens
         Birthdays and Schedule from the Plan's "Edit" entry; sheets over the live screen. Console: Phone dialog.
KIT      phone: page.html's own parts (`.sp`, `.field`, `.seg`, `.group`, `#lost` banner look) per P1a to P1e.
         console: `Switch`, `btn`, `section_label` of the Phone dialog.
RULES    revision = FNV-1a of the fields the phone edits, computed by the one function `edit::revision`; nothing stored.
         Level lives on the server's token and dies with the server (Phone off resets it).
WORDS    phone: `STR.en` keys + briefs in brief-phone.json; console: strings.rs + brief.json. English only.
DATA     RemoteState gains cards, people, events, live_card, locked (keys the console's editor holds unsaved).
         Phone reads them from the long-poll; refresh = every `set_state`; offline = lost banner, Done waits.
STATE    sheet draft: leaf state of the sheet (page); revision opened: the sheet; everything else derived from `state`.
ACTION   Done: sync   read fields, send {cmd:"edit", ...fields, rev}; sheet shows Saving
                 server  level check -> limits -> rev check (stale: 409 + current) -> console-lock check
                         -> optimistic apply to its copy (next stale check sees the new rev) -> relay
                 console  rev check again -> console-wins check -> apply (`apply_card_text`, `apply_person`,
                          `apply_event`) -> decks refresh -> projector shows it when the card is live
                 fails  409 stale: banner "Changed on the computer", Use theirs / Keep mine, draft kept
                        409 console: one-line notice, draft kept
         Cancel: nothing sent. Done twice: second has the new rev's fields equal to its own -> 204, applied once.
         Add / Delete: confirm sheet, then command (delete = person removed with Undo toast; event = date cancelled).
ARRIVE   page first frame is today's (no Edit controls until `level` says edit in the first state).
FLAGS    `level` per token: control (default) | edit. Edit pill, lists and sheets absent for control.
EDGES    empty list says so; long names wrap; keyboard up keeps the header (visualViewport); lost connection keeps the draft.
