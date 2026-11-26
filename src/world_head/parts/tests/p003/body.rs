/// A creation-only store that reports a scripted CAS outcome and records what it was offered.
struct ScriptedStore {
    outcome: WorldHeadMutationOutcome,
    offered_receipt: Option<CanonicalWorldHeadTransitionReceipt>,
    reconciliation_receipt: Option<CanonicalWorldHeadTransitionReceipt>,
}

impl ScriptedStore {
    const fn new(outcome: WorldHeadMutationOutcome) -> Self {
        Self {
            outcome,
            offered_receipt: None,
            reconciliation_receipt: None,
        }
    }
}

impl WorldHeadStatePort for ScriptedStore {
    fn read_head(&self, _branch_id: &WorldBranchId) -> std::result::Result<Option<WorldHeadState>, WorldHeadPortError> {
        Ok(None)
    }

    fn apply_transition<F>(
        &mut self,
        _plan: &WorldHeadTransitionPlan,
        receipt: &CanonicalWorldHeadTransitionReceipt,
        recheck: F,
    ) -> std::result::Result<WorldHeadMutationOutcome, WorldHeadPortError>
    where
        F: FnOnce(Option<&WorldHeadState>) -> std::result::Result<WorldHeadFreshAdmission, WorldHeadPortError>,
    {
        let fresh = recheck(None)?;
        assert!(fresh.authentication_passed);
        assert!(fresh.authority.admitted);
        self.offered_receipt = Some(receipt.clone());
        Ok(self.outcome)
    }
}

impl WorldHeadReconciliationPort for ScriptedStore {
    fn record_uncertain_transition(
        &mut self,
        _plan: &WorldHeadTransitionPlan,
        receipt: &CanonicalWorldHeadTransitionReceipt,
    ) -> std::result::Result<(), WorldHeadPortError> {
        self.reconciliation_receipt = Some(receipt.clone());
        Ok(())
    }
}

fn receipt_decision(receipt: &CanonicalWorldHeadTransitionReceipt) -> String {
    let fields = crate::preserves_rail::simple_record_fields(
        &receipt.value,
        WORLD_HEAD_TRANSITION_RECEIPT_RECORD,
        TRANSITION_RECEIPT_FIELD_COUNT,
    )
    .expect("transition receipt fields");
    crate::preserves_rail::record_string_field(&fields[RECEIPT_DECISION_FIELD_INDEX], "decision", "decision")
        .expect("receipt decision")
}

#[test]
fn checked_schema_artifacts_match_world_head_boundary_specs() {
    // r[verify molten.world_heads.authentication]
    for (source, spec) in WORLD_HEAD_SCHEMA_ARTIFACTS.iter().zip(WORLD_HEAD_BOUNDARY_SCHEMAS) {
        let value = crate::preserves_rail::parse_text(source).expect("schema artifact");
        let fields = crate::preserves_rail::simple_record_fields(
            &value,
            "preserves-boundary-schema-artifact-v1",
            WORLD_HEAD_SCHEMA_FIELD_COUNT,
        )
        .expect("schema fields");
        assert_eq!(crate::preserves_rail::record_string_field(&fields[0], "family", "family").unwrap(), spec.family);
        assert_eq!(crate::preserves_rail::record_string_field(&fields[1], "version", "version").unwrap(), spec.version);
        assert_eq!(
            crate::preserves_rail::record_string_field(
                &fields[RECORD_LABEL_FIELD_INDEX],
                "record-label",
                "record label"
            )
            .unwrap(),
            spec.record_label
        );
        assert_eq!(
            crate::preserves_rail::record_string_field(&fields[SCHEMA_ID_FIELD_INDEX], "schema-id", "schema id",)
                .unwrap(),
            spec.schema_id
        );
    }
}
