pub const SCHEMA: &str = "molten.capability-runtime-execution.v1";
pub const PROFILE: &str = "harness-simulation-v1";
pub const FAMILY: &str = "harness-simulation";
pub const REQUIREMENT: &str = "molten.testing.capability_context.request_binding";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub source_ref: String,
    pub build_ref: String,
    pub dependency_closure_ref: String,
    pub feature_set: Vec<String>,
    pub execution_profile: String,
    pub adapter_refs: Vec<String>,
    pub platform: String,
    pub environment: Environment,
}

#[derive(
    Debug,
    Clone,
    Copy,
    serde::Deserialize,
    serde::Serialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord
)]
#[serde(rename_all = "kebab-case")]
pub enum Environment {
    Simulation,
    SameProcess,
    DistinctProcess,
    NixosVm,
    CrossHost,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Scenario {
    GrantedSend,
    DeniedSend,
}

impl Scenario {
    pub fn id(self) -> &'static str {
        match self {
            Self::GrantedSend => "granted-send",
            Self::DeniedSend => "denied-send",
        }
    }
    pub fn coverage_kind(self) -> &'static str {
        match self {
            Self::GrantedSend => "positive",
            Self::DeniedSend => "negative",
        }
    }
    pub fn parse(value: &str) -> crate::error::Result<Self> {
        match value {
            "granted-send" => Ok(Self::GrantedSend),
            "denied-send" => Ok(Self::DeniedSend),
            _ => Err(super::invalid("unsupported fixed qualification scenario")),
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedSubject {
    pub subject: Subject,
    pub binding_ref: String,
    pub validation_ref: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedReadback {
    pub subject: Subject,
    pub binding_ref: String,
    pub validation_ref: String,
    pub receipt_refs: std::collections::BTreeMap<String, String>,
}

pub struct Produced {
    pub value: preserves::IOValue,
    pub receipt_ref: String,
    pub subject: Subject,
    pub decision: &'static str,
}

#[derive(Debug)]
pub struct Validated {
    pub requirement_id: &'static str,
    pub family: &'static str,
    pub scenario: Scenario,
    pub subject: Subject,
    pub decision: &'static str,
    pub canonical_decision: &'static str,
}

pub fn adapter_ref() -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("capability-harness-adapter-v1", vec![
        crate::preserves_rail::string("native-runtime-core"),
        crate::preserves_rail::string("deterministic-logical-time-seeded-rng-no-live-ports"),
    ]))
}

pub(super) fn subject(binding: &crate::core_api::startup_binding::Binding) -> crate::error::Result<Subject> {
    Ok(Subject {
        source_ref: binding.source_ref.clone(),
        build_ref: binding.executable_ref.clone(),
        dependency_closure_ref: binding.dependencies_ref.clone(),
        feature_set: features()?,
        execution_profile: PROFILE.to_string(),
        adapter_refs: vec![adapter_ref()?],
        platform: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        environment: Environment::Simulation,
    })
}

pub(super) fn features() -> crate::error::Result<Vec<String>> {
    let cohort = [
        ("doltlite-oracle", cfg!(feature = "doltlite-oracle")),
        ("component-rpc", cfg!(feature = "component-rpc")),
        ("component-rpc-telemetry", cfg!(feature = "component-rpc-telemetry")),
        ("executable-extents", cfg!(feature = "executable-extents")),
        ("marble-object-store", cfg!(feature = "marble-object-store")),
        ("profiler", cfg!(feature = "profiler")),
        ("profiler-alloc", cfg!(feature = "profiler-alloc")),
        ("profiler-disabled", cfg!(feature = "profiler-disabled")),
        ("profiler-perf", cfg!(feature = "profiler-perf")),
        ("spacewasm-pilot", cfg!(feature = "spacewasm-pilot")),
        ("world-snapshot-vm-cohort", cfg!(feature = "world-snapshot-vm-cohort")),
        ("content-store-object-storage", cfg!(feature = "content-store-object-storage")),
        ("test-fixtures", cfg!(feature = "test-fixtures")),
    ];
    let manifest: toml::Value = toml::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")))
        .map_err(|_| super::invalid("compiled Cargo feature manifest cannot be decoded"))?;
    let declared = manifest
        .get("features")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| super::invalid("compiled Cargo feature inventory missing"))?;
    if declared.len() != cohort.len()
        || declared.keys().any(|name| !cohort.iter().any(|(known, _)| name.as_str() == *known))
    {
        return Err(super::invalid(
            "compiled feature inventory drift; qualification producer requires explicit update",
        ));
    }
    let target_dependencies = manifest
        .get("target")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|targets| targets.values())
        .filter_map(|target| target.get("dependencies").and_then(toml::Value::as_table));
    for dependencies in
        manifest.get("dependencies").and_then(toml::Value::as_table).into_iter().chain(target_dependencies)
    {
        for (name, dependency) in dependencies {
            let is_optional = dependency.get("optional").and_then(toml::Value::as_bool) == Some(true);
            let is_explicitly_hidden = declared
                .values()
                .filter_map(toml::Value::as_array)
                .flatten()
                .filter_map(toml::Value::as_str)
                .any(|feature| feature.strip_prefix("dep:") == Some(name.as_str()));
            if is_optional && !declared.contains_key(name) && !is_explicitly_hidden {
                return Err(super::invalid(
                    "implicit optional Cargo feature has no compiled observation; qualification denied",
                ));
            }
        }
    }
    let mut result: Vec<_> =
        cohort.into_iter().filter(|(_, enabled)| *enabled).map(|(name, _)| name.to_string()).collect();
    result.sort();
    Ok(result)
}
