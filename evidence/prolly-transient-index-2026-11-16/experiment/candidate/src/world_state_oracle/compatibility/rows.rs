use super::super::*;

#[derive(Debug, Clone, Copy)]
struct CompatibilityRowSpec {
    id: &'static str,
    source_contract: &'static str,
    status: CompatibilityStatus,
    evidence_ref: &'static str,
    fixture: &'static str,
    issue: Option<&'static str>,
    explanation: &'static str,
}

const COMPATIBLE_ROWS: [CompatibilityRowSpec; 8] = [
    CompatibilityRowSpec {
        id: "branch-isolation",
        source_contract: "concurrency.per-connection-branch",
        status: CompatibilityStatus::Compatible,
        evidence_ref: DOLTLITE_CONCURRENCY_CONTRACT_REF,
        fixture: "branch-isolation",
        issue: None,
        explanation: "branch-visible state stays isolated",
    },
    CompatibilityRowSpec {
        id: "compare-and-advance",
        source_contract: "concurrency.vc-head-recheck",
        status: CompatibilityStatus::Compatible,
        evidence_ref: DOLTLITE_CONCURRENCY_CONTRACT_REF,
        fixture: "compare-and-advance",
        issue: None,
        explanation: "stale tips do not replace a winner",
    },
    CompatibilityRowSpec {
        id: "detached-read",
        source_contract: "branch.detached-revision",
        status: CompatibilityStatus::Compatible,
        evidence_ref: DOLTLITE_SQLITE_CONTRACT_REF,
        fixture: "detached-read",
        issue: None,
        explanation: "detached snapshots remain read-only",
    },
    CompatibilityRowSpec {
        id: "exact-format",
        source_contract: "storage.exact-version",
        status: CompatibilityStatus::Compatible,
        evidence_ref: DOLTLITE_FORMAT_CONTRACT_REF,
        fixture: "exact-format",
        issue: None,
        explanation: "format version twelve reopens and other versions deny",
    },
    CompatibilityRowSpec {
        id: "history-independent-state",
        source_contract: "storage.history-independent-table-root",
        status: CompatibilityStatus::Compatible,
        evidence_ref: DOLTLITE_FORMAT_CONTRACT_REF,
        fixture: "history-independent",
        issue: None,
        explanation: "equal primary-key state has equal backend roots",
    },
    CompatibilityRowSpec {
        id: "reader-safe-gc",
        source_contract: "concurrency.gc-reader",
        status: CompatibilityStatus::Compatible,
        evidence_ref: DOLTLITE_CONCURRENCY_CONTRACT_REF,
        fixture: "reader-safe-gc",
        issue: None,
        explanation: "open readers retain committed observations during GC",
    },
    CompatibilityRowSpec {
        id: "serialization",
        source_contract: "sqlite.serialize-native-image",
        status: CompatibilityStatus::Compatible,
        evidence_ref: DOLTLITE_SQLITE_CONTRACT_REF,
        fixture: "serialization",
        issue: None,
        explanation: "native images round trip inside the pinned cohort",
    },
    CompatibilityRowSpec {
        id: "stale-writer",
        source_contract: "concurrency.snapshot-upgrade",
        status: CompatibilityStatus::Compatible,
        evidence_ref: DOLTLITE_CONCURRENCY_CONTRACT_REF,
        fixture: "stale-writer",
        issue: None,
        explanation: "stale read snapshots deny write upgrade",
    },
];

const ADAPTED_ROWS: [CompatibilityRowSpec; 2] = [
    CompatibilityRowSpec {
        id: "custom-collation",
        source_contract: "sqlite.persisted-custom-collation",
        status: CompatibilityStatus::Adapted,
        evidence_ref: DOLTLITE_SQLITE_CONTRACT_REF,
        fixture: "custom-collation-negative",
        issue: None,
        explanation: "oracle schemas use built-in binary ordering",
    },
    CompatibilityRowSpec {
        id: "explicit-primary-key",
        source_contract: "sqlite.clustered-primary-key",
        status: CompatibilityStatus::Adapted,
        evidence_ref: DOLTLITE_SQLITE_CONTRACT_REF,
        fixture: "rowid-negative",
        issue: None,
        explanation: "oracle rows use explicit canonical primary keys",
    },
];

const INTENTIONAL_ROWS: [CompatibilityRowSpec; 7] = [
    CompatibilityRowSpec {
        id: "authority",
        source_contract: "molten.authority",
        status: CompatibilityStatus::Intentional,
        evidence_ref: DOLTLITE_SQLITE_CONTRACT_REF,
        fixture: "authority-nonclaim",
        issue: None,
        explanation: "Molten retains authority admission",
    },
    CompatibilityRowSpec {
        id: "complete-world-atomicity",
        source_contract: "molten.complete-world",
        status: CompatibilityStatus::Intentional,
        evidence_ref: DOLTLITE_SQLITE_CONTRACT_REF,
        fixture: "multi-file-negative",
        issue: None,
        explanation: "Molten commits a complete world under its own protocol",
    },
    CompatibilityRowSpec {
        id: "durable-conflicts",
        source_contract: "molten.durable-conflicts",
        status: CompatibilityStatus::Intentional,
        evidence_ref: DOLTLITE_CONCURRENCY_CONTRACT_REF,
        fixture: "conflict-nonclaim",
        issue: None,
        explanation: "Molten persists typed conflict artifacts",
    },
    CompatibilityRowSpec {
        id: "effect-release",
        source_contract: "molten.effect-release",
        status: CompatibilityStatus::Intentional,
        evidence_ref: DOLTLITE_SQLITE_CONTRACT_REF,
        fixture: "effect-nonclaim",
        issue: None,
        explanation: "Molten owns effect reservation and dispatch",
    },
    CompatibilityRowSpec {
        id: "global-identities",
        source_contract: "molten.blake3-identities",
        status: CompatibilityStatus::Intentional,
        evidence_ref: DOLTLITE_FORMAT_CONTRACT_REF,
        fixture: "identity-overclaim",
        issue: None,
        explanation: "backend hashes stay local evidence",
    },
    CompatibilityRowSpec {
        id: "retention-policy",
        source_contract: "molten.retention",
        status: CompatibilityStatus::Intentional,
        evidence_ref: DOLTLITE_CONCURRENCY_CONTRACT_REF,
        fixture: "retention-nonclaim",
        issue: None,
        explanation: "Molten owns retention and deletion admission",
    },
    CompatibilityRowSpec {
        id: "typed-merge-policy",
        source_contract: "molten.typed-merge",
        status: CompatibilityStatus::Intentional,
        evidence_ref: DOLTLITE_SQLITE_CONTRACT_REF,
        fixture: "merge-nonclaim",
        issue: None,
        explanation: "Molten owns typed merge decisions",
    },
];

const UNSUPPORTED_ROWS: [CompatibilityRowSpec; 1] = [CompatibilityRowSpec {
    id: "multi-file-write",
    source_contract: "sqlite.multi-file-atomic-write",
    status: CompatibilityStatus::Unsupported,
    evidence_ref: DOLTLITE_SQLITE_CONTRACT_REF,
    fixture: "multi-file-write-negative",
    issue: Some("dolthub/doltlite#storage-multi-file"),
    explanation: "DoltLite rejects multiple file-backed writes",
}];

// r[impl molten.world_state_oracle.compatibility]
pub fn standard_compatibility_rows() -> Vec<CompatibilityRow> {
    let mut rows = COMPATIBLE_ROWS
        .into_iter()
        .chain(ADAPTED_ROWS)
        .chain(INTENTIONAL_ROWS)
        .chain(UNSUPPORTED_ROWS)
        .map(into_row)
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    rows
}

fn into_row(spec: CompatibilityRowSpec) -> CompatibilityRow {
    CompatibilityRow {
        id: spec.id.to_string(),
        source_contract: spec.source_contract.to_string(),
        status: spec.status,
        evidence_ref: spec.evidence_ref.to_string(),
        fixture: spec.fixture.to_string(),
        issue: spec.issue.map(str::to_string),
        explanation: spec.explanation.to_string(),
    }
}
