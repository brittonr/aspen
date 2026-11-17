//! Bounded effect-identity comparisons used by acceptance and preflight.

/// The first derived identity outside the allowed bound.
pub(crate) fn first_extra(
    derived: &crate::types::EffSet,
    allowed: &crate::types::EffSet,
) -> Option<crate::types::EffId> {
    let ids = derived.as_slice();
    let mut index = 0;
    let mut extra: Option<crate::types::EffId> = None;
    while index < ids.len() {
        if !allowed.contains(ids[index]) {
            extra = Some(ids[index]);
            break;
        }
        index += 1;
    }
    extra
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; first_unknown bounds both scans by their slice lengths and returns the first missing identity or None; absence is an ordinary diagnostic result, not an assertion failure."
)]
pub(crate) fn first_unknown(
    needed: &[crate::types::EffId],
    known: &[crate::types::EffId],
) -> Option<crate::types::EffId> {
    let mut index = 0;
    let mut found: Option<crate::types::EffId> = None;
    while index < needed.len() {
        let id = needed[index];
        let mut is_known = false;
        let mut known_index = 0;
        while known_index < known.len() {
            if known[known_index] == id {
                is_known = true;
                break;
            }
            known_index += 1;
        }
        if !is_known {
            found = Some(id);
            break;
        }
        index += 1;
    }
    found
}
