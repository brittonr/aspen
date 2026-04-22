pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "CacheDownload",
    "CacheMigrationCancel",
    "CacheMigrationStart",
    "CacheMigrationStatus",
    "CacheMigrationValidate",
    "CacheQuery",
    "CacheStats",
    "NixCacheGetPublicKey",
    "SnixDirectoryGet",
    "SnixDirectoryPut",
    "SnixPathInfoGet",
    "SnixPathInfoPut",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("snix")
    } else {
        None
    }
}
