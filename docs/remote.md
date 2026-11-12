# The phone remote

`crates/artosia-remote` serves one web page to phones on the same Wi-Fi: the live slide, the
sections, the plan, search, a message and a countdown. The operator starts it from Phone in the
top bar (`actions.rs`, port 4780, with a PIN). What it can do and what was tried is row 8 of
[features.md](features.md).

## How the page looks (2026-11-12)

The first redraw (2026-11-11) used the operator window's colours with no reference opened, and the owner
reopened the look. Research was saved first, in `C:/Users/kenne/Shared/inspo/artosia-phone-remote/`: 29 Mobbin
screens, 18 screenshots of six church remotes, 53 files of FreeShow's and OpenLP's own web remotes, and a language
card per product. Four candidates were built as real pages on the example's data, and the owner picked **Slate
drawer**. The page copies **Proclaim Remote** (Faithlife's iOS remote), whole: its navigation, its page skeleton, its
rows and its buttons. It is dark only, as picked.

**Why this product.** The first screen has to be Back and Next under a slide that reads at arm's length, with the
rest one touch away and nothing to learn. Proclaim has no tab bar to hunt in: a top bar of a two-segment switch and
two round buttons, the slide at full width, and a drawer that pulls up for the song's parts. The other three
candidates were ProPresenter Remote (orange cards, five tabs), OpenLP Remote (flat list, header chevrons) and
FreeShow's remote (magenta, five icon-only tabs). Their pages are in git at commit `a13067a`
(`crates/artosia-remote/examples/directions/`), and the pictures of all four are in the research folder, `shots/`.

**Saved files it was built from** (in the research folder; the paths are its own):

| Part | Source |
|---|---|
| No tab bar, grouped dark round-rect buttons around a two-segment pill | `products/07-proclaim-remote-ios-2.jpg`, `languages/proclaim-remote.md` |
| The slide full width on a dark field, a red ring while it is live | `products/07-proclaim-remote-ios-2.jpg`, `products/10-proclaim-remote-ios-5.jpg` |
| The drawer with a grab handle, labelled thumbnails, two columns when pulled up | `products/07-proclaim-remote-ios-2.jpg`, `products/08-proclaim-remote-ios-3.jpg` |
| The plan as headed groups of rows with a picture, a title, a line and two stacked pills; the round green button | `products/06-proclaim-remote-ios-1.jpg` |
| The PIN as dark code cells, a filled Connect | `mobbin/xbox-code.jpg` |
| The lost-connection banner, a bold line and one grey one | `mobbin/alta-offline.jpg` |
| The countdown: a minus and a plus around the figure, a wide Start | `mobbin/opal-timer.jpg` |

Proclaim is an iOS app and only screenshots of it exist, so no source file was ported: the page is drawn from the
screenshots, and the code is this repository's.

**Hand-built, with no source in the saved files.** Back and Next as two large buttons side by side under the slide
(Proclaim steps by touching the slide). The four state buttons in one grouped row. The section buttons in the drawer
(Proclaim's drawer holds only thumbnails). The message, countdown and language blocks, and the keys block that only a
computer with a keyboard shows. The countdown pill on the live screen. The kind line under a plan item's title. The
unit word beside the minutes box.

**What differs from Proclaim, and why.**

| Source | Page | Why |
|---|---|---|
| A yellow ring on the current slide | Blue in the drawer, red while the room sees it, yellow while something keeps it away | Red is what the room sees and yellow is hidden or a warning; yellow for the current slide would say hidden |
| ON AIR and OFF AIR in tracked capitals | The live item carries a red Live pill; headings are the plan's own, in sentence case | No uppercase letter-spaced labels; the plan keeps its order |
| A green LOCAL chip, then Follow over Control | A red Live pill, then Show over Not now on the picked row | The same stacking, with the words a volunteer needs |
| Edit and Preview in the top segment | Live and Plan | The two things the page does first |
| Thumbnails only in the drawer | Section buttons above the thumbnails | A song's parts are jumped to by name |
| A disabled filled button dimmed | Grey with grey words | A dimmed blue still reads as pressable in a dim hall |

Red appears only for what the room sees: the ring and pill of the slide, the thumbnail of the live slide, the live row
of the plan and the Live state button. Yellow is hidden or a warning: the state pill and ring while the words are cleared,
the screen is black, the logo shows or the projector is off, the lit Clear text, Black and Logo buttons, the Hide button
while a message shows, the countdown pill, the lost-connection banner and the PIN error. A chosen state is a fill.

Every string is in `STR`. Indonesian keeps the table of the first redraw; the two keys that are new, `pin_hint` and
`cancel`, say English until a writer who has the briefs in `docs/i18n/brief-phone.json` writes them. Three keys of the
first redraw were dropped with the cards they labelled (`next`, `next_item`, `plan_pick`), and `pin` is now only the name
a screen reader gives the four cells.

Checked on the example server on this computer, dark, at 390x844 and 320x640 in English, and for fit also at 360, 375
and 414 wide in English and in Indonesian. A script pressed every control: Back, Next, a section button, a thumbnail,
the keys Space and B, the four state buttons and their pills, the pull-up drawer, a plan item picked, shown and put back,
search, Show and Add from a result, the plus on the plan, the message, the minutes steppers, the countdown with its pill
on the live screen, the language switch, a wrong PIN, and the lost-connection banner after a failed command. A second
script found no word wider or taller than its box. Not tried: a real phone.

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

## Why Windows Firewall kept asking (2026-11-12)

The owner saw a Windows box with a shield for almost every action. Windows' own log
(`Microsoft-Windows-Windows Firewall With Advanced Security/Firewall`, events 2004 and 2097)
named the programs: 13 questions between 2026-11-10 and 2026-11-11, every one from Artosia's
own files and none from another cause.

| Program asked about | Times | Why it was a new program each time |
|---|---|---|
| The remote's test (`remote-<hash>.exe`) | 8 | Cargo names the test file by a hash, and it changed with the build; three were in agents' own build folders |
| The remote's example (`serve.exe`) | 2 | It listened on the network until 2026-11-11 |
| Copies of `artosia.exe` | 4 | One each in `target/debug`, `target/try`, `target/app` and `target/mine`, started on a library with the phone remote on |

Windows asks when a program opens a socket that other computers can reach, and it keeps its
answer per program file. So the fixes are about never opening such a socket outside the one
real use, and never making new program files that do:

- A test or an example starts the server with `local_only`, which listens on this computer
  only (2026-11-11).
- `RemoteServer::urls` of such a server returns this computer's address and never calls
  `net::lan_ips`, whose first step binds a socket on every network to learn which adapter
  leads out. The remote's own test called it. The test
  `a_local_server_has_only_this_computers_address` holds this (2026-11-12).
- The installed program is one file in one folder, and its self-update keeps the folder, so
  a church is asked once: when the phone remote is first switched on. Allow must be chosen
  there, or no phone can reach it.
- A trial copy is never started from a new folder on a library whose phone remote is on.

Every one of the 13 questions was answered with a block, which Windows keeps: the copies in
`target/try`, `target/debug`, `target/app` and `target/mine` cannot be reached by a phone
until those rules are removed in Windows' firewall settings, which is the owner's to do.

