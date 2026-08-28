use super::*;

const REQUIRED_SCOPE: [&str; 8] = [
    "flake-input:full-pinned-source-checkout",
    "build:non-amalgamation-cli-and-library",
    "src/doltlite_*.c",
    "src/prolly_*.c",
    "src/chunk_*.c",
    "test/sqlite_compatibility_contract.tsv",
    "test/concurrency_contract.tsv",
    "test/storage_format_contract.tsv",
];
const REQUIRED_BUILD_INPUTS: [&str; 3] = ["stdenv.cc", "gnumake", "zlib"];
const REQUIRED_NOTICES: [&str; 2] = [DOLTLITE_APACHE_REF, DOLTLITE_LICENSE_REF];
const REQUIRED_CONTRACTS: [&str; 3] = [
    DOLTLITE_SQLITE_CONTRACT_REF,
    DOLTLITE_CONCURRENCY_CONTRACT_REF,
    DOLTLITE_FORMAT_CONTRACT_REF,
];

// r[impl molten.world_state_oracle.source]
pub fn standard_source_descriptor(build_ref: String) -> OracleSourceDescriptor {
    OracleSourceDescriptor {
        schema: ORACLE_SOURCE_SCHEMA.to_string(),
        repository: DOLTLITE_REPOSITORY.to_string(),
        revision: DOLTLITE_REVISION.to_string(),
        adapter_version: DOLTLITE_ADAPTER_VERSION.to_string(),
        backend_format: DOLTLITE_BACKEND_FORMAT.to_string(),
        imported_scope: REQUIRED_SCOPE.iter().map(|value| (*value).to_string()).collect(),
        build_inputs: REQUIRED_BUILD_INPUTS.iter().map(|value| (*value).to_string()).collect(),
        notice_refs: REQUIRED_NOTICES.iter().map(|value| (*value).to_string()).collect(),
        contract_refs: REQUIRED_CONTRACTS.iter().map(|value| (*value).to_string()).collect(),
        remotes_enabled: false,
        vec1_enabled: false,
        build_ref,
        bounds: OracleBounds::standard(),
    }
}

// r[impl molten.world_state_oracle.source]
pub fn validate_source_descriptor(descriptor: &OracleSourceDescriptor) -> Vec<OracleIssue> {
    let mut issues = Vec::with_capacity(MAX_ORACLE_DIAGNOSTICS);
    if descriptor.schema != ORACLE_SOURCE_SCHEMA {
        issues.push(OracleIssue::SchemaMismatch);
    }
    if descriptor.repository != DOLTLITE_REPOSITORY || descriptor.revision != DOLTLITE_REVISION {
        issues.push(OracleIssue::SourceMismatch);
    }
    if descriptor.adapter_version != DOLTLITE_ADAPTER_VERSION {
        issues.push(OracleIssue::AdapterMismatch);
    }
    if descriptor.backend_format != DOLTLITE_BACKEND_FORMAT {
        issues.push(OracleIssue::BackendFormatMismatch);
    }
    if descriptor.remotes_enabled {
        issues.push(OracleIssue::RemoteSupportEnabled);
    }
    if descriptor.vec1_enabled {
        issues.push(OracleIssue::Vec1Unexpected);
    }
    if descriptor.imported_scope != REQUIRED_SCOPE {
        issues.push(OracleIssue::ImportedScopeMismatch);
    }
    if descriptor.build_inputs != REQUIRED_BUILD_INPUTS {
        issues.push(OracleIssue::BuildInputMismatch);
    }
    if descriptor.notice_refs != REQUIRED_NOTICES {
        issues.push(OracleIssue::NoticeMismatch);
    }
    if descriptor.contract_refs != REQUIRED_CONTRACTS {
        issues.push(OracleIssue::ContractMismatch);
    }
    for reference in descriptor
        .notice_refs
        .iter()
        .chain(&descriptor.contract_refs)
        .chain(core::iter::once(&descriptor.build_ref))
    {
        if !is_blake3_ref(reference) {
            issues.push(OracleIssue::MalformedReference(reference.clone()));
        }
    }
    if !valid_bounds(descriptor.bounds) {
        issues.push(OracleIssue::InvalidBounds);
    }
    issues.sort();
    issues.dedup();
    issues
}

fn valid_bounds(bounds: OracleBounds) -> bool {
    bounds.max_rows > 0
        && bounds.max_rows <= MAX_ORACLE_ROWS
        && bounds.max_key_bytes > 0
        && bounds.max_key_bytes <= MAX_ORACLE_KEY_BYTES
        && bounds.max_value_bytes > 0
        && bounds.max_value_bytes <= MAX_ORACLE_VALUE_BYTES
        && bounds.max_diagnostics > 0
        && bounds.max_diagnostics <= MAX_ORACLE_DIAGNOSTICS
}

pub fn is_blake3_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("blake3:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
