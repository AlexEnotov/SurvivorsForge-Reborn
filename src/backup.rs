//! Local Reborn SavedVariables backups. Account configuration and credentials are excluded.
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
const MAX_BYTES: u64 = 128 * 1024 * 1024;

fn game_root(addons: &Path) -> Result<&Path> {
    if !crate::config::uses_external_game_launcher(addons) {
        return Err(Error::NotInstallable("Select Reborn's Interface/AddOns folder first".into()));
    }
    addons.parent().and_then(Path::parent).ok_or_else(|| Error::UnsafePath(addons.display().to_string()))
}

pub fn directory(addons: &Path) -> Result<PathBuf> { Ok(game_root(addons)?.join(".reborn-addon-manager/backups")) }

fn collect(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() { continue; }
        if ty.is_dir() { collect(&entry.path(), files)?; }
        else if ty.is_file() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name.ends_with(".lua") || name.ends_with(".lua.bak") || name == "addons.txt" { files.push(entry.path()); }
        }
        if files.len() > 20000 { return Err(Error::NotInstallable("Too many SavedVariables files".into())); }
    }
    Ok(())
}

pub fn create(addons: &Path) -> Result<Option<PathBuf>> {
    let wtf = game_root(addons)?.join("WTF");
    if !wtf.is_dir() { return Ok(None); }
    if wtf.symlink_metadata()?.file_type().is_symlink() { return Err(Error::UnsafePath(wtf.display().to_string())); }
    let mut files = Vec::new(); collect(&wtf, &mut files)?; files.sort();
    if files.is_empty() { return Ok(None); }
    let dir = directory(addons)?; std::fs::create_dir_all(&dir)?;
    let target = dir.join(format!("saved-{}-{}.zip",crate::now_secs(),crate::unique_name()));
    let part = target.with_extension("part");
    let result = (|| -> Result<()> {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&part)?);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut total = 0u64;
        for file in files {
            if file.metadata()?.len() > MAX_BYTES - total { return Err(Error::NotInstallable("SavedVariables backup exceeds 128 MB".into())); }
            let mut bytes = Vec::new();
            std::fs::File::open(&file)?.take(MAX_BYTES-total+1).read_to_end(&mut bytes)?;
            total += bytes.len() as u64;
            if total > MAX_BYTES { return Err(Error::NotInstallable("SavedVariables backup exceeds 128 MB".into())); }
            let name = file.strip_prefix(&wtf).map_err(|e| Error::Io(e.to_string()))?.to_string_lossy().replace('\\',"/");
            zip.start_file(name,options).map_err(|e| Error::Zip(e.to_string()))?;
            zip.write_all(&bytes)?;
        }
        zip.finish().map_err(|e| Error::Zip(e.to_string()))?.sync_all()?;
        std::fs::rename(&part,&target)?;
        Ok(())
    })();
    if result.is_err() { let _ = std::fs::remove_file(&part); }
    result?;
    Ok(Some(target))
}

pub fn list(addons: &Path) -> Result<Vec<PathBuf>> {
    let dir = directory(addons)?;
    if !dir.exists() { return Ok(Vec::new()); }
    let mut files = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?.into_iter()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()) && e.path().extension().is_some_and(|e| e == "zip"))
        .map(|e| e.path()).collect::<Vec<_>>();
    files.sort(); files.reverse(); Ok(files)
}

pub fn restore(addons: &Path, backup: &Path) -> Result<()> {
    crate::addon_enable::require_game_closed()?;
    restore_files(addons, backup)
}

fn restore_files(addons: &Path, backup: &Path) -> Result<()> {
    let dir = directory(addons)?.canonicalize()?;
    let file = backup.canonicalize()?;
    if file.parent() != Some(dir.as_path()) { return Err(Error::UnsafePath(file.display().to_string())); }
    let wtf = game_root(addons)?.join("WTF");
    let mut zip = zip::ZipArchive::new(std::fs::File::open(file)?).map_err(|e| Error::Zip(e.to_string()))?;
    let mut changes = Vec::new(); let mut total = 0;
    let mut seen = std::collections::HashSet::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| Error::Zip(e.to_string()))?;
        let name = entry.name().replace('\\',"/");
        if entry.is_dir() || name.split('/').any(|s| !forge_core::folder::windows_safe_folder(s)) || !seen.insert(name.to_ascii_lowercase()) {
            return Err(Error::UnsafePath(name));
        }
        let lower = name.to_ascii_lowercase();
        if !(lower.ends_with(".lua") || lower.ends_with(".lua.bak") || lower.ends_with("/addons.txt")) { return Err(Error::UnsafePath(name)); }
        let destination = wtf.join(&name);
        let mut current = destination.as_path();
        loop {
            if current.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) { return Err(Error::UnsafePath(name)); }
            if current == wtf { break; }
            current = current.parent().ok_or_else(|| Error::UnsafePath(name.clone()))?;
        }
        let mut bytes = Vec::new();
        entry.by_ref().take(MAX_BYTES+1).read_to_end(&mut bytes)?;
        total += bytes.len() as u64;
        if total > MAX_BYTES { return Err(Error::NotInstallable("Backup is too large".into())); }
        changes.push((destination,bytes));
    }
    let _ = create(addons)?;
    let mut prepared = Vec::new();
    for (destination,bytes) in changes {
        let before = match std::fs::read(&destination) { Ok(bytes) => Some(bytes), Err(e) if e.kind()==std::io::ErrorKind::NotFound => None, Err(e) => return Err(e.into()) };
        prepared.push((destination,bytes,before));
    }
    let mut applied: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    for (destination,bytes,before) in prepared {
        if let Err(error) = crate::write_atomic(&destination,&bytes) {
            for (path,before) in applied.into_iter().rev() {
                match before { Some(bytes) => { let _=crate::write_atomic(&path,&bytes); }, None => {let _=std::fs::remove_file(path);} }
            }
            return Err(error.into());
        }
        applied.push((destination,before));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backs_up_saved_variables_and_excludes_account_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let addons = dir.path().join("Interface/AddOns");
        let account = dir.path().join("WTF/Account/TEST");
        std::fs::create_dir_all(&addons).unwrap();
        std::fs::create_dir_all(account.join("SavedVariables")).unwrap();
        std::fs::write(account.join("SavedVariables/Test.lua"),"Test = {} ").unwrap();
        std::fs::write(account.join("config.wtf"),"private fixture").unwrap();
        let backup = create(&addons).unwrap().unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(backup).unwrap()).unwrap();
        assert_eq!(zip.len(),1);
        assert_eq!(zip.by_index(0).unwrap().name(),"Account/TEST/SavedVariables/Test.lua");
    }

    #[test]
    fn restore_saves_current_settings_and_restores_selected_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let addons = dir.path().join("Interface/AddOns");
        let variables = dir.path().join("WTF/Account/TEST/SavedVariables/Test.lua");
        std::fs::create_dir_all(&addons).unwrap();
        std::fs::create_dir_all(variables.parent().unwrap()).unwrap();
        std::fs::write(&variables,"Test = 1").unwrap();
        let old = create(&addons).unwrap().unwrap();
        std::fs::write(&variables,"Test = 2").unwrap();
        restore_files(&addons,&old).unwrap();
        assert_eq!(std::fs::read_to_string(&variables).unwrap(),"Test = 1");
        assert_eq!(list(&addons).unwrap().len(),2);
    }
}
