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

Every string is in `STR`. The page speaks the ten languages ([i18n/README.md](i18n/README.md)), each a table
`docs/i18n/phone-<code>.json` written from `docs/i18n/brief-phone.json`. A phone table has the shape of an operator
table and is flattened when the page is served (`flat` in `crates/artosia-remote/src/languages.rs`). The languages are
listed in one picker, a native select, where a row of buttons stood. The names of a song's parts are built from the
table's own keys (`label.verse` and its neighbours, `label.numbered`) by `labelRules` in `page.html`. The page picks its
first language from the phone's own list with `fit`, the JavaScript copy of the rule in `crates/artosia-locale`. Three
keys of the first redraw were dropped with the cards they labelled (`next`, `next_item`, `plan_pick`), and `pin` is now
only the name a screen reader gives the four cells.

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
off. Since 2026-11-12 the operator does not leave the question to Windows: see the next section.

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

What Microsoft says (learn.microsoft.com, "Windows Firewall rules", read 2026-11-12): a
program that issues a listen call with no rule for it makes a dialog ask "the first time the
app is launched or tries to communicate in the network"; a rule names "the full path to the
application"; when the person answers No or cancels, "block rules are created", two as a
rule, one for TCP and one for UDP; and "once the rules are added, they must be deleted to
generate the prompt again". A person who is not an administrator gets block rules whatever
they press. The page does not say whether binding a UDP socket alone counts as listening, and
the log does not say which socket raised each question, so the UDP socket below is a
possible cause that was removed, not a proven one.

So the fixes are about never opening a socket other computers can reach outside the one real
use, and never making new program files that do:

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
until those rules are deleted. Deleting them changes Windows' security settings and needs an
administrator, so it is the owner's to do and no session does it. The installed program sits
in a folder of its own with no rule yet, so the blocks do not touch it.

## The program's own firewall rule (2026-11-12, FW1)

Windows saves a block when a person cancels its box, and for a person who is not an administrator
whatever they press; a block beats an allow rule and is never asked again (Microsoft, "Windows
Firewall rules"). So the first time Phone is switched on, `crates/artosia/src/firewall.rs` does
what an installer would: it checks, without elevation, for a rule named `Artosia phone remote`
for this program's own path and port 4780 (`netsh advfirewall firewall show rule ... verbose`,
read by its values, never its translated labels). If it is missing, one elevated `cmd.exe`
(the `runas` verb, hidden) deletes every rule for this path (the saved blocks) and adds one
inbound allow rule: TCP, port 4780, every profile, remote address `localsubnet`. The work waits
on a worker thread and the remote starts afterwards whether or not the rule was added. The
Phone dialog says, before Windows' box, that Windows will ask for permission once (a standard
user gets a password box and no Yes button, so the line names neither); if the rule is still
missing afterwards it shows a warning line, and switching Phone off and on asks again.

It never runs in practice, in a headless run, in a test, for a server that listens on this
computer only, or off Windows, and it never asks at start: when Artosia starts with the phone
remote saved as on, the remote starts as before, only the check runs and a missing rule shows
the same warning line. The tests use an injected runner; the elevated command was never run on
the development PC. Everything unconfirmed (the name the elevation box prints, the effect of the
delete on single-profile blocks, a non-English Windows) is listed in
`docs/plans/artosia/firewall.md` and `~/Shared/research/artosia-firewall/notes.md`.

## Editing from a phone (PHONEEDIT1, 2026-11-14)

A paired phone is Control; the Phone dialog gives one phone Edit (a Control/Edit switch per row) and can show a
second PIN that pairs a phone as Edit. Switching Phone off drops the server, so every level and the second PIN go.
Only an Edit phone is sent `cards`, `people` (no birth year), `events` of the next seven days, `live_card` and
`locked`; the server refuses every `edit` command from a Control token (403). A revision is a 48-bit hash of the
fields a phone edits (`edit.rs` `revision`), so any change from any route moves it and nothing is stored: a stale one
is refused with 409 and the current fields, and one the console's editor holds unsaved is refused with 409 `console`.
The server's copy takes an accepted edit at once, so a second phone with the old revision is refused before the
computer answers, and a Done sent twice lands once. The computer checks the revision again (`phone_edit.rs`).
A repeating event's edit is an override of that date (`Change::Moved`); a once-only event is changed itself.
Run a practice window with `--practice --remote-local` to try it: the server then listens on this computer only.
