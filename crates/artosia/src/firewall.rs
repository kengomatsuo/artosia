//! The program's own Windows Firewall rule for the phone remote.
//!
//! Windows asks about a program that listens on the network, and saves a block when the person
//! cancels, or whatever they press when they are not an administrator. A saved block wins over
//! an allow rule and is never asked again, so the program asks once itself, with one elevated
//! command that first deletes every rule for its own path and then adds one allow rule
//! (`docs/plans/artosia/firewall.md`, sources in `~/Shared/research/artosia-firewall/notes.md`).
//!
//! The logic here is pure: what to ask, how to read an answer, what each outcome is. The two
//! things that touch Windows sit behind `Runner`, so tests pass a fake and nothing in a test
//! ever reads or writes the real firewall.

/// The rule's name: one ASCII name, whatever the language of Windows.
pub const RULE_NAME: &str = "Artosia phone remote";

/// What the elevated step did, as far as the program can tell. The rule is checked again after
/// it, so this only says why a rule may be missing.
#[derive(Debug, PartialEq)]
pub enum Elevation {
    /// The command ran and ended with this exit code.
    Ran(u32),
    /// The person refused Windows' box, or could not give an administrator's password.
    Refused,
    /// The command could not be started.
    Failed,
}

/// What a person's press of the Phone switch came to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// The rule was there already.
    Present,
    /// The program added it.
    Added,
    /// It is missing and could not be added: refused, failed, or not asked for.
    Refused,
}

/// Where the firewall step stands, for the Phone dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Standing {
    #[default]
    Idle,
    /// Looking whether the rule exists.
    Checking,
    /// The rule is missing and Windows' box is about to appear or is open.
    Asking,
    /// The rule is missing and could not be added.
    Refused,
}

impl Standing {
    /// The remote has not started yet because this step has not answered.
    pub fn waiting(self) -> bool {
        matches!(self, Standing::Checking | Standing::Asking)
    }
}

/// What the worker tells the window, through `Arrived::Firewall`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Progress {
    Asking,
    Done(Outcome),
}

/// The two calls into Windows, with the checks and the command built by this file.
pub trait Runner {
    /// The text `netsh advfirewall firewall show rule name=<RULE_NAME> verbose` printed, or
    /// None when it could not be run. No elevation.
    fn show_rule(&self) -> Option<String>;
    /// Runs `cmd.exe` with these parameters elevated and hidden (the `runas` verb), and waits.
    fn elevate(&self, parameters: &str) -> Elevation;
}

/// Whether the program should look at the firewall at all: on Windows, in a real window on a
/// real library, and for a server that listens on the network.
pub fn applies(windows: bool, interactive: bool, local_only: bool) -> bool {
    applies_in(windows, interactive, local_only, crate::store::packaged())
}

/// `applies` with the edition given. A packaged program never runs its own elevated step: its
/// manifest declares the rule (`docs/store.md`).
pub fn applies_in(windows: bool, interactive: bool, local_only: bool, packaged: bool) -> bool {
    windows && interactive && !local_only && !packaged
}

/// The parameters of the one elevated `cmd.exe`: delete every rule of this program, then add
/// the allow rule. `&` and not `&&`, because the delete exits 1 when it finds nothing and the
/// add must still run; the exit code is the add's. None for a path `cmd` would read
/// differently from what it is: `%` is expanded inside quotes, and `"` ends them.
pub fn elevated_parameters(system_dir: &str, exe: &str, port: u16) -> Option<String> {
    let unsafe_for_cmd = |text: &str| text.contains('%') || text.contains('"');
    if unsafe_for_cmd(system_dir) || unsafe_for_cmd(exe) {
        return None;
    }
    let netsh = format!("\"{system_dir}\\netsh.exe\" advfirewall firewall");
    Some(format!(
        "/d /s /c \"{netsh} delete rule name=all program=\"{exe}\" & {netsh} add rule name=\"{RULE_NAME}\" dir=in action=allow program=\"{exe}\" protocol=tcp localport={port} profile=any remoteip=localsubnet enable=yes\""
    ))
}

/// Whether `netsh ... show rule ... verbose` printed a rule of this program for this port.
/// A rule is a paragraph. Labels are in the language of Windows, so none is read: the program
/// is the line whose value is the path, the port the line whose value is the number and the
/// protocol the line whose value is TCP. The path is compared without case; netsh prints it in
/// the console's code page, so a path with a non-ASCII character comes back damaged, and then
/// its ASCII stretches must all be found, in order.
pub fn rule_exists(shown: &str, exe: &str, port: u16) -> bool {
    let port = port.to_string();
    shown.replace("\r\n", "\n").split("\n\n").any(|rule| {
        let values: Vec<&str> = rule.lines().filter_map(|line| line.split_once(':').map(|(_, value)| value.trim())).collect();
        values.iter().any(|value| same_path(value, exe)) && values.iter().any(|value| *value == port) && values.iter().any(|value| value.eq_ignore_ascii_case("tcp"))
    })
}

fn same_path(shown: &str, exe: &str) -> bool {
    if shown.eq_ignore_ascii_case(exe) {
        return true;
    }
    if exe.is_ascii() {
        return false;
    }
    let shown = shown.to_ascii_lowercase();
    let stretches: Vec<String> = exe.split(|c: char| !c.is_ascii()).filter(|stretch| !stretch.is_empty()).map(str::to_ascii_lowercase).collect();
    let mut from = 0;
    for stretch in &stretches {
        match shown[from..].find(stretch.as_str()) {
            Some(at) => from += at + stretch.len(),
            None => return false,
        }
    }
    // The damage sits between the stretches, so the path still starts and ends with its own.
    let first_is_start = exe.chars().next().is_some_and(|c| c.is_ascii());
    let last_is_end = exe.chars().next_back().is_some_and(|c| c.is_ascii());
    (!first_is_start || stretches.first().is_some_and(|first| shown.starts_with(first.as_str()))) && (!last_is_end || stretches.last().is_some_and(|last| shown.ends_with(last.as_str())))
}

/// The whole flow, run on a worker. `ask` is called once, before the elevated command, so the
/// window can say what Windows is about to ask. With `may_elevate` false (the program started
/// with the remote already on) a missing rule is only reported.
pub fn ensure(runner: &dyn Runner, system_dir: &str, exe: &str, port: u16, may_elevate: bool, ask: impl FnOnce()) -> Outcome {
    let present = || runner.show_rule().is_some_and(|shown| rule_exists(&shown, exe, port));
    if present() {
        return Outcome::Present;
    }
    if !may_elevate {
        return Outcome::Refused;
    }
    let Some(parameters) = elevated_parameters(system_dir, exe, port) else { return Outcome::Refused };
    ask();
    // Only a second look decides: the exit code of the command is not the rule.
    runner.elevate(&parameters);
    if present() { Outcome::Added } else { Outcome::Refused }
}

/// Runs the flow on a worker and posts its progress to the window, tagged with the press it
/// answers. Nothing off Windows.
#[cfg(windows)]
pub fn spawn(inbox: std::sync::Arc<std::sync::Mutex<Vec<crate::state::Arrived>>>, waker: Option<rune::Waker>, round: u64, port: u16, may_elevate: bool) {
    std::thread::spawn(move || {
        let post = |progress| {
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(crate::state::Arrived::Firewall(round, progress));
            }
            if let Some(wake) = &waker {
                wake();
            }
        };
        let exe = std::env::current_exe().map(|path| path.to_string_lossy().trim_start_matches(r"\\?\").to_string());
        let outcome = match (System::new(), exe) {
            (Some(system), Ok(exe)) => ensure(&system, &system.dir, &exe, port, may_elevate, || post(Progress::Asking)),
            _ => Outcome::Refused,
        };
        post(Progress::Done(outcome));
    });
}

#[cfg(not(windows))]
pub fn spawn(_: std::sync::Arc<std::sync::Mutex<Vec<crate::state::Arrived>>>, _: Option<rune::Waker>, _: u64, _: u16, _: bool) {}

/// Windows itself. Never built into a test: a test passes its own `Runner`.
#[cfg(windows)]
pub struct System {
    pub dir: String,
}

#[cfg(windows)]
impl System {
    /// `%SystemRoot%\System32`, where `netsh.exe` and `cmd.exe` are; None when Windows does not say.
    pub fn new() -> Option<System> {
        std::env::var("SystemRoot").ok().map(|root| System { dir: format!("{root}\\System32") })
    }
}

#[cfg(windows)]
impl Runner for System {
    fn show_rule(&self) -> Option<String> {
        use std::os::windows::process::CommandExt;
        // The rest of the line is passed as it is: netsh reads its own command line, and
        // the name has a space in it.
        let output = std::process::Command::new(format!("{}\\netsh.exe", self.dir))
            .raw_arg(format!("advfirewall firewall show rule name=\"{RULE_NAME}\" verbose"))
            .creation_flags(0x0800_0000)
            .stdin(std::process::Stdio::null())
            .output()
            .ok()?;
        // The console's code page, not UTF-8: a damaged non-ASCII letter is read for (see `same_path`).
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn elevate(&self, parameters: &str) -> Elevation {
        use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED};
        use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize};
        use windows::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
        use windows::Win32::UI::Shell::{SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
        use windows::core::{PCWSTR, w};
        let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(std::iter::once(0)).collect() };
        let (file, parameters) = (wide(&format!("{}\\cmd.exe", self.dir)), wide(parameters));
        // SAFETY: every pointer in `info` points at a buffer that lives until the call returns,
        // the handle is closed once, and COM is uninitialised only when this call initialised it.
        unsafe {
            // The shell may load extensions that need a single-threaded apartment.
            let com = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).is_ok();
            let mut info = SHELLEXECUTEINFOW {
                cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
                fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
                lpVerb: w!("runas"),
                lpFile: PCWSTR(file.as_ptr()),
                lpParameters: PCWSTR(parameters.as_ptr()),
                nShow: 0,
                ..Default::default()
            };
            let result = if let Err(error) = ShellExecuteExW(&mut info) {
                if error.code() == ERROR_CANCELLED.to_hresult() { Elevation::Refused } else { Elevation::Failed }
            } else if info.hProcess.is_invalid() {
                Elevation::Failed
            } else {
                WaitForSingleObject(info.hProcess, INFINITE);
                let mut code = 1;
                let read = GetExitCodeProcess(info.hProcess, &mut code).is_ok();
                let _ = CloseHandle(info.hProcess);
                if read { Elevation::Ran(code) } else { Elevation::Failed }
            };
            if com {
                CoUninitialize();
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    const EXE: &str = r"C:\Users\Church\AppData\Local\Artosia\artosia.exe";
    const SYSTEM: &str = r"C:\Windows\System32";

    /// What `netsh advfirewall firewall show rule name=... verbose` printed on this PC for a
    /// rule with a program, with that rule's values changed to ours (English Windows).
    fn english(program: &str, port: &str, protocol: &str) -> String {
        format!(
            "\r\nRule Name:                            Artosia phone remote\r\n----------------------------------------------------------------------\r\nEnabled:                              Yes\r\nDirection:                            In\r\nProfiles:                             Domain,Private,Public\r\nGrouping:                             \r\nLocalIP:                              Any\r\nRemoteIP:                             LocalSubnet\r\nProtocol:                             {protocol}\r\nLocalPort:                            {port}\r\nRemotePort:                           Any\r\nEdge traversal:                       No\r\nProgram:                              {program}\r\nInterfaceTypes:                       Any\r\nSecurity:                             NotRequired\r\nRule source:                          Local Setting\r\nAction:                               Allow\r\nOk.\r\n\r\n"
        )
    }

    /// The same rule as a German Windows prints it: every label and word translated except the
    /// path, the number and the protocol. Written from the structure of the English output; no
    /// German Windows was available to read (notes.md, "Not confirmed").
    fn german(program: &str) -> String {
        format!(
            "\nRegelname:                            Artosia phone remote\n----------------------------------------------------------------------\nAktiviert:                            Ja\nRichtung:                             Eingehend\nProfile:                              Domäne,Privat,Öffentlich\nLokale IP:                            Beliebig\nRemote-IP:                            LocalSubnet\nProtokoll:                            TCP\nLokaler Port:                         4780\nRemoteport:                           Beliebig\nEdgetraversal:                        Nein\nProgramm:                             {program}\nAktion:                               Zulassen\nOk.\n\n"
        )
    }

    const NOTHING: &str = "No rules match the specified criteria.\r\n";

    /// A runner that answers from a script and records what it was asked.
    struct Fake {
        shown: RefCell<Vec<Option<String>>>,
        elevation: Elevation,
        elevated: RefCell<Vec<String>>,
        looks: Cell<usize>,
    }

    impl Fake {
        fn new(shown: Vec<Option<String>>, elevation: Elevation) -> Fake {
            Fake { shown: RefCell::new(shown), elevation, elevated: RefCell::new(Vec::new()), looks: Cell::new(0) }
        }
    }

    impl Runner for Fake {
        fn show_rule(&self) -> Option<String> {
            self.looks.set(self.looks.get() + 1);
            let mut shown = self.shown.borrow_mut();
            if shown.len() > 1 { shown.remove(0) } else { shown.first().cloned().flatten() }
        }
        fn elevate(&self, parameters: &str) -> Elevation {
            self.elevated.borrow_mut().push(parameters.to_string());
            match &self.elevation {
                Elevation::Ran(code) => Elevation::Ran(*code),
                Elevation::Refused => Elevation::Refused,
                Elevation::Failed => Elevation::Failed,
            }
        }
    }

    fn run(fake: &Fake, may_elevate: bool) -> (Outcome, usize) {
        let asked = Cell::new(0);
        let outcome = ensure(fake, SYSTEM, EXE, 4780, may_elevate, || asked.set(asked.get() + 1));
        (outcome, asked.get())
    }

    #[test]
    fn the_command_line_is_one_delete_then_one_add_for_this_program_and_port() {
        let line = elevated_parameters(SYSTEM, EXE, 4780).expect("a plain path");
        assert_eq!(
            line,
            r#"/d /s /c ""C:\Windows\System32\netsh.exe" advfirewall firewall delete rule name=all program="C:\Users\Church\AppData\Local\Artosia\artosia.exe" & "C:\Windows\System32\netsh.exe" advfirewall firewall add rule name="Artosia phone remote" dir=in action=allow program="C:\Users\Church\AppData\Local\Artosia\artosia.exe" protocol=tcp localport=4780 profile=any remoteip=localsubnet enable=yes""#
        );
    }

    #[test]
    fn a_path_with_spaces_and_a_non_ascii_user_name_goes_into_both_commands_whole() {
        let exe = r"C:\Users\René Müller\AppData\Local\Artosia Operator\artosia.exe";
        let line = elevated_parameters(SYSTEM, exe, 4780).expect("quotes keep the spaces");
        assert_eq!(line.matches(&format!("program=\"{exe}\"")).count(), 2);
        assert!(line.starts_with("/d /s /c \"") && line.ends_with("enable=yes\""));
    }

    #[test]
    fn a_path_cmd_would_read_differently_builds_no_command() {
        assert_eq!(elevated_parameters(SYSTEM, r"C:\Users\100%\artosia.exe", 4780), None);
        assert_eq!(elevated_parameters(SYSTEM, "C:\\Users\\a\"b\\artosia.exe", 4780), None);
        assert_eq!(elevated_parameters(r"C:\Win%dir%", EXE, 4780), None);
    }

    #[test]
    fn the_check_finds_the_rule_of_this_program_and_port() {
        assert!(rule_exists(&english(EXE, "4780", "TCP"), EXE, 4780));
        // Windows keeps the case of what was typed and a person may have typed another.
        assert!(rule_exists(&english(&EXE.to_uppercase(), "4780", "TCP"), EXE, 4780));
    }

    #[test]
    fn the_check_refuses_a_rule_of_another_program_port_or_protocol_and_empty_answers() {
        assert!(!rule_exists(&english(r"C:\Other\artosia.exe", "4780", "TCP"), EXE, 4780));
        assert!(!rule_exists(&english(EXE, "4781", "TCP"), EXE, 4780));
        assert!(!rule_exists(&english(EXE, "4780", "UDP"), EXE, 4780));
        assert!(!rule_exists(NOTHING, EXE, 4780));
        assert!(!rule_exists("", EXE, 4780));
        // Two rules of the name, the program in one and the port in the other, is no rule.
        let split = format!("{}{}", english(EXE, "9999", "TCP"), english(r"C:\Other\a.exe", "4780", "TCP"));
        assert!(!rule_exists(&split, EXE, 4780));
    }

    #[test]
    fn the_check_reads_a_german_windows_by_values_and_never_by_labels() {
        assert!(rule_exists(&german(EXE), EXE, 4780));
        assert!(!rule_exists(&german(r"C:\Other\artosia.exe"), EXE, 4780));
    }

    #[test]
    fn the_check_reads_a_path_netsh_printed_in_the_wrong_code_page() {
        let exe = r"C:\Users\René Müller\AppData\Local\Artosia\artosia.exe";
        // The console's own code page, read as UTF-8 by the program: one replacement mark per byte.
        let damaged = "C:\\Users\\Ren\u{fffd} M\u{fffd}ller\\AppData\\Local\\Artosia\\artosia.exe";
        assert!(rule_exists(&english(damaged, "4780", "TCP"), exe, 4780));
        // The same text as it is when the code page was UTF-8.
        assert!(rule_exists(&english(exe, "4780", "TCP"), exe, 4780));
        // Another user's folder does not pass for it.
        assert!(!rule_exists(&english("C:\\Users\\Ren\u{fffd} Meyer\\AppData\\Local\\Artosia\\artosia.exe", "4780", "TCP"), exe, 4780));
    }

    #[test]
    fn a_rule_that_is_there_asks_nothing_and_runs_nothing() {
        let fake = Fake::new(vec![Some(english(EXE, "4780", "TCP"))], Elevation::Failed);
        assert_eq!(run(&fake, true), (Outcome::Present, 0));
        assert!(fake.elevated.borrow().is_empty());
    }

    #[test]
    fn a_missing_rule_is_asked_for_once_and_added() {
        let fake = Fake::new(vec![Some(NOTHING.into()), Some(english(EXE, "4780", "TCP"))], Elevation::Ran(0));
        assert_eq!(run(&fake, true), (Outcome::Added, 1));
        assert_eq!(fake.elevated.borrow().as_slice(), [elevated_parameters(SYSTEM, EXE, 4780).expect("plain")]);
    }

    #[test]
    fn a_refused_box_leaves_the_rule_missing() {
        let fake = Fake::new(vec![Some(NOTHING.into())], Elevation::Refused);
        assert_eq!(run(&fake, true), (Outcome::Refused, 1));
        assert_eq!(fake.elevated.borrow().len(), 1);
    }

    #[test]
    fn a_command_that_cannot_start_leaves_the_rule_missing() {
        let fake = Fake::new(vec![None], Elevation::Failed);
        assert_eq!(run(&fake, true), (Outcome::Refused, 1));
    }

    #[test]
    fn only_the_second_look_decides_never_the_exit_code() {
        // Exit 0, no rule: a group policy that ignores local rules.
        let fake = Fake::new(vec![Some(NOTHING.into())], Elevation::Ran(0));
        assert_eq!(run(&fake, true).0, Outcome::Refused);
        // Exit 1 (the delete found nothing, as it does the first time), rule there.
        let fake = Fake::new(vec![Some(NOTHING.into()), Some(english(EXE, "4780", "TCP"))], Elevation::Ran(1));
        assert_eq!(run(&fake, true).0, Outcome::Added);
    }

    #[test]
    fn the_program_started_with_the_remote_on_only_looks() {
        let fake = Fake::new(vec![Some(NOTHING.into())], Elevation::Ran(0));
        assert_eq!(run(&fake, false), (Outcome::Refused, 0));
        assert!(fake.elevated.borrow().is_empty());
        assert_eq!(fake.looks.get(), 1);
        let fake = Fake::new(vec![Some(english(EXE, "4780", "TCP"))], Elevation::Ran(0));
        assert_eq!(run(&fake, false).0, Outcome::Present);
    }

    #[test]
    fn a_path_that_cannot_be_put_on_the_line_is_never_elevated_and_never_asked() {
        let fake = Fake::new(vec![Some(NOTHING.into())], Elevation::Ran(0));
        let asked = Cell::new(0);
        let outcome = ensure(&fake, SYSTEM, r"C:\100%\artosia.exe", 4780, true, || asked.set(1));
        assert_eq!((outcome, asked.get(), fake.elevated.borrow().len()), (Outcome::Refused, 0, 0));
    }

    #[test]
    fn the_firewall_is_looked_at_only_on_windows_in_a_real_window_for_a_network_server() {
        assert!(applies_in(true, true, false, false));
        assert!(!applies_in(false, true, false, false), "off Windows");
        assert!(!applies_in(true, false, false, false), "practice, headless, a test");
        assert!(!applies_in(true, true, true, false), "a server that listens on this computer only");
        assert!(!applies_in(true, true, false, true), "packaged: the manifest declares the rule");
    }
}
