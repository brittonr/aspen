//! Shell for runtime admission: the pure capability/deny-rule policy lives in
//! `molten_runtime_admission_core::ingress`, instantiated here with the canonical Preserves
//! `RuntimeValue`.

pub use molten_runtime_admission_core::ingress::AdmissionAction;
pub use molten_runtime_admission_core::ingress::AdmissionDecision;

pub type AdmissionPolicy = molten_runtime_admission_core::ingress::AdmissionPolicy<super::RuntimeValue>;
pub type CapabilityContext = molten_runtime_admission_core::ingress::CapabilityContext<super::RuntimeValue>;
pub type CapabilityGrant = molten_runtime_admission_core::ingress::CapabilityGrant<super::RuntimeValue>;
pub type CapabilityAuthorization = molten_runtime_admission_core::ingress::CapabilityAuthorization<super::RuntimeValue>;
pub type AdmissionDenyRule = molten_runtime_admission_core::ingress::AdmissionDenyRule<super::RuntimeValue>;
pub type AdmissionRequest = molten_runtime_admission_core::ingress::AdmissionRequest<super::RuntimeValue>;

impl molten_runtime_admission_core::ingress::AdmissionStep<super::RuntimeValue> for super::RuntimeStep {
    fn admission_request(&self) -> AdmissionRequest {
        match self {
            super::RuntimeStep::Send { from, to, body } => AdmissionRequest {
                actor: from.clone(),
                action: AdmissionAction::Send,
                target: Some(to.clone()),
                value: Some(body.clone()),
                upper: None,
            },
            super::RuntimeStep::Observe { actor, pattern } => AdmissionRequest {
                actor: actor.clone(),
                action: AdmissionAction::Observe,
                target: None,
                value: Some(pattern.clone()),
                upper: None,
            },
            super::RuntimeStep::Assert { actor, value } => AdmissionRequest {
                actor: actor.clone(),
                action: AdmissionAction::Assert,
                target: None,
                value: Some(value.clone()),
                upper: None,
            },
            super::RuntimeStep::Retract { actor, value } => AdmissionRequest {
                actor: actor.clone(),
                action: AdmissionAction::Retract,
                target: None,
                value: Some(value.clone()),
                upper: None,
            },
            super::RuntimeStep::Clock { actor } => AdmissionRequest {
                actor: actor.clone(),
                action: AdmissionAction::Clock,
                target: None,
                value: None,
                upper: None,
            },
            super::RuntimeStep::Random { actor, upper } => AdmissionRequest {
                actor: actor.clone(),
                action: AdmissionAction::Random,
                target: None,
                value: None,
                upper: Some(*upper),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shape_valid_pattern_evidence_does_not_grant_authority() {
        // r[verify molten.preserves_boundary_codegen.no_schema_authority]
        let value = crate::runtime::RuntimeValue::string("service.privileged").expect("runtime test value");
        let pattern = crate::runtime::RuntimePattern::exact(value.clone());
        pattern.validate().expect("shape-valid pattern");
        let request = super::AdmissionRequest::from_step(&crate::runtime::RuntimeStep::Assert {
            actor: "producer".into(),
            value,
        });
        let policy = super::AdmissionPolicy::allow_all();
        let empty_capabilities = super::CapabilityContext::from_grants(Vec::new());

        let decision = policy.decide_with_capabilities(&empty_capabilities, &request);

        assert!(matches!(decision, super::AdmissionDecision::Deny { .. }));
        assert_eq!(decision.reason(), "missing capability grant");
    }

    #[test]
    fn runtime_steps_project_their_canonical_value_and_bounds() {
        let body = crate::runtime::RuntimeValue::string("hello").expect("runtime test value");
        let send = super::AdmissionRequest::from_step(&crate::runtime::RuntimeStep::Send {
            from: "a".into(),
            to: "b".into(),
            body: body.clone(),
        });
        assert_eq!(send.action, super::AdmissionAction::Send);
        assert_eq!(send.target.as_deref(), Some("b"));
        assert_eq!(send.value, Some(body));
        let random = super::AdmissionRequest::from_step(&crate::runtime::RuntimeStep::Random {
            actor: "a".into(),
            upper: 6,
        });
        assert_eq!((random.action, random.value, random.upper), (super::AdmissionAction::Random, None, Some(6)));
    }

    #[test]
    fn default_runtime_capability_context_denies_and_an_explicit_grant_admits() {
        // r[verify molten.runtime_spine.capability_context_deny_by_default]
        let value = crate::runtime::RuntimeValue::string("service.ready").expect("runtime test value");
        let request = super::AdmissionRequest::from_step(&crate::runtime::RuntimeStep::Assert {
            actor: "producer".into(),
            value: value.clone(),
        });
        let policy = super::AdmissionPolicy::allow_all();
        let denied = policy.decide_with_capabilities(&super::CapabilityContext::default(), &request);
        assert_eq!(denied.reason(), "missing capability grant");
        let explicit = super::CapabilityContext::from_grants(vec![super::CapabilityGrant {
            actor: Some("producer".to_string()),
            action: Some(super::AdmissionAction::Assert),
            target: None,
            value: Some(value),
        }]);
        assert!(policy.decide_with_capabilities(&explicit, &request).is_allowed());
    }
}
