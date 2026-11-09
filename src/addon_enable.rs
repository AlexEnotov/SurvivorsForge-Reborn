//! WoW/Reborn character addon switches. Files stay installed; the game reads AddOns.txt.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

#[derive(Default)]
pub struct Store {
    profiles: Vec<(PathBuf, String)>,
    defaults: HashMap<String, bool>,
}

fn directories(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path).into_iter().flatten().flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path()).collect()
}

fn enabled(text: &str, name: &str, default: bool) -> bool {
    text.lines().filter_map(|l| l.trim_start_matches('\u{feff}').split_once(':'))
        .filter(|(n, _)| n.trim().eq_ignore_ascii_case(name))
        .next_back().map_or(default, |(_, s)| !s.trim().eq_ignore_ascii_case("disabled"))
}

fn changed(text: &str, names: &[String], on: bool) -> String {
    let mut out = text.lines().filter(|line| {
        !line.trim_start_matches('\u{feff}').split_once(':')
            .is_some_and(|(n, _)| names.iter().any(|f| f.eq_ignore_ascii_case(n.trim())))
    }).collect::<Vec<_>>().join("\r\n");
    if !out.is_empty() { out.push_str("\r\n"); }
    for name in names {
        out.push_str(&format!("{name}: {}\r\n", if on { "enabled" } else { "disabled" }));
    }
    out
}

pub fn require_game_closed() -> io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("tasklist.exe")
            .args(["/FI", "IMAGENAME eq AzerothSurvivorsGame.exe", "/FO", "CSV", "/NH"])
            .creation_flags(0x08000000).output()?;
        if !output.status.success() { return Err(io::Error::other("Could not check whether the game is closed.")); }
        if String::from_utf8_lossy(&output.stdout).to_ascii_lowercase().contains("\"azerothsurvivorsgame.exe\"") {
            return Err(io::Error::other("Close AzerothSurvivorsGame before this operation; it may overwrite addon settings on exit."));
        }
    }
    Ok(())
}

impl Store {
    pub fn load(addons: &Path) -> io::Result<Self> {
        let mut store = Self::default();
        if !crate::config::uses_external_game_launcher(addons) { return Ok(store); }
        let Some(game) = addons.parent().and_then(Path::parent) else { return Ok(store) };
        for account in directories(&game.join("WTF/Account")) {
            for realm in directories(&account) {
                for character in directories(&realm) {
                    if !["AddOns.txt", "config-cache.wtf", "SavedVariables.lua", "layout-local.txt"]
                        .iter().any(|f| character.join(f).is_file()) { continue; }
                    let file = character.join("AddOns.txt");
                    let text = match fs::read_to_string(&file) {
                        Ok(text) => text,
                        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
                        Err(e) => return Err(e),
                    };
                    store.profiles.push((file, text));
                }
            }
        }
        for folder in directories(addons) {
            let Some(name) = folder.file_name().and_then(|n| n.to_str()) else { continue };
            if let Ok(toc) = fs::read_to_string(folder.join(format!("{name}.toc"))) {
                let default = toc.lines().filter_map(|l| l.strip_prefix("##")?.split_once(':'))
                    .find(|(k, _)| k.trim().eq_ignore_ascii_case("DefaultState"))
                    .is_none_or(|(_, v)| !v.trim().eq_ignore_ascii_case("disabled"));
                store.defaults.insert(name.to_ascii_lowercase(), default);
            }
        }
        Ok(store)
    }

    pub fn available(&self) -> bool { !self.profiles.is_empty() }

    /// All enabled, and mixed (different characters/folders have different states).
    pub fn state(&self, names: &[String]) -> (bool, bool) {
        let states: Vec<_> = self.profiles.iter().flat_map(|(_, text)| names.iter().map(move |n|
            enabled(text, n, *self.defaults.get(&n.to_ascii_lowercase()).unwrap_or(&true)))).collect();
        let all = !states.is_empty() && states.iter().all(|v| *v);
        (all, !all && states.iter().any(|v| *v))
    }

    /// Apply to all existing characters. Keep unrelated entries and back up original files.
    pub fn set(&self, names: &[String], on: bool) -> io::Result<()> {
        if !self.available() || names.is_empty() {
            return Err(io::Error::other("Start the game and enter a character once to create its addon settings."));
        }
        if names.iter().any(|n| n.contains([':', '\r', '\n', '/', '\\'])) {
            return Err(io::Error::other("Invalid addon folder name"));
        }
        let mut changes = Vec::new();
        for (file, _) in &self.profiles {
            let before = match fs::read(file) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => return Err(e),
            };
            let text = std::str::from_utf8(before.as_deref().unwrap_or_default())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            if let Some(bytes) = &before {
                let backup = file.with_extension("txt.survivorsforge.bak");
                if !backup.exists() { fs::write(backup, bytes)?; }
            }
            changes.push((file, before.clone(), changed(text, names, on)));
        }
        for (index, (file, _, after)) in changes.iter().enumerate() {
            if let Err(e) = crate::write_atomic(file, after.as_bytes()) {
                for (path, before, _) in changes[..index].iter().rev() {
                    match before {
                        Some(bytes) => { let _ = crate::write_atomic(path, bytes); }
                        None => { let _ = fs::remove_file(path); }
                    }
                }
                return Err(e);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggles_existing_characters_without_removing_addons_or_unrelated_entries() {
        let dir = tempfile::tempdir().unwrap();
        let addons = dir.path().join("Interface/AddOns");
        fs::create_dir_all(addons.join("Example")).unwrap();
        fs::write(addons.join("Example/Example.toc"), "## Interface: 30300").unwrap();
        for name in ["A", "B"] {
            let profile = dir.path().join(format!("WTF/Account/Account/Realm/{name}"));
            fs::create_dir_all(&profile).unwrap();
            fs::write(profile.join("config-cache.wtf"), "").unwrap();
        }
        let file = dir.path().join("WTF/Account/Account/Realm/A/AddOns.txt");
        fs::write(&file, "# keep this\nOther: disabled\nExample: disabled\n").unwrap();
        let names = vec!["Example".to_string()];
        let store = Store::load(&addons).unwrap();
        assert_eq!(store.state(&names), (false, true));
        store.set(&names, false).unwrap();
        assert_eq!(Store::load(&addons).unwrap().state(&names), (false, false));
        assert!(addons.join("Example/Example.toc").is_file());
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("# keep this") && text.contains("Other: disabled"));
        assert!(file.with_extension("txt.survivorsforge.bak").is_file());
        store.set(&names, true).unwrap();
        assert_eq!(Store::load(&addons).unwrap().state(&names), (true, false));
    }
}
