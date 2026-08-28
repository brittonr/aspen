use molten_core::world_state_oracle::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleCaseRequest {
    pub case: OracleCaseKind,
    pub database_id: String,
    pub branch: Option<String>,
    pub rows: Vec<SemanticStateRow>,
    pub mutation_order: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OraclePortError {
    pub code: &'static str,
    pub detail: String,
    pub outcome_unknown: bool,
}

impl OraclePortError {
    pub fn new(code: &'static str, detail: impl Into<String>, outcome_unknown: bool) -> Self {
        Self {
            code,
            detail: detail.into(),
            outcome_unknown,
        }
    }
}

pub type OraclePortResult<T> = std::result::Result<T, OraclePortError>;

// r[impl molten.world_state_oracle.boundary]
pub trait SemanticStateOracle {
    fn execute_case(&mut self, request: &OracleCaseRequest) -> OraclePortResult<OracleObservation>;
}
