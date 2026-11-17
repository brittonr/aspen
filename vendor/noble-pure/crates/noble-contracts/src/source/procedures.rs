//! Content-addressed learned procedure records over the typed-chunk/1 library.
//!
//! Procedure identifiers are experimental, encoder-local indexes. They are
//! recomputed from immutable subject fields and never authorize a program;
//! every retained chunk is admitted by `chunks::Library`, and every composed
//! source is freshly checked by `Session::prepare` under the caller's limits.

#![expect(
    tigerstyle::non_trait_imports,
    reason = "Owner: noble-contracts; this public host-side domain uses chunk and companion names directly in its bounded record API"
)]
#![expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-contracts; the public API validates runtime vectors and performs bounded allocation"
)]
#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-contracts; library transitions mutate their explicitly owned generation state after all candidate checks succeed"
)]

mod composition;
mod persistence;
mod retrieval;

pub use composition::{BoundOperand, CertifiedComposedProgram, ComposeContext, ComposedProgram};
pub use persistence::{ArmMetrics, Metrics};
pub use retrieval::{Query, QueryBudget, QueryResult, RuntimeProducer};

use super::chunks;
use alloc::string::String;
use alloc::vec::Vec;
use noble_kernel::types::{EffSet, Ty};

pub const RECORD_FORMAT: u32 = 1;
pub const SEMANTIC_REVISION: u32 = 0;
pub const ENCODER_ID: &str = "noble-procedure-record/1";
pub const ENCODING_REVISION: u32 = 1;
pub const MAX_RECORDS: usize = 64;
pub const MAX_LIBRARY_DEPENDENCIES: usize = 16;
pub const MAX_EVIDENCE: usize = 16;
pub const MAX_GENERATIONS: usize = 16;
pub const MAX_INDEX_BYTES: usize = 65_536;
pub const MAX_MANIFEST_BYTES: usize = 262_144;
pub const MAX_METRICS_BYTES: usize = 65_536;
pub const MAX_STORAGE_BYTES: usize = 262_144;

pub type ProcedureId = [u8; 32];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoder {
    pub id: String,
    pub encoding_revision: u32,
    pub experimental: bool,
}

impl Encoder {
    pub fn current() -> Self {
        Self {
            id: String::from(ENCODER_ID),
            encoding_revision: ENCODING_REVISION,
            experimental: true,
        }
    }
}

#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Producer {
    Chunk,
    Synthesis,
    Composition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub producer: Producer,
    pub producer_tool_revision: u32,
    pub task_id: Option<[u8; 32]>,
    pub grammar_revision: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LibraryDependency {
    pub procedure_id: ProcedureId,
    pub chunk_id: [u8; chunks::IDENTITY_BYTES],
    pub source_revision: [u8; chunks::REVISION_BYTES],
}

#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    Lean,
    AeneasLean,
    Verus,
    Validation,
    Test,
    Review,
    Assumption,
}

impl EvidenceClass {
    const fn rank(self) -> u8 {
        match self {
            Self::Lean => 0,
            Self::AeneasLean => 1,
            Self::Verus => 2,
            Self::Validation => 3,
            Self::Test => 4,
            Self::Review => 5,
            Self::Assumption => 6,
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Lean => "lean",
            Self::AeneasLean => "aeneas-lean",
            Self::Verus => "verus",
            Self::Validation => "validation",
            Self::Test => "test",
            Self::Review => "review",
            Self::Assumption => "assumption",
        }
    }

    const fn proves(self) -> bool {
        matches!(self, Self::Lean | Self::AeneasLean | Self::Verus)
    }
}

#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceOutcome {
    Proved,
    Disproved,
    Unknown,
    Timeout,
    Unsupported,
    Error,
    NotRun,
    Passed,
    Failed,
}

impl EvidenceOutcome {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Proved => "proved",
            Self::Disproved => "disproved",
            Self::Unknown => "unknown",
            Self::Timeout => "timeout",
            Self::Unsupported => "unsupported",
            Self::Error => "error",
            Self::NotRun => "not-run",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }
}

/// This marker is input-only. Any record carrying it is refused before
/// retention; the library never stores an authority-bearing value.
#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityMarker {
    LiveCapability,
    ServiceHandle,
    Session,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub obligation: u32,
    pub class: EvidenceClass,
    pub outcome: EvidenceOutcome,
    pub toolchain: Vec<String>,
    pub claim: String,
    pub subject_binding: ProcedureId,
    pub policy_revision: u32,
    pub capability: Option<CapabilityMarker>,
}

#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Candidate,
    Tested,
    Promoted,
    Superseded,
}

impl Status {
    const fn rank(self) -> u8 {
        match self {
            Self::Promoted => 0,
            Self::Tested => 1,
            Self::Candidate => 2,
            Self::Superseded => 3,
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Tested => "tested",
            Self::Promoted => "promoted",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Promotion {
    pub policy_revision: u32,
    pub obligations: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cost {
    pub expanded_nodes: u32,
    pub emitted_source_bytes: u32,
    pub evaluation_steps: u32,
}

impl Cost {
    const fn key(self) -> [u32; 3] {
        [
            self.expanded_nodes,
            self.emitted_source_bytes,
            self.evaluation_steps,
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetrievalSignature {
    pub stack_in: Vec<Ty>,
    pub stack_out: Vec<Ty>,
    pub effects: EffSet,
    pub status_rank: u8,
    pub cost_key: [u32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alias {
    pub procedure_id: ProcedureId,
    pub certificate: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcedureRecord {
    pub format: u32,
    pub semantic_revision: u32,
    pub encoder: Encoder,
    /// Recomputed on every admission, reload and retrieval.
    pub procedure_id: ProcedureId,
    pub chunk: chunks::Envelope,
    pub provenance: Provenance,
    pub library_dependencies: Vec<LibraryDependency>,
    pub evidence: Vec<EvidenceRecord>,
    pub status: Status,
    pub promotion: Option<Promotion>,
    pub cost: Cost,
    pub retrieval: RetrievalSignature,
    pub supersedes: Option<ProcedureId>,
    pub alias_of: Option<Alias>,
}

impl ProcedureRecord {
    pub fn candidate(chunk: chunks::Envelope, provenance: Provenance) -> Result<Self, Refusal> {
        Self::candidate_with_dependencies(chunk, provenance, Vec::new())
    }

    pub fn candidate_with_dependencies(
        chunk: chunks::Envelope,
        provenance: Provenance,
        library_dependencies: Vec<LibraryDependency>,
    ) -> Result<Self, Refusal> {
        let encoder = Encoder::current();
        let procedure_id = persistence::identity(
            &encoder,
            SEMANTIC_REVISION,
            &chunk,
            &provenance,
            &library_dependencies,
        )?;
        let retrieval = RetrievalSignature {
            stack_in: chunk.interface.stack_in.clone(),
            stack_out: chunk.interface.stack_out.clone(),
            effects: chunk.interface.effects.clone(),
            status_rank: Status::Candidate.rank(),
            cost_key: Cost::default().key(),
        };
        Ok(Self {
            format: RECORD_FORMAT,
            semantic_revision: SEMANTIC_REVISION,
            encoder,
            procedure_id,
            chunk,
            provenance,
            library_dependencies,
            evidence: Vec::new(),
            status: Status::Candidate,
            promotion: None,
            cost: Cost::default(),
            retrieval,
            supersedes: None,
            alias_of: None,
        })
    }

    pub fn identity_digest(&self) -> Result<ProcedureId, Refusal> {
        persistence::identity(
            &self.encoder,
            self.semantic_revision,
            &self.chunk,
            &self.provenance,
            &self.library_dependencies,
        )
    }

    /// The experimental, versioned immutable-subject wire bytes used by the
    /// identity digest. This is not a language-wide canonical encoding.
    pub fn identity_subject_bytes(&self) -> Result<Vec<u8>, Refusal> {
        persistence::identity_bytes(self)
    }

    /// Recompute the public retrieval projection after a state-only edit. This
    /// does not alter content identity or make the edited record admissible.
    pub fn refresh_retrieval_signature(&mut self) {
        self.retrieval = retrieval::signature(self);
    }

    pub fn source_revision(&self) -> &[u8; chunks::REVISION_BYTES] {
        &self.chunk.source_revision
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefusalKind {
    Invalid,
    Unsupported,
    Exhausted,
    Internal,
    DuplicateEntry,
    LiveCapabilityInEvidence,
    UnsupportedEvidenceClass,
    ForgedStatus,
    StaleContext,
    UnresolvedImplication,
    UnknownRecord,
    UnknownGeneration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub kind: RefusalKind,
    pub location: chunks::Location,
    pub message: String,
    pub companion: Option<crate::companion::Refusal>,
}

#[derive(Clone, Copy)]
struct Diagnostic {
    field: &'static str,
    message: &'static str,
}

impl Refusal {
    fn new(kind: RefusalKind, diagnostic: Diagnostic) -> Self {
        Self {
            kind,
            location: chunks::Location::field(diagnostic.field),
            message: String::from(diagnostic.message),
            companion: None,
        }
    }

    fn at(kind: RefusalKind, diagnostic: Diagnostic, index: u32) -> Self {
        Self {
            kind,
            location: chunks::Location::at(diagnostic.field, index),
            message: String::from(diagnostic.message),
            companion: None,
        }
    }

    fn from_chunk(refusal: chunks::Refusal) -> Self {
        let kind = match refusal.class {
            chunks::Class::Invalid => RefusalKind::Invalid,
            chunks::Class::Unsupported => RefusalKind::Unsupported,
            chunks::Class::Exhausted => RefusalKind::Exhausted,
            chunks::Class::Internal => RefusalKind::Internal,
        };
        Self {
            kind,
            location: refusal.location,
            message: refusal.message,
            companion: None,
        }
    }

    fn from_companion(refusal: crate::companion::Refusal) -> Self {
        let kind = match refusal {
            crate::companion::Refusal::UnresolvedImplication => RefusalKind::UnresolvedImplication,
            crate::companion::Refusal::StaleContext => RefusalKind::StaleContext,
            crate::companion::Refusal::UnsupportedEvidenceClass => {
                RefusalKind::UnsupportedEvidenceClass
            }
            crate::companion::Refusal::LiveCapabilityInEvidence => {
                RefusalKind::LiveCapabilityInEvidence
            }
            crate::companion::Refusal::ForgedStatus => RefusalKind::ForgedStatus,
            crate::companion::Refusal::DuplicateEntry => RefusalKind::DuplicateEntry,
            crate::companion::Refusal::ExhaustedRegistry
            | crate::companion::Refusal::ExhaustedReplay => RefusalKind::Exhausted,
            crate::companion::Refusal::UnknownRule
            | crate::companion::Refusal::UnsupportedRuleset
            | crate::companion::Refusal::UnsupportedSemanticRevision
            | crate::companion::Refusal::CyclicDerivation
            | crate::companion::Refusal::DuplicatePremise
            | crate::companion::Refusal::MissingPremise
            | crate::companion::Refusal::WrongPremiseClass
            | crate::companion::Refusal::MismatchedSubject
            | crate::companion::Refusal::MismatchedClaim
            | crate::companion::Refusal::MismatchedContext
            | crate::companion::Refusal::WrongInstantiation
            | crate::companion::Refusal::UnknownContract
            | crate::companion::Refusal::UnknownEvidence
            | crate::companion::Refusal::UnsupportedGuardTemplate
            | crate::companion::Refusal::IneligibleGhost
            | crate::companion::Refusal::InvalidEvidenceEncoding
            | crate::companion::Refusal::CorrespondenceMismatch
            | crate::companion::Refusal::Internal => RefusalKind::Invalid,
        };
        Self {
            kind,
            location: chunks::Location::field("companion"),
            message: String::from("MC2 companion admission refused the composition"),
            companion: Some(refusal),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumerObligation {
    pub id: u32,
    pub allowed_classes: Vec<EvidenceClass>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsumerPolicy {
    pub revision: u32,
    pub obligations: Vec<ConsumerObligation>,
}

#[octet::sealed_enum]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationKind {
    Publish,
    Rollback,
}

impl GenerationKind {
    const fn code(self) -> u8 {
        match self {
            Self::Publish => 0,
            Self::Rollback => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureEntry {
    pub procedure_id: ProcedureId,
    pub state_digest: ProcedureId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationRecord {
    pub generation: u32,
    pub parent_generation: Option<u32>,
    pub rollback_of: Option<u32>,
    pub kind: GenerationKind,
    pub chunk_library_revision: [u8; chunks::REVISION_BYTES],
    pub procedure_library_revision: ProcedureId,
    pub closure: Vec<ClosureEntry>,
}

#[derive(Clone, Copy)]
struct GenerationTransition {
    generation: u32,
    parent_generation: Option<u32>,
    rollback_of: Option<u32>,
    kind: GenerationKind,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Snapshot {
    pub(super) record: GenerationRecord,
    pub(super) records: Vec<ProcedureRecord>,
}

#[derive(Debug)]
pub struct ProcedureLibrary {
    pub(super) chunk_revision: [u8; chunks::REVISION_BYTES],
    pub(super) chunk_library: chunks::Library,
    pub(super) generations: Vec<Snapshot>,
    pub(super) current_generation: u32,
}

impl ProcedureLibrary {
    pub fn new(revision: [u8; chunks::REVISION_BYTES]) -> Self {
        let chunk_library = chunks::Library::new(revision);
        let empty_records = Vec::new();
        let record = GenerationRecord {
            generation: 0,
            parent_generation: None,
            rollback_of: None,
            kind: GenerationKind::Publish,
            chunk_library_revision: *chunk_library.revision(),
            procedure_library_revision: persistence::empty_procedure_revision(),
            closure: Vec::new(),
        };
        Self {
            chunk_revision: revision,
            chunk_library,
            generations: alloc::vec![Snapshot {
                record,
                records: empty_records,
            }],
            current_generation: 0,
        }
    }

    pub const fn chunk_library_revision(&self) -> &[u8; chunks::REVISION_BYTES] {
        self.chunk_library.revision()
    }

    pub fn procedure_library_revision(&self) -> &ProcedureId {
        &self.current().record.procedure_library_revision
    }

    pub const fn current_generation(&self) -> u32 {
        self.current_generation
    }

    pub fn generation_records(&self) -> Vec<&GenerationRecord> {
        self.generations
            .iter()
            .map(|snapshot| &snapshot.record)
            .collect()
    }

    pub fn records(&self) -> &[ProcedureRecord] {
        &self.current().records
    }

    pub const fn chunk_library(&self) -> &chunks::Library {
        &self.chunk_library
    }

    /// Canonical order of the shared typed-chunk/1 view, projected to the
    /// procedure identities that own those exact admitted envelopes.
    pub fn canonical_chunk_order(&self) -> Result<Vec<ProcedureId>, Refusal> {
        let mut order = Vec::with_capacity(self.chunk_library.entries().len());
        for entry_index in chunks::canonical_entry_order(&self.chunk_library) {
            let entry = self
                .chunk_library
                .entries()
                .get(usize::from(entry_index))
                .ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::Invalid,
                        Diagnostic {
                            field: "chunk_order",
                            message: "typed-chunk canonical order names an absent entry",
                        },
                    )
                })?;
            let record = self
                .records()
                .iter()
                .find(|record| {
                    record.chunk.chunk_id == entry.envelope().chunk_id
                        && record.chunk.source_revision == entry.envelope().source_revision
                })
                .ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::Invalid,
                        Diagnostic {
                            field: "chunk_order",
                            message: "typed-chunk entry has no owning procedure record",
                        },
                    )
                })?;
            order.push(record.procedure_id);
        }
        if order.len() != self.records().len() {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                Diagnostic {
                    field: "chunk_order",
                    message: "typed-chunk and procedure generation closures differ in size",
                },
            ));
        }
        Ok(order)
    }

    pub(super) fn current(&self) -> &Snapshot {
        self.generations
            .iter()
            .find(|snapshot| snapshot.record.generation == self.current_generation)
            .unwrap_or(&self.generations[0])
    }

    pub(super) fn generation(&self, id: u32) -> Option<&Snapshot> {
        self.generations
            .iter()
            .find(|snapshot| snapshot.record.generation == id)
    }

    pub fn admit(
        &mut self,
        namespace: &super::Session,
        policy: Option<&ConsumerPolicy>,
        record: ProcedureRecord,
    ) -> Result<ProcedureId, Refusal> {
        self.admit_state(namespace, policy, record, false)
    }

    /// Admit one producer batch as a single immutable generation. Dependencies
    /// may point forward within the batch; the final closure is topologically
    /// re-admitted before publication.
    pub fn admit_batch(
        &mut self,
        namespace: &super::Session,
        policy: Option<&ConsumerPolicy>,
        incoming_records: Vec<ProcedureRecord>,
    ) -> Result<Vec<ProcedureId>, Refusal> {
        if incoming_records.is_empty() {
            return Ok(Vec::new());
        }
        if incoming_records.len() > MAX_RECORDS {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "library.entries",
                    message: "procedure batch exceeds retained record bound",
                },
            ));
        }
        let mut candidate_records = self.current().records.clone();
        let mut identities = Vec::with_capacity(incoming_records.len());
        for record in incoming_records {
            Self::validate_record(&record, policy, true)?;
            if record.identity_digest()? != record.procedure_id {
                return Err(Refusal::new(RefusalKind::Invalid, Diagnostic { field: "procedure_id", message: "retained procedure identity differs from recomputed immutable subject fields" }));
            }
            if record.supersedes.is_some() {
                return Err(Refusal::new(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "supersedes",
                        message:
                            "replacement must use the atomic replace operation, not batch admission",
                    },
                ));
            }
            if let Some(existing_index) = candidate_records
                .iter()
                .position(|existing| existing.procedure_id == record.procedure_id)
            {
                let existing = candidate_records.get(existing_index).ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::Internal,
                        Diagnostic {
                            field: "procedure_id",
                            message: "record index disappeared",
                        },
                    )
                })?;
                if persistence::identity_bytes(existing)? != persistence::identity_bytes(&record)? {
                    return Err(Refusal::new(
                        RefusalKind::Invalid,
                        Diagnostic {
                            field: "procedure_id",
                            message: "identity collision between unequal immutable subjects",
                        },
                    ));
                }
                if existing.library_dependencies != record.library_dependencies {
                    return Err(Refusal::new(
                        RefusalKind::Invalid,
                        Diagnostic {
                            field: "library_dependencies",
                            message: "an existing subject cannot change its dependency closure",
                        },
                    ));
                }
                if existing.status == Status::Superseded {
                    return Err(Refusal::new(
                        RefusalKind::StaleContext,
                        Diagnostic {
                            field: "status",
                            message: "a superseded record cannot be silently reactivated",
                        },
                    ));
                }
                let updated = Self::merge_state(existing, record.clone())?;
                Self::validate_record(&updated, policy, true)?;
                if let Some(slot) = candidate_records.get_mut(existing_index) {
                    *slot = updated;
                }
            } else {
                if candidate_records.len() >= MAX_RECORDS {
                    return Err(Refusal::new(
                        RefusalKind::Exhausted,
                        Diagnostic {
                            field: "library.entries",
                            message: "procedure record limit reached",
                        },
                    ));
                }
                if record.provenance.producer == Producer::Composition
                    && candidate_records.iter().any(|existing| {
                        existing.provenance.producer == Producer::Composition
                            && existing.library_dependencies == record.library_dependencies
                            && existing.chunk.chunk_id == record.chunk.chunk_id
                    })
                {
                    return Err(Refusal::new(
                        RefusalKind::DuplicateEntry,
                        Diagnostic {
                            field: "library_dependencies",
                            message: "composition restates an existing derivation",
                        },
                    ));
                }
                candidate_records.push(record.clone());
            }
            identities.push(record.procedure_id);
        }
        Self::canonicalize_evidence(&mut candidate_records)?;
        candidate_records.sort_by(retrieval::compare_records);
        if candidate_records == self.current().records {
            return Ok(identities);
        }
        let next_generation = self.current_generation.checked_add(1).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "generation",
                    message: "generation counter overflow",
                },
            )
        })?;
        if self.generations.len() >= MAX_GENERATIONS {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "generations",
                    message: "retained generation limit reached",
                },
            ));
        }
        let chunk_library =
            Self::rebuild_chunks(namespace, self.chunk_revision, &candidate_records)?;
        let snapshot = Self::snapshot(
            GenerationTransition {
                generation: next_generation,
                parent_generation: Some(self.current_generation),
                rollback_of: None,
                kind: GenerationKind::Publish,
            },
            *chunk_library.revision(),
            candidate_records,
        )?;
        let mut candidate_generations = self.generations.clone();
        candidate_generations.push(snapshot);
        let encoded = persistence::encode_parts(
            self.chunk_revision,
            &candidate_generations,
            next_generation,
        )?;
        if encoded.len() > MAX_STORAGE_BYTES {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "library.bytes",
                    message: "append-only procedure generations exceed the retained storage bound",
                },
            ));
        }
        self.chunk_library = chunk_library;
        self.generations = candidate_generations;
        self.current_generation = next_generation;
        Ok(identities)
    }

    pub fn replace(
        &mut self,
        namespace: &super::Session,
        policy: Option<&ConsumerPolicy>,
        old_id: ProcedureId,
        mut replacement: ProcedureRecord,
    ) -> Result<ProcedureId, Refusal> {
        if replacement.procedure_id == old_id {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                Diagnostic {
                    field: "supersedes",
                    message: "a replacement must have a distinct immutable subject identity",
                },
            ));
        }
        replacement.supersedes = Some(old_id);
        self.admit_state(namespace, policy, replacement, true)
    }

    fn admit_state(
        &mut self,
        namespace: &super::Session,
        policy: Option<&ConsumerPolicy>,
        record: ProcedureRecord,
        replacement: bool,
    ) -> Result<ProcedureId, Refusal> {
        Self::validate_record(&record, policy, true)?;
        let recomputed = record.identity_digest()?;
        if recomputed != record.procedure_id {
            return Err(Refusal::new(RefusalKind::Invalid, Diagnostic { field: "procedure_id", message: "retained procedure identity differs from recomputed immutable subject fields" }));
        }
        let result_id = record.procedure_id;
        let mut candidate_records = self.current().records.clone();
        if let Some(existing_index) = candidate_records
            .iter()
            .position(|existing| existing.procedure_id == record.procedure_id)
        {
            let existing = candidate_records.get(existing_index).ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Internal,
                    Diagnostic {
                        field: "procedure_id",
                        message: "record index disappeared",
                    },
                )
            })?;
            if persistence::identity_bytes(existing)? != persistence::identity_bytes(&record)? {
                return Err(Refusal::new(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "procedure_id",
                        message: "identity collision between unequal immutable subjects",
                    },
                ));
            }
            if existing.library_dependencies != record.library_dependencies {
                return Err(Refusal::new(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "library_dependencies",
                        message: "an existing subject cannot change its dependency closure",
                    },
                ));
            }
            if existing.status == Status::Superseded && record.status != Status::Superseded {
                return Err(Refusal::new(
                    RefusalKind::StaleContext,
                    Diagnostic {
                        field: "status",
                        message: "a superseded record cannot be silently reactivated",
                    },
                ));
            }
            let updated = Self::merge_state(existing, record)?;
            Self::validate_record(&updated, policy, true)?;
            if *existing == updated {
                return Ok(updated.procedure_id);
            }
            if let Some(slot) = candidate_records.get_mut(existing_index) {
                *slot = updated;
            }
        } else {
            if candidate_records.len() >= MAX_RECORDS {
                return Err(Refusal::new(
                    RefusalKind::Exhausted,
                    Diagnostic {
                        field: "library.entries",
                        message: "procedure record limit reached",
                    },
                ));
            }
            if record.provenance.producer == Producer::Composition
                && candidate_records.iter().any(|existing| {
                    existing.provenance.producer == Producer::Composition
                        && existing.library_dependencies == record.library_dependencies
                        && existing.chunk.chunk_id == record.chunk.chunk_id
                })
            {
                return Err(Refusal::new(
                    RefusalKind::DuplicateEntry,
                    Diagnostic {
                        field: "library_dependencies",
                        message: "composition restates an existing derivation",
                    },
                ));
            }
            if replacement {
                Self::supersede_existing(&mut candidate_records, record.supersedes)?;
            } else if record.supersedes.is_some() {
                return Err(Refusal::new(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "supersedes",
                        message: "replacement references require the atomic replace operation",
                    },
                ));
            }
            Self::check_dependencies(&candidate_records, &record)?;
            candidate_records.push(record.clone());
        }
        Self::canonicalize_evidence(&mut candidate_records)?;
        candidate_records.sort_by(retrieval::compare_records);
        let next_generation = self.current_generation.checked_add(1).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "generation",
                    message: "generation counter overflow",
                },
            )
        })?;
        if self.generations.len() >= MAX_GENERATIONS {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "generations",
                    message: "retained generation limit reached",
                },
            ));
        }
        let chunk_library =
            Self::rebuild_chunks(namespace, self.chunk_revision, &candidate_records)?;
        let parent_generation = Some(self.current_generation);
        let snapshot = Self::snapshot(
            GenerationTransition {
                generation: next_generation,
                parent_generation,
                rollback_of: None,
                kind: GenerationKind::Publish,
            },
            *chunk_library.revision(),
            candidate_records,
        )?;
        let mut candidate_generations = self.generations.clone();
        candidate_generations.push(snapshot);
        let encoded = persistence::encode_parts(
            self.chunk_revision,
            &candidate_generations,
            next_generation,
        )?;
        if encoded.len() > MAX_STORAGE_BYTES {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "library.bytes",
                    message: "append-only procedure generations exceed the retained storage bound",
                },
            ));
        }
        self.chunk_library = chunk_library;
        self.generations = candidate_generations;
        self.current_generation = next_generation;
        Ok(result_id)
    }

    fn merge_state(
        existing: &ProcedureRecord,
        mut incoming: ProcedureRecord,
    ) -> Result<ProcedureRecord, Refusal> {
        for evidence in existing.evidence.iter() {
            if !incoming.evidence.contains(evidence) {
                incoming.evidence.push(evidence.clone());
            }
        }
        if existing.status.rank() < incoming.status.rank() && incoming.status != Status::Superseded
        {
            incoming.status = existing.status;
        }
        if incoming.status == Status::Candidate && existing.status != Status::Candidate {
            incoming.status = existing.status;
        }
        if incoming.promotion.is_none() {
            incoming.promotion = existing.promotion.clone();
        }
        if incoming.supersedes.is_none() {
            incoming.supersedes = existing.supersedes;
        }
        if incoming.alias_of.is_none() {
            incoming.alias_of = existing.alias_of;
        }
        if incoming.cost == Cost::default() {
            incoming.cost = existing.cost;
        }
        incoming.retrieval = retrieval::signature(&incoming);
        if incoming.evidence.len() > MAX_EVIDENCE {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "evidence",
                    message: "evidence record limit reached",
                },
            ));
        }
        Ok(incoming)
    }

    fn canonicalize_evidence(records: &mut [ProcedureRecord]) -> Result<(), Refusal> {
        for record in records.iter_mut() {
            record.evidence.sort_by(persistence::compare_evidence);
            if record.evidence.len() > MAX_EVIDENCE {
                return Err(Refusal::new(
                    RefusalKind::Exhausted,
                    Diagnostic {
                        field: "evidence",
                        message: "evidence record limit reached",
                    },
                ));
            }
        }
        Ok(())
    }

    fn validate_record(
        record: &ProcedureRecord,
        policy: Option<&ConsumerPolicy>,
        require_selected_policy: bool,
    ) -> Result<(), Refusal> {
        if record.format != RECORD_FORMAT || record.semantic_revision != SEMANTIC_REVISION {
            return Err(Refusal::new(
                RefusalKind::Unsupported,
                Diagnostic {
                    field: "format",
                    message: "unsupported procedure record format or semantic revision",
                },
            ));
        }
        if record.encoder.id != ENCODER_ID
            || record.encoder.encoding_revision != ENCODING_REVISION
            || !record.encoder.experimental
        {
            return Err(Refusal::new(
                RefusalKind::Unsupported,
                Diagnostic {
                    field: "encoder",
                    message: "procedure identity encoder is not the selected experimental revision",
                },
            ));
        }
        if record.chunk.interface.effects != EffSet::empty() {
            return Err(Refusal::new(
                RefusalKind::Unsupported,
                Diagnostic {
                    field: "chunk.interface.effects",
                    message: "procedure effect and capability closure must be empty",
                },
            ));
        }
        if record
            .chunk
            .interface
            .stack_in
            .iter()
            .chain(&record.chunk.interface.stack_out)
            .any(|ty| !chunks::profile_type(ty))
        {
            return Err(Refusal::new(
                RefusalKind::Unsupported,
                Diagnostic {
                    field: "chunk.interface",
                    message: "procedure interface exceeds pure-i64-bool/1",
                },
            ));
        }
        if record.library_dependencies.len() > MAX_LIBRARY_DEPENDENCIES {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "library_dependencies",
                    message: "procedure dependency limit reached",
                },
            ));
        }
        if record.evidence.len() > MAX_EVIDENCE {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "evidence",
                    message: "evidence record limit reached",
                },
            ));
        }
        if record.retrieval != retrieval::signature(record) {
            return Err(Refusal::new(RefusalKind::Invalid, Diagnostic { field: "retrieval", message: "retrieval signature differs from recomputed interface, status and integer cost" }));
        }
        for (index, evidence) in record.evidence.iter().enumerate() {
            if evidence.capability.is_some() {
                return Err(Refusal::at(RefusalKind::LiveCapabilityInEvidence, Diagnostic { field: "evidence.capability", message: "capability-bearing evidence cannot enter the resource-free procedure profile" }, index as u32));
            }
            if evidence.subject_binding != record.procedure_id {
                return Err(Refusal::at(
                    RefusalKind::StaleContext,
                    Diagnostic {
                        field: "evidence.subject_binding",
                        message: "evidence is not bound to this immutable procedure subject",
                    },
                    index as u32,
                ));
            }
            if matches!(
                evidence.outcome,
                EvidenceOutcome::Proved | EvidenceOutcome::Disproved
            ) && !evidence.class.proves()
            {
                return Err(Refusal::at(RefusalKind::ForgedStatus, Diagnostic { field: "evidence.outcome", message: "test, validation, review or assumption evidence cannot claim a proof outcome" }, index as u32));
            }
            if evidence.toolchain.is_empty() || evidence.claim.is_empty() {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "evidence",
                        message: "evidence requires an explicit claim and toolchain identity",
                    },
                    index as u32,
                ));
            }
        }
        match record.status {
            Status::Candidate => {
                if record.promotion.is_some() {
                    return Err(Refusal::new(
                        RefusalKind::ForgedStatus,
                        Diagnostic {
                            field: "promotion",
                            message: "candidate record cannot carry a promotion decision",
                        },
                    ));
                }
            }
            Status::Tested => {
                if record.promotion.is_some()
                    || !record.evidence.iter().any(|evidence| {
                        evidence.class == EvidenceClass::Test
                            && evidence.outcome == EvidenceOutcome::Passed
                    })
                {
                    return Err(Refusal::new(RefusalKind::ForgedStatus, Diagnostic { field: "status", message: "tested status requires passing test evidence and no promotion decision" }));
                }
            }
            Status::Promoted => {
                let promotion = record.promotion.as_ref().ok_or_else(|| {
                    Refusal::new(
                        RefusalKind::ForgedStatus,
                        Diagnostic {
                            field: "promotion",
                            message: "promoted status requires a consumer policy decision",
                        },
                    )
                })?;
                if require_selected_policy && policy.is_none() {
                    return Err(Refusal::new(
                        RefusalKind::StaleContext,
                        Diagnostic {
                            field: "promotion.policy_revision",
                            message: "promoted state requires an explicit consumer policy",
                        },
                    ));
                }
                if promotion.obligations.is_empty() {
                    return Err(Refusal::new(
                        RefusalKind::ForgedStatus,
                        Diagnostic {
                            field: "promotion.obligations",
                            message: "promotion must name at least one consumer obligation",
                        },
                    ));
                }
                if let Some(policy) = policy {
                    if policy.revision != promotion.policy_revision {
                        return Err(Refusal::new(
                            RefusalKind::StaleContext,
                            Diagnostic {
                                field: "promotion.policy_revision",
                                message: "consumer policy revision differs from the promoted state",
                            },
                        ));
                    }
                }
                for (index, obligation) in promotion.obligations.iter().enumerate() {
                    if index > 0
                        && promotion
                            .obligations
                            .get(index.saturating_sub(1))
                            .is_some_and(|previous| previous >= obligation)
                    {
                        return Err(Refusal::at(
                            RefusalKind::ForgedStatus,
                            Diagnostic {
                                field: "promotion.obligations",
                                message: "consumer obligations must be strictly increasing",
                            },
                            index as u32,
                        ));
                    }
                    let policy_obligation = if let Some(policy) = policy {
                        Some(
                            policy
                                .obligations
                                .iter()
                                .find(|candidate| candidate.id == *obligation)
                                .ok_or_else(|| {
                                    Refusal::at(RefusalKind::StaleContext, Diagnostic { field: "promotion.obligations", message: "promotion obligation is absent from the selected consumer policy" }, index as u32)
                                })?,
                        )
                    } else {
                        None
                    };
                    let is_satisfied = record.evidence.iter().any(|evidence| {
                        evidence.obligation == *obligation
                            && evidence.policy_revision == promotion.policy_revision
                            && evidence.outcome == EvidenceOutcome::Proved
                            && evidence.class.proves()
                            && policy_obligation.is_none_or(|required| {
                                required.allowed_classes.contains(&evidence.class)
                            })
                    });
                    if !is_satisfied {
                        return Err(Refusal::at(RefusalKind::ForgedStatus, Diagnostic { field: "promotion.obligations", message: "each promoted obligation requires exact-subject proof evidence allowed by policy" }, index as u32));
                    }
                }
            }
            Status::Superseded => {
                if record.promotion.is_some() {
                    return Err(Refusal::new(
                        RefusalKind::ForgedStatus,
                        Diagnostic {
                            field: "promotion",
                            message: "superseded record cannot retain a promotion decision",
                        },
                    ));
                }
            }
        }
        if record
            .supersedes
            .is_some_and(|id| id == record.procedure_id)
        {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                Diagnostic {
                    field: "supersedes",
                    message: "a procedure cannot supersede itself",
                },
            ));
        }
        if let Some(alias) = record.alias_of {
            if alias.procedure_id == record.procedure_id || alias.certificate == [0; 32] {
                return Err(Refusal::new(RefusalKind::Invalid, Diagnostic { field: "alias_of", message: "alias requires a distinct exact-pair identity and nonzero certificate digest" }));
            }
            return Err(Refusal::new(RefusalKind::Unsupported, Diagnostic { field: "alias_of", message: "semantic aliases require a verified noble-ngraph certificate; a digest alone is not proof" }));
        }
        Ok(())
    }

    fn check_dependencies(
        records: &[ProcedureRecord],
        record: &ProcedureRecord,
    ) -> Result<(), Refusal> {
        if record.library_dependencies.len() != record.chunk.dependencies.len() {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                Diagnostic {
                    field: "library_dependencies",
                    message: "procedure and typed-chunk dependency closures differ in length",
                },
            ));
        }
        for (index, dependency) in record.library_dependencies.iter().enumerate() {
            if dependency.procedure_id == record.procedure_id
                || !records.iter().any(|candidate| {
                    candidate.procedure_id == dependency.procedure_id
                        && candidate.chunk.chunk_id == dependency.chunk_id
                        && candidate.chunk.source_revision == dependency.source_revision
                })
            {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "library_dependencies",
                        message: "library dependency does not resolve to an exact retained record",
                    },
                    index as u32,
                ));
            }
        }
        for (index, (dependency, procedure)) in record
            .chunk
            .dependencies
            .iter()
            .zip(&record.library_dependencies)
            .enumerate()
        {
            let mut identity = [0u8; 8];
            identity.copy_from_slice(&procedure.chunk_id[..8]);
            let is_matched = u64::from_be_bytes(identity) == dependency.identity
                && procedure.source_revision == dependency.source_revision
                && records.iter().any(|candidate| {
                    candidate.procedure_id == procedure.procedure_id
                        && candidate.chunk.chunk_id == procedure.chunk_id
                        && candidate.chunk.source_revision == procedure.source_revision
                        && candidate.chunk.format == dependency.format
                        && candidate.chunk.semantic_revision == dependency.semantic_revision
                });
            if !is_matched {
                return Err(Refusal::at(RefusalKind::Invalid, Diagnostic { field: "library_dependencies", message: "typed-chunk and procedure dependency edges must match in retained order" }, index as u32));
            }
        }
        Ok(())
    }

    fn validate_supersession(records: &[ProcedureRecord]) -> Result<(), Refusal> {
        for (index, record) in records.iter().enumerate() {
            if record.status == Status::Superseded
                && !records
                    .iter()
                    .any(|candidate| candidate.supersedes == Some(record.procedure_id))
            {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "supersedes",
                        message: "superseded record must remain linked to a retained replacement",
                    },
                    index as u32,
                ));
            }
            if let Some(replaced_id) = record.supersedes {
                if record.status == Status::Superseded || replaced_id == record.procedure_id {
                    return Err(Refusal::at(RefusalKind::Invalid, Diagnostic { field: "supersedes", message: "replacement relation must point from a live distinct record to the replaced subject" }, index as u32));
                }
                if !records
                    .iter()
                    .any(|old| old.procedure_id == replaced_id && old.status == Status::Superseded)
                {
                    return Err(Refusal::at(
                        RefusalKind::Invalid,
                        Diagnostic {
                            field: "supersedes",
                            message: "replacement target is not retained as superseded",
                        },
                        index as u32,
                    ));
                }
            }
        }
        Ok(())
    }

    fn supersede_existing(
        records: &mut [ProcedureRecord],
        old_id: Option<ProcedureId>,
    ) -> Result<(), Refusal> {
        let old_id = old_id.ok_or_else(|| {
            Refusal::new(
                RefusalKind::Invalid,
                Diagnostic {
                    field: "supersedes",
                    message: "replacement has no prior identity",
                },
            )
        })?;
        let old = records
            .iter_mut()
            .find(|record| record.procedure_id == old_id)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "supersedes",
                        message: "replaced identity is unavailable",
                    },
                )
            })?;
        old.status = Status::Superseded;
        old.promotion = None;
        old.retrieval = retrieval::signature(old);
        Ok(())
    }

    fn rebuild_chunks(
        namespace: &super::Session,
        revision: [u8; chunks::REVISION_BYTES],
        records: &[ProcedureRecord],
    ) -> Result<chunks::Library, Refusal> {
        if records.len() > MAX_RECORDS {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "library.entries",
                    message: "procedure record limit reached",
                },
            ));
        }
        Self::validate_supersession(records)?;
        for (index, record) in records.iter().enumerate() {
            Self::validate_record(record, None, false).map_err(|mut refusal| {
                refusal.location = chunks::Location::at("records", index as u32);
                refusal
            })?;
            if record.identity_digest()? != record.procedure_id {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "procedure_id",
                        message:
                            "retained procedure identity differs from recomputed immutable fields",
                    },
                    index as u32,
                ));
            }
        }
        let order = Self::admission_order(records)?;
        let mut library = chunks::Library::new(revision);
        let mut admitted: Vec<ProcedureRecord> = Vec::with_capacity(records.len());
        for index in order {
            let record = records.get(index).ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Internal,
                    Diagnostic {
                        field: "library.entries",
                        message: "admission order invalid",
                    },
                )
            })?;
            Self::check_dependencies(&admitted, record)?;
            library
                .admit(namespace, record.chunk.clone())
                .map_err(Refusal::from_chunk)?;
            admitted.push(record.clone());
        }
        if library.revision() != &revision {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                Diagnostic {
                    field: "chunk_library_revision",
                    message: "typed-chunk library pin changed while rebuilding a generation",
                },
            ));
        }
        Ok(library)
    }

    fn admission_order(records: &[ProcedureRecord]) -> Result<Vec<usize>, Refusal> {
        let mut order = Vec::with_capacity(records.len());
        while order.len() < records.len() {
            let mut selected = None;
            for (candidate_index, record) in records.iter().enumerate() {
                if order.contains(&candidate_index) {
                    continue;
                }
                let is_dependencies_are_ready =
                    record.library_dependencies.iter().all(|dependency| {
                        records
                            .iter()
                            .enumerate()
                            .any(|(dependency_index, candidate)| {
                                candidate.procedure_id == dependency.procedure_id
                                    && order.contains(&dependency_index)
                            })
                    }) && record.chunk.dependencies.iter().all(|dependency| {
                        records
                            .iter()
                            .enumerate()
                            .any(|(dependency_index, candidate)| {
                                order.contains(&dependency_index)
                                    && candidate.chunk.chunk_id[..8]
                                        == dependency.identity.to_be_bytes()
                                    && candidate.chunk.source_revision == dependency.source_revision
                            })
                    });
                if is_dependencies_are_ready {
                    selected = Some(candidate_index);
                    break;
                }
            }
            match selected {
                Some(index) => order.push(index),
                None => {
                    return Err(Refusal::new(
                        RefusalKind::Invalid,
                        Diagnostic {
                            field: "library_dependencies",
                            message:
                                "procedure dependency closure is cyclic or names a missing chunk",
                        },
                    ))
                }
            }
        }
        Ok(order)
    }

    fn snapshot(
        transition: GenerationTransition,
        chunk_library_revision: [u8; chunks::REVISION_BYTES],
        records: Vec<ProcedureRecord>,
    ) -> Result<Snapshot, Refusal> {
        let mut closure = Vec::with_capacity(records.len());
        for record in records.iter() {
            closure.push(ClosureEntry {
                procedure_id: record.procedure_id,
                state_digest: persistence::state_digest(record)?,
            });
        }
        let procedure_library_revision = persistence::procedure_revision(&records)?;
        Ok(Snapshot {
            record: GenerationRecord {
                generation: transition.generation,
                parent_generation: transition.parent_generation,
                rollback_of: transition.rollback_of,
                kind: transition.kind,
                chunk_library_revision,
                procedure_library_revision,
                closure,
            },
            records,
        })
    }

    pub fn demote(
        &mut self,
        namespace: &super::Session,
        procedure_id: ProcedureId,
    ) -> Result<(), Refusal> {
        let mut record = self
            .records()
            .iter()
            .find(|record| record.procedure_id == procedure_id)
            .cloned()
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::UnknownRecord,
                    Diagnostic {
                        field: "procedure_id",
                        message: "record unavailable",
                    },
                )
            })?;
        if record.status != Status::Promoted {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                Diagnostic {
                    field: "status",
                    message: "only promoted records can be demoted",
                },
            ));
        }
        record.status = if record.evidence.iter().any(|evidence| {
            evidence.class == EvidenceClass::Test && evidence.outcome == EvidenceOutcome::Passed
        }) {
            Status::Tested
        } else {
            Status::Candidate
        };
        record.promotion = None;
        record.retrieval = retrieval::signature(&record);
        let mut candidate_records = self.current().records.clone();
        let slot = candidate_records
            .iter_mut()
            .find(|candidate| candidate.procedure_id == procedure_id)
            .ok_or_else(|| {
                Refusal::new(
                    RefusalKind::Internal,
                    Diagnostic {
                        field: "procedure_id",
                        message: "record disappeared",
                    },
                )
            })?;
        *slot = record;
        candidate_records.sort_by(retrieval::compare_records);
        let generation = self.current_generation.checked_add(1).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "generation",
                    message: "generation counter overflow",
                },
            )
        })?;
        self.publish_snapshot(
            namespace,
            candidate_records,
            GenerationTransition {
                generation,
                parent_generation: Some(self.current_generation),
                rollback_of: None,
                kind: GenerationKind::Publish,
            },
        )
    }

    fn publish_snapshot(
        &mut self,
        namespace: &super::Session,
        records: Vec<ProcedureRecord>,
        transition: GenerationTransition,
    ) -> Result<(), Refusal> {
        if self.generations.len() >= MAX_GENERATIONS {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "generations",
                    message: "retained generation limit reached",
                },
            ));
        }
        let chunk_library = Self::rebuild_chunks(namespace, self.chunk_revision, &records)?;
        let snapshot = Self::snapshot(transition, *chunk_library.revision(), records)?;
        let mut candidate_generations = self.generations.clone();
        candidate_generations.push(snapshot);
        let encoded = persistence::encode_parts(
            self.chunk_revision,
            &candidate_generations,
            transition.generation,
        )?;
        if encoded.len() > MAX_STORAGE_BYTES {
            return Err(Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "library.bytes",
                    message: "append-only procedure generations exceed the retained storage bound",
                },
            ));
        }
        self.chunk_library = chunk_library;
        self.generations = candidate_generations;
        self.current_generation = transition.generation;
        Ok(())
    }

    pub fn rollback(
        &mut self,
        namespace: &super::Session,
        generation_id: u32,
        available_toolchains: &[String],
    ) -> Result<(), Refusal> {
        let retained = self.generation(generation_id).ok_or_else(|| {
            Refusal::new(
                RefusalKind::UnknownGeneration,
                Diagnostic {
                    field: "generation",
                    message: "generation unavailable",
                },
            )
        })?;
        let records = retained.records.clone();
        Self::verify_snapshot(retained)?;
        for (record_index, record) in records.iter().enumerate() {
            for (evidence_index, evidence) in record.evidence.iter().enumerate() {
                if evidence
                    .toolchain
                    .iter()
                    .any(|toolchain| !available_toolchains.contains(toolchain))
                {
                    return Err(Refusal::at(
                        RefusalKind::Invalid,
                        Diagnostic {
                            field: "evidence.toolchain",
                            message:
                                "rollback refused because an evidence toolchain is unavailable",
                        },
                        evidence_index as u32,
                    ));
                }
            }
            if record_index >= MAX_RECORDS {
                return Err(Refusal::new(
                    RefusalKind::Exhausted,
                    Diagnostic {
                        field: "closure",
                        message: "rollback closure exceeds the record bound",
                    },
                ));
            }
        }
        let generation = self.current_generation.checked_add(1).ok_or_else(|| {
            Refusal::new(
                RefusalKind::Exhausted,
                Diagnostic {
                    field: "generation",
                    message: "generation counter overflow",
                },
            )
        })?;
        self.publish_snapshot(
            namespace,
            records,
            GenerationTransition {
                generation,
                parent_generation: Some(generation_id),
                rollback_of: Some(self.current_generation),
                kind: GenerationKind::Rollback,
            },
        )
    }

    fn verify_snapshot(snapshot: &Snapshot) -> Result<(), Refusal> {
        if snapshot.record.procedure_library_revision
            != persistence::procedure_revision(&snapshot.records)?
        {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                Diagnostic {
                    field: "procedure_library_revision",
                    message:
                        "retained procedure revision differs from recomputed generation closure",
                },
            ));
        }
        if snapshot.record.closure.len() != snapshot.records.len() {
            return Err(Refusal::new(
                RefusalKind::Invalid,
                Diagnostic {
                    field: "closure",
                    message: "retained generation closure length differs from its records",
                },
            ));
        }
        for (index, (entry, record)) in snapshot
            .record
            .closure
            .iter()
            .zip(&snapshot.records)
            .enumerate()
        {
            if entry.procedure_id != record.procedure_id {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "closure.procedure_id",
                        message: "retained identity differs from generation closure",
                    },
                    index as u32,
                ));
            }
            if entry.state_digest != persistence::state_digest(record)? {
                return Err(Refusal::at(
                    RefusalKind::Invalid,
                    Diagnostic {
                        field: "closure.state_digest",
                        message: "retained mutable state differs from generation closure",
                    },
                    index as u32,
                ));
            }
        }
        Ok(())
    }
}
