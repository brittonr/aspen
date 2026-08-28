pub const ORACLE_SOURCE_SCHEMA: &str = "molten.semantic-state-oracle-source.v1";
pub const DOLTLITE_REPOSITORY: &str = "https://github.com/dolthub/doltlite";
pub const DOLTLITE_REVISION: &str = "10170ed82c1b12414db8d1b29d2fe9ea2a72fd88";
pub const DOLTLITE_ADAPTER_VERSION: &str = "molten-doltlite-oracle-v1";
pub const DOLTLITE_BACKEND_FORMAT: &str = "doltlite-chunk-store-v12";
pub const DOLTLITE_LICENSE_REF: &str = "blake3:4f91d1a7d7b99eefb5c81ddb148446616d8260fc0c8113999cf2a48d3589267c";
pub const DOLTLITE_APACHE_REF: &str = "blake3:a24e4e2958e399474e4b0913dde32c6be84630b6dcf153af7eae29779399eb2f";
pub const DOLTLITE_SQLITE_CONTRACT_REF: &str =
    "blake3:82d470f924e39e4e6eed5ce48095bcb30e682b15e076476a1caf847dac9ab664";
pub const DOLTLITE_CONCURRENCY_CONTRACT_REF: &str =
    "blake3:9efcaf8c67d3b1d6c1e9eac578810bfd266bb2fb920344230299db143d6afcc8";
pub const DOLTLITE_FORMAT_CONTRACT_REF: &str =
    "blake3:9a24814b1023720459092e2fc0126c09ad6af221b6ec876948d0c67c2bcb5452";

pub const MAX_ORACLE_ROWS: usize = 256;
pub const MAX_ORACLE_KEY_BYTES: usize = 256;
pub const MAX_ORACLE_VALUE_BYTES: usize = 4_096;
pub const MAX_ORACLE_DIAGNOSTICS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OracleBounds {
    pub max_rows: usize,
    pub max_key_bytes: usize,
    pub max_value_bytes: usize,
    pub max_diagnostics: usize,
}

impl OracleBounds {
    pub const fn standard() -> Self {
        Self {
            max_rows: MAX_ORACLE_ROWS,
            max_key_bytes: MAX_ORACLE_KEY_BYTES,
            max_value_bytes: MAX_ORACLE_VALUE_BYTES,
            max_diagnostics: MAX_ORACLE_DIAGNOSTICS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleSourceDescriptor {
    pub schema: String,
    pub repository: String,
    pub revision: String,
    pub adapter_version: String,
    pub backend_format: String,
    pub imported_scope: Vec<String>,
    pub build_inputs: Vec<String>,
    pub notice_refs: Vec<String>,
    pub contract_refs: Vec<String>,
    pub remotes_enabled: bool,
    pub vec1_enabled: bool,
    pub build_ref: String,
    pub bounds: OracleBounds,
}
