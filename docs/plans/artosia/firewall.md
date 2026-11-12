# FW1: the program adds its own Windows Firewall rule (plan, written before any code; nothing is built)

Sources are saved in `C:\Users\kenne\Shared\research\artosia-firewall\` with `notes.md` (permalinks, quotes, commits). Anything not read from a fetched source is marked GUESS.

## Built 2026-11-12 (way 1), decisions taken

- `windows` 0.62 added to `crates/artosia/Cargo.toml` (Windows only) instead of hand-written FFI.
- Elevation only on the person's press of the Phone switch; at start with the remote saved as on, the remote starts as before, only the read-only check runs, and a missing rule shows the warning line.
- Line before the box: "Windows will ask for permission once, so phones can connect" (true for a Yes box and a password box).
- New keys carry the English again as their Indonesian argument until a writer fills `id.json`.

## SOURCES

| need | read | taken |
|---|---|---|
| why Windows' own question fails | Microsoft "Windows Firewall rules" (2026-10-08) | a Cancel, or any answer by a non-admin, saves a block; a block wins over an allow; it asks again only after the rules are deleted; "It's common for the app or the app installer itself to add this firewall rule" |
| what raises the question | the rules page ("listen call"), Microsoft "Windows Firewall for Game Developers" (bind() with a non-zero UDP port, accept() for TCP; the popup names the program from its version info) | Artosia's phone remote is a TCP listen on 0.0.0.0:4780, so it raises it. The game page is XP-era text: current wording not confirmed |
| rule syntax | archived netsh advfirewall firewall page (add rule, delete rule, show rule) | `delete rule name=all program=...` removes every match; `show rule` has no program filter; with a port, `protocol` is required; `localsubnet`; do not combine edge=yes with localsubnet |
| profiles and subnet | rules page, "recommendations" | rules in all three profiles; Local Subnet on private and public (domain left open by Microsoft; the owner's rule limits all three, which is stricter, not contradicted) |
| elevation | ShellExecuteExW and SHELLEXECUTEINFOW pages; UAC "how it works" | `runas`; `SEE_MASK_NOCLOSEPROCESS` + wait + exit code; `ERROR_CANCELLED` when refused; a standard user gets a credential prompt (an administrator's password), an admin a consent prompt |
| how projects do it | RustDesk, Sunshine, OpenLP, Syncthing's Windows setup, FreeShow, OBS WebSocket, Jellyfin | see the table in `notes.md` |

What the projects teach:
- Installers add the rule (RustDesk, Sunshine, OpenLP's MSI, Syncthing's setup); run-time-only programs leave it to Windows (FreeShow, OBS WebSocket, Jellyfin) and their trackers carry the symptom (OBS #608, #824).
- The closest model is Syncthing's per-user path: it checks without elevation first, asks its own plain Yes/No in the program's words, only then launches the elevated step with `runas`, and the rule names the program path. It never sets a port and never deletes old blocks (Artosia must, per the card).
- No project deletes blocks Windows saved earlier. That step is Artosia's own, backed only by Microsoft's "must be deleted" line and the netsh `delete rule` text.

## BASE

A small module `firewall.rs` in `crates/artosia/src/` that knows only: a rule's name, a program path and a port; how to read whether it exists; how to build the one elevated command; and how to turn an outcome into a state. Named for that, not for the phone remote. A second user would be any later listener (the stream, a second remote port).

## BOUNDARY

- `firewall.rs` owns a port `Runner` (trait: `show(&self) -> Output`, `elevate(&self, command: &str) -> Elevation`). Its real implementation (`#[cfg(windows)]`) spawns `netsh` (no console window) and calls `ShellExecuteExW`; tests pass a fake. No other module calls Windows for this.
- `actions.rs` `set_remote` is the one composition root: it chooses the real runner, or none (practice, headless, test, local-only, not Windows) and starts the server after the worker answers.
- No cycle: `firewall.rs` imports nothing from `state.rs`; the answer goes back through the `Arrived` inbox like Bible and update results.
- Hand-written FFI (five `extern "system"` functions) is needed because `crates/artosia/Cargo.toml` is not in the files this card owns; the alternative is a `windows` dependency like `artosia-setup` has. Decision for the owner.

## SCREEN

The Phone dialog (`ui/sheets.rs` `remote`, reached by the Phone button or the search command). Two new lines inside it, no new dialog.

## RULES (what a person sees)

- State is a fill, never a bar; yellow is the warning colour (`theme::warn_ink`, as `remote_failed` and `design-media-problem`).
- "About to ask" line: muted body text (as `SCREENS_OFF`), one line, says Windows is about to ask and that Yes is the answer. GUESS on wording; it must work for a standard user too, who gets a password box and no Yes (see EDGES).
- "Refused" line: warning ink, says Windows is not letting phones in and that switching Phone off and on asks again.
- At most about ten words a line. No dash, middle dot, emoji or exclamation mark. English only in `strings.rs`, each with a brief in `docs/i18n/brief.json`; Indonesian is not written.

## WORDS (keys to add, not added yet)

| key | role | shows when |
|---|---|---|
| `REMOTE_FIREWALL_ASKING` | notice line in the Phone dialog, under the switch row | the rule is missing and the box has not been answered |
| `REMOTE_FIREWALL_REFUSED` | warning line in the Phone dialog | the person refused or the command failed |

Tests that wait on the Indonesian: `the_indonesian_table_answers_every_brief` (strings.rs) once the two briefs exist without `id.json` entries; `tools/check-brief.py` passes as soon as the briefs are written. `--indonesian` renders fall back to English until then (the `text!` macro takes an Indonesian argument; a card that adds the keys must pick an English-only arm or a placeholder: owner decision, I did not write Indonesian).

## DATA

| value | held | read by | refresh |
|---|---|---|---|
| rule exists (bool) | the worker's answer, via `Arrived::Firewall(Outcome)` | `App::poll`, which starts the server | each switch-on |
| `firewall: Firewall` (Idle, Asking, Refused) | `App` | the Phone dialog | set on switch-on, cleared on switch-off |
| program path | `std::env::current_exe()` at the time of the check | `firewall.rs` | each check (the path may change after a move) |
| port | the constant 4780 in `set_remote`, shared with `RemoteConfig` (one constant, not two) | both | none |

## The command (exact, built from the sources)

Rule name: `Artosia phone remote` (ASCII, one name). `<exe>` is the full path, in quotes.

Check (no elevation): `netsh advfirewall firewall show rule name="Artosia phone remote" verbose`. Exit 0 = some rule has that name; 1 = none (read here). A rule counts only when the output holds `<exe>` (case-folded) and the port `4780` as a whole word on a line with the same label as the port line in the sample. GUESS: labels are translated on other languages, so the reader looks for the path and for a line whose value is exactly `4780` and another exactly `TCP`, never for a label. The path may come back in the OEM code page when output is piped: GUESS that decoding lossily and matching the ASCII runs of the path in order is enough; proven only by tests on constructed samples (a German-labelled one, one with the user name replaced by U+FFFD and one UTF-8), and still unproven on a real non-English Windows.

Elevated (one launch, `runas`, hidden, wait): file `cmd.exe`, parameters
`/d /s /c ""%SystemRoot%\System32\netsh.exe" advfirewall firewall delete rule name=all program="<exe>" & "%SystemRoot%\System32\netsh.exe" advfirewall firewall add rule name="Artosia phone remote" dir=in action=allow program="<exe>" protocol=tcp localport=4780 profile=any remoteip=localsubnet enable=yes"`
- `&` runs the add even when the delete finds nothing (netsh exits 1 then); the exit code is the add's. `netsh -f` was read and rejected: it exits 1 when any line fails, so it cannot tell a failed add from an empty delete.
- A path containing `%` or `"` is not put on this line (cmd expands `%`): the flow reports a failure and the warning line shows. GUESS that this is rare enough.
- After the elevated process ends (any exit code) the program runs the check again and trusts only that. A refused UAC (`ERROR_CANCELLED`), a failed launch and a wrong exit all end as "rule still missing".
- Not run on this PC: the line is only built and asserted in tests through the injected runner.

Which program the box names: GUESS "Windows Command Processor", publisher Microsoft Windows (gray, not the yellow unsigned box), from `cmd.exe` version info read locally; the UAC page does not say what it prints. `netsh.exe` ("Network Command Shell") would read about the same and is not usable for two commands in one launch.

## ACTION blocks

press Phone switch on (Windows, interactive, `local_only: false`):
```
sync   settings.remote_on = true; touch; if remote.is_some() or pending: return
       remote_pin = new; firewall = Checking; spawn worker(path, runner)
commit 1: the dialog (switch row shows on; no spinner needed, the check is a few tens of ms, GUESS: measure it)
worker run: exists = check()
       exists            -> inbox <- Firewall(Present)
       missing           -> inbox <- Firewall(Asking)  ; elevate(command) ; exists = check() ; inbox <- Firewall(Added | Refused)
poll   Firewall(Asking)  -> firewall = Asking            (commit 2: the notice line)
       Firewall(Present | Added) -> firewall = Idle ; start the server ; qr
       Firewall(Refused)         -> firewall = Refused ; start the server anyway ; (commit 3: the warning line)
moves  the line enters with the dialog's own state change; no new animation (the dialog has none for its other lines): GUESS, check against the app's toast and `remote_failed` line
fails  a panic or a dead worker: the Drop of the sender is read as Refused
```
press Phone switch off: remote = None; firewall = Idle; a late answer from a worker for an earlier switch-on is dropped by a generation number kept in `App`.
startup with `remote_on` saved (`main.rs` `setup`): only the check runs, never the elevated step (a Windows box nobody asked for while the room is setting up); a missing rule then shows the warning line when the dialog opens. GUESS, owner to confirm.

## STATE

`firewall` leaf: owner `App`, read by the Phone dialog only. The rule's existence is derived from the check, never stored. The generation counter is a ref-like field. Nothing is stored that can be derived.

## EDGES (neighbours the card and its tests would miss)

- A standard user (non-admin, a likely church PC) gets a credential prompt, not a Yes button; the notice must not promise a Yes alone. Wording to settle in the brief ("Windows may ask for an administrator's password").
- Two switch presses in a row, and off while the box is up: the generation number drops the late answer; the elevated process cannot be cancelled (it is the shell's), so a rule may be added after Off, which is harmless.
- `claim_instance` keeps one Artosia per library, so two programs do not race the same rule.
- The delete removes every rule for this exe path, including any allow rule a person made by hand. Acceptable by the card; stated in docs.
- A moved or renamed exe has a new path, so a new check and a new box; the old rule stays (the rule names a path, per Microsoft).
- Uninstall leaves the rule: `artosia-setup` is not changed, because its exe is kept the same file across releases; `docs/setup.md` says so in one paragraph.
- The remote URL uses `urls().first()` (`actions.rs` `remote_url`): FreeShow #2395 shows a virtual adapter's address chosen the same way. Reported, not changed here.
- Windows Group Policy may disable local rules (rules page, "local policy merge"); then the add fails and the warning line shows. The line must not blame the person.
- Window's profile setting: Public vs Private. The card's GUESS that Wi-Fi is often Public is not confirmed by Microsoft's page; the rule covers every profile either way.
- Microsoft's installer guidance (add at install, remove at uninstall) is the better home for the rule; the card keeps the first-use route because the setup must not change. Both are listed under "ways open" below.

## Ways open (evidence)

1. First-use elevated rule (the card): Syncthing's per-user setup is the nearest running example (check first, own question, runas). Costs a UAC box on a church PC and fails for a person who cannot elevate.
2. Rule written by an elevated installer (OpenLP MSI, Sunshine NSIS, RustDesk, Syncthing admin path; Microsoft's own advice): one box at install, no code in the operator. Needs the setup to change and to run elevated, which the card forbids.
3. Leave it to Windows (FreeShow, OBS, Jellyfin): least code; the records are Microsoft's own list of causes, OBS #608 and #824, and this PC's log of saved blocks.

## TIMING and SIZE

The check is one `netsh` process; GUESS 50 to 300 ms, to be measured in a unit test with the fake runner (logic) and once with the real read-only `netsh` (time). It runs on the worker, never the UI thread. The elevated step blocks only the worker. The dialog's size is fixed before the answer: the two lines are reserved rows only when their state is on, so the QR card moves down once; GUESS to avoid by drawing the line in the row where `remote_problem` draws.

## TESTS (pure logic only, with the injected runner)

- command builder: exact string above for a plain path and a path with spaces and non-ASCII characters; refuses `%` and `"`.
- check reader: found / not found / found with another path / wrong port / German-labelled sample / U+FFFD path sample / empty output.
- flow: present (no elevate call), missing then added, missing then refused (cancel), missing then command fails, check fails twice.
- gate: no runner for practice, headless, test, `local_only`, non-Windows.
- no test runs `netsh` against the real firewall for writing; the only real call is `show rule` read-only.

## Not done in this card step

No code, no strings, no briefs, no scenes. The scenes `remote-firewall-asking` and `remote-firewall-refused`, their renders (light, dark, English, `--indonesian`), `docs/setup.md`, `docs/operator.md`, `docs/remote.md` and `docs/features.md` row 8 are all left for the build, once the owner picks among the ways above.
