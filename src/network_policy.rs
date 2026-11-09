//! Runtime network isolation. Reading upstream source with Git is a separate developer operation.
use crate::{Error, Result};

pub fn validate(raw: &str) -> Result<url::Url> {
    let parsed = url::Url::parse(raw).map_err(|_| Error::ForbiddenHost(raw.into()))?;
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    #[cfg(test)]
    if parsed.scheme() == "http" && host == "127.0.0.1" { return Ok(parsed); }
    let blocked = ["rezondes.net", "rezondes.com", "rezondes.de", "rezondes.github.io", "aethus.pl"];
    let author = parsed.path().to_ascii_lowercase();
    if parsed.scheme() != "https" || !parsed.username().is_empty() || parsed.password().is_some()
        || blocked.iter().any(|b| host == *b || host.ends_with(&format!(".{b}")))
        || (["github.com", "raw.githubusercontent.com", "api.github.com", "codeload.github.com"].contains(&host.as_str())
            && (author.starts_with("/rezondes/survivorsforge") || author.starts_with("/repos/rezondes/survivorsforge")))
    {
        return Err(Error::ForbiddenHost(raw.into()));
    }
    Ok(parsed)
}

pub fn validate_download(raw: &str) -> Result<url::Url> {
    let parsed = validate(raw)?;
    #[cfg(test)]
    if parsed.host_str() == Some("127.0.0.1") { return Ok(parsed); }
    if !["github.com", "codeload.github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com", "github-releases.githubusercontent.com"].contains(&parsed.host_str().unwrap_or_default()) {
        return Err(Error::ForbiddenHost(raw.into()));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocks_upstream_services_before_any_request() {
        for address in ["https://api.rezondes.net/v1/registry", "https://wow.aethus.pl/game.zip", "https://github.com/Rezondes/SurvivorsForge/releases/latest", "https://raw.githubusercontent.com/Rezondes/SurvivorsForge/main/registry/verified.json", "http://example.org", "https://user:password@example.org"] {
            assert!(validate(address).is_err(), "{address}");
        }
        assert!(validate_download("https://example.org/addon.zip").is_err());
        assert!(validate_download("https://github.com/example/addon/releases/download/v1/addon.zip").is_ok());
    }
}
