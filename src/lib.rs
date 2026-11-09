pub use forge_core::{archive, toc};
pub mod addon_enable;
pub mod i18n;
pub mod local_scan;
pub mod network_policy;
pub mod config;
pub mod backup;
pub mod controller;
pub mod github;
pub mod icons;
pub mod game;
pub mod install;
pub mod registry;
pub mod repo_detect;
pub mod scan;
pub mod self_update;
pub mod state;

pub mod updates;
#[cfg(test)]
mod test_server;

use std::fmt;
use std::path::Path;

pub const APP_NAME: &str = "RebornAddonManager";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// GitHub repo of SurvivorsForge itself: hosts the verified list and the releases for self-update.
pub const APP_REPO: &str = "AlexEnotov/SurvivorsForge-Reborn";

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Io(String),
    Zip(String),
    UnsafePath(String),
    NoAddonFound,
    NotFound,
    RateLimited,
    Http(u16),
    Network(String),
    Parse(String),
    NoRelease,
    NoTags,
    NoMatchingAsset,
    FolderOwnedBy(String),
    AlreadyVerified(String),
    InvalidRepo(String),
    ForbiddenHost(String),
    NotInstallable(String),
    HashMismatch { expected: String, actual: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ForbiddenHost(url) => write!(f, "Network access is not allowed: {url}"),
            Error::NotInstallable(reason) => f.write_str(reason),
            Error::HashMismatch { expected, actual } => write!(f, "SHA-256 mismatch: expected {expected}, received {actual}"),
            Error::Io(e) => write!(f, "File error: {e}"),
            Error::Zip(e) => write!(f, "The zip file is damaged or invalid: {e}"),
            Error::UnsafePath(p) => write!(f, "The zip contains an unsafe path ({p}), installation cancelled"),
            Error::NoAddonFound => write!(f, "No addon found"),
            Error::NotFound => write!(f, "Not found on GitHub"),
            Error::RateLimited => write!(f, "GitHub is limiting requests right now. Try again in a few minutes"),
            Error::Http(code) => write!(f, "GitHub answered with error {code}"),
            Error::Network(e) => write!(f, "Cannot reach GitHub: {e}"),
            Error::Parse(e) => write!(f, "Unexpected response: {e}"),
            Error::NoRelease => write!(f, "The repo has no release"),
            Error::NoTags => write!(f, "The repo has no tags"),
            Error::NoMatchingAsset => write!(f, "The latest release has no matching addon zip"),
            Error::FolderOwnedBy(name) => write!(f, "The folder already belongs to the addon \"{name}\""),
            Error::AlreadyVerified(name) => write!(f, "\"{name}\" is a verified addon, install it from Browse"),
            Error::InvalidRepo(input) => write!(f, "\"{input}\" is not a GitHub repo (expected owner/repo or a GitHub URL)"),
        }
    }
}

impl From<forge_core::Error> for Error {
 fn from(error: forge_core::Error) -> Self {
  match error {
   forge_core::Error::Io(e) => Self::Io(e),
   forge_core::Error::Zip(e) => Self::Zip(e),
   forge_core::Error::UnsafePath(e) => Self::UnsafePath(e),
   forge_core::Error::NoAddonFound => Self::NoAddonFound,
   forge_core::Error::Parse(e) => Self::Parse(e),
   other => Self::NotInstallable(other.to_string()),
  }
 }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Writes via a temp file and rename, so a crash never leaves a half-written file.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Unique-enough name for temp folders and files of this process.
pub fn unique_name() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{}-{}-{}", std::process::id(), nanos, COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_identifies_own_client() {
        assert_eq!(APP_NAME, "RebornAddonManager");
    }

    #[test]
    fn error_messages_are_human_readable() {
        assert_eq!(Error::NoAddonFound.to_string(), "No addon found");
        assert!(Error::RateLimited.to_string().contains("limiting requests"));
        assert!(Error::FolderOwnedBy("Survivor UI".into()).to_string().contains("Survivor UI"));
    }
}
