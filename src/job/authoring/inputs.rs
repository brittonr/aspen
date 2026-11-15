/// A captured module is inert source, not an ambient filesystem import.
#[derive(Debug, Clone)]
pub struct CapturedJobModule {
    pub key: String,
    pub source: String,
}

pub struct CapturedNickelJobInput<'a> {
    pub source_name: &'a str,
    /// A Nickel function `fun modules => fun parameters => closed_graph`.
    pub source: &'a str,
    pub modules: &'a [CapturedJobModule],
    pub parameters: &'a serde_json::Value,
    /// Exact explicitly selected cohort executable and Linux resource limiter.
    pub nickel: &'a FilePath,
    pub prlimit: &'a FilePath,
    pub expected_evaluator_ref: &'a str,
    pub expected_limiter_ref: &'a str,
    pub workspace: &'a FilePath,
}

pub struct LoweredNickelJob {
    pub dag: JobDag,
    pub provenance: IoValue,
    pub origins: Vec<JobGraphOrigin>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobGraphOrigin {
    pub instance: String,
    pub template: String,
    pub module: String,
    #[serde(default = "absent_origin_port")]
    pub port: Option<String>,
}

fn absent_origin_port() -> Option<String> { None }

const MAX_JOB_AUTHORING_BYTES: usize = 1_048_576;

fn captured_source_check(source: &str) -> Result<()> {
    ensure_count_at_most(source.len(), MAX_JOB_AUTHORING_BYTES, "captured Nickel source bytes")?;
    // Deliberately conservative restricted route, including comments and strings.
    // No lexical rewriting of imports and no filesystem resolver is enabled.
    if source.contains("import") || source.contains("%{") {
        return Err(MoltenError::invalid_harness("captured Nickel forbids raw import/interpolation; select explicit modules"));
    }
    Ok(())
}

include!("inputs/parameters.rs");
include!("inputs/literal.rs");
