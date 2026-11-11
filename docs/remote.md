# The phone remote

`crates/artosia-remote` serves one web page to phones on the same Wi-Fi: the live slide, the
sections, the plan, search, a message and a countdown. The operator starts it from Phone in the
top bar (`actions.rs`, port 4780, with a PIN). What it can do and what was tried is row 8 of
[features.md](features.md).

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
