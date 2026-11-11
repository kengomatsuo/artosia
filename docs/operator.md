# The operator window

How a volunteer runs a service in Artosia, and the rules the window is built on. Built 2026-11-10 from the research in `C:\Users\kenne\Shared\inspo\artosia-presenter-ui\` (Lerret's three columns, OpenLP's preview-then-live, Holyrics' book grid, EasiSlides' `[1]` and `[chorus]` markers).

## The order of work

The window reads left to right in the order things happen (`ui/shell.rs`).

| Panel | What it holds | The one action |
|---|---|---|
| Plan (left) | Today's songs, passages and announcements, in order | Click an item to put it in Preview; drag it to move it |
| Preview (middle) | What comes next, as a numbered list of its slides over a picture of the slide with the ring | Enter, or click a row |
| Live (right) | The same list for what the room sees, over its picture, with Back, Next, Clear text, Black and Logo | Down, Space or Next |

The owner picked this arrangement on 2026-11-10 from four drawn after OpenLP, ProPresenter, FreeShow and Lerret. It is the shape OpenLP and EasiSlides have, which the volunteers already know. A list row shows the words at reading size where a thumbnail could not, and its number is the digit key that jumps there. A deck with no sections, such as a passage, numbers every slide.

Panels differ by tone (`theme::surf`): the plan sits furthest back, the two lists are lifted, and each panel has a title strip. The owner's complaint was that the dark window "doesn't show at all the section hierarchy". A test in `theme.rs` holds the text and the grey text at 4.5 to 1 on every surface in both themes; the light kit's stock grey failed it at 3.7 and is replaced there.

A red ring round the Live pane, a red row and a red pill mark what is on the projector. Red means nothing else anywhere in the window. Yellow replaces all three while Black, Clear text or the logo keeps the slide off the projector. The ring is its own element pinned over the pane: Rune draws a border under the element's children, so the pane's own border showed only down its two sides, which read as edge bars.

### Preview is what comes next

| When | Preview holds |
|---|---|
| A plan item goes live | The plan item after it, ring on its first slide (`state.rs`, `queue_next`) |
| Nothing follows it in the plan | The live item itself, its live slide marked |
| A song from the search goes live | The item after the last plan item that was live, so the plan is not lost (`App::resume`) |
| A person opened something there | That, until it is shown. Artosia replaces only what it queued itself (`OpenDeck::queued`) |

Next at the last slide of the live item shows what waits in Preview, at its ring. That is how a medley is armed: with the next song in Preview, Ctrl and Down put the ring on its chorus, and the Next that ends this song starts that one there. Enter does the same at once. The owner asked for both on 2026-11-10: "how do we switch between the live and the preview selection so next song can start at medley? How do we pre-arm that?" and "is it supposed to be automatic?".

The ring also works inside the live song. Esc puts the live song in Preview, Ctrl and an arrow move the ring to another part, and it stays there while the live slide moves (`OpenDeck::armed`) until Enter, or the Next that ends the song, shows it.

### Shift picks, and letting go shows

Holding Shift, the arrows move a second ring through the live song in the Live list, and the projector does not change. Letting Shift go shows the slide it is on; Esc before that shows nothing (`state.rs`, `pick_slide`, `release_pick`). A far slide is reached without the ones between passing over the room's screen. The owner asked for it twice: "if shift or control hold then those arrows, it only moves the selector not change the actual live text", then on 2026-11-11 "when release the shift/ctrl it activates". Shift got the release and Ctrl kept the ring that waits, because a medley needs a ring that is still there after the fingers leave the keyboard.

The release is heard as a key (`Key::Modifier`, switched on with `App::modifier_keys()`). Rune's `App::on_modifiers` looked like the callback for it and is never called in this app: `src/window.rs` at the pinned rev wires `modifier_keys` for a single-window app and leaves `on_mods` unset, which only the multi-window, web and headless paths set. Every headless script passed with it. Real keys found it (`tools/drive-window.ps1`).

### What left the window

The two message fields and the countdown sat in a strip under both panes until 2026-11-10. The owner had them taken out for the room they cost: "That space can be used to enlargen the preview and live view". They are started from the search (`countdown 5`, `message the car park is full`, `stage one more chorus`) or the phone, and while one shows, a yellow pill in the Live title strip says so; pressing the pill takes it off. The stage message has no field on the phone page.

While a song or the announcements are being edited, the editor takes the Preview pane, and Live stays as a thin column. Next is one click away while a typo is being fixed.

The strip under each list has a fixed height. At a laptop's width the three toggles take two rows, and before the height was fixed the two panes ended on different lines in Indonesian.

Back and Next carry arrows that point up and down, the way the lists run and the way their keys point. They and the three toggles keep their words: the three toggles have no icon a volunteer already knows, and a wrong press takes the words off the room's screen.

### Dragging a plan row

A row follows the pointer after 4 points of travel and the rows it passes step aside (`ui/plan.rs`, `draggable`). It is written after the kit's `Reorder`, whose methods are private to the kit and which takes every row to be one height; a divider here is shorter than an item, so the landing place counts the rows whose middle was passed (`drop_place`, with a test). Rune sends the row's press after the drag's release, so the drop sets `plan_dropped` and the press that follows opens nothing. Escape puts the row back.

## Several plans

A church holds more than one service a week (Sunday morning and evening, youth on Saturday), and each needs its own list, kept and named (owner, 2026-11-11: "Where's the multiple-plan support? For different times of the worship schedule idk what it's called?"). The word stays **plan**: the window and the Indonesian interface (`Acara`) already used it, and it says what the list is without saying "service", which in a church also means the hour itself.

**What others call it, from their own manuals.** OpenLP keeps a *service* as a file you open and save, with New, Open and Save As, so a church loads one at a time ([manual.openlp.org/service.html](https://manual.openlp.org/service.html)). FreeShow keeps *projects* in a list in its Show tab, made with a New project button, opened by a double press, and can turn one into a template for new ones ([freeshow.app/docs/projects](https://freeshow.app/docs/projects)). EasyWorship keeps a *schedule* as a file saved and reopened with File, Save As and Reopen, and has no switcher beside the list ([support.easyworship.com](https://support.easyworship.com/support/solutions/6000085139)). ProPresenter keeps *playlists* in a sidebar list, renamed from a right-click menu and duplicated to roll last week's list into this week's ([learn.renewedvision.com](https://learn.renewedvision.com/propresenter6/introduction-to-propresenter/playlists)). Two of the four are files, so a volunteer must find them on disk; the other two are a list inside the program. Artosia takes the list, because the church's PC is shared by volunteers who should never meet a file dialog, and ProPresenter's Duplicate because a new Sunday is nearly always last Sunday with other songs.

**What a plan is.** A row of the `plans` table: a name, an optional date (`YYYY-MM-DD`), the time it last became the open one, and its items (`artosia-core/src/plan.rs`). The table always allowed many rows and the window only ever opened one; the old `New plan` command left each earlier plan behind as a hidden row named "Today". Those rows show in the list now, so nothing is lost. Plans saved before the date existed load with `#[serde(default)]` fields and are tested to do so (`plan.rs`, `store.rs`).

**The controls.** The plan column's title is a button with the open plan's name; it opens a list, the open plan first (checked) and the others most recently used first, each with its date or, without one, how many items it holds. Under the list: New plan, Duplicate plan, Rename plan, Delete plan, all acting on the open plan (`ui/plans.rs`, `plans.rs`). New plan opens the rename dialog at once with the name empty. Rename asks for a name and a date; a date that is not a day disables Save. Duplicate copies the items and leaves the date empty, since a copy is another day. Delete removes the open plan, opens the one used before it (or an empty one), and offers Undo in a toast, which puts the plan back in the list without switching to it. The search finds a plan by its name and Enter opens it, so the keyboard alone switches; "new plan" is a command there too.

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
| Esc | Empty the search; a second press closes it |

Commands that change the projector (Black, Clear text, Next, Start the service, Loop the announcements) run on Enter, like every command. They reach the top of the list only when their own name is typed from its start, so a song search does not put one under the Enter key.

## Keys in the main window

Nothing in the main window holds keyboard focus, so every key below works at any time. `?` shows this list in the app.

The arrows are the ones EasiSlides taught the volunteers (owner, 2026-11-10: "Easislide users are used to up/down as prev/next, left as start from beginning, right as endmost"). Shift and Ctrl turn them into picking keys, which was the owner's own idea in the same message.

| Key | Does |
|---|---|
| Down, Space, Page Down | Next slide. At the last slide of the item, what waits in Preview, at its ring. With nothing live, Space starts the plan |
| Up, Page Up | The slide before; at the first slide, the last slide of the plan item before |
| Left, Home | First slide of the live item |
| Right, End | Last slide of the live item |
| Shift with an arrow | The same four moves for a ring in the live song. The projector does not change until Shift is let go, which shows the ringed slide. Esc first shows nothing |
| Ctrl with an arrow | The same four moves for the ring in Preview, which stays where it is put. Past either end it goes on into the plan item before or after. With nothing live the arrows do this without Ctrl |
| Enter | Show the slide with the ring; with the ring on the live slide, Next |
| 1 to 9, 0 | Jump to that section of the live song in sung order (0 is the tenth). With nothing live, start the open song there. Each list row shows its number |
| Esc | Put the live item in Preview |
| Tab, Shift Tab | Put the next or the previous plan item in Preview |
| Alt Down, Alt Up | Move the Preview item in the plan |
| Delete | Take the Preview item out of the plan (Undo in the toast) |
| F1, F2, F3 | Clear text, Black, Logo |
| F4 | Put what the listener last recognised in Preview. It never goes to the projector by itself, because a misheard word must not reach the room |
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

Dialogs and the two editors are different: Tab moves through their buttons and fields, and Enter or Space presses the focused button (`Btn::focusable`).

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

- Marker lines are coloured like their slides.
- Verse, Chorus, Pre-chorus, Bridge and Ending write the next free marker at the caret (Ctrl 1 to 5).
- Tidy turns headings such as "Verse 1" or "Reff:" into markers and drops chord lines (`song::tidy_lyrics`).
- The order is built from chips: a click on a chip in the order takes it out, a click on a section under it sings that section again. "Chorus after each verse" writes the usual hymn order. A star repeats: `C*2` (`song::parse_order`).
- A new song's title offers whole songs under it, as an editor offers a completion. Songs already in the library come at once and open on Tab. Songs from the web arrive 0.4 seconds after the typing stops and fill title, author and words on Tab (`artosia-lyrics`, LRCLIB). The line under the title then says where the words came from and to check them.

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
| Fill in by itself | Four choices: Manual, Bible passage, Birthdays this week, Birthdays this month. The two birthday choices write the names from the birthday list each time the card is shown, eight to a slide, and under the choice the card says which days it covers ("Covers 11 November to 17 November", or all of the month) and how many names it holds today, so a volunteer knows what will come up before showing it |
| Bible passage | The volunteer types a reference, in Indonesian or English, and the words come from the Bible in use with the reference and its code as the small print (`deck::passage_state`, `announcement_pages`). It reads the reference with `Bible::guess`, the same parser the search uses, so "Mazmur 23:1-3" and "psalm 23 1-3" both work; a bare book name is not enough. It is read again whenever the Bible changes or finishes loading. With no Bible installed, or a reference the Bible has no verse for, the card says so under the field and offers the Bibles dialog; it shows nothing on the projector meanwhile. The "Verse of the week" ready-made card starts as a passage card; choosing Manual makes it plain text again |
| Picture | A picture shown with the words |
| Import a PowerPoint | Each slide becomes an announcement: a table row becomes a line with its cells in columns, a picture slide keeps its picture (`artosia-office`) |
| Import a birthday list | A Word or PowerPoint table of names and dates replaces the birthday list; each person is kept once |

Search finds the birthday card for "birthday" and "ulang tahun" and a passage card for "verse" and "ayat" (`palette.rs`, `documents`); its detail line says how many names it holds today, or the reference. Ctrl Enter shows it at once. A card with no slide today does not show another card's slide in its place: it says so.

A list longer than ten lines goes over several slides. The church's own bulletin (6 slides) and birthday list (26 tables, 682 rows) were read on 2026-11-10: 6 announcements and 331 people after repeats were removed. Those files hold names, birthdays and gifts and stay out of the repository.

## One Artosia, its own title bar, light and dark

- **A second start hands over to the first and leaves** (`main.rs`, `claim_instance`, port 47824). The owner had two open on 2026-11-10 and the Screens dialog sat on "Starting the projector"; two operators on one library also write the same plan and talk to the same projector. Whether the second copy caused the stuck dialog was not reproduced. Headless runs, `--practice` and `--data` are exempt.
- **The top band is the window's title bar** (`rune::Titlebar`): it drags the window and the system draws its buttons at the end. The mark in it is a loaf with a baker's cuts ([brand.md](brand.md)); the same mark is on the program file and the taskbar.
- **The window follows the computer's light or dark setting**; Look has "Artosia's window" to pin one (`theme::set_dark`). Section colours are picked for dark slides, so on a light window they go through `theme::ink_of` and `theme::tint_of`.

## Two languages

The window speaks English or Indonesian (Look, Language; or type "bahasa" in the search). Section names on slides follow it: Verse 2 is Bait 2, Chorus is Reff. No language is the source the other is translated from: `docs/i18n/brief.json` says what each text has to do, and the Indonesian in `docs/i18n/id.json` was written from those briefs by a writer who never saw the English, then checked by back-translation and rendered. `strings.rs` holds both; a test fails when its Indonesian and the reviewed file differ, and another on spellings an Indonesian reader catches at once. A changed text changes its brief first.

Rendering found what the briefs could not: three toggle buttons, a three-way choice and the key sheet were too narrow for Indonesian words, and the layouts were widened or wrapped (`ui/live.rs`, `ui/announcements.rs`, `ui/sheets.rs`). `--indonesian` renders any scene in Indonesian.

## Listening and streaming

- **Listen** (top bar) opens the microphone, shows how loud the room is, and offers the passage or songs a heard sentence may have named. It only suggests. Recognition needs a build with the `whisper` feature and a model in the library's `models` folder; without them the dialog says so and the meter still works.
- **Stream** (top bar) starts `artosia-stream` beside the operator with the service's address and key, or a recording, or both, and shows its state once a second. The key is kept in the library file on this computer and is never written to a log. Closing Artosia on purpose ends the stream cleanly; a crash leaves it running, as it leaves the projector.

## Updates

Releases are GitHub Releases, made by `tools/publish.sh`; an installed copy needs no setting to find them. Full account: `crates/artosia-update/README.md`.

Artosia looks for a newer version at each start and downloads it beside the program; the files are swapped the next time it starts (`artosia-update`, `update.rs`). Every file is checked against a signature made with the release key, whose public half is built into the program. A development build and a headless run never update themselves. "Check for a new version" in the search reports what it found either way.

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
| Lyrics on the clipboard | Ctrl V in the main window, or "Paste lyrics as a song" | A new song in the editor, split into sections, the first line as the title when a blank line follows it |

Importing twice adds nothing. A song with a source id replaces the song imported earlier under that id, so a title corrected in the source library updates in place; the church's cleaned library keeps its ids stable for this.

**An edit made in Artosia wins over the library the song came from** (`Song::edited`, `Store::import_songs`, 2026-11-10). Saving a change to an imported song marks it edited, and a later import leaves it alone and says how many it kept. To take the library's version back, delete the song and import again. Played on the church library: one song edited through the editor, the library imported again, 1,333 songs and the edit still there.

A song without a source id is skipped when one with its title, author and opening words is already there, and items that only share a title all come in.

`crates/artosia-core/tests/church_library.rs` imports the church's library whenever `fixtures/church/library` is on the machine: every file must read, empty placeholders must be skipped and every alternate title in `index.json` must arrive. The import report stays on screen as a toast, including when the import is started with `--import-openlyrics`; hiding it once concealed two unreadable files, which turned out to be the library being rewritten by another session at that moment. A file that fails is reported beside what the rest of the import did, never in place of it.

## Checking a screen

`tools/snap.sh <folder>` renders every scene in `main.rs` (`SCENES`) to PNG with the practice library; `--query "<text>"` opens the search on any text; `--background <file>` puts a picture behind the text; `--script crates/artosia/scripts/type-and-show.json --at mark:live` plays a keyboard flow first. `--data <folder>` points at a real library.

Rendering a scene shows how a screen looks, and only a script shows what a key or a click does: the Save and Escape defects above passed every scene render. Played through scripts on 2026-11-10: type to search, open, show, Space, F2, click a slide; Ctrl S in the editor; Escape in the editor with and without changes; Undo from the toast; a plan row's menu; adding an announcement; "Add the next verse"; turning the phone remote on. Not yet played: the file pickers, drag and drop, the Screens dialog with two screens, the Look dialog's controls, moving a plan row, the logo.
