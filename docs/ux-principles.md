# What the window is built on

The owner tested the window on 2026-11-11 and wrote: "You're not making an app for another agent reading binary/source code. You're making an app for HUMANS." These are the rules the window follows since, each with the source it came from and what it changed. The research notes with every page read are in `C:\Users\kenne\Shared\inspo\artosia-presenter-ui\ux-principles\` and `feature-audit\`.

## Buttons

| Rule | Source | In Artosia |
|---|---|---|
| Buttons of one set are one size; the preferred one differs by style | Apple, Buttons: "Use style, not size, to visually distinguish the preferred choice among multiple options" | The strip under each list is a grid of tiles (`ui/widgets.rs` `Tile`). Next and Prev are the same size; Next is filled |
| A button's size never comes from its words | The owner: "Inconsistent button sizing due to different word length on different languages bad" | A tile's width is its share of the strip (`ui/shell.rs` `Foot`), so English and Indonesian draw the same strip |
| A label is one word | Microsoft, Command bar: "Button labels should be short, preferably a single word" | Tile labels have a budget of 8 characters in `docs/i18n/brief.json` |
| Every button carries an icon | The owner: "Many buttons not have icons"; Apple, Toolbars: "Prefer simple, recognizable symbols" | Tiles, the tools in a pane's title and the top bar all carry one |
| A state is shown by how the control looks, and named by what is then true | The owner: "it can be represented by the status with different display style if on/off" | No text, Black, Logo and Live are lit while on. None is named by a verb |
| One prominent button per view | Apple, Buttons: "Keep the number of prominent buttons to one or two per view" | Show under Preview, Next under Live |
| A hit area of 44 points where it can be | Apple, Buttons: "a hit region of at least 44x44 pt" | Tiles are 56 to 92 points high (`Foot::tile`) |

## Keys

| Rule | Source | In Artosia |
|---|---|---|
| A key is printed on the control it presses | The owner: a list at the bottom "doesn't really help", a key on the row does | Each tile has its key in a corner; the row Next will show carries the space bar; the ringed row carries Enter |
| A key is drawn as the keyboard prints it | The owner: "show the exact thing showing on the keyboard: the arrow icons" | `keycaps` draws arrows as arrows and the space bar as a bar; key names are never translated |
| A single letter can be switched off | WCAG 2.1.4, Character Key Shortcuts | Letters jump to sections (C, V, B); Settings has the switch |
| A held key shows what it changed, while it is held | Raskin's quasimodes, in the research notes | With Ctrl held the Preview pane is ringed and the bar at the bottom names the keys; it is empty otherwise |

## Preview and Live

| Rule | Source | In Artosia |
|---|---|---|
| Preparing and showing are apart | The owner: "Preview is supposed to be the safe haven" | A click in Preview, on a plan row or on a search result never reaches the projector |
| Sending to the Live list and switching the projector on are two things | The owner: "Show should not be same as live"; EasyWorship: "To turn off the live output, click the LIVE button" | Show moves Preview into the Live list. The Live tile (F6) decides whether the room sees it, and starts on at every launch |
| A colour is never the only sign | Apple's accessibility guidance, in the research notes | The live row is red and carries a sign; a hidden one is yellow with the sign of what hides it |
| Going back is one visible step | The owner: a volunteer who pressed Next twice did not know how to return | The Live title lists what was shown before; choosing one brings it into Preview where it was left |

## Words

| Rule | Source | In Artosia |
|---|---|---|
| A mark replaces a word where it says the same | The owner: "Stop forcing words on everything like the live banner/marker on the plan item" | The live plan item has a red dot where its menu button is; the button returns under the pointer |
| No loose captions | The owner: "stop using plaintext like 'Then: xyz'" | What comes next is the title on the tile that fetches it; a picture wears its section as a chip |
| A warning keeps its words | Microsoft and GNOME on tooltips, in the research notes | Black, No text and Off are written beside their sign in the Live title |

## Where things are

| Rule | Source | In Artosia |
|---|---|---|
| Settings hold what is chosen once, never the work itself | Microsoft, App settings: "Commands that are part of the typical app workflow ... shouldn't be in a settings page"; Apple, Settings: "general settings that affect your overall experience" | The old Look dialog held both. Design has the background, the words and the logo; Settings has the language, the window, the Bibles, the keys and help |
| Settings come last | Microsoft, App settings: the entry point is "one of the last items" | Settings is the last button of the top bar, and Ctrl comma opens it |
| A toolbar has at most three groups | Apple, Toolbars: "aim for a maximum of three" | Design; Screens, Phone, Stream, Listen; Settings |
| A control sits beside what it acts on | Apple, Buttons, on square buttons: "in close proximity to their associated view" | Each strip is under its own list; a row's action is on the row |

## Teaching

| Rule | Source | In Artosia |
|---|---|---|
| A tour is short, skippable and can be taken again | Apple and Nielsen Norman Group on onboarding, in the research notes | Five stops over the real window (`ui/tour.rs`), Skip on each, started again from Settings, the keys sheet, the search and the first screen |
| A tester who sees only pictures must not get lost | The owner: "If they get confused anywhere, it's a fault on you" | `tools/blind.py` drives a headless Artosia by keys or by clicks and shows only screenshots |

## Not done

- An interface size setting up to 200 percent (WCAG 1.4.4): Rune's scale was not looked into.
- A key-badge overlay on holding Alt (Microsoft Key Tips): every key is printed on its control instead.
