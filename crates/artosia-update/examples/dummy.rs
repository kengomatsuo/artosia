//! A stand-in program for the update tests: sleeps for the given milliseconds, then exits.

fn main() {
    let ms: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(0);
    std::thread::sleep(std::time::Duration::from_millis(ms));
}
