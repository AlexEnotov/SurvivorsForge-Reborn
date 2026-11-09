//! Updates SurvivorsForge itself from the releases of `APP_REPO`.
//!
//! Release assets per platform:
//! - Windows: `SurvivorsForge-<version>-x86_64-pc-windows-msvc.zip` holding the `.exe`, which
//!   replaces the running executable.
//! - Linux: `SurvivorsForge-<version>-x86_64.AppImage`, which replaces the AppImage file itself
//!   (`$APPIMAGE`). A bare binary outside an AppImage is not updated in place.

use crate::github::{Client, Release};
use crate::{Error, Result};
use std::path::{Path, PathBuf};

pub const RELEASES_URL: &str = "https://github.com/AlexEnotov/SurvivorsForge-Reborn/releases/latest";

pub fn asset_name_for(windows: bool, version: &str) -> String {
    if windows {
        format!("{}-{version}-x86_64-pc-windows-msvc.zip", crate::APP_NAME)
    } else {
        format!("{}-{version}-x86_64.AppImage", crate::APP_NAME)
    }
}

pub fn asset_name(version: &str) -> String {
    asset_name_for(cfg!(windows), version)
}

/// The AppImage file to replace: `$APPIMAGE`, set by the AppImage runtime, if it points at a file.
pub fn appimage_path(appimage_env: Option<std::ffi::OsString>) -> Option<PathBuf> {
    appimage_env.map(PathBuf::from).filter(|p| p.is_file())
}

/// Whether this process can replace itself; otherwise the update banner links to the release.
pub fn can_replace_self() -> bool {
    false
}

fn parse(version: &str) -> Option<semver::Version> {
    semver::Version::parse(version.trim().trim_start_matches(['v', 'V'])).ok()
}

pub fn is_newer(current: &str, tag: &str) -> bool {
    matches!((parse(current), parse(tag)), (Some(c), Some(t)) if t > c)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Available {
    pub version: String,
    pub url: String,
}

/// The newer version from `release` with an asset for this platform, if any.
pub fn available(current: &str, release: &Release) -> Option<Available> {
    if !is_newer(current, &release.tag) {
        return None;
    }
    let version = parse(&release.tag)?.to_string();
    let wanted = asset_name(&version);
    let asset = release.assets.iter().find(|a| a.name == wanted)?;
    Some(Available { version, url: asset.url.clone() })
}

/// Automatic executable replacement is deliberately unavailable for this rebuild.
pub fn apply(_client: &Client, _update: &Available, _work_dir: &Path) -> Result<()> {
 Err(Error::NotInstallable("Download the new version from the Reborn releases page".into()))
}

/// Starts the (now replaced) program and ends this process. An AppImage restarts from its file:
/// `current_exe` points into the AppImage's temporary mount, which goes away with this process.
pub fn restart() -> ! {
    let program = appimage_path(std::env::var_os("APPIMAGE")).or_else(|| std::env::current_exe().ok());
    if let Some(program) = program {
        let _ = std::process::Command::new(program).spawn();
    }
    std::process::exit(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::Asset;

    fn release(tag: &str, assets: &[&str]) -> Release {
        Release {
            tag: tag.into(),
            assets: assets.iter().map(|n| Asset { name: n.to_string(), url: format!("https://dl/{n}") }).collect(),
        }
    }

    #[test]
    fn newer_version_shows_banner() {
        assert!(is_newer("0.1.0", "v0.2.0"));
        assert!(!is_newer("0.1.0", "v0.1.0"));
        assert!(!is_newer("0.1.0", "v0.0.9"));
        assert!(!is_newer("0.1.0", "nightly"));
    }

    #[test]
    fn asset_name_per_platform() {
        assert_eq!(asset_name_for(true, "0.2.0"), "RebornAddonManager-0.2.0-x86_64-pc-windows-msvc.zip");
        assert_eq!(asset_name_for(false, "0.2.0"), "RebornAddonManager-0.2.0-x86_64.AppImage");
    }

    #[test]
    fn appimage_target_from_env() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("SurvivorsForge.AppImage");
        std::fs::write(&file, b"").unwrap();
        assert_eq!(appimage_path(Some(file.clone().into())), Some(file));
        assert_eq!(appimage_path(Some(dir.path().join("missing").into())), None);
        assert_eq!(appimage_path(None), None);
    }

    #[test]
    fn available_needs_matching_asset() {
        let name = asset_name("0.2.0");
        assert_eq!(
            available("0.1.0", &release("v0.2.0", &[&name])),
            Some(Available { version: "0.2.0".into(), url: format!("https://dl/{name}") })
        );
        assert_eq!(available("0.1.0", &release("v0.2.0", &["other.zip"])), None);
        assert_eq!(available("0.2.0", &release("v0.2.0", &[&name])), None);
    }
}
