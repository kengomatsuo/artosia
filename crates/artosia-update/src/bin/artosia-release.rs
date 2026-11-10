//! The release maker: `keygen`, `build` and `pubkey`. The private key never goes on a command line.

use artosia_update::manifest::{public_key_of, to_hex};
use artosia_update::publish::{ReleaseSpec, build_release, parse_seed};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
artosia-release keygen <private-key-file>
    Writes a new private key file (never commit it) and prints the public key line.
artosia-release pubkey [--key-file <file>]
    Prints the public key line for the private key.
artosia-release build --dir <built folder> --version <x.y.z> --out <folder>
        [--notes <text> | --notes-file <file>] [--platform <os-arch>] [--date <YYYY-MM-DD>]
        [--key-file <file>]
    Writes manifest.json, manifest.json.sig and the flat files into <folder>.
    The private key comes from --key-file or the ARTOSIA_UPDATE_KEY variable.";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn load_seed(args: &[String]) -> Result<[u8; 32], String> {
    let text = match flag(args, "--key-file") {
        Some(path) => std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?,
        None => std::env::var("ARTOSIA_UPDATE_KEY")
            .map_err(|_| "Give --key-file or set ARTOSIA_UPDATE_KEY".to_string())?,
    };
    parse_seed(&text).map_err(|e| e.detail().unwrap_or("bad key").to_string())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("keygen") => {
            let path = args.get(1).ok_or(USAGE)?;
            if std::path::Path::new(path).exists() {
                return Err(format!(
                    "{path} already exists; refusing to overwrite a key"
                ));
            }
            let mut seed = [0u8; 32];
            // The OS random source through ring keeps this free of extra dependencies.
            ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut seed)
                .map_err(|_| "no random source".to_string())?;
            std::fs::write(path, format!("{}\n", to_hex(&seed))).map_err(|e| e.to_string())?;
            let public = public_key_of(&seed).map_err(|e| e.to_string())?;
            println!("Private key written to {path}. Keep it out of the repository.");
            println!("Public key line for update-public-key.txt:");
            println!("{}", to_hex(&public));
            Ok(())
        }
        Some("pubkey") => {
            let public = public_key_of(&load_seed(&args)?).map_err(|e| e.to_string())?;
            println!("{}", to_hex(&public));
            Ok(())
        }
        Some("build") => {
            let need = |name: &str| flag(&args, name).ok_or(format!("Missing {name}\n{USAGE}"));
            let dir = PathBuf::from(need("--dir")?);
            let out = PathBuf::from(need("--out")?);
            let version = need("--version")?;
            let notes = match (flag(&args, "--notes"), flag(&args, "--notes-file")) {
                (Some(n), _) => n,
                (None, Some(f)) => std::fs::read_to_string(&f).map_err(|e| format!("{f}: {e}"))?,
                _ => String::new(),
            };
            let platform = flag(&args, "--platform");
            let date = flag(&args, "--date");
            let seed = load_seed(&args)?;
            let m = build_release(
                &ReleaseSpec {
                    dir: &dir,
                    version: &version,
                    notes: notes.trim(),
                    platform: platform.as_deref(),
                    date: date.as_deref(),
                    out: &out,
                },
                &seed,
            )
            .map_err(|e| format!("{e}: {}", e.detail().unwrap_or("")))?;
            println!(
                "Release {} for {}: {} files in {}",
                m.version,
                m.platform,
                m.files.len(),
                out.display()
            );
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}
