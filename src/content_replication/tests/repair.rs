use super::*;

const REPAIR_COMPLETE: &str = "complete";
const REPAIR_INCOMPLETE: &str = "incomplete";
const REPAIR_BYTES_NOT_AUTHORITY: &str = "repair-recovered-bytes-are-not-authority";

struct StatusFixture {
    instance: ServiceInstance,
    facts: FactPorts,
    effects: EffectPorts,
}

impl StatusFixture {
    fn new() -> Self {
        let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let manifest = manifest();
        let mut facts = FactPorts::admitted(&events);
        let instance = active(manifest.clone(), &mut facts);
        let effects = EffectPorts::admitted(&manifest, &events);
        Self {
            instance,
            facts,
            effects,
        }
    }

    fn run(&mut self) -> crate::error::Result<ReconcileOutcome> {
        reconcile(self.instance.clone(), ports(&mut self.facts, &mut self.effects))
    }
}

// r[verify molten.closure_repair.boundary]
#[test]
fn the_operator_status_reports_a_complete_closure_repair_with_the_bytes_not_authority_claim() {
    let mut fixture = StatusFixture::new();
    let outcome = fixture.run().expect("reconciliation");
    let view = operator_status(&outcome);
    assert_eq!(view.repair_completeness, REPAIR_COMPLETE);
    assert!(view.repair_unrecoverable.is_empty());
    assert!(
        view.repair_non_claims.iter().any(|claim| claim == REPAIR_BYTES_NOT_AUTHORITY),
        "the operator status carries the bytes-not-authority claim"
    );
    assert!(canonical_operator_status(&view).is_ok());
}

// r[verify molten.closure_repair.incomplete_outcome]
#[test]
fn an_incomplete_closure_repair_names_the_unrecoverable_identity() {
    let mut fixture = StatusFixture::new();
    let replica = fixture.effects.content.inventory.replicas[0].clone();
    fixture.effects.content.inventory.replicas[0].present = false;
    fixture.effects.content.inventory.replicas[0].identity_verified = false;
    let outcome = fixture.run().expect("reconciliation");
    let view = operator_status(&outcome);
    assert_eq!(view.repair_completeness, REPAIR_INCOMPLETE);
    assert_eq!(view.repair_unrecoverable, vec![replica.content_ref]);
    assert!(canonical_operator_status(&view).is_ok());
}

// r[verify molten.closure_repair.boundary]
#[test]
fn the_operator_status_refuses_a_missing_claim_an_unknown_completeness_or_an_unnamed_incompleteness() {
    let mut fixture = StatusFixture::new();
    let outcome = fixture.run().expect("reconciliation");
    let base = operator_status(&outcome);

    let mut without_claim = base.clone();
    without_claim.repair_non_claims.retain(|claim| claim != REPAIR_BYTES_NOT_AUTHORITY);
    assert!(canonical_operator_status(&without_claim).is_err());

    let mut unknown_completeness = base.clone();
    unknown_completeness.repair_completeness = "mostly-complete".to_string();
    assert!(canonical_operator_status(&unknown_completeness).is_err());

    let mut unnamed_incomplete = base;
    unnamed_incomplete.repair_completeness = REPAIR_INCOMPLETE.to_string();
    unnamed_incomplete.repair_unrecoverable.clear();
    assert!(canonical_operator_status(&unnamed_incomplete).is_err());
}
