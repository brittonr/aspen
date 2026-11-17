/// Constructor coverage uses canonical payloads, not fabricated valid records.
/// Runtime data-dependent outcomes still require separate invariant evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; CoverageRow is the unchanged public coverage API within the audited schema::coverage module; preserving that source identity avoids schema and extraction drift."
)]
pub struct CoverageRow {
    pub state: crate::async_tasks::State,
    pub event: super::EventKind,
    pub rule: Result<super::Rule, crate::async_tasks::Error>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; CoverageError is the unchanged public rejection type within the audited schema::coverage module; its source identity is part of the coverage extraction contract."
)]
pub enum CoverageError {
    StateSchema,
    EventSchema,
    ExcessRows,
    DuplicatePair,
    MissingPair,
    WrongDisposition,
}

/// Bounded allocation-free coverage gate for the actual production classifier.
/// Unknown textual constructors must be rejected before constructing typed rows.
/// Exact schema bytes include constructor and transitive payload-field layouts.
// r[impl DX-PROTOCOL-03]
#[expect(
    tigerstyle::path_segment_repetition,
    reason = "Owner: noble-maintainers; the compiler inventory and M6 audit bind this exact schema::coverage::validate_coverage path; renaming it would change the source-bound coverage admission contract."
)]
pub fn validate_coverage(
    state_schema: &[u8],
    event_schema: &[u8],
    rows: &[CoverageRow],
) -> Result<(), CoverageError> {
    if state_schema != super::STATE_SCHEMA.as_bytes() {
        return Err(CoverageError::StateSchema);
    }
    if event_schema != super::EVENT_SCHEMA.as_bytes() {
        return Err(CoverageError::EventSchema);
    }
    validate_rows(rows)
}

#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; the complete-pair mask subtracts one from the fixed positive value 2^75, which is representable in u128; the row count is at most 75 and its loop index advances only below that bound."
)]
fn validate_rows(rows: &[CoverageRow]) -> Result<(), CoverageError> {
    if rows.len() > 75 {
        return Err(CoverageError::ExcessRows);
    }
    let mut seen = 0_u128;
    let mut index = 0;
    while index < rows.len() {
        seen = attempt!(check_row(seen, rows[index]));
        index += 1;
    }
    if seen != (1_u128 << 75) - 1 {
        return Err(CoverageError::MissingPair);
    }
    Ok(())
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; check_row compares the complete classified Result<Rule, Error> through derived PartialEq, which is not stable const; reassess when that trait supports const evaluation."
)]
#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; exhaustive state_index and EventKind::index mappings return at most 4 and 14 respectively, so state_index * 15 + event_index is at most 74 and fits u32 and the u128 coverage mask."
)]
fn check_row(seen: u128, row: CoverageRow) -> Result<u128, CoverageError> {
    let bit = 1_u128 << (state_index(row.state) * 15 + row.event.index());
    if seen & bit != 0 {
        return Err(CoverageError::DuplicatePair);
    }
    let expected = super::classify(row.state, row.event.representative());
    if row.rule != expected {
        return Err(CoverageError::WrongDisposition);
    }
    Ok(seen | bit)
}

const fn state_index(state: crate::async_tasks::State) -> u32 {
    match state {
        crate::async_tasks::State::Pending => 0,
        crate::async_tasks::State::Ready => 1,
        crate::async_tasks::State::Delivered => 2,
        crate::async_tasks::State::Retiring => 3,
        crate::async_tasks::State::Retired => 4,
    }
}

impl super::EventKind {
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; all fifteen EventKind constructors require distinct coverage bits; adding a constructor must force an explicit index and coverage-width review rather than inheriting a fallback bit."
    )]
    const fn index(self) -> u32 {
        match self {
            Self::Inspect => 0,
            Self::CompleteSuccess => 1,
            Self::CompleteDomainError => 2,
            Self::Deliver => 3,
            Self::Cancel => 4,
            Self::Trap => 5,
            Self::Deadline => 6,
            Self::Budget => 7,
            Self::InternalFailure => 8,
            Self::NativeStopped => 9,
            Self::SettlePins => 10,
            Self::Cleanup => 11,
            Self::Wake => 12,
            Self::TakeWake => 13,
            Self::Finish => 14,
        }
    }

    /// Stable metadata for constructor classification; this does not claim that
    /// the event's data preconditions hold for any particular retained task.
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; every EventKind must construct its corresponding production Event with canonical payloads; a new constructor must require an explicit coverage representative."
    )]
    pub const fn representative(self) -> crate::async_tasks::Event {
        let native = crate::async_tasks::NativeId(1);
        let completion = crate::async_tasks::Completion {
            inputs: crate::async_tasks::Disposition {
                returned: 0,
                consumed: 0,
                retired: 0,
            },
            produced: 0,
            bytes: 0,
        };
        match self {
            Self::Inspect => crate::async_tasks::Event::Inspect,
            Self::CompleteSuccess => {
                crate::async_tasks::Event::CompleteSuccess { native, completion }
            }
            Self::CompleteDomainError => {
                crate::async_tasks::Event::CompleteDomainError { native, completion }
            }
            Self::Deliver => crate::async_tasks::Event::Deliver,
            Self::Cancel => crate::async_tasks::Event::Cancel,
            Self::Trap => crate::async_tasks::Event::Trap,
            Self::Deadline => crate::async_tasks::Event::Deadline,
            Self::Budget => crate::async_tasks::Event::Budget,
            Self::InternalFailure => crate::async_tasks::Event::InternalFailure,
            Self::NativeStopped => crate::async_tasks::Event::NativeStopped { native },
            Self::SettlePins => crate::async_tasks::Event::SettlePins { native, pins: 0 },
            Self::Cleanup => {
                crate::async_tasks::Event::Cleanup(crate::async_tasks::Obligations::empty())
            }
            Self::Wake => crate::async_tasks::Event::Wake { native },
            Self::TakeWake => crate::async_tasks::Event::TakeWake,
            Self::Finish => crate::async_tasks::Event::Finish,
        }
    }
}
