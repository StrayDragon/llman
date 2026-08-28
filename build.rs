fn main() {
    // i18n! embeds translations at compile time but (as of rust-i18n 4.2) does
    // not register the locale files with Cargo's change detection, so editing
    // `locales/*.yml` alone left stale strings in rebuilds (observed as a false
    // regression in change fix-devx-tooling-traps). Declare them here so any
    // translation edit triggers a real rebuild.
    println!("cargo:rerun-if-changed=locales");
}
