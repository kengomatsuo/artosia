# The operator window

How a volunteer runs a service in Artosia, and the rules the window is built on. Built 2026-11-10 from the research in `C:\Users\kenne\Shared\inspo\artosia-presenter-ui\` (Lerret's three columns, OpenLP's preview-then-live, Holyrics' book grid, EasiSlides' `[1]` and `[chorus]` markers).

## The order of work

The window reads left to right in the order things happen (`ui/shell.rs`).

| Panel | What it holds | The one action |
|---|---|---|
| Plan (left) | The songs, passages and announcements of one service, in order, with Add under them | Click an item to put it in Preview; drag it to move it |
| Preview (middle) | What comes next, as a numbered list of its slides over a picture of the slide with the ring | Show, or Enter: the ringed slide moves into Live |
| Live (right) | The same list for what the projector holds, over its picture, with Prev, Next and the four states | Next, Down or Space |

The owner picked this arrangement on 2026-11-10 from four drawn after OpenLP, ProPresenter, FreeShow and Lerret. It is the shape OpenLP and EasiSlides have, which the volunteers already know. A list row shows the words at reading size where a thumbnail could not, and its number is the digit key that jumps there. A deck with no sections, such as a passage, numbers every slide.

Panels differ by tone (`theme::surf`): the plan sits furthest back, the two lists are lifted, and each panel has a title strip. The owner's complaint was that the dark window "doesn't show at all the section hierarchy". A test in `theme.rs` holds the text and the grey text at 4.5 to 1 on every surface in both themes; the light kit's stock grey failed it at 3.7 and is replaced there.

A red ring round the Live pane, a red row with a round sign and a red dot on the plan item mark what the room sees. Red means nothing else anywhere in the window. Yellow replaces them while Black, No text or the logo keeps the slide off the projector, and there is no ring while the Live switch is off. The ring is its own element pinned over the pane: Rune draws a border under the element's children, so the pane's own border showed only down its two sides, which read as edge bars.

### Preview is safe, and Show is one thing and Live another (2026-11-11, owner-directed)

Nothing reaches the projector from a click in Preview, on a plan row or on a search result. A click in Preview moves the ring (`state.rs` `select_at`). Three things send to the Live list: Show (or Enter), Next at the end of the live item, and Show now in the search. Inside the Live list a click or an arrow moves the projector, because that list is the projector.

Show moves Preview into the Live list. Whether the room sees the Live list is the Live switch (the Live tile, F6, `toggle_live`): on, the tile is red and the pane is ringed; off, the projector's window is gone, the Live list keeps its place in grey, and Show still fills it. The owner: "Show should not be same as live, it should only move to live view but not force live on." EasyWorship has the same switch ("To turn off the live output, click the LIVE button"). The switch is on at every start and is never saved, so a service that ended with it off does not begin the next one dark; the saved switch is the one in the Screens dialog, and pressing Live turns that one on too.

Going back: the Live title has a clock button once something has left the projector. It lists what was shown before, newest first, each with the slide it was on (`App::history`). Choosing one brings it into Preview with the ring where it was left, and Show returns to it in one step, with none of the songs between passing over the room's screen.

### The strips under the lists

Each list stands over a strip: the picture of its slide, and the tiles that act on it (`ui/shell.rs` `Foot`, `ui/widgets.rs` `Tile`, `ui/live.rs`). The picture takes the width the pane can give, up to about a third of the window's height, and wears its slide's section as a chip. The tiles are equal within a row and sized from the strip, never from their words, so every language draws the same strip. Under Preview: Show, and two tiles that bring the plan item before or after into Preview, each carrying the title of the item it brings. Under Live: Prev and Next, then No text, Black, Logo and Live, each lit while on. Every tile prints its key, drawn as the keyboard prints it. The rules and their sources are in [ux-principles.md](ux-principles.md).

### Preview is what comes next

| When | Preview holds |
|---|---|
| A plan item goes live | The plan item after it, ring on its first slide (`state.rs`, `queue_next`) |
| Nothing follows it in the plan | The live item itself, its live slide marked |
| A song from the search goes live | The item after the last plan item that was live, so the plan is not lost (`App::resume`) |
| A person opened something there | That, until it is shown. Artosia replaces only what it queued itself (`OpenDeck::queued`) |

Next at the last slide of the live item shows what waits in Preview, at its ring. That is how a medley is armed: with the next song in Preview, Ctrl and Down put the ring on its chorus, and the Next that ends this song starts that one there. Enter does the same at once. The owner asked for both on 2026-11-10: "how do we switch between the live and the preview selection so next song can start at medley? How do we pre-arm that?" and "is it supposed to be automatic?".

The ring also works inside the live song. Esc puts the live song in Preview, Ctrl and an arrow move the ring to another part, and it stays there while the live slide moves (`OpenDeck::armed`) until Enter, or the Next that ends the song, shows it.

### Where a song begins (medley start points)

The owner, 2026-11-11: "They also want to set the medley checkpoints so the medley auto starts from the reff for example." A song of the plan can carry the section it begins at (`PlanEntry::Song::start`, a section's tag, stored in the plan's JSON and absent from a plan that sets none). It is a tag and not a slide number, so it follows the chorus when a verse is written in front of it, and falls back to the first slide when the section is cut (`artosia_core::plan::start_slide`, tests in `plan.rs`).

Setting it, two ways that need no telling:

- With the song in Preview, move the ring to the slide and press the play icon among Preview's tools (Ctrl M). It is lit while the ring's slide is the start; pressing it again clears it. With nothing marked and the ring on the first section it is dimmed, since the song begins there already. Its tooltip says what it does and its key is printed on it; the state is its look, not words.
- The plan row's menu lists the song's sections under "Starts at", the current one checked, and "The beginning".

Showing it: the plan row carries the section's chip (`C`, `V2`) and the Preview list marks the start slide with a play icon under its chip, so the ring's place on a freshly opened song is explained.

Using it, everywhere a plan item comes into Preview at its start the ring begins on the start slide (`state.rs`: `open_plan_item`, `queue_next`, `walk_plan` through it, `start_service`, and `next` crossing into the next item; `starts.rs`, `show_start_of` for Show now from the search, the phone and the service start). So Enter, Show and the Next that ends the live song all start the song there. A ring a person moved by hand is never taken back.

What the search learned ("Usually started at Chorus, 4 of 4 times", `learning.rs`) is offered on the row menu, under that section, as one press when the item has no start. It is never set by itself.

### Shift picks, and letting go shows

Holding Shift, the arrows move a second ring through the live song in the Live list, and the projector does not change. Letting Shift go shows the slide it is on; Esc before that shows nothing (`state.rs`, `pick_slide`, `release_pick`). A far slide is reached without the ones between passing over the room's screen. The owner asked for it twice: "if shift or control hold then those arrows, it only moves the selector not change the actual live text", then on 2026-11-11 "when release the shift/ctrl it activates". Shift got the release and Ctrl kept the ring that waits, because a medley needs a ring that is still there after the fingers leave the keyboard.

The release is heard as a key (`Key::Modifier`, switched on with `App::modifier_keys()`). Rune's `App::on_modifiers` looked like the callback for it and is never called in this app: `src/window.rs` at the pinned rev wires `modifier_keys` for a single-window app and leaves `on_mods` unset, which only the multi-window, web and headless paths set. Every headless script passed with it. Real keys found it (`tools/drive-window.ps1`).

### What left the window

The two message fields and the countdown sat in a strip under both panes until 2026-11-10. The owner had them taken out for the room they cost: "That space can be used to enlargen the preview and live view". They are started from the search (`countdown 5`, `message the car park is full`, `stage one more chorus`) or the phone, and while one shows, a yellow pill in the Live title strip says so; pressing the pill takes it off. The stage message has no field on the phone page.

While a song or the announcements are being edited, the editor takes the Preview pane, and Live stays as a thin column. Next is one click away while a typo is being fixed.

### Dragging a plan row

A row follows the pointer after 4 points of travel and the rows it passes step aside (`ui/plan.rs`, `draggable`). It is written after the kit's `Reorder`, whose methods are private to the kit and which takes every row to be one height; a divider here is shorter than an item, so the landing place counts the rows whose middle was passed (`drop_place`, with a test). Rune sends the row's press after the drag's release, so the drop sets `plan_dropped` and the press that follows opens nothing. Escape puts the row back.

## Several plans

A church holds more than one service a week (Sunday morning and evening, youth on Saturday), and each needs its own list, kept and named (owner, 2026-11-11: "Where's the multiple-plan support? For different times of the worship schedule idk what it's called?"). The word stays **plan**: the window and the Indonesian interface (`Acara`) already used it, and it says what the list is without saying "service", which in a church also means the hour itself.

**What others call it, from their own manuals.** OpenLP keeps a *service* as a file you open and save, with New, Open and Save As, so a church loads one at a time ([manual.openlp.org/service.html](https://manual.openlp.org/service.html)). FreeShow keeps *projects* in a list in its Show tab, made with a New project button, opened by a double press, and can turn one into a template for new ones ([freeshow.app/docs/projects](https://freeshow.app/docs/projects)). EasyWorship keeps a *schedule* as a file saved and reopened with File, Save As and Reopen, and has no switcher beside the list ([support.easyworship.com](https://support.easyworship.com/support/solutions/6000085139)). ProPresenter keeps *playlists* in a sidebar list, renamed from a right-click menu and duplicated to roll last week's list into this week's ([learn.renewedvision.com](https://learn.renewedvision.com/propresenter6/introduction-to-propresenter/playlists)). Two of the four are files, so a volunteer must find them on disk; the other two are a list inside the program. Artosia takes the list, because the church's PC is shared by volunteers who should never meet a file dialog, and ProPresenter's Duplicate because a new Sunday is nearly always last Sunday with other songs.

**What a plan is.** A row of the `plans` table: a name, an optional date (`YYYY-MM-DD`), the time it last became the open one, and its items (`artosia-core/src/plan.rs`). The table always allowed many rows and the window only ever opened one; the old `New plan` command left each earlier plan behind as a hidden row named "Today". Those rows show in the list now, so nothing is lost. Plans saved before the date existed load with `#[serde(default)]` fields and are tested to do so (`plan.rs`, `store.rs`).

**A plan is a weekly service (2026-11-11, owner: "why cant like every sunday hh:mm you love textfields too much").** A plan has a day of the week and a starting time (`Plan::day`, `Plan::time` in `artosia-core/src/plan.rs`; a plan saved before this loads with neither). Both are picked from chips in one box: seven days, the hours 00 to 23 and the minutes in steps of five, with the result said back in words beside Save (`ui/plans.rs` `view`). Nothing is typed. A name is extra, in a field that may stay empty, and a plan without one reads as its day and time, "Sunday 07:00" (`plans.rs` `plan_words`; Indonesian writes `07.00`). A new plan is offered the open plan's day two hours later, or today at the next full hour. Nothing exists until Save, so Cancel leaves nothing behind. No hour is offered: a guessed one was taken for a setting, so Save waits until an hour is pressed. With keys the time is typed as it is said while a chip has the focus (`0730`, `1830`, `7`; `plan_box_digit`), the arrows choose as they move, and Enter saves. Microsoft's guidance on date and time controls gives a picker for "a single time value" and a calendar where "the day of the week" matters, and NN/g keeps typing only as an addition to picking; the free-text date this replaces had neither.

**Every plan is in view under the plan, and the title drops the same list down (2026-11-11, owner: "how to access plan picker? In easislide the plan list is just below the plan", then "make it show title only id rather have that than a clipping dropdown", then "Nvm keep the dropdown but don't let it clip right side like it does today").** The foot of the plan column lists every plan the church keeps, grouped by day of the week with today's group first, each day in the order of its times, and last the plans that have no day yet (`plans.rs` `plan_groups`, `ui/plans.rs` `strip`). The open plan is filled and one press opens another. The list is at most about five rows high and scrolls past that, with the fade at the edge that hides more. Each row carries its own menu: Day and time, Duplicate plan, Delete plan, so a copy is made from the plan it copies; a tester had looked for exactly that on the row. The plus beside the heading opens the box for a new plan. The column's top is the open plan's name drawn as the box of a drop-down list, with its edge and its arrow (`ui/plans.rs` `title_bar`). Its list holds every plan under its day's heading, a tick on the open one, then Start a new plan, Day and time, Duplicate plan and Delete plan for the open plan. It was taken out for a day and came back whole.

Why it was cut on the right, and what holds it now. The plan column is 256 points wide, and the drop-down measured itself against a second constant of 272: its box was 256 wide from 8 points in, so its last 8 points lay outside the column and were cut, and its list ended exactly on the column's edge with no border to see. There is now one number, `theme::size::PLAN`, and the box is that less 8 points on each side. The list is as wide as the box and grows for a long name, measured with `rune::text::measure`, up to 440 points; it is then drawn over Preview with its edge and shadow whole, because the kit mounts it at the window's root (Rune's `El::anchored`: "Attach it at the root ... so it draws above later siblings and escapes clipping ancestors"). The kit's menu takes the width it is given and no more, so a row's words end in an ellipsis unless that width is measured for them. It is at most three fifths of the window high and scrolls, fading where more waits, and it opens with the open plan in view (`drop_max`, `switcher_entry`). Checked by `tools/check-overlays.py`, which reads the rectangles `--tree` prints for the box and the list in the scenes `plans-drop`, `plans-drop-long` and `plans-drop-many` at two sizes in both languages (on 0.1.7 the same measure puts the box's right end at 264 in a column that ends at 256), by a script that presses the box and turns the wheel, by a real press in a window at 125%, by the real wheel in a window of the released 0.1.10 at 150% on the second screen, and by real keys on the primary screen: Down moves through the plans, Enter opens the one lit, Esc closes the list. On a real start the plan of today's service that is about to begin or began less than an hour and a half ago is the open one (`plan_for_now`). EasiSlides shows its worship list's box at the list (`easislides/01-easislides-main-window.jpg` in the research folder); ProPresenter and FreeShow keep their lists of playlists and projects in view beside the items, as the paragraph above records.

**A divider of the plan reads as a heading (2026-11-11, owner: "I can't differentiate well enough the categories like firman and pujian, with the contents of them").** A divider was a row like any other, with the same grip and the same dots and grey words. It is now the name of the part with a rule drawn on to the column's edge and room above it; its grip and its menu come under the pointer only; and every item after it, until the next divider, is set in by ten points (`ui/plan.rs` `heading_row`, `INSET`).

**What a switch does to the projector: nothing.** Item numbers belong to one plan and two plans both count from 1, so a switch cannot keep `live.item` as it is. `plans.rs` `enter_plan` therefore:

| State | After the switch |
|---|---|
| The live deck and its slide | Unchanged. The output is never sent anything |
| `live.item` | The new plan's item for the same song or passage if it has one, else none. With one, Next goes on after it in the new plan |
| `resume` | Cleared |
| Preview, queued by Artosia | Dropped and queued again from the new plan |
| Preview, opened by a person | Kept, its item looked up in the new plan the same way |
| Next at the end of the live item, when the new plan has none of it | Shows the new plan's first showable item (`Switcher::fresh`, `plan_start`), so a switch mid-service leads straight into the next service's list |

Tests in `plans.rs` hold each row of the table.

### A plan's own look (2026-11-11, owner's example: morning service, Sunday school, afternoon service)

One week holds a morning service that puts the words over the camera, a Sunday school with other songs and no camera, and an afternoon service that repeats the morning's songs. One look for everything cannot do that, so a plan may carry its own (`Plan.look`, `plans.rs` `look`, `plan_look`, `use_own_look`, `use_usual_look`).

- **Storage.** `Plan.look` is an optional JSON value, because `artosia-core` cannot name `artosia_slides::look::Look`; the operator window reads and writes it as a `Look`. A plan saved before this has none and uses the usual look (`serde(default)`). Nothing is written for a plan without one.
- **Which look is in use.** `App::look()` is the open plan's own look, else `settings.look`. `output_state`, `preview_state`, the pane pictures, the moving-background picture, the search's look previews and `deck_of_song` (lines per slide) all read it, so the projector follows the open plan. Nothing reads `settings.look` directly except `look()` and the first load.
- **The Look dialog** starts its style controls with two buttons of one size: the usual look (screen icon) and the open plan's own (the plan's name and the same mark the plan list uses); the chosen one is filled. Everything below edits whichever is chosen (`set_look` writes to the plan when it has its own, to the settings otherwise). The language, Bible and logo settings further down are not looks and stay shared. Choosing the plan's own copies the usual look as a start. Choosing the usual look drops the plan's own and offers Undo in a toast (`Undo::PlanLook`, which only restores while that plan is still open).
- **The plan list** marks a plan that has its own look with the same small icon before its name; the other rows keep the room so names line up. Duplicate copies the look (`Plan::copy_named`).
- **The search's saved looks** (`Hit::Look`) apply through `set_look`, so they change whichever look the dialog would edit.
- **A switch while something is live.** The words stay on the projector (see the table above). The whole look follows the new plan at once, background and text style together, because the output draws one `Look` for the screen. The slide's split into lines is part of the deck already on the projector and does not change; the deck in Preview and the next one queued are rebuilt with the new plan's lines per slide.

Tests in `plans.rs` and `plan.rs` hold: an old plan loads, a plan's look wins and falls back when cleared (with Undo), a duplicate keeps it. Scenes `look-plan` and `plans-own-look`.

**The learning log.** `palette_picks`, `service_songs` and `service_passages` already carry `plan_id`; the log writes the open plan's id and never reads it for ranking. Two services on one day broke one query: "what came after this song" partitioned by day, so the last song of the 7:00 plan counted as followed by the first of the 10:00 plan. It now partitions by day and plan (`learn.rs`, `followers_of`, with a test). The weekday group counts days and is unchanged. A deleted plan leaves its id in old rows; nothing reads it, and Undo restores the row under the same id.

## Search is the library

One box finds songs, Bible passages, announcements and commands. Typing a letter anywhere in the window opens it; so does Ctrl K. It is built after the owner's house-kit palette (`core/palette` in `kengomatsuo/house-kit`): every part of the app contributes documents, the order is a cascade of single rules where a rule only speaks when every rule above it tied, and a document answers to its words in English and Indonesian (`crates/artosia-core/src/palette.rs`, `crates/artosia/src/palette.rs`).

What a typed search shows, top to bottom:

| Order | What | Example |
|---|---|---|
| 1 | A command typed with what it takes | `countdown 5`, `hitung mundur 10`, `message the car park is full` |
| 2 | A Bible reference | `joh 3 16-18`, `1 pet 3:3` |
| 3 | Songs in today's plan that hold every typed word | marked "Plan" |
| 4 | Commands, announcements and looks found by the start of their name, four at most | `black`, `start`, `youth` |
| 5 | Every other song: the title before the lyrics, whole words before word starts, a word inside a longer word last | `berkat` finds `diberkati` after `Berkat` |
| 6 | What another word found | `hitam` finds Black, `mulai ibadah` finds Start the service |
| 7 | Verses that hold every typed word | |

The song order is `Facts` in `crates/artosia-core/src/search.rs`. It differs from the kit in one line: a title outranks a whole-word match, because a word common in lyrics would otherwise bury the song named after it.

Nothing typed lists every song. The Bible tab shows the 66 books as buttons, then the chapters, then the verses: nobody has to type.

| Key | Does |
|---|---|
| Enter | Open the result in the Preview pane, or run the command |
| Ctrl Enter | Open it and show it now |
| Shift Enter | Add it to the plan and stay in the search |
| Tab | On a song: search inside it (its sections become the results, so another song's chorus is two keys away). On a book: step into it. Otherwise: switch between All, Songs and Bible |
| Backspace in an empty box | Let go of the song the search is inside |
| Esc | Close the search at once; from inside a song, first back out to the whole search. It used to empty the box first, and a tester who opened the search by a slip of a letter key pressed it once and was still inside. The panel also has a close button for the pointer |

Commands that change the projector (Black, Clear text, Next, Start the service, Loop the announcements) run on Enter, like every command. They reach the top of the list only when their own name is typed from its start, so a song search does not put one under the Enter key.

## Keys in the main window

Nothing in the main window holds keyboard focus, so every key below works at any time. `?` shows this list in the app.

The arrows are the ones EasiSlides taught the volunteers (owner, 2026-11-10: "Easislide users are used to up/down as prev/next, left as start from beginning, right as endmost"). Shift and Ctrl turn them into picking keys, which was the owner's own idea in the same message.

| Key | Does |
|---|---|
| Down, Space, Page Down | Next slide. At the last slide of the item, what waits in Preview, at its ring; the space bar is then drawn on that ringed row, beside Enter. With nothing live, Space starts the plan |
| Up, Page Up | The slide before; at the first slide, the last slide of the plan item before |
| Left, Home | First slide of the live item |
| Right, End | Last slide of the live item |
| Shift with an arrow | The same four moves for a ring in the live song. The projector does not change until Shift is let go, which shows the ringed slide. Esc first shows nothing |
| Ctrl with an arrow | The same four moves for the ring in Preview, which stays where it is put. Past either end it goes on into the plan item before or after. With nothing live the arrows do this without Ctrl |
| Enter | Show the slide with the ring; with the ring on the live slide, Next |
| 1 to 9, 0 | Jump to that section of the live song in sung order (0 is the tenth). With nothing live, the ring goes there and nothing starts. Each list row shows its number as a key |
| C, V, B, P, I, E | The next section of that kind in the live song, going round to the first: chorus, verse, bridge, pre-chorus, intro, ending (`jump_to_marker`). Only letters the song has are taken; any other letter starts a search. Settings switches this off, as WCAG 2.1.4 asks of single-letter keys |
| Ctrl with 1 to 9 | The ring in Preview goes to that section |
| / | The search, when a letter would jump |
| Esc | Put the live item in Preview |
| Backspace | Put what was on the projector before this into Preview, with the ring where it was left; pressed again, the one before that, then round to the newest (`open_earlier`). The room sees nothing until Enter. The Earlier button of the Live title prints the key |
| Tab, Shift Tab | Put the next or the previous plan item in Preview. The row Tab opens carries the key where its menu button is; the button takes its place under the pointer |
| Shift Enter | Add what Preview holds to the plan, as in the search. It never shows anything |
| Ctrl Tab, Ctrl Shift Tab | The plan after or before the open one in the list of plans, going round, as the same keys move between tabs in Windows (Microsoft, Keyboard shortcuts in Windows: "Ctrl+Tab: Move to the next tab"). The projector is not touched. Ctrl Tab is drawn on the row of the plan it opens: beside the heading of the list it ran past the column's edge in Indonesian (2026-11-11) |
| Alt Down, Alt Up | Move the Preview item in the plan |
| Ctrl M | Start the Preview song at the ring's section, or clear it |
| Delete | Take the Preview item out of the plan (Undo in the toast) |
| F1, F2, F3 | No text, Black, Logo |
| F6 | The Live switch: the projector on or off |
| Ctrl comma | Settings |
| F4 | Put the first suggestion in Preview: what the listener heard, else what usually follows the live song. It never goes to the projector by itself, because a misheard word must not reach the room |
| F5 | Put the announcements in Preview |
| Ctrl E, Ctrl N, Ctrl S | Edit the Preview song, new song, save |
| Ctrl V with no field focused | Make a new song from the lyrics on the clipboard |
| Ctrl Z | Undo the last removal |
| ? or Ctrl / | The key sheet |

In the search: Enter puts the result in Preview, Ctrl Enter shows it at once, and Shift Enter adds it to the plan right after the live item, so it is the next one (`actions.rs`, `queue_entry`). With nothing from the plan live it goes to the end.

A song is edited without the mouse: Ctrl E opens the Preview song, Tab moves through the fields, Ctrl 1 to 5 start a verse, chorus, pre-chorus, bridge or ending at the caret, Ctrl S saves and Esc leaves.

A digit starts a search when the live or open item has fewer than two sections, so a hymn number can still be typed straight in. While a song is live, Ctrl K comes first.

Page Down and Page Up are bound for presenter clickers. Which keys a given clicker sends has not been checked against one.

Played through `crates/artosia/scripts/arrows-and-ring.json` on 2026-11-10: Space starting the plan, Down, a ring moved in Preview, Right, the Next that goes on at the ring, Left, Tab, a dragged plan row, F5 and F4.

Pressed in a real window on 2026-11-11 with `tools/drive-window.ps1`, which sends real key and mouse input to one program's window and saves a picture after each step: Space, Down, Shift held with two Downs and then let go, Esc while Shift was held, Ctrl Down, Tab, Right, the Next that goes on, a plan row dragged, F5. The Shift release was the one step that failed the first time.

`tools/answer-dialog.ps1` answers a Windows save or folder window that the program opened (2026-11-11): it sets the focused field's text, reads it back and presses Enter, twice for a folder window, and stops when the window in front is another program's. Keys typed into such a window lost a character of a long path, and the colon after the drive letter.

`tools/drive-window.ps1` `post-click` now puts the real pointer on the spot for the click and back again: Rune presses what is under the real pointer whatever the posted message says, so a posted click used to land wherever the mouse happened to rest (found 2026-11-11, when clicks on the plan list pressed a tile under the resting pointer).

Pressed again in a real practice window on 2026-11-11 after the tile rework: Enter moved the tour one stop and Esc ended it; Space, Tab and Enter put a song up; C jumped to its chorus; F6 switched the Live tile off (no ring, the row grey, the picture dimmed, the plan dot grey) and on again; Ctrl held ringed Preview and wrote its keys in the bar. Shift let go was pressed on a last slide, where there is nowhere to pick, so that run says nothing new about it.

Dialogs and the two editors are different: Tab moves through their buttons and fields, and Enter or Space presses the focused button (`Btn::focusable`).

## The first run

A new church sees one question at a time, each already answered from what Windows knows (`onboard.rs` is the state machine, `ui/onboard.rs` the window; board card ONBOARD1, 2026-11-12). The look is the one the owner picked from four mocks of other programs' first runs, the one that copies Windows setup: a tinted panel at the left with the page's picture and eight dots, the question in large type at the right, the answer already ringed, and one filled button under it. Eight pages:

1. **Language.** "Is this the right language?" Every listed language in its own name, the one Windows asks for first (`artosia_locale::pick_first`) picked, English when none fits. Picking a row changes the window at once, so the page reads in it.
2. **Country.** "Is this the right country or region?" A filter and every country in the window's language, the one Windows is set to picked (`artosia_locale::region`), Indonesia when it does not say. Typing filters from anywhere on the page. Yes keeps two capital letters in `Settings::country`.
3. **Bible.** The existing eBible and Beblia catalogue, with the picked country's languages first and the first whole Bible of its first language picked. A filter above the list takes a language, a name or a short code such as TB, and picks the first Bible it shows ([bibles.md](bibles.md)). The picked Bible's own copyright line shows before anything downloads (`docs/bibles.md`). Next starts the download in the background and goes on; Skip for now goes on without one; with no internet the page says why in the program's own words (`why.rs`) and offers Try again and Skip for now.
4. **Songs.** EasiSlides on this computer (only when `import::easislides_folder` finds it, with the count of its songs, and picked then), a folder (EasiSlides data or OpenLyrics files; the row is picked once a folder is chosen), a zip file (the same files packed, picked in a file window that shows zips; added 2026-11-13 after the owner downloaded the church's songs as one zip and the folder window showed an empty Downloads), or nowhere yet (picked when nothing is found). A zip is unpacked into a scratch folder and read as a folder (`artosia_core::import::as_folder`), and a picked folder is searched three folders deep, because Windows' Extract All puts a zip's folder inside one of the same name. Next reads the files on a thread and puts the songs into the library 40 a frame.
5. **Sing.** "Which languages does your church sing in?" A filter and a list of languages with ticks, in the window's language: the window's own language is ticked, the picked country's languages come first (`artosia_core::languages::first_run_rows`), and the rest follow by name. The filter holds the keys, so the space bar arrives as a space typed into it: with nothing typed it ticks the language the arrows rest on and the space is dropped, and with something typed it is a letter of a name such as Jamaican Creole. Enter with something typed ticks that language, never unticks it, and empties the filter; Enter with nothing typed is Next (`Onboard::sing_query_changed`, `enter_takes_typed`, after react-select's keys, read in its `Select.tsx` on 2026-11-13). Before 0.1.30 no key ticked a language at all, because the space only ticked when nothing had focus and the page always opens with the filter focused. Next keeps the ticked languages, in the order ticked, as `Settings::song_languages`; one ticked means songs show one language.
6. **Screens.** Artosia shows the Screens dialog's own test picture, the number on every screen (`ToOutput::Identify`), on the screen it would use as the projector, and three plain rows say which screen this window is on, which shows the picture and its size. Yes, it works keeps that screen as the projector; No, try another screen moves the picture to the next one that is not this window's. With one screen the page says no projector is connected yet and Next goes on.
7. **Getting ready.** One row per job: language and country (done), the Bible (a percent, done, or failed with its reason and the line that Artosia goes on without it and where to add one later), the songs (n of m, then the import's own summary). Next is always pressable; nothing here blocks a person.
8. **Done.** Three cards: take the tour (the five-stop tour of `ui/tour.rs`), open the plan, new song. Each closes the first run.

**When it opens.** At the start of a real window on a library that had no settings file before this start (`onboard::opens_at_start`, `App::first_run`). A church that already uses Artosia has a settings file and never sees it; a headless run, `--practice` and a test never open it by themselves, and the scenes `onboard-language`, `onboard-country`, `onboard-bible`, `onboard-bible-offline`, `onboard-songs`, `onboard-sing`, `onboard-screens`, `onboard-one-screen`, `onboard-ready`, `onboard-ready-failed` and `onboard-done` draw it with made-up answers. **How to open it again.** Settings, About, "Set up Artosia again", and the same command in the search ("setup", "first run", "welcome"); it then starts on the church's own language and country. Closing it by any way marks the tour as offered (`Settings::toured`), so the tour never starts by itself after it.

**Why the country is asked.** Only to put the Bibles in the languages spoken there ahead of the others (`ebible::ranking`: Indonesia keeps its hand-ordered list, every other country takes Unicode CLDR's languages for it, most spoken first, from `crates/artosia-core/assets/countries.json`, which `tools/make-countries.py` writes from CLDR 49.0.0; the licence notice is [third-party.md](third-party.md)). It changes nothing else, and a library that was never asked keeps Indonesia's order. A day printed day then month read wrong in Chinese, Japanese and Korean, so a date is one sentence (`day_month`).

**It owns the keyboard while open**, as the tour does (`App::onboard_key`, called after the tour's in `on_key`): Enter presses the primary button, Escape goes Back, the arrows move in a list, typing filters the countries, and Tab moves between the list and the buttons, which Rune handles; every other key is taken, so Space never reaches the projector. The card is a Rune modal. A state is a fill, never an edge bar; the list of a page is one Tab stop.

**Every page's list is one element** (`onboard-list`), so its scroll position outlives a change of page. A page that reveals no row asks for the top of the list (`TOP` in `ui/onboard.rs`, `Tx::scroll_to_y`); before 0.1.18 the Bible page opened where the country page had been scrolled to, which only a real window showed. A revealed row stops clear of the list's fade (`scroll_margin`).

**A page lands in one frame (2026-11-13, owner-reported: lag on the song-languages page and "so many frame-related bugs and glitches").** Four faults, each found by rendering the pages frame by frame (`--script`, `--strip`) in a release build:

| Fault | Cause | Now |
|---|---|---|
| The song-languages page took 2.9 seconds a frame and the country page 0.2 | The page asked for its whole list (512 languages, 1.8 ms a time) for every row of every frame | The list is worked out once for what was asked and kept (`Kept` in `onboard.rs`): 22 ms and 20 ms |
| A page change drew the old list and the new one over each other for about a third of a second, with two Yes buttons | Every page's rows sit under the same keys, so the new ones faded in while the old ones faded out | The page's column is `instant_changes()`, and a press or key hands the window its focus and scroll in its own commit with `Tx::instant()` (`App::onboard_land`), so the page is drawn once, already at its row |
| Typing a filter that matches nothing on the country or song-languages page, or in the song-languages dialog, stopped the program | The list and the "Nothing matches" line had no keys, so the line took the list's path and Rune asked its text for a row key (`runtime/commit/measure.rs`) | Both are keyed, and a test holds every virtual list in `src/ui` to a key (`every_virtual_list_has_its_own_key`) |
| A title left one word alone on its last line ("... sing / in?") | Plain wrapping | The last two words are joined by a no-break space (`text::last_words_together`) |

Checked in a real practice window on 2026-11-13 with `tools/drive-window.ps1`: the country page was whole 120 ms after Enter with Indonesia in view, Down then Space ticked a language, "zzzz" showed "Nothing matches" and the window stayed, "jav" then Enter ticked Javanese and emptied the filter, and Escape went back at once.

**Checked** on 2026-11-12 by the state machine's tests (defaults, each page's buttons, Skip setup at each page, one screen and two, a failed catalogue and a failed download, EasiSlides found and not found, an existing settings file), by renders of every scene light and dark, English and Indonesian, and by a scripted Enter, Enter, Escape that went back once. It was not opened in a real window: the test picture on a real second screen, the folder picker, a real download and a real EasiSlides import were not tried.

## The tour

A first start on a library, and every practice run, opens a tour of this window (`ui/tour.rs`, 2026-11-11, owner-asked: "Not even a tutorial onboarding phase at all? Especially one that can be re-triggered easily?"). It has five stops over the real window and its plan, one part ringed in the accent colour at each, with a card beside it holding the stop's words, "2 of 5", and three buttons of one fixed width, each with an icon: Skip, Back, and Next (the filled one, a tick and Done at the end). Headless `--snap` runs never start it; the scenes `tour-plan`, `tour-preview`, `tour-show`, `tour-live` and `tour-search` render each stop.

The research the shape follows (`ux-principles/principles.md`, section 5): tutorials mostly fail when they are long or in the way, so five stops of a few words each, Skip on every stop, never shown again by itself once finished or skipped (`Settings::toured`), and findable again.

The stops and their words are one table, `STOPS` in `tour.rs`, so the words can change without touching the drawing. A ring goes round a container the shell keys, never one button, because the buttons are rearranged often.

| Stop | Ring (key) | Says |
|---|---|---|
| 1 | The plan (`plan`) | It holds everything to show, in order; drag to move |
| 2 | Preview's slide list (`slide-list`) | It is the slide that comes next; nothing here reaches the room |
| 3 | The buttons under Preview (`preview-controls`) | Show moves Preview into Live; the room sees it only while the projector is on |
| 4 | The buttons under Live (`transport`) | Live is what the room sees; the Live button switches the projector on and off |
| 5 | The search box (`search-button`) | It finds any song or verse; open it, then Add to plan |

Stops 3 and 4 are written to the model the owner gave on 2026-11-11: Show moves Preview into the Live view, and a separate Live button turns the projector on and off. If that model changes again, change those four strings and nothing else.

**It teaches the model, never keys, and it owns the keyboard while open.** Enter or Right is next, Left is back, Esc skips, and every other key is taken and does nothing, so nothing in it can change the projector (`App::tour_key`, called first in `on_key`). Enter is left to a focused button, which Rune presses itself, so the window does not step twice. The card is a Rune modal: the background is inert and Tab stays on the card's buttons, which take focus as dialogs' buttons do; Next has it first.

Started again from the keys sheet (button at the bottom), from the search ("tour", "tutorial", "help", "panduan", "bantuan"), and from a quiet button on the first screen that fills Preview while nothing is open. A part that is not on screen (an editor has the Preview pane) is skipped by dimming the whole window and centring the card.

## The song editor

The words are plain text with marker lines, because that is what pastes in from anywhere and what EasiSlides wrote. The editor does the rest around the text (`ui/editor.rs`):

- The words come right after the title, one Tab away; author, key, copyright and the CCLI number are the last rows, filled in when at all (two testers pressed Tab five times to reach the words).
- Marker lines are coloured like their slides.
- Verse, Chorus, Pre-chorus, Bridge and Ending write the next free marker at the caret (Ctrl 1 to 5). Under a line of words the marker brings the empty line that stanzas are written with (`insert_marker`).
- Tidy turns headings such as "Verse 1" or "Reff:" into markers and drops chord lines (`song::tidy_lyrics`).
- A quiet strip over the words proposes marks while any stanza has none (`artosia-core/src/marks.rs`, `ui/editor.rs` `marks_strip`). It shows the sections the song would get as a row of chips, tinted where the words already say it and outlined where Mark would write it; the chip's tooltip says why (read from a written name, sung again word for word, short and late, next verse). Mark (Ctrl M) writes the marker lines as one edit that Ctrl Z undoes, and the caret stays before the same letter; the cross hides the strip for this song. Nothing is marked without a press. Pasting a song, filling one from the web and pressing Tidy leave words with no heading unmarked on purpose, so the strip is what offers the marks.
- With the caret in a stanza that has no mark, a second row names what it would be and offers the six kinds (verse, chorus, pre-chorus, bridge, intro, ending) as chips of one width. A press writes one marker line before that stanza, numbered by `Proposal::tag_as`: a second verse is Verse 2 by itself, a second chorus worded differently is Chorus 2.
- The proposal follows these rules, in this order, and its doc comment is the authority. A name in the text is read (`Reff`, `Bait 2`, `Chorus:`, `Jembatan`, also before the first line as `Reff: words`). A stanza that comes back word for word, ignoring case, punctuation and spacing, is the chorus; the most repeated wins. A short stanza sung once in the last third is a bridge, when the song has a chorus and four stanzas. The rest are verses in order, skipping numbers the text uses. A stanza under a mark keeps it. It cannot tell a pre-chorus from a verse, an intro or ending from a verse, a chorus sung once, or a chorus whose second singing is worded differently. After a mark, a stanza with none is read as that mark's next slide (that is how `parse_lyrics` reads it) unless fewer than half the stanzas are marked, so a song marked in more than half its stanzas by hand is never second-guessed.
- The order is built from chips: a click on a chip in the order takes it out, a click on a section under it sings that section again. "Chorus after each verse" writes the usual hymn order. A star repeats: `C*2` (`song::parse_order`).
- A new song's title offers whole songs under it, as an editor offers a completion. Songs already in the library come at once and open with the down arrow and Enter. Songs from the web arrive 0.4 seconds after the typing stops and fill title, author and words the same way (`artosia-lyrics`, LRCLIB). The line under the title then says where the words came from and to check them.

LRCLIB found 9 of 30 sampled titles from the church library: 7 of 10 English and 2 of 20 Indonesian (`crates/artosia-lyrics/README.md`). Paste stays the main way in for Indonesian songs. A song's title is sent to LRCLIB when a new song is being typed; nothing else leaves the computer.

## Announcements

The announcement editor takes the Preview pane and the dock: the list on the left with move, duplicate and delete on the picked row, the form on the right under a preview that changes as you type (`ui/announcements.rs`). What is being written is saved when another row is picked, when the editor closes and on Ctrl S.

- **Every row has a Show now control** (the play icon), and the form has a labelled one beside Save. It puts that one card on the projector alone (`App::show_notice_now`, deck source `Announcement(id)`); the other cards stay out, the loop does not start. A card with nothing to show today (nobody has a birthday, a passage the Bible cannot read) leaves the button grey in the form and says so in a toast from the row and from search.
- **Pressing a row's main area opens the card for editing and puts it in Preview** behind the editor (`pick_notice`, `preview_card`); closing the editor leaves it there. Opening it in Preview does not close the editor, because `open_deck` would.
- **Add to plan sits in the editor's header** and reads "In today's plan" (grey) once the announcements are a plan row. The plan column has its own way to add; this one is for a volunteer who is already here.

| Part | What it does |
|---|---|
| Seconds on screen | How long this one stays up in the loop |
| Start the loop | Shows each announcement in turn on the projector until stopped or until something else is shown (`App::loop_step`). Showing one announcement by hand does not start it |
| Fill in by itself | Four choices: Manual, Bible passage, Birthdays this week, Birthdays this month. The two birthday choices write the names from the birthday list each time the card is shown, thirty to a slide in three columns of ten (`BIRTHDAYS_PER_SLIDE`, drawn by `NoticeKind::Birthdays`), each with its date as the window's language reads one (`Today::date`, the `day_month` text: "30 December", "12月30日"), and under the choice the card says which days it covers ("Covers 11 November to 17 November", or all of the month) and how many names it holds today, so a volunteer knows what will come up before showing it |
| Bible passage | The volunteer types a reference, in Indonesian or English, and the words come from the Bible in use with the reference and its code as the small print (`deck::passage_state`, `announcement_pages`). It reads the reference with `Bible::guess`, the same parser the search uses, so "Mazmur 23:1-3" and "psalm 23 1-3" both work; a bare book name is not enough. It is read again whenever the Bible changes or finishes loading. With no Bible installed, or a reference the Bible has no verse for, the card says so under the field and offers the Bibles dialog; it shows nothing on the projector meanwhile. The "Verse of the week" ready-made card starts as a passage card; choosing Manual makes it plain text again |
| Picture | A picture shown with the words |
| Import a PowerPoint | Each slide becomes an announcement: a table row becomes a line with its cells in columns, a picture slide keeps its picture (`artosia-office`) |
| Import a birthday list | A Word or PowerPoint table of names and dates replaces the birthday list; each person is kept once |

Search finds the birthday card for "birthday" and "ulang tahun" and a passage card for "verse" and "ayat" (`palette.rs`, `documents`); its detail line says how many names it holds today, or the reference. Ctrl Enter shows it at once. A card with no slide today does not show another card's slide in its place: it says so.

A list longer than ten lines goes over several slides. The church's own bulletin (6 slides) and birthday list (26 tables, 682 rows) were read on 2026-11-10: 6 announcements and 331 people after repeats were removed. Those files hold names, birthdays and gifts and stay out of the repository.

## One Artosia, its own title bar, light and dark

The window opens filling its screen (`main.rs`, `App::maximized`, 2026-11-11). At its own size Windows placed it 128 points in from the corner, and its foot hung under the taskbar of a 1080 point screen and off a 768 point one, where the status bar and the lowest tiles could not be pressed; the owner saw it from the outside ("summoning the window partly out of bounds making clicking impossible").

- **A second start hands over to the first and leaves** (`main.rs`, `claim_instance`, port 47824). The owner had two open on 2026-11-10 and the Screens dialog sat on "Starting the projector"; two operators on one library also write the same plan and talk to the same projector. Whether the second copy caused the stuck dialog was not reproduced. Headless runs, `--practice` and `--data` are exempt.
- **The top band is the window's title bar** (`rune::Titlebar`): it drags the window and the system draws its buttons at the end. The mark in it is a loaf with a baker's cuts ([brand.md](brand.md)); the same mark is on the program file and the taskbar.
- **The window follows the computer's light or dark setting**; Look has "Artosia's window" to pin one (`theme::set_dark`). Section colours are picked for dark slides, so on a light window they go through `theme::ink_of` and `theme::tint_of`.

## A song in several languages (board card MULTI1, 2026-11-12, owner-picked layout: a block per language)

One song may hold its words in several languages, and a slide draws them together: one block per language, the lines of a language together, every language at one size, each block in its own colour (`artosia-slides/src/voices.rs`, `Look::voice_ink`). Which languages show is the church's choice, asked in the first run, with a switch beside Live to show one language alone for a while. Nothing is ever generated or transliterated: a language's words exist only because they were typed, pasted or imported.

- **What a song holds.** `Song::lang` (a BCP 47 tag such as `id` or `zh-Hans`, empty when nobody named it) and `Song::translations` (each a code and the same sections with the same marks). The store keeps them in `songs.lang` and `songs.texts`, and still writes the first further language into the old `second` column, so an older Artosia opening the library shows it. A library from 0.1.16 moves its `second` text into `texts` once, with no code.
- **Which languages a slide shows.** `Settings::song_languages` is the church's usual list, in order; a plan may hold its own (`Plan::languages`, as it may hold its own look). The deck of every song is built with the plan's list or, when it has none, the church's (`App::song_languages`, `App::deck_of_song`). A change builds the decks again and every live and open slide keeps its place (`song_languages::same_place`); switching plans never touches what is on the projector. A listed code fits a song's language by the rule of `artosia-locale` (`en` fits `en-US`, `zh-Hant` fits `zh-TW`; a transliteration such as `ja-Latn` fits only itself). A translation with no code that the list does not name follows the others, exactly as 0.1.16 drew `second`: a church that updates sees no change.
- **The split.** Blocks that cannot be set at 48 points (at 1080 high) split the slide in step, every language at the same line, marked 1/2 (`deck::split_in_step`). The room is measured against the real projector's shape once the output process has reported its screen (`Link::projector_size`, `App::projector_shape`); until then, with no projector, in a headless run and in the previews (which are the 1920x1080 view scaled), it is 16:9.
- **Names of languages.** `artosia-core/src/languages.rs`, from `assets/languages.json` (`tools/make-languages.py`, CLDR 49.0.0, files in `~/Shared/research/artosia-multilingual/cldr/`): `name(code, window)` is the name where it stands alone, on a chip or a row, with a capital first letter (CLDR writes Spanish, French and Portuguese names small); `name_in_sentence` is the name inside a text, as CLDR writes it, with Vietnamese's "Tiếng" small. Chinese in one script is named as CLDR composes it where a locale has no short name ("Tionghoa (Sederhana)"). `ordered(window, country, taken, extra)` is the one order of every list of languages: the window's language, English, the country's languages as CLDR ranks them, the other languages the window speaks, then the rest by name, with those CLDR does not name in the window's language last and those already taken left out. A code CLDR does not name shows as itself.
- **The dialog "Languages on the slide"** (`ui/languages.rs`). From a plan's menu, from Settings (Songs) and from the search ("languages"). From a plan it opens on the two tiles of the look dialog, the usual languages or this plan's own; dropping a plan's own list offers Undo. Under them one row for each language, a switch and two arrows that move it, and Add a language, which opens the picker: pick, never type (a filter and a list in the window's language, Enter takes the first).
- **The editor.** A row of chips over the words, one for each language, the first language first; a press puts that language's words in the same field. The last chip adds a language through the picker; a chip's menu removes its language with an Undo toast. A song from before holds no code for its first language: its chip asks for one and the window's language is never guessed. Section marks, the marks strip and Tidy work on whichever language is in the field (`Editor::show`).
- **The Live pane.** A chip for each language the live song shows, beside the heading, none for one language. A press shows that language alone (a lit fill, the status line turns yellow and says so), a second press shows all. Never saved, off at every start, and it never takes the keyboard's focus. In the thin pane beside an editor the chips share the room equally and cut their names with dots (`live_chips`, `THIN_PANE_ROOM`); they were clipped at the pane's edge. The search has one command for each language and one that shows all again, each named on its second line by what it belongs to.
- **The first run** has a page after Songs, "Which languages does your church sing in?": the window's language ticked, the country's languages first, and Next keeps the ticked ones in the order ticked.
- **The phone** is sent each language's lines of every slide and draws them as blocks in the same order, the further ones dimmer and smaller.
- **Search.** Every language's lines are searched; a song found by a line in another language shows that line and a chip that says whose lyrics they are (`match_language`): six of nine native readers took the bare name for the window's language.

Scenes: `editor-languages`, `editor-language-picker`, `editor-language-empty`, `languages-plan`, `languages-settings`, `languages-picker`, `live-languages`, `live-language-solo`, `onboard-sing`, `search-other-language`, `search-only-language`; renders in `target/multilingual/operator/`. The practice library's song in three languages shows all three (its settings list them).

Checked on 2026-11-12 in a real practice window with `tools/drive-window.ps1`: search, Enter and Enter show the song; the search's command shows one language alone and another shows all again; a real pointer press on a Live chip shows that language alone and Space still moves the projector; in the editor Shift Tab reaches the chips, Enter on the last opens the picker with the filter focused, three typed letters and Enter add the language with its empty field, a chip's menu removes a language with Undo, and Escape closes with Undo. The released 0.1.17 projector program, started on a spare port, drew a made-up slide in three languages on the real second screen (1920 by 1080 at 150%): twelve lines in three blocks, nothing clipped, and its size check read "same" (`python tools/send-state.py <port> <seconds> languages`, `tools/grab-window.ps1`). Nine native second readers then read the eleven scenes in their languages (`target/i18n-round/round26`), and each writer answered its reader.

## Two languages

The window speaks English or Indonesian (Look, Language; or type "bahasa" in the search). Section names on slides follow it: Verse 2 is Bait 2, Chorus is Reff. No language is the source the other is translated from: `docs/i18n/brief.json` says what each text has to do, and the Indonesian in `docs/i18n/id.json` was written from those briefs by a writer who never saw the English, then checked by back-translation and rendered. `strings.rs` holds both; a test fails when its Indonesian and the reviewed file differ, and another on spellings an Indonesian reader catches at once. A changed text changes its brief first.

Rendering found what the briefs could not: three toggle buttons, a three-way choice and the key sheet were too narrow for Indonesian words, and the layouts were widened or wrapped (`ui/live.rs`, `ui/announcements.rs`, `ui/sheets.rs`). `--indonesian` renders any scene in Indonesian.

## Listening and streaming

- **Listen** (top bar) opens a source (the computer's own microphone, any other by name, or System audio, which is what the computer plays), shows how loud it is and its tempo, and offers the passage or songs that were heard. It only suggests. Recognition needs a build with the `whisper` feature and a model in the library's `models` folder; the dialog offers the model's download, and a build without the feature says it hears the beat only. A source that will not open is said in the window's own words (`why.rs`), never the library's.
- **Phone** (top bar) switches the phone remote on and shows its address, QR code and PIN. Switched on for the first time it asks Windows once, in its own command, to let phones reach this program (`firewall.rs`, [remote.md](remote.md)); the dialog first says Windows will ask for permission, and if the rule could not be added it shows a yellow line saying Windows is not letting phones in and that switching Phone off and on asks again. Left on at start, it only checks and warns; it never opens Windows' box unasked.
- **A camera on the network** has a row of its own in Design, Background, under this computer's cameras (`ui/sheets.rs` `looks`, `actions.rs` `use_camera_address`, 2026-11-11): its address is typed or pasted once and Use puts it behind the words. A chosen camera is lit in the accent; it was yellow, which means hidden or a warning. What is wrong with a background is said in the status bar and under these rows, in the window's language (`why.rs` `media_problem`). Full account: [output.md](output.md)
- **Two speech models** (`speech_model.rs`, 2026-11-11, owner-reported: the words heard were bad). The first, 60 MB, runs on any PC. The listener times every phrase it reads, and a PC that reads three in a tenth of their own length is shown a row in Listen that fetches the better one, 190 MB, on a press; the listener then starts again with it. On eight real recordings of sung Indonesian the better one named six songs and the first three, and neither offered a song during a sermon. A PC on which the better one takes more than six tenths of a phrase's length, or on which three ended phrases wait unread, goes back to the first and is not asked again; if the first falls behind too, the dialog says that this computer reads words too slowly to keep up, and the listener reads only the two newest waiting phrases. Held to one slow core in a real window, the app went from the better model to the first after 40 seconds of music and kept the choice. In a real window on 2026-11-11: the row came up after 45 seconds of music, its button fetched the file in 9 seconds, and the song playing was offered 14 seconds after the better model took over, where the first had offered nothing in a minute. Full account: [crates/artosia-listen/README.md](../crates/artosia-listen/README.md)
- **Half, As heard, Double** sit beside the tempo number (`ui/sheets.rs` `tempo_row`, `actions.rs` `set_tempo_count`, 2026-11-11). The sound cannot say whether a pulse is the beat or half of it: on real music a ballad at 60 read 120, and the tracker flipped between the two within the song. A press says what the tempo is. From then on every reading is counted at half, as heard or double, whichever reaches that tempo within 6%, so the number holds while the tracker flips; when no count reaches it the song has changed and the reading stands as heard again. The dot and the pulse of the Live marker follow the count. Pressed in a real window on that ballad: 120 became 60 and stayed 60 when the tracker went to 60 by itself. With real keys, Tab reaches the three choices after the language and Left and Right change them; a click on a choice leaves the keys where they were.
- **Recordings** are the second part of the Listen dialog (`recordings.rs`, 2026-11-11, owner-asked). Switched on, each phrase the listener cuts is kept as a wave file in a folder on this computer, beside a log of what was heard, what Artosia guessed and what was on the projector; a filled mark reading "Recording" stays in the bar at the foot of the window meanwhile. Nothing is sent: Save as one file makes a zip to hand over, Move folder puts the folder where a cloud drive copies it, Delete all removes everything with Undo in its toast. `python tools/learn-from.py <folder or zip>` reads it back. Full account: [recordings.md](recordings.md)
- **Stream** (top bar) starts `artosia-stream` beside the operator with the service's address and key, or a recording, or both, and shows its state once a second. The key is kept in the library file on this computer and is never written to a log. Closing Artosia on purpose ends the stream cleanly; a crash leaves it running, as it leaves the projector.

## Updates

Releases are GitHub Releases, made by `tools/publish.sh`; an installed copy needs no setting to find them. Full account: `crates/artosia-update/README.md`.

What a person meets (2026-11-12, the owner's question): one check in the background a few seconds after each start, once the window has shown it works (`actions.rs`, beside `update::confirm`), the changed files fetched beside the program, and a single toast once they are there ("Version 0.1.19 is ready. It installs when Artosia next starts.", scene `toast-update`). No screen before the window, no dialog, nothing when there is no newer version or no internet. The swap happens at the next start, before the window opens, so a service is never interrupted. The Store edition has no updater of its own: the Microsoft Store installs its updates. After a start on new files the shortcuts are pointed at the new icon ([setup.md](setup.md), "The icon after an update").

The check waits for the confirmation on purpose. Until 0.1.20 it ran at the start, and on the first start of a version that had just been swapped in the confirmation three seconds later emptied the `update` folder the download was filling (`artosia_update::confirm_started` drops what was staged). A copy two versions behind therefore needed a start more than it should: 0.1.14 with 0.1.17 waiting took three starts to reach 0.1.20 in the test of 2026-11-12. From 0.1.21 the same copy takes two. This was read from the code and the folder's contents; a run that shows two starts needs two further releases and was not made.

Artosia looks for a newer version at each start and downloads it beside the program; the files are swapped the next time it starts (`artosia-update`, `update.rs`). Every file is checked against a signature made with the release key, whose public half is built into the program. A development build and a headless run never update themselves. "Check for a new version" in the search reports what it found either way.

## What to show next

One strip, `ui/suggest.rs`, over the slides of the Preview pane offers at most three things, each with an icon for why (an ear for what was heard, a clock for what usually follows), the song's title, its section as the coloured chip the lists use, and one icon button of a fixed size that brings it into Preview, with its key on it (F4 on the first). The reason in words is the tooltip. The model is `suggest.rs`, `suggestions(&App)`. An offer only ever fills Preview; nothing goes on the projector, because a misheard word must not reach the room.

| Order | Offer | When |
|---|---|---|
| 1 | The song or passage the listener heard, at the section where the words are | A phrase of at least four words has a line holding 70% of a six-word chunk of it (and at least three words), or a Bible reference was spoken. Offered for 60 seconds, unless it is already in Preview at that section |
| 1 | A far part of the live song ("the singing is here") | The heard words are in the live song at a section two or more slides from the live slide |
| 2 | A warning that the words being sung are in no song | A song is live, the listener matched something in the last four minutes, and then two phrases in a row (within 90 seconds, the newest within 30) matched nothing. It keeps its words, in yellow, with a search button, because a warning cannot be a tooltip |
| 3 | What usually follows the live song | Learning is on and recording (not in practice), Preview holds nothing a person chose, the plan has nothing after the live item. The song is not already in the plan |

A phrase of the live song at or next to the live slide offers nothing. A phrase of fewer than four words neither matches nor misses. Stopping the listener forgets what it heard. Pressing an offer quiets the warning until a newer phrase arrives. Scenes `suggest-heard`, `suggest-follows` and `suggest-lost` draw the strip through a hook in `ui/mod.rs` (`Hearing::overlay`) until `ui/shell.rs` places it. Full account of how sure the warning can be: [palette-suggestions.md](palette-suggestions.md).

## Rules the code keeps

- **Buttons in the main window never take keyboard focus** (`ui/widgets.rs`, `Btn`). Rune presses the focused control on Space, so a focusable Black button would toggle again when the volunteer pressed Space for the next slide. Buttons in dialogs and the two editors ask for focus with `.focusable()`.
- **Anything sent to the screen shows.** Clicking a slide, Next and Back lift Black, Clear text and Logo (`App::reveal`).
- **A removal offers Undo and asks no question first.** Removing a plan item, deleting a song, deleting an announcement and closing the song editor with unsaved words show a toast with Undo for eight seconds (`App::say`, `Undo`). Escape inside an editor field closes the editor at once; left to Rune the first Escape only takes focus off the field (`ui/editor.rs`, `escape_closes`).
- **The projector never follows the Preview pane by accident.** Opening another item while one is live leaves the live item alone; Next keeps advancing the live item, and the Show button under the Preview list says Enter shows the open one.
- **A text field's `on_edit` accessor never creates state.** Rune calls it once more after the field's screen has closed, to tell the field it lost focus. The song editor's accessors first used `get_or_insert_with`, so pressing Save showed "Song saved" over a new, empty editor. A scripted Ctrl S found it; the accessors now write to a spare edit when the editor is gone (`ui/editor.rs`, `edit_of`).
- **Every string comes from `strings.rs`.** Counted nouns go through a function. A test fails on an em-dash, en-dash or middot.
- **The repository holds no real song lyrics.** The practice songs in `data.rs` were written for Artosia, and tests use made-up lines.

## What a slide is made from

Lyrics are plain text. A line holding only `[1]`, `[chorus]`, `[bridge 2]`, `Verse 1` or `Chorus:` starts a section; with no markers a blank line starts a verse. The order field takes `V1 C V2 C B C` or EasiSlides' `1,c,2,c,b,t,e` (`t` is the second chorus and `e` the ending; those are the only letters the 961-song database uses). A section longer than the look's lines-per-slide is split evenly: six lines at a limit of four become three and three.

Text size is fitted to each slide: a lyric line shrinks until it fits on one line (down to 60% of the standard size) before any line wraps, then the block shrinks until it fits the screen. Previews draw on a 1920x1080 canvas scaled down, so they break lines where the projector does.

## Bringing a library in

| From | How | What arrives |
|---|---|---|
| EasiSlides 5 on the same computer | The start screen offers it once; later, search for "EasiSlides" | Every song database under `Admin/Databases` and the Bibles under `Admin/HolyBibles`, copied; EasiSlides is left as it was |
| An OpenLyrics folder | Search for "Import songs from a folder" | Every `.xml` song; a library's `index.json` adds its alternate titles and skips empty placeholders. The file name is kept as the song's source id |
| A zip of songs | Search for "Import songs from a zip file" | The zip as it was downloaded: unpacked into a scratch folder, read as the folder above, and the scratch folder removed. In a real practice window on 2026-11-13 the church's own zip went in through the real file window: "Added 1333 songs" |
| Lyrics on the clipboard | Ctrl V in the main window, or "Paste lyrics as a song" | A new song in the editor, split into sections, the first line as the title when a blank line follows it |

Importing twice adds nothing. A song with a source id replaces the song imported earlier under that id, so a title corrected in the source library updates in place; the church's cleaned library keeps its ids stable for this.

**An edit made in Artosia wins over the library the song came from** (`Song::edited`, `Store::import_songs`, 2026-11-10). Saving a change to an imported song marks it edited, and a later import leaves it alone and says how many it kept. To take the library's version back, delete the song and import again. Played on the church library: one song edited through the editor, the library imported again, 1,333 songs and the edit still there.

A song without a source id is skipped when one with its title, author and opening words is already there, and items that only share a title all come in.

`crates/artosia-core/tests/church_library.rs` imports the church's library whenever `fixtures/church/library` is on the machine: every file must read, empty placeholders must be skipped and every alternate title in `index.json` must arrive. The import report stays on screen as a toast, including when the import is started with `--import-openlyrics`; hiding it once concealed two unreadable files, which turned out to be the library being rewritten by another session at that moment. A file that fails is reported beside what the rest of the import did, never in place of it.

## What keys reach (2026-11-11, after a keys-only blind test)

A tester who could use only the keyboard and saw only screenshots failed two of twelve everyday goals and stumbled in five more. Each cause, and what now holds:

| What happened | Cause | Now |
|---|---|---|
| Enter did nothing on a dialog's buttons | Rune presses the focused control on Space and gives Return to the default button, as AppKit does | `ui/widgets.rs` `enter_key` hangs on every element that asks for focus and passes Enter on as Space |
| No way to copy a plan | The plan's menu could only be opened with the pointer | Every action of a menu is a command in search: Plans, Duplicate plan, Rename plan, Delete plan, Start the song here, New announcement (`palette.rs`) |
| Fourteen Tab presses to reach New announcement | Every button of every card was a Tab stop | The card list and the examples are focus groups: one Tab stop each, arrows inside; Alt and an arrow moves a card, Delete removes it, Ctrl N starts a new one, Ctrl E opens the editor from the announcements item |
| Preview kept the old plan's item after a switch | `enter_plan` left what a person had opened | What Preview holds leaves with its plan, and with nothing live the new plan's first item opens |
| Alt Enter put a slide in front of the room | Enter ignored its modifiers | Enter with Alt held does nothing |
| Letters landed in the wrong field | The test harness sent the next key while a long text was still being typed | `tools/blind.py` waits for the text's length |
| A dialog opened from search took no typing until it was clicked (real window only) | The search blurred the focus after the pick, which ran after the new dialog's field had asked for it | The search leaves the focus alone when its pick opened a dialog, the box that names a plan or an editor (`App::wants_focus`) |
| "Ctrl held" stayed lit | A headless script never sends a key's release | Headless runs no longer set the badge from key events |

Found while pressing real keys afterwards: **Enter did not start a new line in the lyrics box**, in any release so far. Rune at the pinned rev treats a field of several lines as a chat composer (Return submits, Shift Return breaks the line; `edit/input.rs`). `ui/widgets.rs` `enter_breaks_line` is called first in the `on_key` of every `TextArea` and passes Enter on as Shift Return.

F1 hides the words here, where Windows programs open help; the tester pressed it for help once and undid it at once, because the tile lights up yellow. The keys of the three screen states stay where two releases put them, and `?` opens the keys.

The second keys-only test (2026-11-11, on the build before 0.1.5) did six of its goals, failed one and ran out of harness time; what it met:

| What happened | Now |
|---|---|
| Tab in a new song's title took the first song offered under it and threw the new song away | Tab is the next field; an offered song is taken only after the down arrow chose it, with Enter; the list shows only while the title has the focus |
| No way to reach any announcement card but the first | Ctrl and an arrow goes to the card before or after, from any field; the keys are drawn beside the count |
| "new plan" listed two songs above the command "Start a new plan" | Two or more typed words that stand together in a command's name find it before songs holding one of the words; a name typed whole leads the list |
| Arrows did not move among the colour squares | The squares are one Tab stop with arrows inside |
| A grey example in an empty title was taken for a card | The hints begin with the words for "such as" |
| Enter on the tour's first card ended the tour in the test | While the tour is up Enter is the tour's, whatever holds the focus |
| The test tool stopped at step 108 | `tools/blind.py` waits up to fifteen minutes for a replay |

The third keys-only test (2026-11-11, on 0.1.6, two testers) reached all twelve of its goals; where it had to guess, and what changed:

| What happened | Now |
|---|---|
| Nothing on the plan said how to open an item | The row Tab opens carries Tab |
| Space on a song's last slide went on into the next song, unannounced | The space bar is drawn on the ringed row of Preview, the slide it will show |
| The Earlier button showed no key, and Up was a guess | Backspace brings what was shown before into Preview, as it goes back in Windows' own file windows; the button prints it, the search has it as a command |
| "Tab: Songs or Bible" at the bottom of the search read as a riddle | Tab is drawn beside the switch it turns |
| Shift Enter in Preview put the song in front of the room | It adds what Preview holds to the plan, as it does in the search, and the Add to plan button prints it |
| Ctrl E with the ring on the second announcement opened the first | The card of the ringed slide opens (`card_at_slide`) |
| "yellow" found Bible verses and no setting | Design answers to what a person wants from it (a colour's name, font, outline, lines) and opens on that part |
| The arrows moved over the colour squares without choosing | A colour is one choice of several: the arrows choose as they move, as in the switches beside them |
| Five Tab presses from a new song's title to its words | The words follow the title |
| A chorus started with Ctrl 2 sat right under the verse | A heading started under words brings its empty line |
| A picked backdrop took a Tab to reach Use (met while pressing real keys) | A second Enter, or a second click, on the picked backdrop uses it |
| Design opened with the focus on its close button (the same) | Each dialog opens on what the person came for; back from the gallery, on the button that led there |

Left as it was: Preview goes on to the next plan item when a song is shown, which one tester took for the wrong song going live until the Live title was read. That is the owner's rule ("Preview is what comes next"), and the tour's second stop teaches it. A black outline on the dark sample of Design cannot be seen, because it cannot be seen on the projector either.

The fourth test (2026-11-11, on the build with the changes above, one tester with keys and one with the pointer in Indonesian) reached all fifteen of its goals; what it met:

| What happened | Now |
|---|---|
| A song's title in Preview was cut to "Q..." beside its buttons, in Indonesian on a 1366 point screen | What a pane holds is named on a line of its own under the pane's name, in both panes |
| Tab stopped on the first colour square and the arrows were never tried | The two arrow keys are drawn beside the squares |
| No picture of the gallery showed which one the keys were on | The focused picture has a ring, drawn over the picture (`ui/backdrops.rs`) |
| A tick on a fetched picture read as "already chosen", and the Use button with the sign for fetching read as a download | A tick is the backdrop behind the words now; a fetched file has the sign for fetched; Use carries a tick |
| The six backgrounds still offered white words after the words were made yellow | A chosen text colour stays with a new background where it can be read on it (`Look::with_preset`, 4.5 to 1) |
| Which of "The usual look" and "This plan's own look" was on could not be told | The chosen one carries a tick as well as its fill |
| The button for what was shown before read as a label cut short in Indonesian | Its brief asks for a name that is complete as it stands |
| The practice announcements were English in the Indonesian window | They are written in the window's language and follow a switch while nobody has changed them (`strings::PRACTICE_NOTICES`) |

Left as it was: a double click on a plan row opens it in Preview and shows nothing, which is the owner's rule; Backspace and the Earlier menu fill Preview and wait for Enter, for the same reason; a toast lies over the tiles under Preview for the seconds it shows. The practice songs and the bundled Bible are English in every language.

The test of weekly plans (2026-11-11, one tester with the pointer in Indonesian, one with keys). The pointer did all eight goals in 31 steps and told the dividers from the items at once; the keys did one goal in thirty steps. What they met:

| What happened | Now |
|---|---|
| Four plans in two day groups did not fit the list's five rows, and the open plan scrolled out of sight | The list takes about a third of the column, and a plan just made or changed is scrolled into view |
| The open plan's fill read as a hover | It carries a tick as well |
| A plan's menu opened beside another row | The menu's first line names its plan |
| The box offered an hour nobody chose | No hour is chosen until one is pressed, and Save waits |
| After a song was added with its plus the search stayed open, with no way out for the pointer | The search has a close button |
| Nine arrow presses from 16 to 07, six for the minutes | Digits type the time |
| Enter on an hour did nothing | Enter saves the box |
| No key between plans | Ctrl Tab and Ctrl Shift Tab |
| "copy plan" was not offered while the open plan was empty | It always is |
| "plan" filled the search with chapters of a book of the Bible | A book guessed from a word that names a command or a plan does not go first |
| A plan with no name and no day read "Today" under a heading that can read Today | It reads as a plan |

## What a pointer-only volunteer met (2026-11-11, second blind test, in Indonesian)

All fifteen goals were reached; these are the places where the tester had to guess, and what changed.

| What happened | Now |
|---|---|
| No visible way to write a new song from the plan | "New song" is in the plan's Add menu, and a song written from there joins the plan when it is saved |
| The box that names a new plan was titled "Rename plan" | It is titled with the action that opened it |
| The way back after too many presses of Next was a clock named for what it lists | The button reads as the way back to what was shown before |
| Gallery tiles were blank grey until their previews arrived | A tile without its picture shows the backdrop's title |
| One announcement card in Preview offered "Add to plan" though the plan held the announcements | A single card counts as in the plan when the plan holds the announcements |
| Save on an announcement confirmed nothing | A toast says the announcement was saved |
| The plan's name was cut in its header | The name has the column's whole width: room was kept for two buttons that no longer exist |
| A play triangle with "Ctrl M" explained nothing | The two tools of the Preview title carry a word each: Start here, Edit |
| A plan row's title was cut by the chip of the section it begins at | That section is a small line under the title |

## Checking a screen

`tools/snap.sh <folder>` renders every scene in `main.rs` (`SCENES`) to PNG with the practice library; `--query "<text>"` opens the search on any text; `--background <file>` puts a picture behind the text; `--script crates/artosia/scripts/type-and-show.json --at mark:live` plays a keyboard flow first. `--data <folder>` points at a real library.

Rendering a scene shows how a screen looks, and only a script shows what a key or a click does: the Save and Escape defects above passed every scene render. Played through scripts on 2026-11-10: type to search, open, show, Space, F2, click a slide; Ctrl S in the editor; Escape in the editor with and without changes; Undo from the toast; a plan row's menu; adding an announcement; "Add the next verse"; turning the phone remote on. Not yet played: the file pickers, drag and drop, the Screens dialog with two screens, the Look dialog's controls, moving a plan row, the logo.

## The UI overhaul (2026-11-14, owner-picked)

The owner asked for as few plain text fields as possible ("the one method you keep falling back
to without research"), a colour picker instead of hex codes, a real calendar for the schedule,
birthdays and money as lists and tables, video with sound and OBS-like scenes in the rundown.
Research: `~/Shared/inspo/artosia-ui-overhaul/` (README, directions.md, skeleton.md; 11 worship
and church office programs, Mobbin for seven controls) and
`~/Shared/research/artosia-scenes-pptx-video/README.md` (PowerPoint, video items, scenes).

Three directions were drawn in the real window as scenes `mock-a-p1` to `mock-c-p7`
(`crates/artosia/src/ui/mock_a.rs`, `mock_b.rs`, `mock_c.rs`; sheets by `tools/overhaul-sheets.ps1`).
**The owner picked B, Data windows after Planning Center and ChurchSuite, with A's rundown.**

| Part | What B looks like (mock page) |
|---|---|
| Window | pill tabs under the top bar: Rencana ibadah, Pengumuman, Kalender, Jemaat, Persembahan, Templat, Tampilan (`mock-b-p1`) |
| Announcements | a template list, the slide, and a data source panel; nothing typed on the slide but its title (`mock-b-p1`) |
| Colour | colours named by role, theme swatches, a swatch grid, a square picker with hue strip and eyedropper; hex only behind Lainnya (`mock-b-p2`) |
| Calendar | a month grid with coloured chips and a repeating-event dialog: day toggles, time and date pickers, the rule as a sentence (`mock-b-p3`) |
| People | a table with month strip, a birth-month filter and import with column mapping (`mock-b-p4`) |
| Offerings | a statement table, amounts right-aligned, a total that sums itself (`mock-b-p5`) |
| Templates | live tiles from the church's data in each style, apply modes (`mock-b-p6`) |
| Rundown | A's layout: header bands, video and audio items with end action (stop, loop, next) and volume, a scene of layers with actions that clear, music lowered under a video with sound (`mock-a-p7`) |

### The tabs (OVSHELL1, built 2026-11-14)

Seven tabs sit under the title bar (`ui/tabs.rs`): the service plan, Announcements, Calendar,
Members, Offerings, Templates and Design. Every start opens on the plan and the tab is never saved.
Ctrl Page Down and Ctrl Page Up walk them, as Excel walks its sheets and Chrome its tabs (Google's
Chrome shortcuts page, read 2026-11-14); Ctrl and a digit stay the Preview ring's, and FreeShow's
plain digits for its tabs (`src/frontend/utils/shortcuts.ts`) would take the live song's jumps.

- The projector's keys (arrows, Space, Enter, digits, letters that search) act only on the plan
  tab, where Preview and Live are in view; F1, F2, F3 and F6 act on every tab.
- Anything brought into Preview (`open_deck`) opens the plan tab, and so do the song editor and a new plan.
- The announcement editor lives in its own tab, beside a narrow Live pane, and is saved and closed
  on the way out (`App::open_tab`), so `notices.is_some()` holds only on that tab.
- Design left the title bar for its tab: the settings in a column, the sample slide as large as
  the rest of the window allows (`ui/sheets.rs` `design`).

### One colour picker (OVCOLOR1, built 2026-11-14)

Every colour the program keeps is chosen in one picker (`ui/colour.rs`): a row offers the usual
colours as squares and More, and More opens the picker under the row, so the large slide beside it
shows each colour as it is tried. The colours already in the design come first, then forty samples,
then a saturation square with a hue strip and a dropper that takes a colour from the slide, then
the colours used last; a colour code is typed only behind its own disclosure. Cancel and Esc put
back the colour it opened with.

### The calendar (OVCAL1, built 2026-11-14)

The Calendar tab (`ui/calendar.rs`, `calendar.rs`, `artosia-core/src/calendar.rs`) keeps the church's
events in named, coloured calendars: a rail with a tick per calendar, a month or a week of coloured
chips (Sunday first, today filled, "+N more" past the room a day has), and a dialog per event. Every
value has the control of its shape: days are toggles, times come from a list in 15-minute steps,
dates from a month, a calendar from a drop-down with its colour; only an event's and a calendar's
name are typed. A rule is once, every week on some days, or every month on the first to fourth or
last weekday, with an end and skipped dates (RFC 5545's BYDAY, UNTIL and EXDATE); one function,
`calendar::occurrences`, works out every date. A calendar's colour is `ColourFor::Calendar(id)` in
the one picker. A card whose fill is `Fill::ScheduleWeek` writes the seven days from today from the
calendar (a day once, then its times), so a change of an event changes the slide; hiding a calendar
in the rail hides it on screen only. Ctrl N is a new event on this tab, a double press on a day
makes one there, and Esc cancels an open colour picker before it closes a dialog.

### Members (OVPEOPLE1, built 2026-11-14)

The Members tab (`ui/people.rs`, `people.rs`, `artosia-core/src/people.rs`) is the table of the
church's people that the birthday card reads: rune-kit's `Table` (virtual rows, `TableState` in
`App::members`) sorted by next birthday, a strip of twelve months with their counts that filters,
a name search, and a side panel with the birthday slide as the card reads it. A person is a name and
a birth date from day, month and year lists (the year is optional, 0 when unknown, and only the
window's age and group, child under 13, youth under 25, adult, use it; no slide carries it).
Delete takes a person out with Undo in a toast. Import takes .docx and .pptx as before and .xlsx
and .csv through `artosia-office` (`sheet.rs`, no new crate: the zip and XML readers were there),
then a column step shows the first rows with a list over each column, guessed from the headings and
the values (`mapping.rs`), and the choice of Add (the default, repeats are left out) or Replace (Undo
brings the old list back). A birthday card writes up to 30 names on one slide in three columns of ten
(`BIRTHDAYS_PER_SLIDE`, `NoticeKind::Birthdays`). Ctrl N is a new person on this tab, and a file
dropped on it is an import.

### Offerings (OVGIVING1, built 2026-11-14)

The Offerings tab (`ui/giving.rs`, `giving.rs`, `artosia-core/src/giving.rs`) is the table the money
cards read. A gift is a date, a giver (empty is the collection plate), a fund, cash or transfer and
an amount in whole rupiah as `i64`; the funds (a name and a colour, `ColourFor::Fund`) and the gifts
are two tables of the library, and the transfer account (bank, number, holder, note, all typed as
the bank prints them) is one setting. `rupiah()` is the one function that writes an amount ("Rp
2.500.000") and `parse_amount()` the one that reads typed digits; the total, the count and the sum per
fund are worked out from the rows shown every time they are drawn and never stored. The table
filters by a range of days (two presses in the month picker `month_grid`, shared with the event
dialog), a fund and a method, sorts by any column, and an amount is edited in its cell: a press makes
the cell a field of digits, each key writes the gift and the slides that read it, and the cell shows
rupiah again when the field is left. The other values are edited in a dialog (month picker, giver or
the plate, fund and method lists). Delete and Delete fund (which takes the fund's gifts) offer Undo.
`Fill::Offerings` gives one slide of the funds that received something on the newest date a gift
exists, with their total, drawn by the Money kind of `notice.rs`; `Fill::Transfer` gives the account
(bank, number, holder, and the note when there is one) on the Account kind, and both give no slide
while there is no gift or the account lacks its bank, number or holder. A range chosen in the table
filters the table only; a card with a range of its own waits for OVTEMPL1's data-source panel.
Scenes: `tab-giving`, `giving-edit`, `giving-empty`, `notice-offerings`, `notice-transfer`, and
`giving-gift`, `giving-gift-date`, `giving-dates`, `giving-funds`, `giving-account` for the dialogs.

### Announcement templates (OVTEMPL1, built 2026-11-14)

Every announcement card is a template that reads one window: the service schedule reads the
Calendar, birthdays the Members, the offering and the transfer the Offerings tab, a verse the Bible,
and only plain text holds typed words. A card carries one of three styles (Hairline, Ledger, Band,
drawn by `artosia-slides/src/notice.rs`) and a title colour from the one picker. A template whose
window is empty says so where the slide would be, never as an empty slide. The Templates tab draws
every template with this church's own data and applies a style to one card, every card or a plan.
