use super::super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfLifecycleFact {
    Complete,
    Partial,
    Unavailable,
    Permission,
    Timeout,
    Bounds,
    Malformed,
    Binding,
    Sequence,
    Accounting,
    Stale,
    Cancelled,
    Closed,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EbpfServiceDecision {
    pub kind: AdapterOutcomeKind,
    pub health: HealthState,
    pub deny_required_readiness: bool,
}

// r[impl linux_ebpf_observability.failure]
// r[impl linux_ebpf_observability.confidentiality]
pub fn decide_ebpf_service(required: bool, fact: EbpfLifecycleFact) -> EbpfServiceDecision {
    let kind = match fact {
        EbpfLifecycleFact::Complete => AdapterOutcomeKind::Exported,
        EbpfLifecycleFact::Partial | EbpfLifecycleFact::Sequence | EbpfLifecycleFact::Accounting => {
            AdapterOutcomeKind::Partial
        }
        EbpfLifecycleFact::Unavailable | EbpfLifecycleFact::Closed => AdapterOutcomeKind::Unavailable,
        EbpfLifecycleFact::Permission => AdapterOutcomeKind::PermissionDenied,
        EbpfLifecycleFact::Timeout => AdapterOutcomeKind::Timeout,
        EbpfLifecycleFact::Bounds => AdapterOutcomeKind::Backpressure,
        EbpfLifecycleFact::Malformed | EbpfLifecycleFact::Binding => AdapterOutcomeKind::Corrupt,
        EbpfLifecycleFact::Stale => AdapterOutcomeKind::Stale,
        EbpfLifecycleFact::Cancelled => AdapterOutcomeKind::Cancelled,
        EbpfLifecycleFact::Unsupported => AdapterOutcomeKind::Unsupported,
    };
    let complete = fact == EbpfLifecycleFact::Complete;
    EbpfServiceDecision {
        kind,
        health: if complete {
            HealthState::Healthy
        } else {
            HealthState::Unavailable
        },
        deny_required_readiness: required && !complete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify linux_ebpf_observability.failure]
    #[test]
    fn every_incomplete_required_lifecycle_denies_readiness() {
        for fact in [
            EbpfLifecycleFact::Partial,
            EbpfLifecycleFact::Unavailable,
            EbpfLifecycleFact::Permission,
            EbpfLifecycleFact::Timeout,
            EbpfLifecycleFact::Bounds,
            EbpfLifecycleFact::Malformed,
            EbpfLifecycleFact::Binding,
            EbpfLifecycleFact::Sequence,
            EbpfLifecycleFact::Accounting,
            EbpfLifecycleFact::Stale,
            EbpfLifecycleFact::Cancelled,
            EbpfLifecycleFact::Closed,
            EbpfLifecycleFact::Unsupported,
        ] {
            let decision = decide_ebpf_service(true, fact);
            assert!(decision.deny_required_readiness);
            assert_eq!(decision.health, HealthState::Unavailable);
            assert!(!decide_ebpf_service(false, fact).deny_required_readiness);
        }
        assert_eq!(decide_ebpf_service(true, EbpfLifecycleFact::Complete).health, HealthState::Healthy);
    }
}
