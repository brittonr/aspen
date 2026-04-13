pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    match variant_name {
        "CacheDownload"
        | "CacheMigrationCancel"
        | "CacheMigrationStart"
        | "CacheMigrationStatus"
        | "CacheMigrationValidate"
        | "CacheQuery"
        | "CacheStats"
        | "NixCacheGetPublicKey"
        | "SnixDirectoryGet"
        | "SnixDirectoryPut"
        | "SnixPathInfoGet"
        | "SnixPathInfoPut" => Some("snix"),
        _ => None,
    }
}
