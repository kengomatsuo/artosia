//! `cargo run -p artosia-update --example published -- [version]`
//!
//! Asks the published feed what a copy of `version` (0.0.1 when none is given) would be
//! offered on this computer. The list of this system is fetched and its signature checked
//! with the keys built into the crate, exactly as an installed copy does it. Nothing is
//! downloaded and nothing is changed: it is how a release is checked from each system.
fn main() {
    let have = std::env::args().nth(1).unwrap_or_else(|| "0.0.1".to_string());
    let platform = artosia_update::manifest::this_platform();
    let Some(feed) = artosia_update::feed_address(None) else {
        println!("no feed is set");
        std::process::exit(1);
    };
    match artosia_update::check(&feed, &have) {
        Ok(Some(release)) => println!(
            "{have} on {platform} is offered {}: {} MB in all, released {}, signature good",
            release.version,
            release.size / 1_000_000,
            release.released
        ),
        Ok(None) => println!("{have} is the newest on {platform}"),
        Err(error) => {
            println!("{error} ({})", error.detail().unwrap_or("no detail"));
            std::process::exit(1);
        }
    }
}
