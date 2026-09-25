
    /// One fixture per direct negative fault, each denied before any side effect.
    fn negative_fault_fixtures() -> [NegativeFaultFixture; 6] {
        [
            NegativeFaultFixture {
                fault_kind: FAULT_STALE_EVIDENCE,
                operation_id: "op-stale",
                fault_diagnostic: "stale-ledger-ref",
                expected_diagnostic: "stale-evidence-denied-before-side-effects",
                requires_quorum: false,
                drop_authority: false,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_CORRUPTED_RECEIPT,
                operation_id: "op-corrupt",
                fault_diagnostic: "tampered-receipt",
                expected_diagnostic: "corrupted-receipt-denied-before-side-effects",
                requires_quorum: false,
                drop_authority: false,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_RESOURCE_PRESSURE,
                operation_id: "op-pressure",
                fault_diagnostic: "budget-exhausted",
                expected_diagnostic: "resource-pressure-denied-before-side-effects",
                requires_quorum: false,
                drop_authority: false,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_UNAUTHORIZED_TRANSPORT,
                operation_id: "op-transport",
                fault_diagnostic: "transport-only",
                expected_diagnostic: "transport-evidence-does-not-grant-authority",
                requires_quorum: false,
                drop_authority: true,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_AMBIENT_STATE_DRIFT,
                operation_id: "op-ambient",
                fault_diagnostic: "host-path-drift",
                expected_diagnostic: "undeclared-ambient-state",
                requires_quorum: false,
                drop_authority: false,
            },
            NegativeFaultFixture {
                fault_kind: FAULT_PARTITION,
                operation_id: "op-quorum",
                fault_diagnostic: "partition-window",
                expected_diagnostic: "partitioned-quorum-denied-before-side-effects",
                requires_quorum: true,
                drop_authority: false,
            },
        ]
    }
