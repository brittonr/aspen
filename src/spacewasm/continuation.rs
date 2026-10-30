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
