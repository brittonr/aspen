pub(super) const MAX_CALLBACK_PUBLICATIONS: usize = 128;

pub(super) fn publish_outcome_values(
    values: &mut dyn crate::system_extension::NativeCallbackValuePort,
    outcome: &crate::system_extension::MaterializedCallbackOutcome,
    maximum: u64,
    observation: &mut super::HostedObservation,
) -> Result<(), String> {
    if outcome.values().count() > MAX_CALLBACK_PUBLICATIONS {
        return Err("component callback publication capacity exhausted before publication".to_string());
    }
    for value in outcome.values() {
        // Retain the attempted boundary before dispatch, with capacity reserved
        // before execution. Invalid receipts must not erase accepted attempts.
        let attempt_index = observation.publications.len();
        // Admitted value and publication references have the same canonical
        // length. Reserve the duplicate reference before this effect boundary.
        let mut publication_ref = String::with_capacity(value.value_ref.len());
        observation.publications.push(super::PublicationObservation {
            value_ref: value.value_ref.clone(),
            byte_count: value.bytes.len() as u64,
            receipt: None,
            failure: None,
        });
        let attempt = observation
            .publications
            .get_mut(attempt_index)
            .ok_or_else(|| "component callback publication attempt missing before dispatch".to_string())?;
        let failure = match values.publish(value, maximum) {
            Err(failure) => Some(failure),
            Ok(receipt) => {
                let receipt = attempt.receipt.insert(receipt);
                let is_invalid = receipt.value_ref != value.value_ref
                    || receipt.byte_count != value.bytes.len() as u64
                    || crate::preserves_rail::validate_content_ref(&receipt.publication_ref).is_err()
                    || receipt.publication_ref.len() > publication_ref.capacity();
                // Keep the original receipt, even when it cannot be admitted.
                if !is_invalid {
                    publication_ref.push_str(&receipt.publication_ref);
                }
                is_invalid.then(|| {
                    crate::system_extension::ValuePortFailure::new(
                        crate::system_extension::ValuePortFailureKind::UnknownAfterAcceptance,
                        "component callback publication receipt invalid or mismatched after acceptance",
                    )
                })
            }
        };
        if let Some(failure) = failure {
            observation.is_publication_uncertain |= failure.may_have_published();
            let failure = attempt.failure.insert(failure);
            return Err(format!("component callback value publication: {failure:?}"));
        }
        observation.publication_refs.push(publication_ref);
    }
    Ok(())
}
