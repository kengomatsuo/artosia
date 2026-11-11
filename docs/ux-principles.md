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
| A key is printed on the control it presses | The owner: a list at the bottom "doesn't really help", a key on the row does | Each tile has its key in a corner; the row Next will show carries the space bar, and at a song's last slide that is the ringed row of Preview; the ringed row carries Enter; the plan row Tab opens carries Tab; Tab sits beside the switch of the search |
| A key is drawn as the keyboard prints it | The owner: "show the exact thing showing on the keyboard: the arrow icons" | `keycaps` draws arrows as arrows and the space bar as a bar; key names are never translated |
| A single letter can be switched off | WCAG 2.1.4, Character Key Shortcuts | Letters jump to sections (C, V, B); Settings has the switch |
| Everything a pointer can do, keys can do | Microsoft, Keyboard accessibility: "Any UI that can be activated by pointer should also be invokable by keyboard" | Every action of a menu is also a command in search (copy a plan, rename it, delete it, start a song at a slide, a new announcement); the keys sheet ends with a line that says so |
| A focused button takes Enter and Space | Microsoft, the same page: "For controls that can be invoked, implement keyboard event handlers for the Spacebar and Enter keys"; WAI-ARIA button pattern: "Enter: Activates the button" | `enter_key` on every element that asks for focus; Rune alone follows AppKit, where Return belongs to the default button |
| A list is one Tab stop, and arrows move inside it | Microsoft, the same page: "keep the composite root in tab order and manage active descendants internally, rather than exposing every child as a separate tab stop" | The cards of the announcements and the row of examples are focus groups; a keys-only tester needed fourteen Tab presses to pass four cards before |
| The keys between tabs go between plans | Microsoft, Keyboard shortcuts in Windows: "Ctrl+Tab: Move to the next tab" and "Ctrl+Shift+Tab: Move to the previous tab" | Ctrl Tab and Ctrl Shift Tab open the next and the previous plan; the keys are drawn at the list of plans |
| The key that goes back is Backspace | Microsoft, Keyboard shortcuts in Windows: File Explorer "Alt+Left arrow / Backspace: Navigate to the previous folder", Settings "Backspace: Navigate backwards" | Backspace brings what was on the projector before into Preview; the Earlier button prints it |
| One chord does one thing in every pane | Third keys-only test: Shift Enter added to the plan in the search and showed the song in Preview | Shift Enter adds what Preview holds to the plan and shows nothing |
| A choice of one follows the arrows | Rune at the pinned rev, `El::focus_follows`: "Radio and tab items always do" | The colour squares of Design choose as the arrows move, as its switches do |
| A held key shows what it changed, while it is held | Raskin's quasimodes, in the research notes | With Ctrl held the Preview pane is ringed and the bar at the bottom names the keys; it is empty otherwise |

## Names

| Rule | Source | In Artosia |
|---|---|---|
| A section is named in full, never by a code | Two native readers, each on their own: a volunteer who reads Bait and Reff everywhere "never learns that C means Reff" | Chips read Verse 1, Chorus, Bait 1, Reff on every list, picture and menu; the letters stay only as keys, drawn as key caps beside the name, and as the typed song order |
| What the program writes into a text, it writes in the window's language | The same readers: the button said Reff and `[chorus]` appeared | Markers are written with the language's section names and read back in any language (`song::set_marker_words`) |
| A reason is on the row, in words | House rule: hover is never the sole carrier of information | A suggestion says why after its title; its button has a word beside its arrow |

## Dialogs

| Rule | Source | In Artosia |
|---|---|---|
| No dialog ends in half a row | Both readers took a row cut by the fold for a defect | Design shows one part at a time (background, words, logo); Settings fits a 768 point window; the keys sheet scrolls under a title that stays |
| What a pane holds is never cut to make room for buttons | Fourth blind test: "Q..." for a song's title on the target screen | The title has a line of its own under the pane's name (`ui/shell.rs` `title_bar`) |
| What a person switches between is in view, never behind a title | The owner asked where the plan picker was; EasiSlides shows its worship list's box at the list, ProPresenter and FreeShow list playlists and projects beside the items | Every plan is a row under the plan, one press apart, with its own menu; the title drops nothing down |
| What can be picked is never typed | The owner: "you love textfields too much"; Microsoft, Guidelines for date and time controls: a time picker "to pick a single time value"; NN/g, Date-Input Form Fields: pickers for "events close to the present time" | A plan's day, hour and minute are chips; only its optional name is a field |
| A heading looks unlike what it heads | The owner could not tell Pujian and Firman from the songs under them | A plan divider is a name with a rule and room above it, its items set in under it |
| Cancel leaves nothing behind | Found while pressing the plus: an empty "New plan" stayed after Cancel | A new plan exists only once Save is pressed |
| A dialog opens on what the person came for, never on its close button | House rule on dialog focus; Rune at the pinned rev picks "its `El::autofocus` control, else ... its first Tab stop", and the first stop is the close button | Design opens on the look being changed and, back from the gallery, on the gallery's button; Phone and Listen on their switch; Stream on the address, or on Start once there is one; Settings on its first row (`Tx::focus`, because a kit switch of segments cannot be named) |
| What scrolls fades at the edge that hides more | The same finding, where scrolling cannot be avoided | `scroll_edge_fade` on every scrolling area (`theme::size::FADE`) |

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
