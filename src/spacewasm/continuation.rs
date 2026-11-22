use std::rc;

use super::receipt;

// r[impl molten.spacewasm_core.resume]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub actor: String,
    pub generation: u64,
    pub artifact: String,
    pub profile: String,
    pub input: String,
    pub authority: String,
    pub resource: String,
    pub effects: String,
}

impl Binding {
    pub fn identity(&self) -> Result<String, String> {
        if self.actor.is_empty()
            || self.actor.len() > 256
            || [
                &self.artifact,
                &self.profile,
                &self.input,
                &self.authority,
                &self.resource,
                &self.effects,
            ]
            .iter()
            .any(|value| {
                !value.strip_prefix("blake3:").is_some_and(|digest| {
                    digest.len() == 64
                        && digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                })
            })
        {
            return Err("invalid continuation reference domain".into());
        }
        let fields = vec![
            crate::codec::string(&self.actor),
            crate::codec::u64_value(self.generation),
            crate::codec::string(&self.artifact),
            crate::codec::string(&self.profile),
            crate::codec::string(&self.input),
            crate::codec::string(&self.authority),
            crate::codec::string(&self.resource),
            crate::codec::string(&self.effects),
        ];
        crate::codec::canonical_hash(&crate::codec::record("spacewasm-continuation-v1", fields))
            .map_err(|error| error.to_string())
    }
}

pub trait CurrentAdmission {
    /// Resolve current generation, policy, resource and effect state, not a cached grant.
    fn current(&self) -> Result<Binding, String>;
}

pub struct ResumeContext {
    pub current: rc::Rc<dyn CurrentAdmission>,
    pub logical_turn: u64,
}

/// The requested runner grant is the smaller of the segment ceiling and the remaining turn work.
// r[impl molten.spacewasm_core.functional_core]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentPlan {
    logical_turn: u64,
    segment_ceiling: u64,
    total_ceiling: u64,
    prior_spent: u64,
    grant: u64,
}

impl SegmentPlan {
    pub fn new(logical_turn: u64, segment_ceiling: u64, total_ceiling: u64, prior_spent: u64) -> Result<Self, Failure> {
        if segment_ceiling == 0 || segment_ceiling > total_ceiling || prior_spent > total_ceiling {
            return Err(Failure::Proof("invalid-segment-budget".into()));
        }
        let grant = segment_ceiling.min(total_ceiling - prior_spent);
        if grant == 0 {
            return Err(Failure::TotalFuel);
        }
        Ok(Self {
            logical_turn,
            segment_ceiling,
            total_ceiling,
            prior_spent,
            grant,
        })
    }

    pub fn grant(&self) -> u64 {
        self.grant
    }

    pub fn observe(self, consumed: u64, end: SegmentEnd) -> Result<SegmentObservation, Failure> {
        if consumed == 0 || consumed > self.grant || (end == SegmentEnd::OutOfFuel && consumed != self.grant) {
            return Err(Failure::Proof("segment-work-does-not-match-grant".into()));
        }
        Ok(SegmentObservation {
            plan: self,
            post_spent: self.prior_spent + consumed,
            end,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentEnd {
    Finished,
    OutOfFuel,
    HostPause,
    Trap,
    HarnessFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentObservation {
    plan: SegmentPlan,
    post_spent: u64,
    end: SegmentEnd,
}

impl SegmentObservation {
    pub fn prior_spent(&self) -> u64 {
        self.plan.prior_spent
    }

    pub fn post_spent(&self) -> u64 {
        self.post_spent
    }

    pub(super) fn detail(&self) -> &'static str {
        match self.end {
            SegmentEnd::Finished => "finished",
            SegmentEnd::OutOfFuel => "out-of-fuel",
            SegmentEnd::HostPause => "host-pause",
            SegmentEnd::Trap => "deterministic-trap",
            SegmentEnd::HarnessFailure => "harness-failure",
        }
    }

    pub(super) fn value(&self) -> preserves::IOValue {
        crate::codec::record("spacewasm-segment-v1", vec![
            crate::codec::u64_value(self.plan.logical_turn),
            crate::codec::u64_value(self.plan.segment_ceiling),
            crate::codec::u64_value(self.plan.total_ceiling),
            crate::codec::u64_value(self.plan.grant),
            crate::codec::u64_value(self.plan.prior_spent),
            crate::codec::u64_value(self.post_spent),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    Denied(String),
    Allocation(String),
    Memory,
    InvalidOutput,
    Stack,
    TotalFuel,
    Trap(String),
    Harness(String),
    Uncertain(String),
    Cancelled(String),
    Proof(String),
}

pub enum Progress {
    Yield { host_pause: bool, continuation: String },
    Terminal(receipt::Observation),
    ProofFailed(Failure),
}

impl Failure {
    pub fn class(&self) -> &'static str {
        match self {
            Self::Denied(_) => "denied",
            Self::Allocation(_) => "allocation-exhausted",
            Self::Memory => "memory-bound",
            Self::InvalidOutput => "invalid-canonical-output",
            Self::Stack => "stack-bound",
            Self::TotalFuel => "total-instruction-bound",
            Self::Trap(_) => "deterministic-trap",
            Self::Harness(_) => "harness-failure",
            Self::Uncertain(_) => "effect-uncertain",
            Self::Cancelled(_) => "cancelled",
            Self::Proof(_) => "proof-failure",
        }
    }

    pub fn is_proof_error(&self) -> bool {
        matches!(self, Self::Harness(_) | Self::Proof(_))
    }

    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::Denied(reason)
            | Self::Allocation(reason)
            | Self::Trap(reason)
            | Self::Harness(reason)
            | Self::Uncertain(reason)
            | Self::Cancelled(reason)
            | Self::Proof(reason) => reason.capacity(),
            Self::Memory | Self::InvalidOutput | Self::Stack | Self::TotalFuel => 0,
        }
    }

    pub fn value(&self) -> preserves::IOValue {
        let detail = match self {
            Self::Denied(reason)
            | Self::Allocation(reason)
            | Self::Trap(reason)
            | Self::Harness(reason)
            | Self::Uncertain(reason)
            | Self::Cancelled(reason)
            | Self::Proof(reason) => vec![crate::codec::string(reason)],
            Self::Memory | Self::InvalidOutput | Self::Stack | Self::TotalFuel => Vec::new(),
        };
        crate::codec::record("spacewasm-failure-v1", vec![
            crate::codec::string(self.class()),
            crate::codec::sequence(detail),
        ])
    }
}
