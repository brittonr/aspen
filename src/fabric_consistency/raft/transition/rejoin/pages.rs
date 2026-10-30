use super::*;

pub(super) enum Progress {
    Ignored,
    More(u64),
    Complete,
}

pub(super) fn append(
    observation: &mut RecoveryObservation,
    from_index: u64,
    entries: Vec<ReplicatedEntry>,
) -> crate::error::Result<Progress> {
    let previous = observation.entries.last().map_or_else(
        || observation.snapshot.as_ref().map_or(0, |snapshot| snapshot.last_included_index),
        |entry| entry.index,
    );
    let expected = previous
        .checked_add(1)
        .ok_or_else(|| crate::error::Failure::invalid_harness("recovery page index overflow"))?;
    if from_index != INITIAL_LOG_INDEX && from_index != expected {
        return Ok(Progress::Ignored);
    }
    if observation
        .entries
        .len()
        .checked_add(entries.len())
        .is_none_or(|length| length > MAX_REPLICA_LOG_ENTRIES)
    {
        return Err(crate::error::Failure::invalid_harness("recovery observation exceeds the log bound"));
    }
    let through = entries.last().map_or(previous, |entry| entry.index);
    observation.entries.extend(entries);
    observation.is_complete = through == observation.last_log_index;
    if !observation.is_complete {
        return through
            .checked_add(1)
            .map(Progress::More)
            .ok_or_else(|| crate::error::Failure::invalid_harness("recovery continuation overflow"));
    }
    if canonical::recovery_corpus_ref(
        observation.current_term,
        observation.commit_index,
        &observation.entries,
        observation.snapshot.as_deref(),
    )? != observation.observation_ref
    {
        return Err(crate::error::Failure::invalid_harness("assembled recovery observation identity mismatch"));
    }
    Ok(Progress::Complete)
}
