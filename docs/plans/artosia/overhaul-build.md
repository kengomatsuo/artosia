# Building the overhaul (UIOVER1), card by card

The owner picked direction B (the church's data in its own windows) with direction A's rundown on
2026-11-14, from the scenes `mock-b-p1` to `mock-b-p7` and `mock-a-p7`. Each card below ports its
page of `crates/artosia/src/ui/mock_b.rs` into a real screen. Research: `~/Shared/inspo/artosia-ui-overhaul/`.

## OVSHELL1: the tab bar

```
SOURCES  mock_b.rs frame() (Planning Center's product tabs, ChurchSuite's module bar, per directions.md)
         FreeShow src/frontend/utils/shortcuts.ts: number keys change its top tabs; Artosia's digits jump
         inside the live song, so Ctrl 1 to 7 change tabs (Chrome and Edge's tab keys on Windows)
BASE     a row of pills, one per window of the program; Tab is the program's top level
BOUNDARY state.rs owns Tab and App.tab; actions.rs open_tab is the one way in; ui/tabs.rs draws;
         ui/mod.rs view routes; main.rs on_key maps Ctrl+digit
SCREEN   under the title bar, every start on Rencana ibadah (never saved: a service starts on the plan)
TREE     tabs bar: pill x7 (icon, name, keycap while Ctrl is held), each a button with help "Ctrl N"
ROUTE    Plan: today's window (shell::view) · Notices: the announcement editor beside Live ·
         Calendar, People, Giving, Templates: their windows (stubs until OVCAL1, OVPEOPLE1, OVGIVING1, OVTEMPL1) ·
         Look: the Design dialog's content as a page beside a large sample slide
WORDS    seven tab names; Design keeps BTN_LOOK, Announcements keeps CMD_ANNOUNCEMENTS
ACTION   press a pill or Ctrl+N: open_tab(tab); Notices with no announcement open opens the first;
         Look closes any sheet; the palette closes
         every caller that opened Sheet::Looks opens Tab::Look; the Look button leaves the title bar
KEYS     Ctrl PgDn and Ctrl PgUp walk the tabs; Ctrl and a digit stay the Preview ring's
EDGES    1366 wide in Indonesian: seven pills fit with room; dark and light
```

## OVCOLOR1: one colour picker

```
SOURCES  mock_b.rs p2; directions.md P2 (ProPresenter text fill, Adobe Express fill colour, Planning
         Center colour themes, FreeShow MaterialColorInput.svelte, Canva text colour panel)
BASE     ui/colour.rs: ColourFor names what is coloured; one picker, one state (App.colour), every
         colour of the program a ColourFor variant (calendar, title roles and item kinds join later)
TREE     row: the usual colours as squares, More (rainbow or the colour in use) · panel under the row:
         head (colour, name, Cancel Esc, Use colour), In this design, Samples (4 x 10 OKLCH),
         Your own (saturation square, hue strip, dropper), Used lately (8, in settings), Colour code
         behind its disclosure
ACTION   a pick applies at once so the slide shows it; Cancel or Esc puts back the colour it opened
         with; Use keeps it and records it as used lately; a drag on the square or strip moves the
         colour; arrows move both; the dropper's next press on the Design slide takes its colour
         (solid, gradient, or the picture decoded at 320 points; none for a video or camera)
EDGES    the panel opens in the scrolling column, never over the slide; 620 wide in ten languages
```

## OVCAL1: Kalender

```
SOURCES  mock_b.rs p3 and repeat_dialog; directions.md P3 (Planning Center Calendar multiple calendars and
         colours, ChurchSuite event series, ProPresenter calendar scheduling, FreeShow EditEvent.svelte,
         Luma's weekly times on Mobbin); RFC 5545 RRULE for the rule shapes (weekly BYDAY, monthly BYDAY
         with a position, UNTIL, EXDATE)
BASE     artosia-core/src/calendar.rs: Date (civil date, weekday, days in month with leap years),
         Calendar (a named, coloured group), Event (name, calendar, first date, start and end minute,
         Repeat, until, skipped dates), Repeat = Once | Weekly(day mask) | Monthly(position 1..4 or last,
         weekday); occurrences(event, from, to) is the one place dates are worked out
STORE    tables calendars(id, name, colour) and events(...), added by the existing schema path; CRUD on Store
SLIDE    Fill::ScheduleWeek: the seven days from today, one row per occurrence (day, time, name), the day
         written once per day (an empty first cell, which NoticeBody already draws as the same day);
         announcement_pages reads events; Today gains the year
SCREEN   the Calendar tab: rail (calendars with colour and a tick that hides them, Add, the schedule slide
         as the announcement reads it), toolbar (Today, back, forward, month and year, Month or Week, New
         event), month grid Sunday first with today ringed and coloured chips (time and name), a week view
         of seven columns
DIALOG   new or edit event: name (a text, a name has no other shape), calendar (drop-down with colours),
         repeat (Once, Every week, Every month), day toggles for a weekly event, position and weekday for a
         monthly one, start and end time from a list in 15-minute steps, first date and until from a month
         picker, the rule written as a sentence, the next dates with an x to skip one, Save, Delete, Cancel
COLOUR   a calendar's colour is ColourFor::Calendar(id) in the one picker
ACTION   press a chip: the dialog on that event · double press an empty day: a new event on that day ·
         Ctrl N on the Calendar tab: a new event · Esc closes the dialog
EDGES    no calendars yet (the practice library has made-up ones; a new library gets one named for
         services) · a day with more chips than fit (+N more) · a month of six weeks · 31st monthly
         impossible (position rules only) · leap February · a week across New Year
```

## OVPEOPLE1: Jemaat

```
SOURCES  mock_b.rs p4; directions.md P4 (Planning Center People lists, ChurchSuite address book, Mobbin
         tables); rune-kit table.rs (virtualised rows, TableState in the app, sort and selection)
BASE     Person gains an optional birth year (0 unknown) and a group the age gives (child under 13,
         youth under 25, adult); the people table keeps id, name, day, month, year; the birthday
         list already read from docx and pptx (artosia-office) fills it
SCREEN   the Members tab: title and a count sentence, a strip of twelve months each with its count
         (the chosen month lit, a press filters), filters (a name search, the birth month chip with a
         clear x), Import and Add person; the rune-kit Table (name with initials, birth date, next
         birthday with "today" or "in N days", group chip, age, a row menu), sorted by next birthday;
         a side panel: the birthday slide as the announcement reads it, Open template, and Import list
ADD/EDIT a dialog: name (typed), birth date from day, month and year pickers (year optional); Delete
         with undo in a toast
IMPORT   a file from docx, pptx, xlsx or csv; a column step maps each column (name, date, day, month,
         year, ignore) with the first rows shown, guessed from the headers and the values; replace
         or add to the list
SLIDE    the birthday card reads this table as it does today; 30 names fit one slide in three columns
EDGES    nobody yet (an empty state with Import and Add) · 600 people (virtual rows) · a month with none
         · 29 February · names longer than the column (truncated, full name in the tooltip)
```

## OVGIVING1: Persembahan

```
SOURCES  mock_b.rs p5; directions.md P5 (Planning Center Giving batches and funds, ChurchSuite giving,
         a bank statement's layout: amounts right-aligned, the total under them)
BASE     artosia-core giving.rs: Fund (name, colour), Gift (date, giver or empty for the plate, fund,
         cash or transfer, amount in whole rupiah as i64), the account (bank, number, holder, note)
         kept once in the library; totals are always summed, never stored
SCREEN   the Offerings tab: title and the service it covers, filters (date range from a month picker,
         fund, method), the count and the sum; a table (date, giver, fund chip, method, amount right
         aligned, delete), Add row, the total row; a side panel: the money slide as the announcement
         reads it, the sum per fund, and the transfer account
EDIT     an amount is edited in place in its cell (digits only, grouped as rupiah when it leaves the
         field); date from a month picker; fund and method from drop-downs; giver typed or Plate
SLIDES   Fill::Offerings: the funds with their sums and the total of the last service's date or the
         chosen range; Fill::Transfer: bank, number, holder and note from the account
EDGES    no gifts (empty state) · a fund with nothing · 12-digit sums · a gift without a giver

BUILD PLAN (builder, 2026-11-14)
BOUNDARY artosia-core giving.rs (no UI: Fund, Gift, Method, Account, rupiah(), parse_amount(), sums, Filter,
         offering_lines) <- store.rs (funds, gifts tables; the account is one setting) <- deck.rs
         (Fill::Offerings, Fill::Transfer read a `Giving` bundle through announcement_pages) <- artosia
         giving.rs (Window state, buttons) <- ui/giving.rs (draws). Month picker: ui/calendar.rs
         `month_grid`, taken out of the event dialog's picker and used by both.
STATE    funds, gifts, account: App (store is the owner, reloaded after each write) · filter, sort, form,
         amount edit, open list: giving::Window · view (indices into gifts): derived by refresh_giving ·
         totals and sums: derived in render, never stored.
ACTION   press an amount: that row's amount edit opens focused, digits only; each key writes the gift and
         refreshes decks; leaving the field shows rupiah · Add gift: dialog (date from the month picker,
         giver typed or Plate, fund and method from lists, amount) · row press or pencil: same dialog ·
         trash: gift removed, Undo in the toast · Funds: dialog like the calendars dialog, colour is
         ColourFor::Fund(id) · Edit account: dialog of four typed fields.
SLIDES   Fill::Offerings -> NoticeKind::Money rows (fund, rupiah) plus "== Total"; the date is the latest
         date in the gifts. Fill::Transfer -> NoticeKind::Account lines bank, number, holder, note.
         The range chosen on the Offerings tab filters the table only; a card holding its own range waits
         for the template data-source panel (OVTEMPL1).
TIMING   no animation added; the amount edit and the sums are plain re-renders. Per-event path measured:
         filtering and summing 5000 gifts in jest-style test (giving.rs test).
```

## OVTEMPL1: announcement templates

```
SOURCES  mock_b.rs p1 (template editor) and p6 (gallery); artosia-slides notice.rs (Hairline, Ledger,
         Band drawn for five kinds, demo scenes only until now); Planning Center's publishing colour
         themes; ProPresenter's theme gallery
BASE     an announcement card gains a template (Schedule, Birthdays, Offerings, Transfer, Plain) and a
         style (Hairline, Ledger, Band); Fill becomes the template's data source, so a card's words come
         from its window and only Plain holds typed words; its title colour is ColourFor::NoticeTitle
SCREEN   the Announcements tab becomes p1: the card list and templates on the left, the slide large in
         the middle with what it reads ("Read from Calendar, 6 events"), the data source panel on the
         right (which calendars, this week or this month, seconds on screen, title, title colour)
GALLERY  the Templates tab is p6: a rail of kinds and styles, live tiles drawn with this church's own
         data, Use template adds a card; apply to this slide, every announcement, or this plan
EDGES    a template whose window is empty says so on its tile and in the panel, never an empty slide ·
         the practice library fills every template · Indonesian words longest
```

## OVRUN1: rundown items for video, audio and scenes

```
SOURCES  mock_a.rs p7; ~/Shared/research/artosia-scenes-pptx-video/README.md (FreeShow nextAfterMedia,
         loop and muted per item, fadeoutAllPlayingAudio under an audible video; OBS restart on show
         and nothing when ended; ProPresenter Looks and macros; Quelea's playbin end of stream)
BASE     PlanEntry gains Video { path, end: Stop | Loop | Next, volume }, Audio { path, end, volume } and
         Scene { name, layers (background, video, music, logo), clears on its end }; the end action
         belongs to the item; OutputState carries the item's media; artosia-output plays it with
         artosia-video's looper unmuted (video) or an audio-only playbin, reports MediaEnded, and lowers
         playing music to a third while a video with sound plays
SCREEN   the plan list draws A's rows: a colour per kind, the end action as three small toggles, a
         volume slider, a scene expanded into its layers, what it shows and what it clears
VERIFY   RAM with tools/ram/measure.ps1 while a video item plays, only after its playback position is
         logged wrapping twice (CLAUDE.md); a stop item ends on black, a next item shows the next plan
         item, on the real projector
EDGES    a missing file (the row says so, Show does nothing) · a build without video (rows still edit,
         Show says video needs the full edition) · two audible items (the later one wins)
```

## OVPPTX1: PowerPoint slides as made

```
SOURCES  research README section 1 (ProPresenter: editable or images; FreeShow: redraw, PDF via
         LibreOffice then PowerPoint, live; OpenLP and Quelea through PowerPoint COM); tools/pptx-slides.ps1
         (PowerPoint COM Slide.Export works on this PC)
BASE     an import offers both: As made (each slide a picture, exported by the installed PowerPoint, else
         LibreOffice's soffice --convert-to, else a sentence saying neither is installed) and Redraw
         (today's text import); a redraw maps a birthday table to Members and a money table to Offerings
SCREEN   the import dialog shows the file's slides as thumbnails with the two choices side by side
VERIFY   a made-up Warta-shaped file built by the tests both ways; the church's own file from fixtures/
         rendered locally and never committed
```
