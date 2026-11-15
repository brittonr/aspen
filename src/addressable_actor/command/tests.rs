use super::*;

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

#[test]
fn signing_dialogue_requires_exact_current_request_and_one_bounded_frame() {
    let current = reference("current");
    assert!(signing::exact_ready(&format!("READY {current}\n"), &current).is_ok());
    assert!(signing::exact_ready(&format!("READY {}\n", reference("stale")), &current).is_err());
    assert!(signing::exact_ready(&format!("READY {current}\nREADY {current}\n"), &current).is_err());
    assert!(signing::exact_ready(&format!("READY {current}"), &current).is_err());
}

#[test]
fn operator_placement_json_cannot_supply_active_state_or_admission_booleans() {
    let input = serde_json::json!({"assignment_id":"assignment-one","extension_id":"molten.addressable-counter",
        "service_id":reference("actor"),"role_id":"counter-owner","role_kind":"counter","node_id":"node-one",
        "service_generation":1,"assignment_epoch":1,"fencing_token":1,"fencing_profile_ref":reference("fencing"),
        "resource_reservation_ref":reference("reservation"),"placement_plan_ref":reference("plan"),"authority_ref":reference("authority"),
        "durable_state_ref":null,"predecessor_assignment_ref":null,"predecessor_epoch":null});
    let valid: placement::Selection = serde_json::from_value(input.clone()).expect("typed proposal");
    valid.validate(&reference("actor")).expect("pure Proposed admission");
    assert!(valid.validate(&reference("other-actor")).is_err());
    let mut active = input.clone();
    active["state"] = serde_json::json!("active");
    assert!(serde_json::from_value::<placement::Selection>(active).is_err());
    let mut admitted = input;
    admitted["placement_current"] = serde_json::json!(true);
    assert!(serde_json::from_value::<placement::Selection>(admitted).is_err());
}

#[test]
fn placement_initial_proposal_rejects_generation_and_predecessor_substitution() {
    let value = serde_json::json!({"assignment_id":"assignment-one","extension_id":"molten.addressable-counter",
        "service_id":reference("actor"),"role_id":"counter-owner","role_kind":"counter","node_id":"node-one",
        "service_generation":1,"assignment_epoch":1,"fencing_token":1,"fencing_profile_ref":reference("fencing"),
        "resource_reservation_ref":reference("reservation"),"placement_plan_ref":reference("plan"),"authority_ref":reference("authority"),
        "durable_state_ref":null,"predecessor_assignment_ref":null,"predecessor_epoch":null});
    let mut proposal: placement::Selection = serde_json::from_value(value).expect("typed proposal");
    proposal.service_generation = 2;
    assert!(proposal.validate(&reference("actor")).is_err());
    proposal.service_generation = 1;
    proposal.predecessor_assignment_ref = Some(reference("predecessor"));
    proposal.predecessor_epoch = Some(1);
    assert!(proposal.validate(&reference("actor")).is_err());
}
