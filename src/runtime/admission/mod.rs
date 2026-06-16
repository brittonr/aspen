use super::RuntimeStep;
use super::RuntimeValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionPolicy {
    deny_rules: Vec<AdmissionDenyRule>,
}

impl AdmissionPolicy {
    pub fn allow_all() -> Self {
        Self { deny_rules: Vec::new() }
    }

    pub fn deny_rules(&self) -> &[AdmissionDenyRule] {
        &self.deny_rules
    }

    pub fn from_deny_rules(deny_rules: Vec<AdmissionDenyRule>) -> Self {
        Self { deny_rules }
    }

    pub fn decide_with_capabilities(
        &self,
        capabilities: &CapabilityContext,
        request: &AdmissionRequest,
    ) -> AdmissionDecision {
        let authorization = capabilities.authorize(request);
        if !authorization.authorized {
            return AdmissionDecision::Deny {
                reason: "missing capability grant".to_string(),
            };
        }
        self.decide(request)
    }

    pub fn decide(&self, request: &AdmissionRequest) -> AdmissionDecision {
        for rule in &self.deny_rules {
            if rule.matches(request) {
                return AdmissionDecision::Deny {
                    reason: rule.reason.clone(),
                };
            }
        }
        AdmissionDecision::Allow {
            reason: "default-allow".to_string(),
        }
    }
}

impl Default for AdmissionPolicy {
    fn default() -> Self {
        Self::allow_all()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityContext {
    grants: Vec<CapabilityGrant>,
}

impl CapabilityContext {
    pub fn allow_all() -> Self {
        Self {
            grants: AdmissionAction::all()
                .iter()
                .cloned()
                .map(|action| CapabilityGrant {
                    actor: None,
                    action: Some(action),
                    target: None,
                    value: None,
                })
                .collect(),
        }
    }

    pub fn from_grants(grants: Vec<CapabilityGrant>) -> Self {
        Self { grants }
    }

    pub fn grants(&self) -> &[CapabilityGrant] {
        &self.grants
    }

    pub fn authorize(&self, request: &AdmissionRequest) -> CapabilityAuthorization {
        for grant in &self.grants {
            if grant.matches(request) {
                return CapabilityAuthorization {
                    authorized: true,
                    grant: Some(grant.clone()),
                };
            }
        }
        CapabilityAuthorization {
            authorized: false,
            grant: None,
        }
    }
}

impl Default for CapabilityContext {
    fn default() -> Self {
        Self::allow_all()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGrant {
    pub actor: Option<String>,
    pub action: Option<AdmissionAction>,
    pub target: Option<String>,
    pub value: Option<RuntimeValue>,
}

impl CapabilityGrant {
    pub fn matches(&self, request: &AdmissionRequest) -> bool {
        self.actor.as_ref().is_none_or(|actor| actor == &request.actor)
            && self.action.as_ref().is_none_or(|action| action == &request.action)
            && self.target.as_ref().is_none_or(|target| request.target.as_ref() == Some(target))
            && self.value.as_ref().is_none_or(|value| request.value.as_ref() == Some(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAuthorization {
    pub authorized: bool,
    pub grant: Option<CapabilityGrant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionDenyRule {
    pub actor: Option<String>,
    pub action: Option<AdmissionAction>,
    pub target: Option<String>,
    pub value: Option<RuntimeValue>,
    pub reason: String,
}

impl AdmissionDenyRule {
    pub fn matches(&self, request: &AdmissionRequest) -> bool {
        self.actor.as_ref().is_none_or(|actor| actor == &request.actor)
            && self.action.as_ref().is_none_or(|action| action == &request.action)
            && self.target.as_ref().is_none_or(|target| request.target.as_ref() == Some(target))
            && self.value.as_ref().is_none_or(|value| request.value.as_ref() == Some(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRequest {
    pub actor: String,
    pub action: AdmissionAction,
    pub target: Option<String>,
    pub value: Option<RuntimeValue>,
    pub upper: Option<u64>,
}

impl AdmissionRequest {
    pub fn from_step(step: &RuntimeStep) -> Self {
        match step {
            RuntimeStep::Send { from, to, body } => Self {
                actor: from.clone(),
                action: AdmissionAction::Send,
                target: Some(to.clone()),
                value: Some(body.clone()),
                upper: None,
            },
            RuntimeStep::Observe { actor, pattern } => Self {
                actor: actor.clone(),
                action: AdmissionAction::Observe,
                target: None,
                value: Some(pattern.clone()),
                upper: None,
            },
            RuntimeStep::Assert { actor, value } => Self {
                actor: actor.clone(),
                action: AdmissionAction::Assert,
                target: None,
                value: Some(value.clone()),
                upper: None,
            },
            RuntimeStep::Retract { actor, value } => Self {
                actor: actor.clone(),
                action: AdmissionAction::Retract,
                target: None,
                value: Some(value.clone()),
                upper: None,
            },
            RuntimeStep::Clock { actor } => Self {
                actor: actor.clone(),
                action: AdmissionAction::Clock,
                target: None,
                value: None,
                upper: None,
            },
            RuntimeStep::Random { actor, upper } => Self {
                actor: actor.clone(),
                action: AdmissionAction::Random,
                target: None,
                value: None,
                upper: Some(*upper),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionAction {
    Send,
    Observe,
    Assert,
    Retract,
    Clock,
    Random,
}

impl AdmissionAction {
    pub fn all() -> &'static [AdmissionAction] {
        &[
            AdmissionAction::Send,
            AdmissionAction::Observe,
            AdmissionAction::Assert,
            AdmissionAction::Retract,
            AdmissionAction::Clock,
            AdmissionAction::Random,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AdmissionAction::Send => "send",
            AdmissionAction::Observe => "observe",
            AdmissionAction::Assert => "assert",
            AdmissionAction::Retract => "retract",
            AdmissionAction::Clock => "clock",
            AdmissionAction::Random => "random",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionDecision {
    Allow { reason: String },
    Deny { reason: String },
}

impl AdmissionDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, AdmissionDecision::Allow { .. })
    }

    pub fn status(&self) -> &'static str {
        match self {
            AdmissionDecision::Allow { .. } => "allow",
            AdmissionDecision::Deny { .. } => "deny",
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            AdmissionDecision::Allow { reason } | AdmissionDecision::Deny { reason } => reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AdmissionAction;
    use super::AdmissionDecision;
    use super::AdmissionDenyRule;
    use super::AdmissionPolicy;
    use super::AdmissionRequest;
    use crate::runtime::RuntimeStep;
    use crate::runtime::RuntimeValue;

    #[test]
    fn deny_rule_matches_specific_actor_action_and_value() {
        let policy = AdmissionPolicy::from_deny_rules(vec![AdmissionDenyRule {
            actor: Some("producer".to_string()),
            action: Some(AdmissionAction::Assert),
            target: None,
            value: Some(RuntimeValue::string("service.ready").expect("runtime test value")),
            reason: "producer cannot assert readiness".to_string(),
        }]);
        let request = AdmissionRequest::from_step(&RuntimeStep::Assert {
            actor: "producer".into(),
            value: RuntimeValue::string("service.ready").expect("runtime test value"),
        });
        assert!(matches!(policy.decide(&request), AdmissionDecision::Deny { .. }));
    }
}
