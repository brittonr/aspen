use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use bounded_exec::CommandSpec;
use bounded_exec::Completion;
use bounded_exec::Disposition;
use bounded_exec::EnvironmentMode;
use bounded_exec::ExecutionLimits;
use bounded_exec::Input;
use bounded_exec::OutcomePolicy;
use bounded_exec::RunRequest;
use bounded_exec::TerminationScope;
use molten_core::world_state_oracle::*;

use super::*;

const ORACLE_TIMEOUT_MS: u64 = 10_000;
const ORACLE_POLL_INTERVAL_MS: u64 = 10;
const ORACLE_TEARDOWN_TIMEOUT_MS: u64 = 1_000;
const ORACLE_STDIN_BYTES: usize = 65_536;
const ORACLE_STDOUT_BYTES: usize = 262_144;
const ORACLE_STDERR_BYTES: usize = 65_536;
const ACCEPTED_EXIT_CODE: i32 = 0;
const MAX_DATABASE_ID_BYTES: usize = 96;

#[derive(Debug)]
pub struct DoltLiteProcessOracle {
    executable: PathBuf,
    workspace: PathBuf,
    source: OracleSourceDescriptor,
    adapter_ref: String,
}

impl DoltLiteProcessOracle {
    pub fn new(
        executable: PathBuf,
        workspace: PathBuf,
        source: OracleSourceDescriptor,
        adapter_ref: String,
    ) -> OraclePortResult<Self> {
        if !executable.is_absolute() || !workspace.is_absolute() {
            return Err(OraclePortError::new(
                "oracle-path-not-absolute",
                "oracle executable and workspace must be absolute",
                false,
            ));
        }
        let issues = validate_source_descriptor(&source);
        if !issues.is_empty() {
            return Err(OraclePortError::new(
                "oracle-source-denied",
                format!("source descriptor denied: {issues:?}"),
                false,
            ));
        }
        if !is_blake3_ref(&adapter_ref) {
            return Err(OraclePortError::new("oracle-adapter-ref-invalid", "adapter reference is invalid", false));
        }
        Ok(Self {
            executable,
            workspace,
            source,
            adapter_ref,
        })
    }

    fn run_case(&self, request: &OracleCaseRequest) -> OraclePortResult<ParsedOracleOutput> {
        validate_request(request, self.source.bounds)?;
        let database = self.workspace.join(format!("{}.db", request.database_id));
        remove_if_present(&database)?;
        match request.case {
            OracleCaseKind::HistoryIndependentState => self.run_sql(&database, &state_script(request, true)),
            OracleCaseKind::BranchIsolation => self.run_sql(&database, &branch_script(request)),
            OracleCaseKind::ReaderSafeGarbageCollection => self.run_sql(&database, &gc_script(request)),
            OracleCaseKind::ExactFormatReopen => {
                self.run_sql(&database, &state_script(request, true))?;
                self.run_sql(&database, &read_script(OracleOutcome::Applied))
            }
            OracleCaseKind::DetachedRead => self.run_detached_case(&database, request),
            OracleCaseKind::RemoteDisabled
            | OracleCaseKind::RowIdRejected
            | OracleCaseKind::CustomCollationRejected
            | OracleCaseKind::MultiFileWriteUnsupported => self.run_expected_denial(&database, request),
            _ => Ok(ParsedOracleOutput {
                branch: request.branch.clone(),
                rows: Vec::new(),
                outcome: OracleOutcome::Unsupported,
                backend_root: None,
                diagnostics: vec!["case-requires-reviewed-external-evidence".to_string()],
                commit_ref: None,
            }),
        }
    }

    fn run_detached_case(&self, database: &Path, request: &OracleCaseRequest) -> OraclePortResult<ParsedOracleOutput> {
        let first = self.run_sql(database, &detached_setup_script(request))?;
        let commit_ref = first.commit_ref.ok_or_else(|| {
            OraclePortError::new("oracle-detached-commit-missing", "setup did not emit a commit", true)
        })?;
        let qualified = PathBuf::from(format!("{}/{}", database.display(), commit_ref));
        let mut output = self.run_sql(&qualified, &read_script(OracleOutcome::ReadOnly))?;
        output.branch = None;
        Ok(output)
    }

    fn run_expected_denial(
        &self,
        database: &Path,
        request: &OracleCaseRequest,
    ) -> OraclePortResult<ParsedOracleOutput> {
        let script = denial_script(request.case, request);
        let output = self.execute(database, &script)?;
        let stderr = String::from_utf8_lossy(&output.stderr.bytes).to_string();
        let expected = expected_denial_needle(request.case);
        if output.disposition == Disposition::Succeeded || !stderr.contains(expected) {
            return Err(OraclePortError::new(
                "oracle-denial-mismatch",
                format!("expected denial containing {expected:?}, got {stderr:?}"),
                false,
            ));
        }
        Ok(ParsedOracleOutput {
            branch: request.branch.clone(),
            rows: Vec::new(),
            outcome: OracleOutcome::Rejected,
            backend_root: None,
            diagnostics: vec![denial_code(request.case).to_string()],
            commit_ref: None,
        })
    }

    fn run_sql(&self, database: &Path, script: &str) -> OraclePortResult<ParsedOracleOutput> {
        let output = self.execute(database, script)?;
        if output.completion != Completion::Exited || output.disposition != Disposition::Succeeded {
            return Err(OraclePortError::new(
                "oracle-process-failed",
                String::from_utf8_lossy(&output.stderr.bytes).to_string(),
                output.completion != Completion::Exited,
            ));
        }
        parse_output(&output.stdout.bytes)
    }

    fn execute(&self, database: &Path, script: &str) -> OraclePortResult<bounded_exec::ExecutionOutput> {
        let policy = OutcomePolicy::new(vec![ACCEPTED_EXIT_CODE], true, true)
            .map_err(|error| OraclePortError::new("oracle-outcome-policy-invalid", format!("{error:?}"), false))?;
        bounded_exec::run(RunRequest {
            command: CommandSpec {
                program: self.executable.clone(),
                args: vec![
                    OsString::from("-batch"),
                    OsString::from("-noheader"),
                    OsString::from("-separator"),
                    OsString::from("\t"),
                    database.as_os_str().to_os_string(),
                ],
                current_dir: self.workspace.clone(),
                environment_mode: EnvironmentMode::Clear,
                environment: Vec::new(),
                input: Input::Bytes(script.as_bytes().to_vec()),
            },
            limits: ExecutionLimits {
                timeout_ms: ORACLE_TIMEOUT_MS,
                stdin_max_bytes: ORACLE_STDIN_BYTES,
                stdout_max_bytes: ORACLE_STDOUT_BYTES,
                stderr_max_bytes: ORACLE_STDERR_BYTES,
                poll_interval_ms: ORACLE_POLL_INTERVAL_MS,
                teardown_timeout_ms: ORACLE_TEARDOWN_TIMEOUT_MS,
            },
            termination_scope: TerminationScope::ProcessGroup,
            outcome_policy: policy,
        })
        .map_err(|error| OraclePortError::new("oracle-process-mechanism", format!("{error:?}"), false))
    }
}

impl SemanticStateOracle for DoltLiteProcessOracle {
    // r[impl molten.world_state_oracle.boundary]
    fn execute_case(&mut self, request: &OracleCaseRequest) -> OraclePortResult<OracleObservation> {
        let parsed = self.run_case(request)?;
        build_oracle_observation(&self.source, OracleObservationInput {
            adapter_ref: self.adapter_ref.clone(),
            case: request.case,
            branch: parsed.branch,
            rows: parsed.rows,
            outcome: parsed.outcome,
            backend_root: parsed.backend_root,
            diagnostics: parsed.diagnostics,
        })
        .map_err(|issues| OraclePortError::new("oracle-observation-denied", format!("{issues:?}"), false))
    }
}

struct ParsedOracleOutput {
    branch: Option<String>,
    rows: Vec<SemanticStateRow>,
    outcome: OracleOutcome,
    backend_root: Option<String>,
    diagnostics: Vec<String>,
    commit_ref: Option<String>,
}

fn validate_request(request: &OracleCaseRequest, bounds: OracleBounds) -> OraclePortResult<()> {
    if !is_safe_database_id(&request.database_id) {
        return Err(OraclePortError::new("oracle-database-id-invalid", "database id is invalid", false));
    }
    if request.rows.len() > bounds.max_rows || request.mutation_order.len() != request.rows.len() {
        return Err(OraclePortError::new("oracle-case-bound", "row or mutation-order bound is invalid", false));
    }
    let mut seen = vec![false; request.rows.len()];
    for index in &request.mutation_order {
        let Some(slot) = seen.get_mut(*index) else {
            return Err(OraclePortError::new("oracle-mutation-order", "mutation index is out of range", false));
        };
        if *slot {
            return Err(OraclePortError::new("oracle-mutation-order", "mutation index is duplicated", false));
        }
        *slot = true;
    }
    for row in &request.rows {
        if row.key.is_empty() || row.key.len() > bounds.max_key_bytes || row.value.len() > bounds.max_value_bytes {
            return Err(OraclePortError::new("oracle-row-bound", "semantic row is invalid", false));
        }
    }
    Ok(())
}

pub(super) fn is_safe_database_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DATABASE_ID_BYTES
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn remove_if_present(path: &Path) -> OraclePortResult<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(OraclePortError::new("oracle-cleanup-failed", error.to_string(), false)),
    }
}

fn parse_output(bytes: &[u8]) -> OraclePortResult<ParsedOracleOutput> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| OraclePortError::new("oracle-output-utf8", error.to_string(), false))?;
    let mut parsed = ParsedOracleOutput {
        branch: None,
        rows: Vec::new(),
        outcome: OracleOutcome::Applied,
        backend_root: None,
        diagnostics: Vec::new(),
        commit_ref: None,
    };
    for line in text.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.as_slice() {
            ["ROW", key, value] => parsed.rows.push(SemanticStateRow {
                key: (*key).to_string(),
                value: (*value).to_string(),
            }),
            ["ROOT", root] => parsed.backend_root = Some((*root).to_string()),
            ["BRANCH", branch] => parsed.branch = Some((*branch).to_string()),
            ["COMMIT", commit] => parsed.commit_ref = Some((*commit).to_string()),
            ["OUTCOME", outcome] => parsed.outcome = parse_outcome(outcome)?,
            _ => {}
        }
    }
    parsed.rows.sort();
    Ok(parsed)
}

fn parse_outcome(value: &str) -> OraclePortResult<OracleOutcome> {
    match value {
        "applied" => Ok(OracleOutcome::Applied),
        "equal-state" => Ok(OracleOutcome::EqualState),
        "read-only" => Ok(OracleOutcome::ReadOnly),
        _ => Err(OraclePortError::new("oracle-outcome-unknown", value.to_string(), false)),
    }
}

fn state_script(request: &OracleCaseRequest, include_commit: bool) -> String {
    let mut sql = base_state_script(request, include_commit);
    sql.push_str(&read_script(OracleOutcome::EqualState));
    sql
}

fn base_state_script(request: &OracleCaseRequest, include_commit: bool) -> String {
    let mut sql = setup_prefix();
    sql.push_str("CREATE TABLE semantic_state (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;\n");
    for index in &request.mutation_order {
        let row = &request.rows[*index];
        sql.push_str(&format!(
            "INSERT INTO semantic_state(key,value) VALUES('{}','{}');\n",
            sql_text(&row.key),
            sql_text(&row.value)
        ));
    }
    if include_commit {
        sql.push_str("SELECT dolt_commit('-Am','semantic state');\n");
    }
    sql
}

fn branch_script(request: &OracleCaseRequest) -> String {
    let mut sql = base_state_script(request, true);
    sql.push_str("SELECT dolt_branch('feature');\nSELECT dolt_checkout('feature');\n");
    if let Some(row) = request.rows.first() {
        sql.push_str(&format!(
            "UPDATE semantic_state SET value='{}-feature' WHERE key='{}';\n",
            sql_text(&row.value),
            sql_text(&row.key)
        ));
    }
    sql.push_str("SELECT dolt_commit('-Am','feature');\nSELECT dolt_checkout('main');\n");
    sql.push_str(&read_script(OracleOutcome::Applied));
    sql
}

fn gc_script(request: &OracleCaseRequest) -> String {
    let mut sql = base_state_script(request, true);
    sql.push_str("SELECT dolt_gc();\n");
    sql.push_str(&read_script(OracleOutcome::Applied));
    sql
}

fn detached_setup_script(request: &OracleCaseRequest) -> String {
    let mut sql = setup_prefix();
    sql.push_str("CREATE TABLE semantic_state (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;\n");
    for row in &request.rows {
        sql.push_str(&format!(
            "INSERT INTO semantic_state(key,value) VALUES('{}','{}');\n",
            sql_text(&row.key),
            sql_text(&row.value)
        ));
    }
    sql.push_str("SELECT dolt_commit('-Am','detached base');\nSELECT 'COMMIT',dolt_hashof('HEAD');\n");
    if let Some(row) = request.rows.first() {
        sql.push_str(&format!(
            "UPDATE semantic_state SET value='{}-later' WHERE key='{}';\n",
            sql_text(&row.value),
            sql_text(&row.key)
        ));
    }
    sql.push_str("SELECT dolt_commit('-Am','later');\n");
    sql
}

fn read_script(outcome: OracleOutcome) -> String {
    format!(
        "SELECT 'ROOT',dolt_hashof_table('semantic_state');\nSELECT 'BRANCH',active_branch();\nSELECT 'ROW',key,value FROM semantic_state ORDER BY key;\nSELECT 'OUTCOME','{}';\n",
        outcome.as_str()
    )
}

fn denial_script(case: OracleCaseKind, request: &OracleCaseRequest) -> String {
    match case {
        OracleCaseKind::RemoteDisabled => "SELECT dolt_clone('file:///oracle-denied');\n".to_string(),
        OracleCaseKind::RowIdRejected => {
            "CREATE TABLE semantic_state(key TEXT PRIMARY KEY, value TEXT) WITHOUT ROWID; UPDATE semantic_state SET rowid=2;\n".to_string()
        }
        OracleCaseKind::CustomCollationRejected => {
            "CREATE TABLE semantic_state(key TEXT PRIMARY KEY COLLATE oracle_custom, value TEXT);\n".to_string()
        }
        OracleCaseKind::MultiFileWriteUnsupported => format!(
            "CREATE TABLE main_state(key TEXT PRIMARY KEY,value TEXT); ATTACH DATABASE '{}-other.db' AS other; CREATE TABLE other.other_state(key TEXT PRIMARY KEY,value TEXT); BEGIN; INSERT INTO main_state VALUES('a','1'); INSERT INTO other.other_state VALUES('b','2'); COMMIT;\n",
            sql_text(&request.database_id)
        ),
        _ => String::new(),
    }
}

fn expected_denial_needle(case: OracleCaseKind) -> &'static str {
    match case {
        OracleCaseKind::RemoteDisabled => "remotes are disabled",
        OracleCaseKind::RowIdRejected => "no such column",
        OracleCaseKind::CustomCollationRejected => "no such collation sequence",
        OracleCaseKind::MultiFileWriteUnsupported => {
            "atomic commit across multiple file-backed databases is not supported"
        }
        _ => "unsupported oracle denial case",
    }
}

fn denial_code(case: OracleCaseKind) -> &'static str {
    match case {
        OracleCaseKind::RemoteDisabled => "remote-disabled",
        OracleCaseKind::RowIdRejected => "rowid-rejected",
        OracleCaseKind::CustomCollationRejected => "custom-collation-rejected",
        OracleCaseKind::MultiFileWriteUnsupported => "multi-file-write-unsupported",
        _ => "unsupported-denial",
    }
}

fn setup_prefix() -> String {
    "SELECT dolt_config('user.name','Molten Oracle');\nSELECT dolt_config('user.email','oracle@invalid');\n".to_string()
}

fn sql_text(value: &str) -> String {
    value.replace('\'', "''")
}
