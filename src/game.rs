//! Reborn executes and updates its own game. No game files are downloaded here.
use std::path::{Path, PathBuf};
use crate::{Error, Result};

#[derive(Clone, Copy)]
pub enum Target { Launcher, Game }

pub fn executable(addons: &Path, target: Target) -> Result<PathBuf> {
    let interface = addons.parent().filter(|p| p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("Interface")))
        .ok_or_else(|| Error::NotInstallable("Select the Reborn Interface/AddOns folder in Settings".into()))?;
    if !addons.file_name().is_some_and(|n| n.eq_ignore_ascii_case("AddOns")) {
        return Err(Error::NotInstallable("Select the Reborn Interface/AddOns folder in Settings".into()));
    }
    let root = interface.parent().ok_or_else(|| Error::NotInstallable("Game folder was not found".into()))?;
    let name = match target { Target::Launcher => "AzerothSurvivorsLauncher.exe", Target::Game => "AzerothSurvivorsGame.exe" };
    let path = root.join(name);
    if !path.is_file() { return Err(Error::NotInstallable(format!("{name} was not found in {}", root.display()))); }
    Ok(path)
}

pub fn launch(addons: &Path, target: Target) -> Result<std::process::Child> {
    let path = executable(addons, target)?;
    Ok(std::process::Command::new(&path).current_dir(path.parent().unwrap()).spawn()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolves_both_executables_relative_to_selected_installation() {
        let dir = tempfile::tempdir().unwrap();
        let addons = dir.path().join("Game with spaces/Interface/AddOns");
        std::fs::create_dir_all(&addons).unwrap();
        let root = addons.parent().unwrap().parent().unwrap();
        for (target, name) in [(Target::Launcher,"AzerothSurvivorsLauncher.exe"),(Target::Game,"AzerothSurvivorsGame.exe")] {
            assert!(executable(&addons,target).is_err());
            std::fs::write(root.join(name),b"fixture").unwrap();
            assert_eq!(executable(&addons,target).unwrap(),root.join(name));
        }
        assert!(executable(&dir.path().join("other/AddOns"),Target::Game).is_err());
    }
}
