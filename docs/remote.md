# The phone remote

`crates/artosia-remote` serves one web page to phones on the same Wi-Fi: the live slide, the
sections, the plan, search, a message and a countdown. The operator starts it from Phone in the
top bar (`actions.rs`, port 4780, with a PIN). What it can do and what was tried is row 8 of
[features.md](features.md).

## How the page looks (2026-11-11)

The owner on the page as it was: "it looks so ugly make it UIUX perfection and professional
mature app with mature style, maybe matching the Rune". The page had its own black and grey
palette, a blue fill for the live slide where the window uses red, text-only tabs and four
grey blocks for the states. It is now drawn from the operator window's own tokens and means
the same things by the same colours.

| What | Now | From |
|---|---|---|
| Surfaces | The window's tones for bars, page, cards and controls, hairlines between them, in dark and in light after the phone's own setting | `crates/artosia/src/theme.rs` `surf_for`, `dark_seed`, `light_seed` |
| Red | Only what the room sees: the frame and pill of the live card, the live row of the list, the Live tile, the pill on the plan's live row | `theme::live` |
| Yellow | Hidden or a warning: the lit No text, Black and Logo tiles, the pill that says why the room does not see the words, the live row while it is kept from the room, the countdown, the lost-connection bar | `theme::warn`; "a live slide kept off the projector is yellow, never red" |
| States | Four equal tiles in one row, an icon over each word, lit while what they name is true; a long word takes two lines | the window's dock tiles |
| Sections | Named in full on a chip in the section's own colour, in one row that slides sideways, and on each row of the list | the window's `section_chip` |
| Slides | A numbered row: number, section chip, words | the window's Live list |
| Plan | A kind's icon on each row, a heading as words and a hairline, the picked row in the accent | the window's plan column |
| Tabs | An icon over each word; the open one is a fill | house rule: a selected state is a fill, never an edge bar |
| Icons | Sixteen drawn in the page as one hidden sprite; the page still loads nothing from anywhere | `tests/remote.rs` `page_is_self_contained_and_in_house_punctuation` |

Checked in the app's browser pane against the example server on this computer, at 320x568,
375x812 and 414x896, dark and light, English and Indonesian, on all four tabs and the pairing
screen: a script measured every button, tile, tab, pill and chip for words wider or taller
than their box, and found none. Two were found and fixed on the way: "Mundur" was cut in the
Back button, and "Bahasa Indonesia" in the language choice at 320 points. On a phone lower
than 700 points the card of what comes next gives way, so the live words stay whole. Every
token pair for words on a surface reads at 4.5 to 1 or more (computed: the lowest is the faint
grey on a dark card, 4.9). Not tried: a real phone.

To look at it: `.claude/launch.json` starts the example server on this computer only
(`cargo run -p artosia-remote --example serve 47931`), and the page opens with `?pin=1234`.

## Where it listens, and the firewall's question (2026-11-11)

`RemoteServer::start` listens on every network address of the computer, because a phone has to
reach it. The first time a program does that, Windows shows a Windows Security box: "Do you want
to allow public and private networks to access this app?", with Allow and Cancel. The box stays
on the screen, over everything, until a person answers it, and while it is up it takes every
real key press, click and turn of the wheel.

The example server (`cargo run -p artosia-remote --example serve`) listened the same way. Two
test runs on 2026-11-11 each raised that box for `serve.exe`, at 10:57 and at 15:51, and both
were still on the owner's screen that evening. Nobody was there to answer them, and an agent
must not: allowing a program through the firewall is a security decision.

So `RemoteConfig` has `local_only`. With it the server listens on 127.0.0.1, which only this
computer can reach. The tests and the example set it; the example listens on the network only
with `--lan`, for trying a real phone, and says so in its first line. The operator leaves it
off, so the first time Phone is switched on in a new copy of Artosia the owner will be asked
once for `artosia.exe`, and Allow is the answer that lets a phone in.

Not confirmed from Microsoft's own pages: that a listener on 127.0.0.1 never raises the box.
A search found only user reports saying so (a Stack Overflow thread, "avoid windows firewall
popup with sockets on localhost", and others). On this PC the remote's sixteen tests ran on
127.0.0.1 and no new box appeared.

The box's own window is a proxy of zero size owned by a process named PickerHost; the box that
is drawn belongs to the system. Measuring that window says nothing is there. A picture of the
screen is the only check.
