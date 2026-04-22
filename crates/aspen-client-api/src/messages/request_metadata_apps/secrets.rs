pub(super) const REQUIRED_APP_VARIANTS: &[&str] = &[
    "SecretsKvDelete",
    "SecretsKvDeleteMetadata",
    "SecretsKvDestroy",
    "SecretsKvList",
    "SecretsKvMetadata",
    "SecretsKvRead",
    "SecretsKvUndelete",
    "SecretsKvUpdateMetadata",
    "SecretsKvWrite",
    "SecretsNixCacheCreateKey",
    "SecretsNixCacheDeleteKey",
    "SecretsNixCacheGetPublicKey",
    "SecretsNixCacheListKeys",
    "SecretsNixCacheRotateKey",
    "SecretsPkiCreateRole",
    "SecretsPkiGenerateIntermediate",
    "SecretsPkiGenerateRoot",
    "SecretsPkiGetCrl",
    "SecretsPkiGetRole",
    "SecretsPkiIssue",
    "SecretsPkiListCerts",
    "SecretsPkiListRoles",
    "SecretsPkiRevoke",
    "SecretsPkiSetSignedIntermediate",
    "SecretsTransitCreateKey",
    "SecretsTransitDatakey",
    "SecretsTransitDecrypt",
    "SecretsTransitEncrypt",
    "SecretsTransitListKeys",
    "SecretsTransitRewrap",
    "SecretsTransitRotateKey",
    "SecretsTransitSign",
    "SecretsTransitVerify",
];

pub(super) fn required_app(variant_name: &str) -> Option<&'static str> {
    if REQUIRED_APP_VARIANTS.contains(&variant_name) {
        Some("secrets")
    } else {
        None
    }
}
