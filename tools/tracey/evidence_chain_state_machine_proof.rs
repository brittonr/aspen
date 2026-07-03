//! Tracey markers for evidence-chain state-machine proof coverage.
//!
//! Implementation and tests live in `src/evidence/parts/chain/**`; Aspen's
//! current Cairn Tracey scanner records references from `tools/**`, so this
//! file anchors accepted evidence-chain proof IDs to focused tests.
//!
//! Verification evidence:
//! - `src/evidence/parts/chain/tests/m000/p004/body.rs`

// r[verify molten.evidence_chain_state_machine_proof.head_transition_continuity]
// r[verify molten.evidence_chain_state_machine_proof.gap_fork_denial]
// r[verify molten.evidence_chain_state_machine_proof.checkpoint_anchor_preservation]
