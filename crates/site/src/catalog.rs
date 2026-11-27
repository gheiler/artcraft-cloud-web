use types::Registry;

/// The signed app registry, bundled at build time from the drive image.
pub fn registry() -> Registry {
    Registry::parse(include_str!("../static/assets/apps.json")).expect("bundled apps.json is valid")
}
