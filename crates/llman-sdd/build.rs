fn main() {
    // i18n! embeds translations at compile time but does not register the
    // locale files with Cargo's change detection. Declare them here so any
    // translation edit triggers a real rebuild (mirrors the facade build.rs;
    // path is relative to this crate's CARGO_MANIFEST_DIR).
    println!("cargo:rerun-if-changed=locales");
}
