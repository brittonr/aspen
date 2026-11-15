use serde::{Deserialize, Serialize};

pub use molten::capability_qualification::{Environment, Subject};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema: String,
    pub source_root_path: String,
    pub subject: Subject,
    pub capabilities: Vec<CapabilityInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityInput { pub id: String, pub evidence: Vec<Evidence> }

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub requirement_id: String,
    pub coverage_kind: String,
    pub family: String,
    pub scenario: String,
    pub subject: Subject,
    pub artifact_path: String,
    pub artifact_ref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping { pub schema: String, pub capabilities: Vec<Capability> }

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    pub requirements: Vec<String>,
    pub owners: Vec<String>,
    pub profiles: Vec<String>,
    pub families: Vec<Family>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    pub id: String,
    pub environment: Environment,
    pub matrix_scopes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Matrix {
    pub schema: String,
    pub entries: Vec<MatrixEntry>,
    #[serde(default)]
    pub exemptions: Vec<Exemption>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct MatrixEntry {
    pub requirement_id: String,
    pub coverage_kind: String,
    pub evidence_scope: String,
    pub target: String,
    pub command: String,
    #[serde(default)]
    pub artifact_refs: Vec<String>,
    pub receipt_ref: String,
    #[serde(default)]
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Exemption {
    pub requirement_id: String,
    pub reason_class: String,
    pub evidence_ref: String,
    pub scope: String,
    pub review_note: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Observation {
    pub requirement_id: String,
    pub coverage_kind: String,
    pub family: String,
    pub scenario: String,
    pub artifact_ref: String,
    pub native_decision: String,
    pub canonical_decision: String,
    pub validated: bool,
    pub exact_subject_bound: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub subject: Subject,
    pub capabilities: Vec<CapabilityReport>,
    pub non_claim: &'static str,
}

#[derive(Debug, Serialize)]
pub struct CapabilityReport {
    pub definition: Capability,
    pub implementation: Vec<MatrixEntry>,
    pub reviewed_exemptions: Vec<Exemption>,
    pub execution: Execution,
}

#[derive(Debug, Serialize)]
pub struct Execution {
    pub status: &'static str,
    pub evidence: Vec<Observation>,
    pub missing: Vec<String>,
    pub diagnostics: Vec<String>,
}
