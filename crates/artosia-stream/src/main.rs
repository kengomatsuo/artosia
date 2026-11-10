//! `artosia-stream`: see `README.md` for the flags and the JSON lines.

use artosia_stream::{args, run};

fn main() {
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
