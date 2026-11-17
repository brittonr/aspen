use super::*;
use alloc::vec::Vec;
use core::cmp::Ordering;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
    pub exact_interface: Option<(Vec<Ty>, Vec<Ty>)>,
    /// Concrete full stacks immediately before and after the hole. The common
    /// bottom tail is solved exactly; no suffix-only signature guess is used.
    pub composition_context: Option<(Vec<Ty>, Vec<Ty>)>,
    /// Return only records whose transitive closure contains this identity.
    pub dependency_root: Option<ProcedureId>,
    pub status: Option<Status>,
}

#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeProducer {
    Proposer,
    Teacher,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryBudget {
    queries: u32,
    work: u32,
    proposer_attempts: u32,
    teacher_attempts: u32,
}

impl QueryBudget {
    pub const fn new() -> Self {
        Self {
            queries: 0,
            work: 0,
            proposer_attempts: 0,
            teacher_attempts: 0,
        }
    }

    pub const fn queries(&self) -> u32 {
        self.queries
    }

    pub const fn work(&self) -> u32 {
        self.work
    }

    pub const fn proposer_attempts(&self) -> u32 {
        self.proposer_attempts
    }

    pub const fn teacher_attempts(&self) -> u32 {
        self.teacher_attempts
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResult {
    /// Candidate identities only. These are not programs or authority.
    pub candidates: Vec<ProcedureId>,
    pub examined_entries: u32,
    pub index_bytes: u32,
}

pub(super) fn signature(record: &ProcedureRecord) -> RetrievalSignature {
    RetrievalSignature {
        stack_in: record.chunk.interface.stack_in.clone(),
        stack_out: record.chunk.interface.stack_out.clone(),
        effects: record.chunk.interface.effects.clone(),
        status_rank: record.status.rank(),
        cost_key: record.cost.key(),
    }
}

pub(super) fn compare_records(left: &ProcedureRecord, right: &ProcedureRecord) -> Ordering {
    compare_stack(
        &left.chunk.interface.stack_in,
        &right.chunk.interface.stack_in,
    )
    .then_with(|| {
        compare_stack(
            &left.chunk.interface.stack_out,
            &right.chunk.interface.stack_out,
        )
    })
    .then_with(|| {
        left.chunk
            .interface
            .effects
            .is_empty()
            .cmp(&right.chunk.interface.effects.is_empty())
    })
    .then_with(|| left.status.rank().cmp(&right.status.rank()))
    .then_with(|| left.cost.key().cmp(&right.cost.key()))
    .then_with(|| left.procedure_id.cmp(&right.procedure_id))
}

fn compare_stack(left: &[Ty], right: &[Ty]) -> Ordering {
    let mut at = 0usize;
    while let (Some(left_ty), Some(right_ty)) = (left.get(at), right.get(at)) {
        let order = type_rank(left_ty).cmp(&type_rank(right_ty));
        if order != Ordering::Equal {
            return order;
        }
        at = at.saturating_add(1);
    }
    left.len().cmp(&right.len())
}

const fn type_rank(ty: &Ty) -> u8 {
    match ty {
        Ty::I64 => 0,
        Ty::Bool => 1,
        _ => 2,
    }
}

impl ProcedureLibrary {
    /// A retrieval miss must not invoke a host proposer or teacher. The
    /// caller's callback is held only to demonstrate that it is never reached;
    /// the attempt is counted separately from actual producer calls.
    pub fn request_runtime_producer(
        &self,
        producer: RuntimeProducer,
        budget: &mut QueryBudget,
        _invoke: impl FnOnce() -> ProcedureRecord,
    ) -> Result<(), Refusal> {
        let attempts = match producer {
            RuntimeProducer::Proposer => &mut budget.proposer_attempts,
            RuntimeProducer::Teacher => &mut budget.teacher_attempts,
        };
        *attempts = attempts.checked_add(1).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "retrieval.runtime_producer",
                    message: "runtime producer attempt counter overflow",
                },
            )
        })?;
        Err(Refusal::new(
            RefusalKind::Unsupported,
            super::Diagnostic {
                field: "retrieval.runtime_producer",
                message: "runtime proposer and teacher fallback is disabled",
            },
        ))
    }

    pub fn retrieve(
        &self,
        query: &Query,
        budget: &mut QueryBudget,
    ) -> Result<QueryResult, Refusal> {
        if budget.queries >= 32 {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "retrieval.queries",
                    message: "retrieval query limit reached",
                },
            ));
        }
        budget.queries = budget.queries.checked_add(1).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "retrieval.queries",
                    message: "query counter overflow",
                },
            )
        })?;
        let index_bytes = self.records().len().checked_mul(32).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "retrieval.index",
                    message: "index size overflow",
                },
            )
        })?;
        if self.records().len() > MAX_RECORDS || index_bytes > MAX_INDEX_BYTES {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "retrieval.index",
                    message: "canonical retrieval index exceeds its declared bound",
                },
            ));
        }
        let mut examined_entries = 0u32;
        let mut candidates = Vec::with_capacity(self.records().len());
        let dependency_matches = if let Some(root) = query.dependency_root {
            let (matches, closure_work) = dependent_closure(self.records(), root)?;
            let work = budget.work.checked_add(closure_work).ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "retrieval.work",
                        message: "work counter overflow",
                    },
                )
            })?;
            budget.work = work;
            if work > 4_096 {
                return Err(Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "retrieval.work",
                        message: "dependency closure work limit reached",
                    },
                ));
            }
            Some(matches)
        } else {
            None
        };
        for record in self.records() {
            examined_entries = examined_entries.checked_add(1).ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "retrieval.examined",
                        message: "entry count overflow",
                    },
                )
            })?;
            if examined_entries > 128 {
                return Err(Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "retrieval.examined",
                        message: "candidate examination limit reached",
                    },
                ));
            }
            let next_work = budget.work.checked_add(1).ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "retrieval.work",
                        message: "work counter overflow",
                    },
                )
            })?;
            budget.work = next_work;
            if next_work > 4_096 {
                return Err(Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "retrieval.work",
                        message: "retrieval work limit reached",
                    },
                ));
            }
            if query.status.is_some_and(|status| status != record.status) {
                continue;
            }
            if query
                .exact_interface
                .as_ref()
                .is_some_and(|(input, output)| {
                    input != &record.chunk.interface.stack_in
                        || output != &record.chunk.interface.stack_out
                })
            {
                continue;
            }
            if query
                .composition_context
                .as_ref()
                .is_some_and(|(before, after)| {
                    !compatible_context(
                        &record.chunk.interface.stack_in,
                        &record.chunk.interface.stack_out,
                        before,
                        after,
                    )
                })
            {
                continue;
            }
            if dependency_matches
                .as_ref()
                .is_some_and(|matches| !matches.contains(&record.procedure_id))
            {
                continue;
            }
            candidates.push(record.procedure_id);
        }
        Ok(QueryResult {
            candidates,
            examined_entries,
            index_bytes: u32::try_from(index_bytes).map_err(|_| {
                Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "retrieval.index",
                        message: "index byte count exceeds the result wire domain",
                    },
                )
            })?,
        })
    }
}

fn compatible_context(stack_in: &[Ty], stack_out: &[Ty], before: &[Ty], after: &[Ty]) -> bool {
    let Some(input_tail) = before.len().checked_sub(stack_in.len()) else {
        return false;
    };
    let Some(output_tail) = after.len().checked_sub(stack_out.len()) else {
        return false;
    };
    input_tail == output_tail
        && before.get(input_tail..) == Some(stack_in)
        && after.get(output_tail..) == Some(stack_out)
        && before.get(..input_tail) == after.get(..output_tail)
}

fn dependent_closure(
    records: &[ProcedureRecord],
    root: ProcedureId,
) -> Result<(Vec<ProcedureId>, u32), Refusal> {
    if !records.iter().any(|record| record.procedure_id == root) {
        return Ok((Vec::new(), 0));
    }
    let bounded_record_count = records.len().min(MAX_RECORDS);
    let mut pending = Vec::with_capacity(bounded_record_count);
    let mut visited = Vec::with_capacity(bounded_record_count);
    pending.push(root);
    let mut work = 0u32;
    while let Some(current) = pending.pop() {
        if visited.contains(&current) {
            continue;
        }
        if visited.len() >= MAX_RECORDS {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "retrieval.dependency_root",
                    message: "dependency closure exceeds its record bound",
                },
            ));
        }
        visited.push(current);
        debug_assert!(visited.len() <= MAX_RECORDS);
        for record in records {
            work = work.checked_add(1).ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "retrieval.work",
                        message: "dependency work overflow",
                    },
                )
            })?;
            let mut is_dependent_on_current = false;
            for dependency in &record.library_dependencies {
                work = work.checked_add(1).ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::Exhausted,
                        super::Diagnostic {
                            field: "retrieval.work",
                            message: "dependency work overflow",
                        },
                    )
                })?;
                if dependency.procedure_id == current {
                    is_dependent_on_current = true;
                    break;
                }
            }
            if is_dependent_on_current
                && !visited.contains(&record.procedure_id)
                && !pending.contains(&record.procedure_id)
            {
                if pending.len() >= MAX_RECORDS {
                    return Err(Refusal::new(
                        RefusalKind::Exhausted,
                        super::Diagnostic {
                            field: "retrieval.dependency_root",
                            message: "dependency closure exceeds its record bound",
                        },
                    ));
                }
                pending.push(record.procedure_id);
            }
        }
    }
    Ok((visited, work))
}
