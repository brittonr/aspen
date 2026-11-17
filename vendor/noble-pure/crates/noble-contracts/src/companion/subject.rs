//! Subjects, subject identities, and the plain-compose observation relation.
//!
//! A [`crate::companion::Subject`] is a deterministic, versioned 64-bit fold over the observed
//! recipe events plus the subject/contract bindings. Two companions that
//! differ in contract, evidence, capture, or observations MUST NOT share an
//! identity; replacing evidence MUST NOT change the underlying program's
//! digest. The identity is an integrity index (VC-ID-03), not proof validity.

/// Maximum observed events folded into one subject identity. `observe` is
/// total: events beyond this cap are ignored rather than trapping.
pub const OBSERVATION_CAP: usize = 2048;

/// The recipe event kind that carries one captured runtime `I64` value
/// (`noble-wasm/runtime/reflection.wat` recipe event 1, "I64 payload"). A
/// produced-program observation that contains such an event has genuinely
/// captured that integer, so `instantiate` can decide capture binding from
/// real observation data instead of the driver's word.
pub const CAPTURE_I64_EVENT: u32 = 1;

/// crate::companion::digest::Fold complete observed recipe events into a subject identity. Total and
/// bounded: at most [`OBSERVATION_CAP`] events contribute; the remainder are
/// ignored. Captured runtime `I64` values are additionally surfaced as
/// [`crate::companion::CaptureBinding`] entries: one binding per [`CAPTURE_I64_EVENT`] event,
/// whose `slot` is the event index and whose `value` is the observed payload.
/// The exact revision is committed by the domain separator and
/// [`crate::companion::digest::DIGEST_VERSION`].
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; observe is total over external events, bounds every read and retained push by OBSERVATION_CAP, and commits the original length; rejecting oversized subjects belongs to registry admission, not panic assertions."
)]
pub fn observe(
    events: &[(u32, u64)],
    signatures: crate::companion::InterfaceSignatures,
) -> crate::companion::Subject {
    let mut fold = crate::companion::digest::Fold::new(crate::companion::digest::DOMAIN_SUBJECT);
    fold.absorb(signatures.input);
    fold.absorb(signatures.output);
    fold.absorb_count(events.len());
    let mut captures = alloc::vec::Vec::with_capacity(events.len().min(OBSERVATION_CAP));
    let mut retained = alloc::vec::Vec::with_capacity(events.len().min(OBSERVATION_CAP));
    let mut index = 0usize;
    let mut slot = 0u32;
    while index < events.len() && index < OBSERVATION_CAP {
        let (kind, value) = events[index];
        fold.absorb(u64::from(kind));
        fold.absorb(value);
        retained.push((kind, value));
        if kind == CAPTURE_I64_EVENT {
            captures.push(crate::companion::CaptureBinding { slot, value });
        }
        index = index.saturating_add(1);
        slot = slot.saturating_add(1);
    }
    crate::companion::Subject {
        identity: crate::companion::SubjectDigest(fold.finish()),
        input_signature: signatures.input,
        output_signature: signatures.output,
        captures,
        events: retained,
    }
}

fn initial_captures(
    left: &crate::companion::Subject,
    entry_bound: usize,
) -> alloc::vec::Vec<crate::companion::CaptureBinding> {
    let mut captures = alloc::vec::Vec::with_capacity(entry_bound);
    let mut index = 0usize;
    while index < left.captures.len() {
        captures.push(left.captures[index]);
        index = index.saturating_add(1);
    }
    captures
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; capture_bindings checks event-count conversion and every shifted capture slot, returning ExhaustedReplay or MismatchedSubject instead of asserting representable external observations."
)]
pub(super) fn capture_bindings(
    left: &crate::companion::Subject,
    right: &crate::companion::Subject,
) -> Result<alloc::vec::Vec<crate::companion::CaptureBinding>, crate::companion::Refusal> {
    // Capture slots address events, not the number of earlier captures.
    let event_index = match u32::try_from(left.events.len()) {
        Ok(event_index) => event_index,
        Err(_) => return Err(crate::companion::Refusal::ExhaustedReplay),
    };
    let mut captures = initial_captures(
        left,
        left.captures.len().saturating_add(right.captures.len()),
    );
    let mut index = 0usize;
    while index < right.captures.len() {
        let slot = match right.captures[index].slot.checked_add(event_index) {
            Some(slot) => slot,
            None => return Err(crate::companion::Refusal::MismatchedSubject),
        };
        captures.push(crate::companion::CaptureBinding {
            slot,
            value: right.captures[index].value,
        });
        index = index.saturating_add(1);
    }
    Ok(captures)
}
