//! Runtime capability and deny-rule admission.
//!
//! The policy is generic over the runtime value `V` it matches by equality; the `molten` shell
//! instantiates it with its canonical Preserves `RuntimeValue` and projects runtime steps into
//! requests through [`AdmissionStep`]. Capability possession is checked before deny rules, and a
//! shape-valid value never grants authority by itself.

use alloc::string::ToString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionPolicy<V> {
    deny_rules: alloc::vec::Vec<AdmissionDenyRule<V>>,
}

impl<V> AdmissionPolicy<V> {
    pub fn allow_all() -> Self {
        Self {
            deny_rules: alloc::vec::Vec::new(),
        }
    }

    pub fn deny_rules(&self) -> &[AdmissionDenyRule<V>] {
        &self.deny_rules
    }

    pub fn from_deny_rules(deny_rules: alloc::vec::Vec<AdmissionDenyRule<V>>) -> Self {
        Self { deny_rules }
    }
}

impl<V: PartialEq + Clone> AdmissionPolicy<V> {
    // r[impl molten.preserves_boundary_codegen.no_schema_authority]
    pub fn decide_with_capabilities(
        &self,
        capabilities: &CapabilityContext<V>,
        request: &AdmissionRequest<V>,
    ) -> AdmissionDecision {
        let authorization = capabilities.authorize(request);
        if !authorization.authorized {
            return AdmissionDecision::Deny {
                reason: "missing capability grant".to_string(),
            };
        }
        self.decide(request)
    }

    pub fn decide(&self, request: &AdmissionRequest<V>) -> AdmissionDecision {
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

impl<V> Default for AdmissionPolicy<V> {
    fn default() -> Self {
        Self::allow_all()
    }
}

/// Capability grants an actor holds. A context grants nothing unless built from explicit grants;
/// granting every request requires the named [`CapabilityContext::allow_all`] constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityContext<V> {
    grants: alloc::vec::Vec<CapabilityGrant<V>>,
}

impl<V> CapabilityContext<V> {
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

    pub fn from_grants(grants: alloc::vec::Vec<CapabilityGrant<V>>) -> Self {
        Self { grants }
    }

    pub fn grants(&self) -> &[CapabilityGrant<V>] {
        &self.grants
    }
}

impl<V: PartialEq + Clone> CapabilityContext<V> {
    pub fn authorize(&self, request: &AdmissionRequest<V>) -> CapabilityAuthorization<V> {
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

/// The default context is deny-by-default: it holds no grants and authorizes nothing.
// r[impl molten.runtime_spine.capability_context_deny_by_default]
impl<V> Default for CapabilityContext<V> {
    fn default() -> Self {
        Self::from_grants(alloc::vec::Vec::new())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGrant<V> {
    pub actor: Option<alloc::string::String>,
    pub action: Option<AdmissionAction>,
    pub target: Option<alloc::string::String>,
    pub value: Option<V>,
}

impl<V: PartialEq> CapabilityGrant<V> {
    pub fn matches(&self, request: &AdmissionRequest<V>) -> bool {
        self.actor.as_ref().is_none_or(|actor| actor == &request.actor)
            && self.action.as_ref().is_none_or(|action| action == &request.action)
            && self.target.as_ref().is_none_or(|target| request.target.as_ref() == Some(target))
            && self.value.as_ref().is_none_or(|value| request.value.as_ref() == Some(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAuthorization<V> {
    pub authorized: bool,
    pub grant: Option<CapabilityGrant<V>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionDenyRule<V> {
    pub actor: Option<alloc::string::String>,
    pub action: Option<AdmissionAction>,
    pub target: Option<alloc::string::String>,
    pub value: Option<V>,
    pub reason: alloc::string::String,
}

impl<V: PartialEq> AdmissionDenyRule<V> {
    pub fn matches(&self, request: &AdmissionRequest<V>) -> bool {
        self.actor.as_ref().is_none_or(|actor| actor == &request.actor)
            && self.action.as_ref().is_none_or(|action| action == &request.action)
            && self.target.as_ref().is_none_or(|target| request.target.as_ref() == Some(target))
            && self.value.as_ref().is_none_or(|value| request.value.as_ref() == Some(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRequest<V> {
    pub actor: alloc::string::String,
    pub action: AdmissionAction,
    pub target: Option<alloc::string::String>,
    pub value: Option<V>,
    pub upper: Option<u64>,
}

/// Projection of one runtime step into the request admission evaluates.
pub trait AdmissionStep<V> {
    fn admission_request(&self) -> AdmissionRequest<V>;
}

impl<V> AdmissionRequest<V> {
    pub fn from_step<S: AdmissionStep<V> + ?Sized>(step: &S) -> Self {
        step.admission_request()
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
    Allow { reason: alloc::string::String },
    Deny { reason: alloc::string::String },
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
mod tests;
