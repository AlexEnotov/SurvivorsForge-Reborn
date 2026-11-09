fn main() {
    println!("cargo:rerun-if-changed=translations/catalog.json");
    let catalog: std::collections::BTreeMap<String, [String; 2]> =
        serde_json::from_str(include_str!("translations/catalog.json")).expect("valid translation catalog");
    let translations = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("translations");
    for (index, language) in ["ru", "de"].into_iter().enumerate() {
        let dir = translations.join(language).join("LC_MESSAGES");
        std::fs::create_dir_all(&dir).unwrap();
        let mut po = String::from("msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\n");
        for (source, translated) in &catalog {
            po.push_str(&format!("msgid {}\nmsgstr {}\n\n", serde_json::to_string(source).unwrap(), serde_json::to_string(&translated[index]).unwrap()));
        }
        std::fs::write(dir.join("survivorsforge.po"), po).unwrap();
    }
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations(translations)
        .with_default_translation_context(slint_build::DefaultTranslationContext::None)
        .with_library_paths([("toast".to_string(), slint_toast::UI_DIR.into())].into());
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint UI compiles");

    // Icon and version info of the .exe (Explorer, taskbar pin, Alt+Tab before the window shows).
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("assets/icon.ico")
        .set("FileDescription", "Reborn Addon Manager")
        .set("ProductName", "Reborn Addon Manager")
        .compile()
        .expect("Windows resources compile");
}
