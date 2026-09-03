//! Focused strict-Octet compilation surface for the pure Prolly map.

#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![forbid(unsafe_code)]

pub mod world_benchmark {
    #![allow(
        tigerstyle::path_segment_repetition,
        reason = "compatibility stubs preserve exact published benchmark API names"
    )]

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WorldBenchmarkReceipt;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WorldBenchmarkIssue;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WorldBenchmarkExtractionEvidence {
        pub receipt: WorldBenchmarkReceipt,
        pub owned_adapter: bool,
        pub product_neutral_limit_failed: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WorldBenchmarkExtractionPolicy {
        pub minimum_accepted_receipts_per_consumer: u32,
        pub minimum_credible_consumers: u32,
        pub require_product_neutral_limit: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WorldBenchmarkExtractionDecision;

    pub fn classify_world_benchmark_extraction(
        _evidence: &[WorldBenchmarkExtractionEvidence],
        _policy: &WorldBenchmarkExtractionPolicy,
    ) -> Result<WorldBenchmarkExtractionDecision, Vec<WorldBenchmarkIssue>> {
        Ok(WorldBenchmarkExtractionDecision)
    }
}

pub mod world_state_oracle {
    #![allow(
        tigerstyle::path_segment_repetition,
        reason = "compatibility stubs preserve exact published oracle API names"
    )]

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OracleOutcome {
        Applied,
        EqualState,
        Unsupported,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SemanticStateRow {
        pub key: String,
        pub value: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OracleObservation {
        pub observation_ref: String,
        pub rows: Vec<SemanticStateRow>,
        pub outcome: OracleOutcome,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OracleBounds;

    impl OracleBounds {
        pub const fn standard() -> Self {
            Self
        }
    }

    pub fn validate_observation(
        _observation: &OracleObservation,
        _bounds: OracleBounds,
        _require_identity: bool,
    ) -> Vec<String> {
        Vec::new()
    }
}

pub mod prolly_map;
