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
