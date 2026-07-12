use super::model::ComponentDenial;
use super::model::ComponentDenialClass;
use super::model::ComponentProfileExport;
use super::model::ComponentResult;
use super::model::ComponentRuntimeProfile;
use super::model::EvidenceScope;

pub const COMPONENT_PROFILE_SCHEMA: &str = "molten.wasm-component-profile.v1";
pub const COMPONENT_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const COMPONENT_PROFILE_SOURCE_LANGUAGE: &str = "nickel";
pub const COMPONENT_PROFILE_ID: &str = "molten.wasm.component.v1";
pub const COMPONENT_RUNTIME_STRATEGY: &str = "portable-component";
pub const COMPONENT_WASMTIME_VERSION: &str = "45.0.0";
pub const COMPONENT_WASM_TOOLS_VERSION: &str = "1.240.0";
pub const COMPONENT_WASMPARSER_VERSION: &str = "0.240.0";
pub const COMPONENT_WIT_BINDGEN_VERSION: &str = "0.46.0";
pub const COMPONENT_WASMTIME_WASI_VERSION: &str = "45.0.0";
pub const COMPONENT_WASI_PACKAGE_VERSION: &str = "0.2.6";
pub const COMPONENT_WIT_PACKAGE: &str = "molten:component-runtime@1.0.0";
pub const COMPONENT_WIT_WORLD: &str = "actor";
pub const COMPONENT_WIT_SOURCE_REF: &str = "blake3:83be349bb27975ada30dbe60817c5404df7862babdf02ae229b399886e76d5e8";
pub const COMPONENT_FUEL_LIMIT: u64 = 100_000;
pub const COMPONENT_MAX_BYTES: u64 = 4_194_304;
pub const COMPONENT_MAX_WIT_BYTES: u64 = 65_536;
pub const COMPONENT_MAX_MEMORY_BYTES: u64 = 16_777_216;
pub const COMPONENT_MAX_TABLE_ELEMENTS: u64 = 1_024;
pub const COMPONENT_MAX_INSTANCES: u64 = 16;
pub const COMPONENT_MAX_MEMORIES: u64 = 4;
pub const COMPONENT_MAX_TABLES: u64 = 4;
pub const COMPONENT_MAX_STACK_BYTES: u64 = 524_288;
pub const COMPONENT_MAX_HOSTCALL_BYTES: u64 = 65_536;
pub const COMPONENT_MAX_RESULT_BYTES: u64 = 65_536;
pub const COMPONENT_MAX_CONCURRENCY: u64 = 1;
pub const COMPONENT_MAX_IMPORTS: u64 = 128;
pub const COMPONENT_MAX_EXPORTS: u64 = 128;
pub const COMPONENT_FIXED_GROWTH: &str = "fixed";
pub const COMPONENT_RECORDED_HOST_INPUTS: &str = "recorded-effects-only";

pub const COMPONENT_NON_CLAIMS: &[&str] = &[
    "not-behavioral-correctness",
    "not-semantic-equivalence",
    "not-authority",
    "not-release-eligibility",
    "not-whole-system-safety",
    "not-build-trust-transfer",
];

const PROFILE_EXPORT_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/wasm-component-runtime/generated/profile.json"));

pub fn supported_component_profile() -> ComponentResult<ComponentRuntimeProfile> {
    let exported: ComponentProfileExport = serde_json::from_str(PROFILE_EXPORT_JSON).map_err(|error| {
        ComponentDenial::classified(
            ComponentDenialClass::ProfileDenial,
            format!("component profile export is invalid: {error}"),
        )
    })?;
    validate_export(&exported)?;
    Ok(exported.profile)
}

pub fn component_profile_ref(profile: &ComponentRuntimeProfile) -> String {
    let mut lines = vec![
        format!("profile-id:{}", profile.profile_id),
        format!("evidence-scope:{}", profile.evidence_scope.as_str()),
        format!("runtime-strategy:{}", profile.runtime_strategy),
        format!("wasmtime:{}", profile.toolchain.wasmtime),
        format!("wasm-tools:{}", profile.toolchain.wasm_tools),
        format!("wasmparser:{}", profile.toolchain.wasmparser),
        format!("wit-bindgen:{}", profile.toolchain.wit_bindgen),
        format!("wasmtime-wasi:{}", profile.toolchain.wasmtime_wasi),
        format!("wasi-package:{}", profile.toolchain.wasi_package),
        format!("wit-package:{}", profile.wit.package),
        format!("wit-world:{}", profile.wit.world),
        format!("wit-source-ref:{}", profile.wit.source_ref),
        format!("features:{:?}", profile.features),
        format!("determinism:{:?}", profile.determinism),
        format!("resources:{:?}", profile.resources),
    ];
    let mut imports = profile.allowed_imports.clone();
    imports.sort();
    lines.extend(imports.into_iter().map(|value| format!("import:{value}")));
    let mut wasi = profile.allowed_wasi_interfaces.clone();
    wasi.sort();
    lines.extend(wasi.into_iter().map(|value| format!("wasi:{value}")));
    lines.extend(profile.non_claims.iter().map(|value| format!("non-claim:{value}")));
    super::model::content_ref(lines.join("\n").as_bytes())
}

pub fn validate_component_profile(profile: &ComponentRuntimeProfile) -> ComponentResult<()> {
    let mut blockers = Vec::new();
    require_equal(&mut blockers, "profile id", &profile.profile_id, COMPONENT_PROFILE_ID);
    if profile.evidence_scope != EvidenceScope::Production {
        blockers.push("component profile evidence scope must be production".to_string());
    }
    require_equal(&mut blockers, "runtime strategy", &profile.runtime_strategy, COMPONENT_RUNTIME_STRATEGY);
    validate_toolchain(profile, &mut blockers);
    validate_wit(profile, &mut blockers);
    validate_features(profile, &mut blockers);
    validate_determinism(profile, &mut blockers);
    validate_resources(profile, &mut blockers);
    if !profile.allowed_imports.is_empty() || !profile.allowed_wasi_interfaces.is_empty() {
        blockers.push("initial component cohort must deny every host and WASI import".to_string());
    }
    let expected_non_claims = COMPONENT_NON_CLAIMS.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    if profile.non_claims != expected_non_claims {
        blockers.push("component profile non-claims do not match the reviewed set".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(ComponentDenial::from_blockers(blockers))
    }
}

fn validate_export(exported: &ComponentProfileExport) -> ComponentResult<()> {
    let mut blockers = Vec::new();
    require_equal(&mut blockers, "schema id", &exported.schema_id, COMPONENT_PROFILE_SCHEMA);
    if exported.schema_version != COMPONENT_PROFILE_SCHEMA_VERSION {
        blockers.push("component profile schema version is unsupported".to_string());
    }
    require_equal(&mut blockers, "source language", &exported.source_language, COMPONENT_PROFILE_SOURCE_LANGUAGE);
    if !blockers.is_empty() {
        return Err(ComponentDenial::from_blockers(blockers));
    }
    validate_component_profile(&exported.profile)
}

fn validate_toolchain(profile: &ComponentRuntimeProfile, blockers: &mut Vec<String>) {
    for (label, actual, expected) in [
        ("wasmtime", profile.toolchain.wasmtime.as_str(), COMPONENT_WASMTIME_VERSION),
        ("wasm-tools", profile.toolchain.wasm_tools.as_str(), COMPONENT_WASM_TOOLS_VERSION),
        ("wasmparser", profile.toolchain.wasmparser.as_str(), COMPONENT_WASMPARSER_VERSION),
        ("wit-bindgen", profile.toolchain.wit_bindgen.as_str(), COMPONENT_WIT_BINDGEN_VERSION),
        ("wasmtime-wasi", profile.toolchain.wasmtime_wasi.as_str(), COMPONENT_WASMTIME_WASI_VERSION),
        ("WASI package", profile.toolchain.wasi_package.as_str(), COMPONENT_WASI_PACKAGE_VERSION),
    ] {
        require_equal(blockers, label, actual, expected);
    }
}

fn validate_wit(profile: &ComponentRuntimeProfile, blockers: &mut Vec<String>) {
    require_equal(blockers, "WIT package", &profile.wit.package, COMPONENT_WIT_PACKAGE);
    require_equal(blockers, "WIT world", &profile.wit.world, COMPONENT_WIT_WORLD);
    require_equal(blockers, "WIT source ref", &profile.wit.source_ref, COMPONENT_WIT_SOURCE_REF);
}

fn validate_features(profile: &ComponentRuntimeProfile, blockers: &mut Vec<String>) {
    let features = &profile.features;
    if !(features.component_model
        && features.multi_value
        && features.bulk_memory
        && features.reference_types
        && features.simd)
    {
        blockers.push("component profile omits a required Wasm feature".to_string());
    }
    let unsupported = [
        features.relaxed_simd,
        features.threads,
        features.tail_call,
        features.multi_memory,
        features.exceptions,
        features.gc,
        features.memory64,
        features.extended_const,
        features.function_references,
        features.custom_page_sizes,
        features.wide_arithmetic,
        features.component_async,
    ];
    if unsupported.into_iter().any(|enabled| enabled) {
        blockers.push("component profile enables an unsupported Wasm feature".to_string());
    }
}

fn validate_determinism(profile: &ComponentRuntimeProfile, blockers: &mut Vec<String>) {
    let determinism = &profile.determinism;
    if !(determinism.fuel_interruption && determinism.nan_canonicalization && determinism.relaxed_simd_deterministic) {
        blockers.push("component deterministic runtime controls are incomplete".to_string());
    }
    require_equal(blockers, "memory growth", &determinism.memory_growth, COMPONENT_FIXED_GROWTH);
    require_equal(blockers, "table growth", &determinism.table_growth, COMPONENT_FIXED_GROWTH);
    require_equal(blockers, "host inputs", &determinism.host_inputs, COMPONENT_RECORDED_HOST_INPUTS);
}

fn validate_resources(profile: &ComponentRuntimeProfile, blockers: &mut Vec<String>) {
    let actual = &profile.resources;
    let expected = [
        ("fuel", actual.fuel, COMPONENT_FUEL_LIMIT),
        ("component bytes", actual.max_component_bytes, COMPONENT_MAX_BYTES),
        ("WIT bytes", actual.max_wit_bytes, COMPONENT_MAX_WIT_BYTES),
        ("memory bytes", actual.max_memory_bytes, COMPONENT_MAX_MEMORY_BYTES),
        ("table elements", actual.max_table_elements, COMPONENT_MAX_TABLE_ELEMENTS),
        ("instances", actual.max_instances, COMPONENT_MAX_INSTANCES),
        ("memories", actual.max_memories, COMPONENT_MAX_MEMORIES),
        ("tables", actual.max_tables, COMPONENT_MAX_TABLES),
        ("stack bytes", actual.max_stack_bytes, COMPONENT_MAX_STACK_BYTES),
        ("hostcall bytes", actual.max_hostcall_bytes, COMPONENT_MAX_HOSTCALL_BYTES),
        ("result bytes", actual.max_result_bytes, COMPONENT_MAX_RESULT_BYTES),
        ("concurrency", actual.max_concurrency, COMPONENT_MAX_CONCURRENCY),
        ("imports", actual.max_imports, COMPONENT_MAX_IMPORTS),
        ("exports", actual.max_exports, COMPONENT_MAX_EXPORTS),
    ];
    for (label, actual, expected) in expected {
        if actual != expected {
            blockers.push(format!("component profile {label} limit is unsupported"));
        }
    }
}

fn require_equal(blockers: &mut Vec<String>, label: &str, actual: &str, expected: &str) {
    if actual != expected {
        blockers.push(format!("component profile {label} must be {expected}, got {actual}"));
    }
}
