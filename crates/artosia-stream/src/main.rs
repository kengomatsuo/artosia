//! `artosia-stream`: see `README.md` for the flags and the JSON lines.

use artosia_stream::{args, run};

fn main() {
    // The operator asks whether this program can start at all before it offers streaming: off
    // Windows it links a GStreamer that is not its own, and without those libraries the system's
    // loader ends it before this line. The projector program answers the same question.
    if std::env::args().any(|arg| arg == "--probe") {
        return;
    }
    let parsed = match args::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(message) => {
            let line = artosia_stream::status::problem(&message, None);
            println!("{line}");
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    if parsed.help {
        print!("{}", args::USAGE);
        return;
    }
    std::process::exit(run::run(parsed));
}
