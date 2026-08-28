use std::collections::BTreeSet;

pub const TRELLIS_PROOF_REFERENCE_REVISION: &str = "0bf65150d4c75da5887d5cc53392c3da6b94b9d2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofObligationStatus {
    ModelChecked,
    OpenFormal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProllyProofObligation {
    pub id: &'static str,
    pub target_owner: &'static str,
    pub statement: &'static str,
    pub assumptions: &'static [&'static str],
    pub status: ProofObligationStatus,
    pub proves_database_correctness: bool,
    pub proves_collision_impossibility: bool,
}

// r[impl molten.prolly_map.proof_boundary]
pub fn standard_proof_obligations() -> Vec<ProllyProofObligation> {
    vec![
        obligation(
            "prolly.sorted-unique",
            "canonical build preserves strict key order and uniqueness",
            &["input entries are admitted canonical bytes"],
            ProofObligationStatus::ModelChecked,
        ),
        obligation(
            "prolly.search-containment",
            "point and range reads return exactly entries within the requested key interval",
            &["all supplied node bytes pass canonical validation"],
            ProofObligationStatus::ModelChecked,
        ),
        obligation(
            "prolly.boundary-determinism",
            "equal profile key and encoded-size inputs yield equal split decisions",
            &["content identity framing is deterministic"],
            ProofObligationStatus::ModelChecked,
        ),
        obligation(
            "prolly.edit-preservation",
            "admitted edits preserve canonical ordering and yield the canonical equal-state root",
            &["the rebuild-first pilot receives a complete valid prior snapshot"],
            ProofObligationStatus::ModelChecked,
        ),
        obligation(
            "prolly.diff-soundness",
            "complete diff records exactly added removed and modified entries",
            &["equal BLAKE3 node identities are treated as equal under collision resistance"],
            ProofObligationStatus::ModelChecked,
        ),
        obligation(
            "prolly.reachability",
            "complete supplied graph facts classify every node reachable from roots or pins",
            &["graph facts are complete and bind the supplied node identities"],
            ProofObligationStatus::ModelChecked,
        ),
        obligation(
            "prolly.formal-refinement",
            "future Trellis proofs refine the production-linked Rust profile and node codecs",
            &["a reviewed production linkage is supplied"],
            ProofObligationStatus::OpenFormal,
        ),
    ]
}

pub fn validate_proof_obligations(obligations: &[ProllyProofObligation]) -> bool {
    let mut ids = BTreeSet::new();
    !obligations.is_empty()
        && obligations.iter().all(|obligation| {
            ids.insert(obligation.id)
                && obligation.target_owner == "trellis"
                && !obligation.statement.is_empty()
                && !obligation.assumptions.is_empty()
                && !obligation.proves_database_correctness
                && !obligation.proves_collision_impossibility
        })
}

fn obligation(
    id: &'static str,
    statement: &'static str,
    assumptions: &'static [&'static str],
    status: ProofObligationStatus,
) -> ProllyProofObligation {
    ProllyProofObligation {
        id,
        target_owner: "trellis",
        statement,
        assumptions,
        status,
        proves_database_correctness: false,
        proves_collision_impossibility: false,
    }
}
