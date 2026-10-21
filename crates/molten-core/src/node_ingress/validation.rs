const BLAKE3_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_CHAR_COUNT: usize = 64;
const MAX_OPERATION_CHARS: usize = 256;

pub(super) fn has_valid_identity(input: &super::ReconcileInput<'_>) -> bool {
    is_canonical_ref(input.operation_ref)
        && is_canonical_ref(input.request_ref)
        && is_valid_operation(input.operation)
        && input
            .intent
            .is_none_or(|intent| intent.operation_ref == input.operation_ref && intent.request_ref == input.request_ref)
}

pub(super) fn has_conflicting_observation(input: &super::ReconcileInput<'_>) -> bool {
    [input.inbox, input.queue, input.dispatch].into_iter().any(|observation| match observation {
        super::PublicationObservation::Absent | super::PublicationObservation::Unavailable => false,
        super::PublicationObservation::Conflict => true,
        super::PublicationObservation::Bound { operation, request_ref } => {
            operation != input.operation || request_ref != input.request_ref
        }
    })
}

pub(super) fn has_unavailable_observation(input: &super::ReconcileInput<'_>) -> bool {
    [input.inbox, input.queue, input.dispatch].into_iter().any(|observation| match observation {
        super::PublicationObservation::Unavailable => true,
        super::PublicationObservation::Absent
        | super::PublicationObservation::Bound { .. }
        | super::PublicationObservation::Conflict => false,
    })
}

// Follow the fabric boundary's lowercase BLAKE3 reference and bounded token rules.
fn is_canonical_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return false;
    };
    hex.len() == BLAKE3_HEX_CHAR_COUNT && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}

fn is_valid_operation(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_OPERATION_CHARS
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':'))
}
