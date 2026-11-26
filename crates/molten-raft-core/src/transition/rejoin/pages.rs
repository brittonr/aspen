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
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<Progress> {
    let previous = observation.entries.last().map_or_else(
        || observation.snapshot.as_ref().map_or(0, |snapshot| snapshot.last_included_index),
        |entry| entry.index,
    );
    let expected = previous
        .checked_add(1)
        .ok_or(crate::denial::ReplicaError::Overflow("recovery page index overflow"))?;
    if from_index != INITIAL_LOG_INDEX && from_index != expected {
        return Ok(Progress::Ignored);
    }
    if observation
        .entries
        .len()
        .checked_add(entries.len())
        .is_none_or(|length| length > MAX_REPLICA_LOG_ENTRIES)
    {
        return Err(crate::denial::ReplicaError::Denied("recovery observation exceeds the log bound"));
    }
    let through = entries.last().map_or(previous, |entry| entry.index);
    observation.entries.extend(entries);
    observation.is_complete = through == observation.last_log_index;
    if !observation.is_complete {
        return through
            .checked_add(1)
            .map(Progress::More)
            .ok_or(crate::denial::ReplicaError::Overflow("recovery continuation overflow"));
    }
    let assembled = corpus_ref(identity, &crate::identity::RecoveryCorpus {
        term: observation.current_term,
        commit_index: observation.commit_index,
        log: &observation.entries,
        snapshot: observation.snapshot.as_deref(),
    })?;
    if assembled != observation.observation_ref {
        return Err(crate::denial::ReplicaError::Denied("assembled recovery observation identity mismatch"));
    }
    Ok(Progress::Complete)
}
