//! Self-update from a static folder of files with a signed manifest. No UI.
//!
//! Flow: `check` on a worker thread, `download` into `staging_dir(install_dir)`, then on the
//! next start `apply_on_start` swaps the files in; `confirm_started` makes it permanent.
//! See README.md for the manifest format and how a release is made.

mod apply;
mod error;
mod fetch;
pub mod manifest;
pub mod publish;

pub use apply::{
    Applied, RollbackReason, apply_on_start, apply_on_start_with, confirm_started, relaunch,
    roll_back, staging_dir,
};
pub use error::UpdateError;
pub use fetch::{Release, Staged, check, check_with, download, download_with};
pub use manifest::TrustedKeys;
