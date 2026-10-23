// The reference extension's protocol projections.
//
// The reference services own their projection meaning: each state projects to
// a canonical Preserves value with explicit field order and sorted keys, never
// to Rust debug text. A separate decoder reads projected values back for the
// independent oracle, so the oracle never inspects the transition's own state.

use std::collections::BTreeMap;

use super::*;
use crate::protocol_oracle::CanonicalValue;

/// The protocol name every reference projection carries.
pub const REFERENCE_PROTOCOL: &str = "molten.reference-services";

/// The canonical projection of a reference service state.
// r[impl molten.fabric_simulation.protocol_projection]
pub fn reference_state_value(state: &ReferenceServiceState) -> CanonicalValue {
    match state {
        ReferenceServiceState::TransactionalKeyValue(state) => CanonicalValue::record("transactional-key-value", vec![
            CanonicalValue::unsigned(state.version),
            CanonicalValue::Sequence(
                state
                    .values
                    .iter()
                    .map(|(key, value)| {
                        CanonicalValue::record("entry", vec![
                            CanonicalValue::string(key),
                            CanonicalValue::string(value),
                        ])
                    })
                    .collect(),
            ),
            CanonicalValue::optional(state.recovered_version.map(CanonicalValue::unsigned)),
        ]),
        ReferenceServiceState::ReplicatedLog(state) => CanonicalValue::record("replicated-log", vec![
            CanonicalValue::unsigned(state.next_offset),
            CanonicalValue::unsigned(state.retained_from),
            CanonicalValue::optional(state.replicated_through.map(CanonicalValue::unsigned)),
            CanonicalValue::Sequence(
                state
                    .entries
                    .iter()
                    .map(|entry| {
                        CanonicalValue::record("entry", vec![
                            CanonicalValue::unsigned(entry.offset),
                            CanonicalValue::string(&entry.payload_ref),
                        ])
                    })
                    .collect(),
            ),
        ]),
        ReferenceServiceState::DistributedScheduler(state) => {
            CanonicalValue::record("distributed-scheduler", vec![CanonicalValue::Sequence(
                state
                    .jobs
                    .iter()
                    .map(|(job_id, job)| {
                        CanonicalValue::record("job", vec![
                            CanonicalValue::string(job_id),
                            CanonicalValue::symbol(job.phase.as_str()),
                            CanonicalValue::optional(job.lease_owner.as_deref().map(CanonicalValue::string)),
                            CanonicalValue::unsigned(job.lease_epoch),
                            CanonicalValue::optional(
                                job.authoritative_completion_ref.as_deref().map(CanonicalValue::string),
                            ),
                        ])
                    })
                    .collect(),
            )])
        }
    }
}

/// The canonical identity of a reference state.
pub fn reference_state_ref(state: &ReferenceServiceState) -> String {
    crate::protocol_oracle::state_ref_of(&reference_state_value(state))
}

/// The logical position of a reference state: the transaction version, the
/// next log offset, or the scheduler's applied transition count.
pub const fn reference_logical_position(state: &ReferenceServiceState, applied_transitions: u64) -> u64 {
    match state {
        ReferenceServiceState::TransactionalKeyValue(state) => state.version,
        ReferenceServiceState::ReplicatedLog(state) => state.next_offset,
        ReferenceServiceState::DistributedScheduler(_) => applied_transitions,
    }
}

/// Decode a projected value back into a reference state, or `None` when the
/// value is not a well-formed reference projection.
pub fn decode_reference_state(value: &CanonicalValue) -> Option<ReferenceServiceState> {
    let CanonicalValue::Record { label, fields } = value else {
        return None;
    };
    match (label.as_str(), fields.as_slice()) {
        ("transactional-key-value", [version, values, recovered]) => {
            let mut decoded = BTreeMap::new();
            for entry in sequence(values)? {
                let [key, value] = record(entry, "entry")? else {
                    return None;
                };
                decoded.insert(string(key)?, string(value)?);
            }
            Some(ReferenceServiceState::TransactionalKeyValue(TransactionalKeyValueState {
                version: unsigned(version)?,
                values: decoded,
                recovered_version: optional(recovered, unsigned)?,
            }))
        }
        ("replicated-log", [next_offset, retained_from, replicated, entries]) => {
            let mut decoded = Vec::with_capacity(sequence(entries)?.len());
            for entry in sequence(entries)? {
                let [offset, payload_ref] = record(entry, "entry")? else {
                    return None;
                };
                decoded.push(ReplicatedLogEntry {
                    offset: unsigned(offset)?,
                    payload_ref: string(payload_ref)?,
                });
            }
            Some(ReferenceServiceState::ReplicatedLog(ReplicatedLogState {
                next_offset: unsigned(next_offset)?,
                retained_from: unsigned(retained_from)?,
                replicated_through: optional(replicated, unsigned)?,
                entries: decoded,
            }))
        }
        ("distributed-scheduler", [jobs]) => {
            let mut decoded = BTreeMap::new();
            for job in sequence(jobs)? {
                let [job_id, phase, owner, epoch, completion] = record(job, "job")? else {
                    return None;
                };
                decoded.insert(string(job_id)?, ReferenceJobState {
                    phase: phase_of(phase)?,
                    lease_owner: optional(owner, string)?,
                    lease_epoch: unsigned(epoch)?,
                    authoritative_completion_ref: optional(completion, string)?,
                });
            }
            Some(ReferenceServiceState::DistributedScheduler(DistributedSchedulerState { jobs: decoded }))
        }
        _ => None,
    }
}

fn record<'a>(value: &'a CanonicalValue, expected: &str) -> Option<&'a [CanonicalValue]> {
    match value {
        CanonicalValue::Record { label, fields } if label == expected => Some(fields),
        _ => None,
    }
}

fn sequence(value: &CanonicalValue) -> Option<&[CanonicalValue]> {
    match value {
        CanonicalValue::Sequence(items) => Some(items),
        _ => None,
    }
}

fn unsigned(value: &CanonicalValue) -> Option<u64> {
    match value {
        CanonicalValue::Integer(integer) => u64::try_from(*integer).ok(),
        _ => None,
    }
}

fn string(value: &CanonicalValue) -> Option<String> {
    match value {
        CanonicalValue::String(text) => Some(text.clone()),
        _ => None,
    }
}

fn optional<T>(value: &CanonicalValue, decode: fn(&CanonicalValue) -> Option<T>) -> Option<Option<T>> {
    match value {
        CanonicalValue::Record { label, fields } if label == "none" && fields.is_empty() => Some(None),
        CanonicalValue::Record { label, fields } if label == "some" => match fields.as_slice() {
            [inner] => decode(inner).map(Some),
            _ => None,
        },
        _ => None,
    }
}

fn phase_of(value: &CanonicalValue) -> Option<ReferenceJobPhase> {
    match value {
        CanonicalValue::Symbol(name) => match name.as_str() {
            "pending" => Some(ReferenceJobPhase::Pending),
            "leased" => Some(ReferenceJobPhase::Leased),
            "completed" => Some(ReferenceJobPhase::Completed),
            _ => None,
        },
        _ => None,
    }
}
