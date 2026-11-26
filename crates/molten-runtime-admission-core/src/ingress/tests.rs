use alloc::string::ToString;

use super::*;

/// A minimal runtime step whose values are plain strings.
enum Step {
    Assert { actor: &'static str, value: &'static str },
    Random { actor: &'static str, upper: u64 },
}

impl AdmissionStep<alloc::string::String> for Step {
    fn admission_request(&self) -> AdmissionRequest<alloc::string::String> {
        match self {
            Step::Assert { actor, value } => AdmissionRequest {
                actor: (*actor).into(),
                action: AdmissionAction::Assert,
                target: None,
                value: Some((*value).into()),
                upper: None,
            },
            Step::Random { actor, upper } => AdmissionRequest {
                actor: (*actor).into(),
                action: AdmissionAction::Random,
                target: None,
                value: None,
                upper: Some(*upper),
            },
        }
    }
}

// r[verify molten.preserves_boundary_codegen.no_schema_authority]
#[test]
fn shape_valid_value_without_a_grant_is_denied_even_by_an_allow_all_policy() {
    let request = AdmissionRequest::from_step(&Step::Assert {
        actor: "producer",
        value: "service.privileged",
    });
    let policy = AdmissionPolicy::allow_all();
    let decision = policy.decide_with_capabilities(&CapabilityContext::from_grants(alloc::vec::Vec::new()), &request);
    assert_eq!(decision, AdmissionDecision::Deny {
        reason: "missing capability grant".to_string()
    });
    assert_eq!(decision.status(), "deny");
    let allowed = policy.decide_with_capabilities(&CapabilityContext::allow_all(), &request);
    assert!(allowed.is_allowed());
    assert_eq!(allowed.reason(), "default-allow");
}

#[test]
fn deny_rule_matches_specific_actor_action_and_value() {
    let policy = AdmissionPolicy::from_deny_rules(alloc::vec![AdmissionDenyRule {
        actor: Some("producer".to_string()),
        action: Some(AdmissionAction::Assert),
        target: None,
        value: Some(alloc::string::String::from("service.ready")),
        reason: "producer cannot assert readiness".to_string(),
    }]);
    let denied = AdmissionRequest::from_step(&Step::Assert {
        actor: "producer",
        value: "service.ready",
    });
    assert_eq!(policy.decide(&denied).reason(), "producer cannot assert readiness");
    let other_value = AdmissionRequest::from_step(&Step::Assert {
        actor: "producer",
        value: "service.draining",
    });
    assert!(policy.decide(&other_value).is_allowed());
    let other_actor = AdmissionRequest::from_step(&Step::Assert {
        actor: "consumer",
        value: "service.ready",
    });
    assert!(policy.decide(&other_actor).is_allowed());
}

// r[verify molten.project.tier1_no_std_kernel_crates]
#[test]
fn authorization_returns_the_first_matching_grant() {
    let narrow = CapabilityGrant {
        actor: Some("producer".to_string()),
        action: Some(AdmissionAction::Random),
        target: None,
        value: None,
    };
    let context = CapabilityContext::from_grants(alloc::vec![narrow.clone(), CapabilityGrant {
        actor: None,
        action: Some(AdmissionAction::Random),
        target: None,
        value: None,
    }]);
    let request = AdmissionRequest::<alloc::string::String>::from_step(&Step::Random {
        actor: "producer",
        upper: 6,
    });
    assert_eq!(request.upper, Some(6));
    assert_eq!(context.authorize(&request).grant, Some(narrow));
    let other = AdmissionRequest::<alloc::string::String>::from_step(&Step::Random {
        actor: "other",
        upper: 6,
    });
    assert_eq!(context.authorize(&other).grant.and_then(|grant| grant.actor), None);
    let assert = AdmissionRequest::from_step(&Step::Assert {
        actor: "producer",
        value: "v",
    });
    assert!(!context.authorize(&assert).authorized);
}

#[hegel::test(test_cases = 64)]
fn hegel_allow_all_capabilities_authorize_every_action_and_no_grant_denies_all(tc: hegel::TestCase) {
    let actions = AdmissionAction::all();
    let index = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(actions.len() - 1));
    let salt = tc.draw(hegel::generators::integers::<u64>());
    let request = AdmissionRequest {
        actor: alloc::format!("actor-{salt}"),
        action: actions[index].clone(),
        target: None,
        value: Some(salt),
        upper: None,
    };
    assert!(CapabilityContext::allow_all().authorize(&request).authorized);
    let empty = CapabilityContext::<u64>::from_grants(alloc::vec::Vec::new());
    assert!(!AdmissionPolicy::allow_all().decide_with_capabilities(&empty, &request).is_allowed());
}

// r[verify molten.runtime_spine.capability_context_deny_by_default]
#[hegel::test(test_cases = 64)]
fn hegel_default_capability_context_denies_every_action(tc: hegel::TestCase) {
    let actions = AdmissionAction::all();
    let index = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(actions.len() - 1));
    let salt = tc.draw(hegel::generators::integers::<u64>());
    let request = AdmissionRequest {
        actor: alloc::format!("actor-{salt}"),
        action: actions[index].clone(),
        target: Some(alloc::format!("target-{salt}")),
        value: Some(salt),
        upper: Some(salt),
    };
    let default_context = CapabilityContext::<u64>::default();
    assert!(default_context.grants().is_empty());
    assert!(!default_context.authorize(&request).authorized);
    assert!(!AdmissionPolicy::allow_all().decide_with_capabilities(&default_context, &request).is_allowed());
}

// r[verify molten.runtime_spine.capability_context_deny_by_default]
#[test]
fn explicit_matching_grant_admits_where_the_default_context_denies() {
    let request = AdmissionRequest::from_step(&Step::Assert {
        actor: "producer",
        value: "service.ready",
    });
    let policy = AdmissionPolicy::allow_all();
    let denied = policy.decide_with_capabilities(&CapabilityContext::default(), &request);
    assert_eq!(denied, AdmissionDecision::Deny {
        reason: "missing capability grant".to_string()
    });
    let grant = CapabilityGrant {
        actor: Some("producer".to_string()),
        action: Some(AdmissionAction::Assert),
        target: None,
        value: Some(alloc::string::String::from("service.ready")),
    };
    let explicit = CapabilityContext::from_grants(alloc::vec![grant.clone()]);
    assert_eq!(explicit.authorize(&request).grant, Some(grant));
    assert!(policy.decide_with_capabilities(&explicit, &request).is_allowed());
}
