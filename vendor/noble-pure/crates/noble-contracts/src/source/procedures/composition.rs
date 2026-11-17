use super::super::Session;
use super::*;
use crate::companion::{interface_signature, Core, Derived, EvidenceId, Subject};
use crate::Limits;
use alloc::vec::Vec;

struct BoundedProgramBytes {
    bytes: Vec<u8>,
}

impl BoundedProgramBytes {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn append(&mut self, part: &[u8]) -> Result<(), Refusal> {
        let should_add_separator = !self.bytes.is_empty()
            && self.bytes.last() != Some(&b' ')
            && !part.is_empty()
            && part.first() != Some(&b' ');
        let separator_bytes = if should_add_separator { 1usize } else { 0usize };
        let next_len = self
            .bytes
            .len()
            .checked_add(separator_bytes)
            .and_then(|length| length.checked_add(part.len()))
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Exhausted,
                    super::Diagnostic {
                        field: "composition.source",
                        message: "composed source length overflow",
                    },
                )
            })?;
        let max_source_bytes = usize::try_from(chunks::MAX_SOURCE_BYTES).map_err(|_| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "composition.source",
                    message: "source limit exceeds the host address space",
                },
            )
        })?;
        if next_len > max_source_bytes {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "composition.source",
                    message: "composed source exceeds typed-chunk profile bound",
                },
            ));
        }
        if should_add_separator {
            self.bytes.push(b' ');
        }
        self.bytes.extend_from_slice(part);
        Ok(())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComposeContext {
    pub prefix: Vec<u8>,
    pub suffix: Vec<u8>,
    pub stack_in: Vec<Ty>,
    pub stack_out: Vec<Ty>,
    pub allowed_effects: EffSet,
    pub limits: Limits,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComposedProgram {
    pub source: Vec<u8>,
    pub interface: RetrievalSignature,
    pub selected: Vec<ProcedureId>,
}

#[derive(Debug)]
pub struct BoundOperand {
    procedure_id: ProcedureId,
    evidence: EvidenceId,
    subject: Subject,
}

impl BoundOperand {
    pub const fn procedure_id(&self) -> &ProcedureId {
        &self.procedure_id
    }

    pub const fn evidence(&self) -> EvidenceId {
        self.evidence
    }
}

#[derive(Debug)]
pub struct CertifiedComposedProgram {
    pub composition: ComposedProgram,
    pub derivation: Derived,
}

impl ProcedureLibrary {
    /// Compose selected records as a fresh source candidate. The caller supplies
    /// the exact one-hole context and limits; neither retrieval nor stored IDs
    /// authorize the resulting source.
    pub fn compose(
        &self,
        namespace: &Session,
        selected: &[ProcedureId],
        context: &ComposeContext,
    ) -> Result<ComposedProgram, Refusal> {
        let max_composed_chunks = usize::try_from(chunks::MAX_COMPOSED_CHUNKS).map_err(|_| {
            Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "composition.selected",
                    message: "composition limit exceeds the host address space",
                },
            )
        })?;
        if selected.is_empty() || selected.len() > max_composed_chunks {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                super::Diagnostic {
                    field: "composition.selected",
                    message: "composition must select one through four admitted procedures",
                },
            ));
        }
        let mut source = BoundedProgramBytes::new();
        let mut retained_ids = Vec::with_capacity(selected.len());
        source.append(&context.prefix)?;
        for id in selected {
            let record = self
                .records()
                .iter()
                .find(|record| record.procedure_id == *id)
                .ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::UnknownRecord,
                        super::Diagnostic {
                            field: "composition.selected",
                            message: "selected identity is not in the current generation",
                        },
                    )
                })?;
            if record.chunk.interface.effects != EffSet::empty() {
                return Err(Refusal::new(
                    RefusalKind::Unsupported,
                    super::Diagnostic {
                        field: "chunk.interface.effects",
                        message: "a selected procedure has a nonempty effect closure",
                    },
                ));
            }
            let entry = self
                .chunk_library
                .entries()
                .iter()
                .find(|entry| {
                    entry.envelope().chunk_id == record.chunk.chunk_id
                        && entry.envelope().source_revision == record.chunk.source_revision
                })
                .ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::Invalid,
                        super::Diagnostic {
                            field: "composition.selected",
                            message:
                                "selected procedure is absent from the admitted #73 chunk view",
                        },
                    )
                })?;
            source.append(entry.source())?;
            retained_ids.push(*id);
        }
        source.append(&context.suffix)?;
        let source = source.into_bytes();
        let prepared = namespace
            .prepare(&source, &context.stack_in, context.limits)
            .map_err(|_| {
                Refusal::new(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field: "composition.source",
                        message: "fresh composed source was refused by ordinary source preparation",
                    },
                )
            })?;
        if prepared.output() != context.stack_out {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "composition.interface",
                    message: "freshly derived output interface differs from caller contract",
                },
            ));
        }
        let checked = prepared.checked().ok_or_else(|| {
            Refusal::new(
                RefusalKind::Internal,
                super::Diagnostic {
                    field: "composition.checked",
                    message: "prepared source omitted its checked kernel derivation",
                },
            )
        })?;
        if !checked
            .interface
            .effects
            .is_subset_of(&context.allowed_effects)
        {
            return Err(Refusal::new(
                RefusalKind::Unsupported,
                super::Diagnostic {
                    field: "composition.effects",
                    message: "fresh source admission widened the caller's allowed effect set",
                },
            ));
        }
        let interface = RetrievalSignature {
            stack_in: context.stack_in.clone(),
            stack_out: prepared.output().to_vec(),
            effects: checked.interface.effects.clone(),
            status_rank: Status::Candidate.rank(),
            cost_key: [0; 3],
        };
        Ok(ComposedProgram {
            source,
            interface,
            selected: retained_ids,
        })
    }

    /// Bind an MC2 proof to the exact retained source artifact. The host-side
    /// core remains the proof authority; this API only carries its checked
    /// result into a later composition.
    pub fn bind_companion_evidence(
        &self,
        namespace: &Session,
        core: &Core,
        procedure_id: ProcedureId,
        evidence: EvidenceId,
        limits: Limits,
    ) -> Result<BoundOperand, Refusal> {
        let record = self
            .records()
            .iter()
            .find(|record| record.procedure_id == procedure_id)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::UnknownRecord,
                    super::Diagnostic {
                        field: "procedure_id",
                        message: "procedure record unavailable",
                    },
                )
            })?;
        let entry = self
            .chunk_library
            .entries()
            .iter()
            .find(|entry| {
                entry.envelope().chunk_id == record.chunk.chunk_id
                    && entry.envelope().source_revision == record.chunk.source_revision
            })
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field: "chunk",
                        message: "admitted chunk source unavailable",
                    },
                )
            })?;
        let prepared = namespace
            .prepare(entry.source(), &record.chunk.interface.stack_in, limits)
            .map_err(|_| {
                Refusal::new(
                    RefusalKind::Invalid,
                    super::Diagnostic {
                        field: "companion.artifact",
                        message: "source artifact was refused by ordinary preparation",
                    },
                )
            })?;
        if prepared.output() != record.chunk.interface.stack_out {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "companion.interface",
                    message: "prepared source interface differs from retained exact contract",
                },
            ));
        }
        let submission = prepared.submission().ok_or_else(|| {
            Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "companion.artifact",
                    message: "prepared artifact has no kernel submission",
                },
            )
        })?;
        let subject = core
            .bind_checked_program(evidence, submission)
            .map_err(Refusal::from_companion)?;
        let expected_input = interface_signature(&record.chunk.interface.stack_in);
        let expected_output = interface_signature(&record.chunk.interface.stack_out);
        if subject.input_signature != expected_input || subject.output_signature != expected_output
        {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                super::Diagnostic {
                    field: "companion.subject",
                    message: "bound MC2 subject interface differs from the retained procedure",
                },
            ));
        }
        Ok(BoundOperand {
            procedure_id,
            evidence,
            subject,
        })
    }

    /// Re-enter MC2 after fresh source preparation; each operand must have been
    /// bound from its exact chunk artifact, and the joint subject is supplied by
    /// the MC2 composition relation itself.
    pub fn compose_certified(
        &self,
        namespace: &Session,
        core: &mut Core,
        left: &BoundOperand,
        right: &BoundOperand,
        context: &ComposeContext,
    ) -> Result<CertifiedComposedProgram, Refusal> {
        if !context.prefix.is_empty() || !context.suffix.is_empty() {
            return Err(Refusal::new(
                RefusalKind::Unsupported,
                super::Diagnostic {
                    field: "composition.context",
                    message:
                        "MC2 ComposeV1 certifies only the exact ordered operands, not added context bytes",
                },
            ));
        }
        let composition =
            self.compose(namespace, &[left.procedure_id, right.procedure_id], context)?;
        let left_subject = core
            .bind(left.evidence, &left.subject)
            .map_err(Refusal::from_companion)?;
        let right_subject = core
            .bind(right.evidence, &right.subject)
            .map_err(Refusal::from_companion)?;
        let joint = Core::compose_subject(&left_subject, &right_subject)
            .map_err(Refusal::from_companion)?;
        let derivation = core
            .derive_compose(left.evidence, right.evidence, &joint)
            .map_err(Refusal::from_companion)?;
        Ok(CertifiedComposedProgram {
            composition,
            derivation,
        })
    }
}
