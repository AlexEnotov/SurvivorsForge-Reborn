use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

const FILE: &str = "config.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub addons_dir: PathBuf,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub window: Option<WindowState>,
    #[serde(default)]
    pub sort_order: u8,
    #[serde(default)]
    pub tile_view: bool,
    /// First observation times, scoped to the selected game folder and addon id.
    #[serde(default)]
    pub first_seen: std::collections::BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32, pub y: i32,
    pub width: u32, pub height: u32,
    pub page: i32,
    pub maximized: bool,
}

/// `%APPDATA%\SurvivorsForge` on Windows, `~/.config/SurvivorsForge` on Linux.
/// `REBORN_ADDON_MANAGER_DATA_DIR` overrides it, for testing against a copy of an AddOns folder.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("REBORN_ADDON_MANAGER_DATA_DIR") {
        return dir.into();
    }
    directories::BaseDirs::new()
        .map(|d| d.config_dir().join(crate::APP_NAME))
        .unwrap_or_else(|| PathBuf::from(crate::APP_NAME))
}

fn default_language() -> String { "en".into() }
impl Default for Config { fn default() -> Self { Self { addons_dir: PathBuf::new(), language: default_language(), window: None, sort_order: 0, tile_view: false, first_seen: Default::default() } } }

impl Config {
    fn observation_key(&self, id: &str) -> String {
        let scope = self.addons_dir.to_string_lossy();
        format!("{}|{}", if cfg!(windows) { scope.to_lowercase() } else { scope.into_owned() }, id.to_lowercase())
    }

    pub fn observed_at(&self, id: &str) -> u64 { self.first_seen.get(&self.observation_key(id)).copied().unwrap_or(0) }

    pub fn observe_addon(&mut self, id: &str, installed_at: u64, now: u64) -> bool {
        let key = self.observation_key(id);
        if self.first_seen.contains_key(&key) { return false; }
        self.first_seen.insert(key, if installed_at > 0 { installed_at } else { now });
        true
    }

    pub fn load(dir: &Path) -> Option<Config> {
        let text = std::fs::read_to_string(dir.join(FILE)).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        let json = serde_json::to_vec_pretty(self).expect("config serializes");
        crate::write_atomic(&dir.join(FILE), &json)
    }
}

#[derive(Debug, PartialEq)]
pub enum DirError {
    NotFound(PathBuf),
    NotADirectory,
    NotWritable,
}

impl fmt::Display for DirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DirError::NotFound(path) => {
                write!(f, "No folder at \"{}\". Check the spelling", path.display())?;
                f.write_str(if cfg!(windows) { "." } else { "; paths on Linux are case-sensitive." })
            }
            DirError::NotADirectory => f.write_str("The path is not a folder."),
            DirError::NotWritable => f.write_str("The folder is not writable."),
        }
    }
}

/// A typed path with a leading `~` resolved to the home folder, as a shell would.
pub fn expand_home(input: &str, home: Option<&Path>) -> PathBuf {
    let input = input.trim();
    if let (Some(home), Some(rest)) = (home, input.strip_prefix('~')) {
        if rest.is_empty() {
            return home.to_path_buf();
        }
        if let Some(rest) = rest.strip_prefix(['/', '\\']) {
            return home.join(rest);
        }
    }
    PathBuf::from(input)
}

/// Button label for a found AddOns folder: the game folder above `Interface/AddOns`,
/// with the home folder shortened to `~` on Linux.
pub fn suggestion_label(addons: &Path, home: Option<&Path>) -> String {
    let game = addons.parent().and_then(Path::parent).unwrap_or(addons);
    match home.filter(|_| !cfg!(windows)).and_then(|h| game.strip_prefix(h).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => game.display().to_string(),
    }
}

#[derive(Debug, PartialEq)]
pub enum DirWarning {
    NotNamedAddOns,
}

impl fmt::Display for DirWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("The folder is not named \"AddOns\". Is this really the game's AddOns folder?")
    }
}

/// Where the game is usually unpacked: the home folder and its Games, Downloads and Desktop
/// folders, removable media on Linux (Steam Deck SD card), every drive root on Windows.
pub fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(user) = directories::UserDirs::new() {
        roots.push(user.home_dir().join("Games"));
        roots.extend(user.download_dir().map(Path::to_path_buf));
        roots.extend(user.desktop_dir().map(Path::to_path_buf));
        roots.push(user.home_dir().to_path_buf());
    }
    if cfg!(windows) {
        roots.extend((b'C'..=b'Z').map(|d| PathBuf::from(format!("{}:\\", d as char))).filter(|p| p.is_dir()));
    } else {
        // `/run/media/<user>/<label>` on most distros, `/run/media/<device>` on SteamOS
        roots.push(PathBuf::from("/run/media"));
    }
    roots.into_iter().filter(|p| p.is_dir()).collect()
}

/// Folders that look like the game's AddOns folder (`<game>/Interface/AddOns`), found by a
/// short walk below each root. Hidden folders and symlinks are skipped, and each root has a budget
/// of directory entries, so a huge drive can't stall the setup.
pub fn find_addons_dirs(roots: &[PathBuf], max_depth: usize) -> Vec<PathBuf> {
    const BUDGET_PER_ROOT: usize = 20_000;
    let mut found: Vec<PathBuf> = Vec::new();
    for root in roots {
        let mut budget = BUDGET_PER_ROOT;
        let mut stack = vec![(root.clone(), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            let candidate = dir.join("Interface").join("AddOns");
            if candidate.is_dir() && !found.contains(&candidate) {
                found.push(candidate);
            }
            if depth >= max_depth {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                if budget == 0 {
                    break;
                }
                budget -= 1;
                let hidden = entry.file_name().to_string_lossy().starts_with(['.', '$']);
                if !hidden && entry.file_type().is_ok_and(|t| t.is_dir()) {
                    stack.push((entry.path(), depth + 1));
                }
            }
        }
    }
    found.sort();
    found
}

/// Steam Deck Game Mode (and any gamescope session): the app runs fullscreen there.
pub fn is_game_mode(xdg_current_desktop: Option<&str>, gamescope_display: bool) -> bool {
    gamescope_display || xdg_current_desktop.is_some_and(|d| d.eq_ignore_ascii_case("gamescope"))
}

pub fn validate_addons_dir(path: &Path) -> Result<Option<DirWarning>, DirError> {
    if !path.exists() {
        return Err(DirError::NotFound(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(DirError::NotADirectory);
    }
    let probe = path.join(format!(".survivorsforge-probe-{}", crate::unique_name()));
    std::fs::write(&probe, b"").map_err(|_| DirError::NotWritable)?;
    let _ = std::fs::remove_file(&probe);
    let named_addons = path
        .file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("AddOns"));
    Ok((!named_addons).then_some(DirWarning::NotNamedAddOns))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_dates_survive_updates_and_are_scoped_to_the_game_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config { addons_dir: PathBuf::from("Game-A/AddOns"), sort_order: 3, tile_view: true, ..Config::default() };
        assert!(config.observe_addon("example", 100, 200));
        assert!(!config.observe_addon("example", 300, 400));
        assert_eq!(config.observed_at("example"), 100);
        config.addons_dir = PathBuf::from("Game-B/AddOns");
        assert_eq!(config.observed_at("example"), 0);
        assert!(config.observe_addon("example", 0, 500));
        config.save(dir.path()).unwrap();
        let loaded = Config::load(dir.path()).unwrap();
        assert_eq!(loaded.observed_at("example"), 500);
        assert_eq!(loaded.sort_order, 3); assert!(loaded.tile_view);
        let old: Config = serde_json::from_str(r#"{"addons_dir":"Game/AddOns"}"#).unwrap();
        assert_eq!(old.sort_order,0); assert!(!old.tile_view);
    }

    #[test]
    fn suggests_existing_addon_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let game = root.join("Games/AzerothSurvivors/Interface/AddOns");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(root.join(".hidden/AzerothSurvivors/Interface/AddOns")).unwrap();
        std::fs::create_dir_all(root.join("a/b/c/d/Interface/AddOns")).unwrap();
        assert_eq!(find_addons_dirs(&[root.to_path_buf()], 3), std::slice::from_ref(&game));
        assert_eq!(find_addons_dirs(&[root.to_path_buf(), root.join("Games")], 3), [game], "no duplicates");
    }

    #[test]
    fn detects_game_mode_from_env() {
        assert!(is_game_mode(Some("gamescope"), false));
        assert!(is_game_mode(None, true));
        assert!(!is_game_mode(Some("KDE"), false));
        assert!(!is_game_mode(None, false));
    }

    #[test]
    fn load_returns_none_without_file() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Config::load(dir.path()), None);
    }

    #[test]
    fn validate_rejects_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        assert_eq!(validate_addons_dir(&missing), Err(DirError::NotFound(missing.clone())));
    }

    #[test]
    fn expands_leading_tilde_only() {
        let home = Path::new("/home/deck");
        assert_eq!(expand_home(" ~ ", Some(home)), home);
        assert_eq!(expand_home("~/Games/AddOns", Some(home)), home.join("Games/AddOns"));
        assert_eq!(expand_home("~deck/AddOns", Some(home)), PathBuf::from("~deck/AddOns"));
        assert_eq!(expand_home("/opt/~/AddOns", Some(home)), PathBuf::from("/opt/~/AddOns"));
        assert_eq!(expand_home("~/AddOns", None), PathBuf::from("~/AddOns"));
    }

    #[test]
    fn labels_suggestion_by_game_folder() {
        let home = Path::new("/home/deck");
        let addons = home.join("Games/AzerothSurvivors/Interface/AddOns");
        let expected = if cfg!(windows) { home.join("Games/AzerothSurvivors").display().to_string() } else { "~/Games/AzerothSurvivors".into() };
        assert_eq!(suggestion_label(&addons, Some(home)), expected);
        let card = Path::new("/run/media/mmcblk0p1/AS/Interface/AddOns");
        assert_eq!(suggestion_label(card, Some(home)), Path::new("/run/media/mmcblk0p1/AS").display().to_string());
    }

    #[test]
    fn validate_accepts_addons_and_warns_on_unusual_name() {
        let dir = tempfile::tempdir().unwrap();
        let addons = dir.path().join("AddOns");
        let foo = dir.path().join("foo");
        std::fs::create_dir(&addons).unwrap();
        std::fs::create_dir(&foo).unwrap();
        assert_eq!(validate_addons_dir(&addons), Ok(None));
        assert_eq!(validate_addons_dir(&foo), Ok(Some(DirWarning::NotNamedAddOns)));
        assert_eq!(std::fs::read_dir(&addons).unwrap().count(), 0, "probe file is removed");
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config { addons_dir: PathBuf::from(r"P:\AddOns"), ..Config::default() };
        config.save(dir.path()).unwrap();
        assert_eq!(Config::load(dir.path()), Some(config));
    }
}

/// Reborn owns game installation and game updates.
pub fn uses_external_game_launcher(addons: &Path) -> bool {
 addons.parent().and_then(Path::file_name).is_some_and(|name| name.eq_ignore_ascii_case("Interface"))
}
