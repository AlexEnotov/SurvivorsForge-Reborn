//! Shared English-source catalog for native UI translations and Rust-generated messages.
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub const LANGUAGES: [&str; 3] = ["en", "ru", "de"];
pub fn index(language: &str) -> i32 {
    LANGUAGES.iter().position(|l| *l == language).unwrap_or(0) as i32
}
pub fn language(index: i32) -> &'static str {
    LANGUAGES.get(index as usize).copied().unwrap_or("en")
}
fn catalog() -> &'static BTreeMap<String, [String; 2]> {
    static CATALOG: OnceLock<BTreeMap<String, [String; 2]>> = OnceLock::new();
    CATALOG.get_or_init(|| serde_json::from_str(include_str!("../translations/catalog.json")).expect("valid translations"))
}
pub fn text(language: &str, source: &str) -> String {
    translate(language, source, 0)
}
fn translate(language: &str, source: &str, depth: u8) -> String {
    let column = match language { "ru" => 0, "de" => 1, _ => return source.into() };
    if let Some(value) = catalog().get(source) { return value[column].clone(); }
    if depth < 4 {
        if source.contains('\n') {
            return source.split('\n').map(|line| translate(language, line, depth + 1)).collect::<Vec<_>>().join("\n");
        }
        // App-authored messages with one value (version, addon name or technical error).
        for (template, value) in catalog() {
            let Some((before, after)) = template.split_once("{}") else { continue };
            if let Some(inner) = source.strip_prefix(before).and_then(|s| s.strip_suffix(after)) {
                return value[column].replace("{}", &translate(language, inner, depth + 1));
            }
        }
    }
    source.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn english_is_default_and_language_choices_are_bounded() {
        assert_eq!(language(index("unknown")), "en");
        assert_eq!(language(-1), "en");
        assert_eq!(text("en", "Settings"), "Settings");
        assert_eq!(text("ru", "Settings"), "Настройки");
        assert_eq!(text("de", "Settings"), "Einstellungen");
        assert_eq!(text("de", "Update: v1.2"), "Update: v1.2");
        assert_eq!(text("ru", "Update: v1.2"), "Обновление: v1.2");
        assert_eq!(text("ru", "Author's addon description"), "Author's addon description");
    }
    #[test]
    fn every_catalog_entry_has_two_translations_and_matching_placeholders() {
        for (source, values) in catalog() {
            for value in values {
                assert!(!value.trim().is_empty(), "{source}");
                assert_eq!(source.matches("{}").count(), value.matches("{}").count(), "{source}");
            }
        }
    }
}
