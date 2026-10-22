//! Explicit readmission of work at the system-extension boundary.
//!
//! Durable logical work may survive a restart, but it acts only under a fresh
//! delivery claim bound to the current incarnation. Runtime-local work is
//! fenced by the incarnation alone and is never readmitted as durable work.
//! Durable work is never treated as a still-valid callback of a previous
//! instance.

use super::LifecycleState;
use super::valid_ref;

/// The class of the work being presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkClass {
    RuntimeLocal,
    Durable,
}

impl WorkClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeLocal => "runtime-local",
            Self::Durable => "durable",
        }
    }
}

/// A delivery claim that authorizes a presentation to act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryClaim {
    pub claim_ref: String,
    pub class: WorkClass,
    pub generation: u64,
    pub incarnation: u64,
}

/// The work presented for admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkPresentation {
    pub work_ref: String,
    pub class: WorkClass,
    pub generation: u64,
    pub incarnation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadmissionIssue {
    MalformedWorkRef(String),
    MalformedClaimRef(String),
    MissingClaim,
    ClassMismatch { presented: WorkClass, claimed: WorkClass },
    StaleGeneration { actual: u64, active: u64 },
    StaleIncarnation { actual: u64, active: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadmissionAdmission {
    pub work_ref: String,
    pub claim_ref: Option<String>,
    pub class: WorkClass,
    pub generation: u64,
    pub incarnation: u64,
}

/// Admit one presented work item against the executing instance.
///
/// Runtime-local work must carry the current incarnation and never carries a
/// durable claim. Durable work may carry an earlier incarnation, but it needs
/// a claim bound to the current generation and incarnation, and a claim of one
/// class never admits work of the other.
// r[impl molten.system_extension.durable_readmission]
pub fn admit_work_delivery(
    state: &LifecycleState,
    presentation: &WorkPresentation,
    claim: Option<&DeliveryClaim>,
) -> Result<ReadmissionAdmission, Vec<ReadmissionIssue>> {
    let mut issues = Vec::new();
    if !valid_ref(&presentation.work_ref) {
        issues.push(ReadmissionIssue::MalformedWorkRef(presentation.work_ref.clone()));
    }
    if presentation.generation != state.generation {
        issues.push(ReadmissionIssue::StaleGeneration {
            actual: presentation.generation,
            active: state.generation,
        });
    }
    match presentation.class {
        WorkClass::RuntimeLocal => {
            if let Some(claim) = claim {
                if claim.class != WorkClass::RuntimeLocal {
                    issues.push(ReadmissionIssue::ClassMismatch {
                        presented: presentation.class,
                        claimed: claim.class,
                    });
                }
                if !valid_ref(&claim.claim_ref) {
                    issues.push(ReadmissionIssue::MalformedClaimRef(claim.claim_ref.clone()));
                }
            }
            if presentation.incarnation != state.incarnation {
                issues.push(ReadmissionIssue::StaleIncarnation {
                    actual: presentation.incarnation,
                    active: state.incarnation,
                });
            }
        }
        WorkClass::Durable => {
            let Some(claim) = claim else {
                issues.push(ReadmissionIssue::MissingClaim);
                return Err(issues);
            };
            if !valid_ref(&claim.claim_ref) {
                issues.push(ReadmissionIssue::MalformedClaimRef(claim.claim_ref.clone()));
            }
            if claim.class != WorkClass::Durable {
                issues.push(ReadmissionIssue::ClassMismatch {
                    presented: presentation.class,
                    claimed: claim.class,
                });
            }
            if claim.generation != state.generation {
                issues.push(ReadmissionIssue::StaleGeneration {
                    actual: claim.generation,
                    active: state.generation,
                });
            }
            if claim.incarnation != state.incarnation {
                issues.push(ReadmissionIssue::StaleIncarnation {
                    actual: claim.incarnation,
                    active: state.incarnation,
                });
            }
            if presentation.incarnation > state.incarnation {
                issues.push(ReadmissionIssue::StaleIncarnation {
                    actual: presentation.incarnation,
                    active: state.incarnation,
                });
            }
        }
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(ReadmissionAdmission {
        work_ref: presentation.work_ref.clone(),
        claim_ref: claim.map(|claim| claim.claim_ref.clone()),
        class: presentation.class,
        generation: state.generation,
        incarnation: state.incarnation,
    })
}
