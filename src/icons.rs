//! Optional addon artwork: bounded local reads and GitHub-only cached downloads.
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::io::{Cursor, Read};
use std::time::{Duration, SystemTime};

pub const MAX_BYTES: u64 = 2 * 1024 * 1024;
#[derive(Clone)]
pub struct Pixels { pub width: u32, pub height: u32, pub rgba: Vec<u8> }

pub fn validate_remote(raw: &str, repo: &str) -> Result<url::Url> {
    let parsed = crate::network_policy::validate(raw)?;
    #[cfg(test)]
    if parsed.host_str() == Some("127.0.0.1") { return Ok(parsed); }
    let prefix = format!("/{}/", repo.to_ascii_lowercase());
    if parsed.host_str() != Some("raw.githubusercontent.com") || !parsed.path().to_ascii_lowercase().starts_with(&prefix)
        || parsed.query().is_some() || parsed.fragment().is_some() || parsed.port().is_some() {
        return Err(Error::ForbiddenHost(raw.into()));
    }
    Ok(parsed)
}

pub fn decode(bytes: &[u8], extension: Option<&str>) -> Option<Pixels> {
    if bytes.len() as u64 > MAX_BYTES { return None; }
    let mut reader = image::ImageReader::new(Cursor::new(bytes));
    if let Some(format) = extension.and_then(image::ImageFormat::from_extension) { reader.set_format(format); }
    let mut reader = reader.with_guessed_format().ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2048); limits.max_image_height = Some(2048); limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let pixels = reader.decode().ok()?.thumbnail(64,64).to_rgba8();
    Some(Pixels { width: pixels.width(), height: pixels.height(), rgba: pixels.into_raw() })
}

fn read(path: &Path) -> Option<Pixels> {
    if path.metadata().ok()?.len() > MAX_BYTES { return None; }
    let mut bytes = Vec::new(); std::fs::File::open(path).ok()?.take(MAX_BYTES+1).read_to_end(&mut bytes).ok()?;
    decode(&bytes,path.extension().and_then(|v| v.to_str()))
}

fn safe_file(root: &Path, candidate: &Path) -> Option<Pixels> {
    let root = root.canonicalize().ok()?;
    let candidate = candidate.canonicalize().ok()?;
    if !candidate.starts_with(root) || !candidate.is_file() { return None; }
    read(&candidate)
}

pub fn local(addons: &Path, folders: &[String]) -> Option<Pixels> {
    for folder in folders {
        if !forge_core::folder::windows_safe_folder(folder) { continue; }
        let base = if addons.join(folder).is_dir() { addons.to_path_buf() } else { crate::scan::disabled_dir(addons) };
        let dir = base.join(folder);
        let mut toc = String::new();
        if let Ok(file) = std::fs::File::open(dir.join(format!("{folder}.toc"))) { let _ = file.take(65536).read_to_string(&mut toc); }
        if let Some(texture) = crate::toc::Toc::parse(&toc,folder).icon_texture {
            let value = texture.trim().trim_matches('"').replace('\\',"/");
            if !value.starts_with('/') && !value.contains(':') && !value.split('/').any(|part| part == ".." || part == ".") {
                let candidate = if value.to_ascii_lowercase().starts_with("interface/addons/") { base.join(&value[17..]) } else { dir.join(&value) };
                if let Some(icon) = safe_file(&base,&candidate) { return Some(icon); }
                if candidate.extension().is_none() {
                    for ext in ["png","tga","jpg","webp","bmp"] {
                        if let Some(icon) = safe_file(&base,&candidate.with_extension(ext)) { return Some(icon); }
                    }
                }
            }
        }
        for name in ["icon.png","Icon.png","logo.png","icon.tga","assets/icon.png","assets/logo.png","media/icon.png","Media/icon.tga","Textures/icon.tga"] {
            if let Some(icon) = safe_file(&base,&dir.join(name)) { return Some(icon); }
        }
    }
    None
}

fn cache_file(url: &str, cache: &Path) -> PathBuf {
    use sha2::{Digest,Sha256};
    cache.join("icons").join(format!("{}.png",crate::install::hex(&Sha256::digest(url.as_bytes()))))
}
fn fresh(path: &Path, seconds: u64) -> bool {
    path.metadata().and_then(|m| m.modified()).ok().and_then(|time| SystemTime::now().duration_since(time).ok()).is_some_and(|age| age < Duration::from_secs(seconds))
}
pub fn cached(url: &str, cache: &Path) -> Option<Pixels> { read(&cache_file(url,cache)) }

pub fn fetch(client: &crate::github::Client, url: &str, repo: &str, cache: &Path) -> Option<Pixels> {
    validate_remote(url,repo).ok()?;
    let file = cache_file(url,cache); let missing = file.with_extension("missing");
    if fresh(&file,86400) { return read(&file); }
    if fresh(&missing,3600) { return None; }
    let bytes = match client.icon_bytes(url,repo) {
        Ok(bytes) => bytes,
        Err(Error::NotFound) => { let _=std::fs::create_dir_all(file.parent()?); let _=std::fs::write(missing,[]); let _=std::fs::remove_file(file); return None; }
        Err(_) => return read(&file),
    };
    let extension = url::Url::parse(url).ok().and_then(|u| Path::new(u.path()).extension().and_then(|s| s.to_str()).map(str::to_owned));
    let Some(icon) = decode(&bytes,extension.as_deref()) else {
        let _=std::fs::create_dir_all(file.parent()?); let _=std::fs::write(missing,[]); let _=std::fs::remove_file(file); return None;
    };
    let _=std::fs::create_dir_all(file.parent()?);
    let part = file.with_extension("part");
    if image::save_buffer_with_format(&part,&icon.rgba,icon.width,icon.height,image::ColorType::Rgba8,image::ImageFormat::Png).is_ok() {
        let _=std::fs::rename(part,file); let _=std::fs::remove_file(missing);
    }
    Some(icon)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decorative_icons_cannot_point_at_other_hosts_or_repositories() {
        assert!(validate_remote("https://raw.githubusercontent.com/example/addon/main/icon.png","example/addon").is_ok());
        for url in ["https://forgeapi.rezondes.net/icon.png","https://raw.githubusercontent.com/other/repo/main/icon.png","https://github.com/example/addon/icon.png"] {
            assert!(validate_remote(url,"example/addon").is_err());
        }
    }
    #[test]
    fn reads_toc_icons_without_leaving_the_selected_addons_folder() {
        let root=tempfile::tempdir().unwrap(); let addons=root.path().join("Interface/AddOns"); let dir=addons.join("Fixture");
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        let image=image::RgbaImage::from_pixel(8,8,image::Rgba([80,140,220,255]));
        image.save(dir.join("assets/icon.png")).unwrap();
        std::fs::write(dir.join("Fixture.toc"),"## IconTexture: Interface\\AddOns\\Fixture\\assets\\icon.png").unwrap();
        assert!(local(&addons,&["Fixture".into()]).is_some());
        std::fs::write(dir.join("Fixture.toc"),"## IconTexture: ../../../outside.png").unwrap();
        std::fs::remove_file(dir.join("assets/icon.png")).unwrap();
        assert!(local(&addons,&["Fixture".into()]).is_none());
        assert!(decode(b"broken image",Some("png")).is_none());
    }
}
