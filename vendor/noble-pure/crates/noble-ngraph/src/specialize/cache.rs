//! Bounded, session-local reuse of #32 residual Programs.
//!
//! Entries are generated only by [`Session::specialize`] after source emission
//! and ordinary-path admission succeed. There is no loader or persistence API.
//! Digests select candidates; exact manifests, retained content, provenance,
//! and current-policy re-admission decide reuse.

use crate::identity::IdentityScheme;

const SPECIALIZATION_DOMAIN: &[u8] = b"noble.ngraph.specialization-key.v1";
const SUBJECT_DOMAIN: &[u8] = b"noble.ngraph.specialization-subject.v1";
const BUILD_DOMAIN: &[u8] = b"noble.ngraph.program-build-key.v1";
const ARTIFACT_DOMAIN: &[u8] = b"noble.ngraph.specialization-artifact.v1";
const PROVENANCE_DOMAIN: &[u8] = b"noble.ngraph.specialization-provenance.v1";
const SPECIALIZER_REVISION: &[u8] = b"ngraph-pe-specializer/2";
const SPECIALIZER_OPTIONS: &[u8] = b"";
const BUILD_TARGET: &[u8] = b"noble-core-v1";
const BUILD_REPRESENTATION: &[u8] = b"declared-text-v1";
const BUILD_BACKEND: &[u8] = b"noble-kernel/residual-readmit-v1";
const BUILD_BACKEND_FLAGS: &[u8] = b"canonical-source-admission";
const BUILD_RUNTIME: &[u8] = b"noble-core-effect-profile-v1";
const BUILD_ABI_REVISION: &[u8] = b"noble-kernel-stack-abi/1";
const BUILD_COMPILER_EPOCH: &[u8] = b"noble-ngraph-residual-readmit/1";

/// Bounded session-cache resources. Zero entries or variants disables
/// retention while preserving ordinary specialization behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Maximum number of retained positive entries.
    pub entries: usize,
    /// Maximum distinct static maps retained for one exact original recipe.
    pub variants_per_subject: usize,
    /// Maximum bytes in either full key manifest.
    pub manifest_bytes: usize,
    /// Maximum accounted retained bytes (keys, content, reports and entries).
    pub retained_bytes: usize,
    /// Maximum candidate entries inspected for one lookup.
    pub lookup_work: usize,
    /// Maximum candidate/movement units spent evicting for one insertion.
    pub eviction_work: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 64,
            variants_per_subject: 8,
            manifest_bytes: 4 * 1024 * 1024,
            retained_bytes: 64 * 1024 * 1024,
            lookup_work: 64,
            eviction_work: 4096,
        }
    }
}

/// Why a cache session cannot be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitsError {
    /// Lookup must be able to inspect every possible entry.
    LookupWork,
    /// Eviction work must at least be finite and nonzero when entries are kept.
    EvictionWork,
}

/// Result of a cache-enabled specialization request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Full keys and content matched; the residual was freshly re-admitted.
    Hit,
    /// A cache miss generated and retained a verified positive entry.
    MissStored,
    /// A cache miss generated a valid residual that did not fit retention bounds.
    MissUncached,
    /// A damaged candidate was discarded and the request was regenerated.
    CorruptRebuilt,
    /// Manifest bounds were exceeded before a lookup; ordinary specialization ran.
    BudgetBypass,
}

impl Status {
    /// Stable report spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Hit => "hit",
            Self::MissStored => "miss-stored",
            Self::MissUncached => "miss-uncached",
            Self::CorruptRebuilt => "corrupt-rebuilt",
            Self::BudgetBypass => "budget-bypass",
        }
    }
}

/// Measured portion of a cache-enabled specialization request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Exact manifest construction, index lookup and content verification.
    Lookup,
    /// #32 residual specialization after a cache miss or budget bypass.
    Specialization,
    /// Canonical source emission after residual specialization.
    SourceEmission,
    /// Current graph checking and ordinary source readmission.
    Admission,
    /// Build verified content identities and retain or decline the entry.
    Publication,
}

/// Synchronous boundary notification for callers that measure stage costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageEvent {
    /// Stage starts.
    Begin(Stage),
    /// Stage completes, including a refusal.
    End(Stage),
}

/// Residual returned from a cache-enabled specialization.
#[derive(Clone, Debug)]
pub struct Result {
    /// Residual graph and exact #32 derivation report.
    pub residual: super::Residual,
    /// Graph freshly reached through ordinary source readmission. Its subject
    /// provenance is independently derived from the emitted source.
    pub readmitted: crate::check::CheckedGraph,
    /// Canonical emitted source for the residual Program.
    pub source: alloc::string::String,
    /// Cache status for this request.
    pub status: Status,
    /// Specialization-key index, absent only when the manifest limit bypassed caching.
    pub specialization_key: Option<crate::ir::Digest>,
    /// Separate residual Program build/admission key, absent if it exceeded the same bound.
    pub program_key: Option<crate::ir::Digest>,
    /// Number of entries inspected by lookup.
    pub lookup_entries: usize,
    /// Accounted retained bytes after this request completes.
    pub retained_bytes: usize,
}

struct Entry {
    specialization_key: crate::ir::Digest,
    specialization_manifest: alloc::vec::Vec<u8>,
    subject_manifest: alloc::vec::Vec<u8>,
    program_key: crate::ir::Digest,
    program_manifest: alloc::vec::Vec<u8>,
    graph_text: alloc::vec::Vec<u8>,
    source: alloc::vec::Vec<u8>,
    artifact_digest: crate::ir::Digest,
    provenance: crate::ir::Digest,
    report: super::Report,
    retained_bytes: usize,
}

struct EntryDraft {
    specialization_key: crate::ir::Digest,
    specialization_manifest: alloc::vec::Vec<u8>,
    subject_manifest: alloc::vec::Vec<u8>,
    program_key: crate::ir::Digest,
    program_manifest: alloc::vec::Vec<u8>,
    graph_text: alloc::vec::Vec<u8>,
    source: alloc::vec::Vec<u8>,
    report: super::Report,
}

struct Candidate {
    graph: crate::ir::Graph,
    report: super::Report,
    source: alloc::string::String,
    program_key: crate::ir::Digest,
}

/// Complete input to one session-cache specialization request.
#[derive(Clone, Copy)]
pub struct Invocation<'a> {
    /// Identity scheme for exact content digests.
    pub ids: &'a dyn IdentityScheme,
    /// Independently checked root graph.
    pub original: &'a crate::check::CheckedGraph,
    /// Exact graph or recipe bytes supplied by the caller.
    pub recipe_bytes: &'a [u8],
    /// Static map and residual-check budgets.
    pub request: super::Request<'a>,
    /// Current limits for the ordinary source admission path.
    pub frontend_limits: noble_contracts::Limits,
}

impl<'a> Invocation<'a> {
    /// Builds one complete cache invocation from checked request inputs.
    #[must_use]
    pub const fn new(
        ids: &'a dyn IdentityScheme,
        original: &'a crate::check::CheckedGraph,
        recipe_bytes: &'a [u8],
        request: super::Request<'a>,
        frontend_limits: noble_contracts::Limits,
    ) -> Self {
        Self {
            ids,
            original,
            recipe_bytes,
            request,
            frontend_limits,
        }
    }
}

/// A reusable successful-specialization session. This owns bytes only; no
/// process-local Program or backend handle is retained or transported.
#[derive(Default)]
pub struct Session {
    limits: Limits,
    entries: alloc::vec::Vec<Entry>,
    retained_bytes: usize,
}

impl Session {
    /// Constructs an empty bounded session.
    ///
    /// # Errors
    /// If configured lookup or eviction work cannot cover the entry bound.
    pub fn new(limits: Limits) -> core::result::Result<Self, LimitsError> {
        if limits.entries > limits.lookup_work {
            return Err(LimitsError::LookupWork);
        }
        if limits.entries > 0 && limits.eviction_work == 0 {
            return Err(LimitsError::EvictionWork);
        }
        Ok(Self {
            limits,
            entries: alloc::vec::Vec::new(),
            retained_bytes: 0,
        })
    }

    /// Accounted retained bytes, including key manifests and cached content.
    #[must_use]
    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    /// Number of retained positive entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no positive entry is retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Specializes or reuses one checked graph under a complete request key.
    ///
    /// `recipe_bytes` are the exact original graph/recipe bytes consumed by
    /// the caller. The full request manifest and full build manifest are
    /// compared; their digests are indexes only. Both a hit and a newly
    /// generated residual pass through ordinary source readmission using the
    /// current `frontend_limits` and `budgets` before they are returned.
    ///
    /// # Errors
    /// Any refusal from the ordinary specializer, source emitter, graph
    /// checker, or independent source admission. Failed requests never publish
    /// an entry.
    pub fn specialize(
        &mut self,
        invocation: Invocation<'_>,
    ) -> core::result::Result<Result, crate::diag::Diagnostic> {
        self.specialize_with_observer(invocation, |_| {})
    }

    /// Specializes or reuses one graph while reporting synchronous stage boundaries.
    /// The observer is called once at each begin/end boundary and is not retained.
    ///
    /// # Errors
    /// As [`Self::specialize`].
    pub fn specialize_with_observer(
        &mut self,
        invocation: Invocation<'_>,
        mut observe: impl FnMut(StageEvent),
    ) -> core::result::Result<Result, crate::diag::Diagnostic> {
        let ids = invocation.ids;
        let original = invocation.original;
        let recipe_bytes = invocation.recipe_bytes;
        let request = &invocation.request;
        let frontend_limits = invocation.frontend_limits;
        observe(StageEvent::Begin(Stage::Lookup));
        let specialization_manifest = specialization_manifest_with(SpecializationManifestInput {
            original,
            recipe: recipe_bytes,
            request: *request,
            frontend_limits,
            max_bytes: self.limits.manifest_bytes,
            epoch: SpecializationEpoch {
                semantic_revision: crate::ir::SEMANTIC_REVISION.as_bytes(),
                specializer_revision: SPECIALIZER_REVISION,
                options: SPECIALIZER_OPTIONS,
            },
        });
        let Some(specialization_manifest) = specialization_manifest else {
            observe(StageEvent::End(Stage::Lookup));
            let (residual, readmitted, source) = generate_and_admit(&invocation, &mut observe)?;
            return Ok(Result {
                residual,
                readmitted,
                source,
                status: Status::BudgetBypass,
                specialization_key: None,
                program_key: None,
                lookup_entries: 0,
                retained_bytes: self.retained_bytes,
            });
        };
        let specialization_key = digest(ids, &specialization_manifest);
        let Some(subject_manifest) = subject_manifest(original, recipe_bytes) else {
            observe(StageEvent::End(Stage::Lookup));
            let (residual, readmitted, source) = generate_and_admit(&invocation, &mut observe)?;
            return Ok(Result {
                residual,
                readmitted,
                source,
                status: Status::BudgetBypass,
                specialization_key: None,
                program_key: None,
                lookup_entries: 0,
                retained_bytes: self.retained_bytes,
            });
        };
        let mut lookup_entries = 0usize;
        let mut is_corrupted = false;
        let mut candidate_index = None;
        let mut index = 0usize;
        while index < self.entries.len() {
            lookup_entries = lookup_entries.saturating_add(1);
            if self.entries[index].specialization_key != specialization_key
                || self.entries[index].specialization_manifest != specialization_manifest
            {
                index += 1;
                continue;
            }
            candidate_index = Some(index);
            break;
        }
        let candidate = candidate_index.and_then(|index| {
            self.load_candidate(
                index,
                &invocation,
                &specialization_manifest,
                &subject_manifest,
            )
        });
        observe(StageEvent::End(Stage::Lookup));
        if let Some(candidate) = candidate {
            observe(StageEvent::Begin(Stage::Admission));
            let admitted = admit_candidate(ids, original, request, frontend_limits, candidate);
            observe(StageEvent::End(Stage::Admission));
            if let Some((residual, readmitted, source, program_key)) = admitted {
                return Ok(Result {
                    residual,
                    readmitted,
                    source,
                    status: Status::Hit,
                    specialization_key: Some(specialization_key),
                    program_key: Some(program_key),
                    lookup_entries,
                    retained_bytes: self.retained_bytes,
                });
            }
        }
        if let Some(index) = candidate_index {
            self.remove(index);
            is_corrupted = true;
        }

        let (residual, readmitted, source) = generate_and_admit(&invocation, &mut observe)?;
        observe(StageEvent::Begin(Stage::Publication));
        let residual_text = crate::format::graph::print(residual.checked.graph());
        let build = build_manifest(BuildManifestBase {
            residual: residual.checked.graph(),
            graph_text: residual_text.as_bytes(),
            source: source.as_bytes(),
            budgets: request.budgets,
            frontend_limits,
            max_bytes: self.limits.manifest_bytes,
        });
        let mut program_key = None;
        let mut status = if is_corrupted {
            Status::CorruptRebuilt
        } else {
            Status::MissUncached
        };
        if let Some(program_manifest) = build {
            let key = digest(ids, &program_manifest);
            program_key = Some(key);
            if let Some(entry) = self.entry(
                ids,
                EntryDraft {
                    specialization_key,
                    specialization_manifest,
                    subject_manifest,
                    program_key: key,
                    program_manifest,
                    graph_text: exact_bytes(residual_text.into_bytes()),
                    source: exact_bytes(source.as_bytes().to_vec()),
                    report: residual.report.clone(),
                },
            ) {
                if self.retain(entry) {
                    status = if is_corrupted {
                        Status::CorruptRebuilt
                    } else {
                        Status::MissStored
                    };
                }
            }
        }
        observe(StageEvent::End(Stage::Publication));
        Ok(Result {
            residual,
            readmitted,
            source,
            status,
            specialization_key: Some(specialization_key),
            program_key,
            lookup_entries,
            retained_bytes: self.retained_bytes,
        })
    }

    fn load_candidate(
        &self,
        index: usize,
        invocation: &Invocation<'_>,
        expected_manifest: &[u8],
        expected_subject: &[u8],
    ) -> Option<Candidate> {
        let entry = &self.entries[index];
        if entry.specialization_manifest != expected_manifest {
            return None;
        }
        if entry.subject_manifest != expected_subject {
            return None;
        }
        if digest(invocation.ids, &entry.specialization_manifest) != entry.specialization_key {
            return None;
        }
        self.check_entry_provenance(invocation.ids, entry)?;
        let graph_text = core::str::from_utf8(&entry.graph_text).ok()?;
        let graph = crate::format::graph::parse(invocation.ids, graph_text).ok()?;
        if crate::format::graph::print(&graph).as_bytes() != entry.graph_text {
            return None;
        }
        let source = alloc::string::String::from_utf8(entry.source.clone()).ok()?;
        let program_manifest = build_manifest(BuildManifestBase {
            residual: &graph,
            graph_text: entry.graph_text.as_slice(),
            source: source.as_bytes(),
            budgets: invocation.request.budgets,
            frontend_limits: invocation.frontend_limits,
            max_bytes: self.limits.manifest_bytes,
        })?;
        if program_manifest != entry.program_manifest
            || digest(invocation.ids, &program_manifest) != entry.program_key
        {
            return None;
        }
        Some(Candidate {
            graph,
            report: entry.report.clone(),
            source,
            program_key: entry.program_key,
        })
    }

    fn check_entry_provenance(&self, ids: &dyn IdentityScheme, entry: &Entry) -> Option<()> {
        let artifact = artifact_digest(ids, &entry.graph_text, &entry.source)?;
        if artifact != entry.artifact_digest {
            return None;
        }
        let report = report_manifest(&entry.report, self.limits.manifest_bytes)?;
        let provenance = encode_parts(
            PROVENANCE_DOMAIN,
            &[
                &entry.specialization_manifest,
                &entry.subject_manifest,
                &entry.program_manifest,
                &entry.artifact_digest,
                &report,
            ],
            self.limits.manifest_bytes,
        )?;
        if digest(ids, &provenance) != entry.provenance {
            return None;
        }
        Some(())
    }

    fn entry(&self, ids: &dyn IdentityScheme, draft: EntryDraft) -> Option<Entry> {
        let EntryDraft {
            specialization_key,
            specialization_manifest,
            subject_manifest,
            program_key,
            program_manifest,
            graph_text,
            source,
            report,
        } = draft;
        let artifact_digest = artifact_digest(ids, &graph_text, &source)?;
        let report_bytes = report_manifest(&report, self.limits.manifest_bytes)?;
        let provenance_bytes = encode_parts(
            PROVENANCE_DOMAIN,
            &[
                &specialization_manifest,
                &subject_manifest,
                &program_manifest,
                &artifact_digest,
                &report_bytes,
            ],
            self.limits.manifest_bytes,
        )?;
        let entry = Entry {
            specialization_key,
            specialization_manifest,
            subject_manifest,
            program_key,
            program_manifest,
            graph_text,
            source,
            artifact_digest,
            provenance: digest(ids, &provenance_bytes),
            report,
            retained_bytes: 0,
        };
        let mut entry = entry;
        entry.retained_bytes = entry_bytes(&entry)?;
        Some(entry)
    }

    fn retain(&mut self, entry: Entry) -> bool {
        if self.limits.entries == 0
            || self.limits.variants_per_subject == 0
            || entry.retained_bytes > self.limits.retained_bytes
        {
            return false;
        }
        let mut eviction_work = 0usize;
        while self
            .entries
            .iter()
            .filter(|old| old.subject_manifest == entry.subject_manifest)
            .count()
            >= self.limits.variants_per_subject
        {
            if !self.evict_oldest(Some(&entry.subject_manifest), &mut eviction_work) {
                return false;
            }
        }
        while self.entries.len() >= self.limits.entries
            || self
                .retained_bytes
                .checked_add(entry.retained_bytes)
                .is_none_or(|total| total > self.limits.retained_bytes)
        {
            if !self.evict_oldest(None, &mut eviction_work) {
                return false;
            }
        }
        self.retained_bytes += entry.retained_bytes;
        self.entries.push(entry);
        true
    }

    fn evict_oldest(&mut self, subject: Option<&[u8]>, work: &mut usize) -> bool {
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| subject.is_none_or(|key| entry.subject_manifest == key))
        else {
            return false;
        };
        let cost = self
            .entries
            .len()
            .saturating_add(self.entries.len() - index);
        if work
            .checked_add(cost)
            .is_none_or(|total| total > self.limits.eviction_work)
        {
            return false;
        }
        *work += cost;
        self.remove(index);
        true
    }

    fn remove(&mut self, index: usize) {
        let entry = self.entries.remove(index);
        self.retained_bytes = self.retained_bytes.saturating_sub(entry.retained_bytes);
    }
}

fn generate(
    ids: &dyn IdentityScheme,
    original: &crate::check::CheckedGraph,
    request: &super::Request<'_>,
    observe: &mut impl FnMut(StageEvent),
) -> core::result::Result<(super::Residual, alloc::string::String), crate::diag::Diagnostic> {
    observe(StageEvent::Begin(Stage::Specialization));
    let result = super::specialize(ids, original, request);
    observe(StageEvent::End(Stage::Specialization));
    let residual = result?;
    observe(StageEvent::Begin(Stage::SourceEmission));
    let result = super::emit_source(&residual.checked);
    observe(StageEvent::End(Stage::SourceEmission));
    let source = result?;
    Ok((residual, source))
}

fn admit_source(
    ids: &dyn IdentityScheme,
    request: &super::Request<'_>,
    frontend_limits: noble_contracts::Limits,
    residual: &crate::check::CheckedGraph,
    source: &str,
) -> core::result::Result<crate::check::CheckedGraph, crate::diag::Diagnostic> {
    let types = super::interface_types(residual)?;
    // Source re-extraction can assign a different node order to the same
    // stack program, so the ordinary checked graph is returned as its own
    // authority rather than compared by graph identity or node numbering.
    super::readmit(ids, source, &types, frontend_limits, request.budgets)
}

fn generate_and_admit(
    invocation: &Invocation<'_>,
    observe: &mut impl FnMut(StageEvent),
) -> core::result::Result<
    (
        super::Residual,
        crate::check::CheckedGraph,
        alloc::string::String,
    ),
    crate::diag::Diagnostic,
> {
    let (residual, source) = generate(
        invocation.ids,
        invocation.original,
        &invocation.request,
        observe,
    )?;
    observe(StageEvent::Begin(Stage::Admission));
    let result = admit_source(
        invocation.ids,
        &invocation.request,
        invocation.frontend_limits,
        &residual.checked,
        &source,
    );
    observe(StageEvent::End(Stage::Admission));
    Ok((residual, result?, source))
}

fn admit_candidate(
    ids: &dyn IdentityScheme,
    original: &crate::check::CheckedGraph,
    request: &super::Request<'_>,
    frontend_limits: noble_contracts::Limits,
    candidate: Candidate,
) -> Option<(
    super::Residual,
    crate::check::CheckedGraph,
    alloc::string::String,
    crate::ir::Digest,
)> {
    // `load_candidate` already verified the full artifact and build manifests.
    // `check_with` preserves this parsed graph; source admission below still
    // runs afresh under the current limits.
    let checked = crate::check::check_with(ids, candidate.graph, request.budgets).ok()?;
    if checked.id() != candidate.report.residual_graph
        || !report_matches(&candidate.report, original, request, &checked)
        || checked.graph().subject
            != super::derive_residual_subject(
                ids,
                original.id(),
                original.graph().subject.closure.clone(),
                request.static_inputs,
            )
    {
        return None;
    }
    if super::emit_source(&checked).ok()?.as_bytes() != candidate.source.as_bytes() {
        return None;
    }
    let readmitted =
        admit_source(ids, request, frontend_limits, &checked, &candidate.source).ok()?;
    Some((
        super::Residual {
            checked,
            report: candidate.report,
        },
        readmitted,
        candidate.source,
        candidate.program_key,
    ))
}

fn report_matches(
    report: &super::Report,
    original: &crate::check::CheckedGraph,
    request: &super::Request<'_>,
    residual: &crate::check::CheckedGraph,
) -> bool {
    report.accounting == super::ACCOUNTING_REVISION
        && report.original_graph == original.id()
        && report.residual_graph == residual.id()
        && report.static_inputs == request.static_inputs
        && report.dynamic_inputs
            == residual
                .graph()
                .slots
                .iter()
                .map(|slot| slot.ty)
                .collect::<alloc::vec::Vec<_>>()
        && report.original_nodes == original.node_count() as u64
        && report.residual_nodes == residual.node_count() as u64
        && report.variants == 1
}

fn digest(ids: &dyn IdentityScheme, bytes: &[u8]) -> crate::ir::Digest {
    let mut hasher = ids.start();
    hasher.update(bytes);
    hasher.finish()
}

/// The Lean cache-key encoding: domain tag, then an eight-byte little-endian
/// length and exact contents for each part.
fn encode_parts(domain: &[u8], parts: &[&[u8]], max_bytes: usize) -> Option<alloc::vec::Vec<u8>> {
    let mut total = domain.len();
    for part in parts {
        total = total.checked_add(8)?.checked_add(part.len())?;
    }
    if total > max_bytes {
        return None;
    }
    let mut encoded = alloc::vec::Vec::with_capacity(total);
    encoded.extend_from_slice(domain);
    for part in parts {
        let length = u64::try_from(part.len()).ok()?.to_le_bytes();
        encoded.extend_from_slice(&length);
        encoded.extend_from_slice(part);
    }
    Some(encoded)
}

fn encoded_parts_len(domain: &[u8], parts: &[usize]) -> Option<usize> {
    let mut total = domain.len();
    for part_len in parts {
        total = total.checked_add(8)?.checked_add(*part_len)?;
    }
    Some(total)
}

#[derive(Clone, Copy)]
struct SpecializationEpoch<'a> {
    semantic_revision: &'a [u8],
    specializer_revision: &'a [u8],
    options: &'a [u8],
}

struct SpecializationManifestInput<'a> {
    original: &'a crate::check::CheckedGraph,
    recipe: &'a [u8],
    request: super::Request<'a>,
    frontend_limits: noble_contracts::Limits,
    max_bytes: usize,
    epoch: SpecializationEpoch<'a>,
}

fn specialization_manifest_with(
    input: SpecializationManifestInput<'_>,
) -> Option<alloc::vec::Vec<u8>> {
    let SpecializationManifestInput {
        original,
        recipe,
        request,
        frontend_limits,
        max_bytes,
        epoch,
    } = input;
    let static_inputs = request.static_inputs;
    let budgets = request.budgets;
    let graph = original.graph();
    let dynamic_count = static_inputs.iter().filter(|value| value.is_none()).count();
    let part_lengths = [
        recipe.len(),
        32,
        8usize.checked_add(graph.slots.len().checked_mul(2)?)?,
        8usize.checked_add(dynamic_count)?,
        8usize.checked_add(graph.subject.closure.len().checked_mul(8)?)?,
        crate::ir::IR_VERSION.len(),
        graph.profile.name().len(),
        epoch.semantic_revision.len(),
        crate::ir::FACT_RULE_REVISION.len(),
        b"ngraph-pe/0-assumptions:none;effect-profile:checked-i64".len(),
        static_map_len(static_inputs)?,
        epoch.specializer_revision.len(),
        epoch.options.len(),
        18usize.checked_mul(8)?,
        16,
    ];
    if encoded_parts_len(SPECIALIZATION_DOMAIN, &part_lengths)? > max_bytes {
        return None;
    }
    let interface = interface_bytes(graph)?;
    let residual_interface = residual_interface_bytes(graph, static_inputs)?;
    let static_map = static_map_bytes(static_inputs)?;
    let closure = closure_bytes(&graph.subject.closure)?;
    let budget_bytes = budget_bytes(budgets);
    let source_limits = source_limit_bytes(frontend_limits);
    let graph_id = original.id();
    encode_parts(
        SPECIALIZATION_DOMAIN,
        &[
            recipe,
            &graph_id,
            &interface,
            &residual_interface,
            &closure,
            crate::ir::IR_VERSION.as_bytes(),
            graph.profile.name().as_bytes(),
            epoch.semantic_revision,
            crate::ir::FACT_RULE_REVISION.as_bytes(),
            b"ngraph-pe/0-assumptions:none;effect-profile:checked-i64",
            &static_map,
            epoch.specializer_revision,
            epoch.options,
            &budget_bytes,
            &source_limits,
        ],
        max_bytes,
    )
}

fn subject_manifest(
    original: &crate::check::CheckedGraph,
    recipe: &[u8],
) -> Option<alloc::vec::Vec<u8>> {
    let graph_id = original.id();
    // An original graph and recipe have already passed bounded graph admission.
    encode_parts(SUBJECT_DOMAIN, &[recipe, &graph_id], usize::MAX)
}

fn build_manifest(base: BuildManifestBase<'_>) -> Option<alloc::vec::Vec<u8>> {
    build_manifest_with_epoch(BuildManifestInput {
        base,
        epoch: BuildEpoch {
            target: BUILD_TARGET,
            representation: BUILD_REPRESENTATION,
            backend: BUILD_BACKEND,
            backend_flags: BUILD_BACKEND_FLAGS,
            runtime: BUILD_RUNTIME,
            abi_revision: BUILD_ABI_REVISION,
            compiler_epoch: BUILD_COMPILER_EPOCH,
        },
    })
}

#[derive(Clone, Copy)]
struct BuildEpoch<'a> {
    target: &'a [u8],
    representation: &'a [u8],
    backend: &'a [u8],
    backend_flags: &'a [u8],
    runtime: &'a [u8],
    abi_revision: &'a [u8],
    compiler_epoch: &'a [u8],
}

#[derive(Clone, Copy)]
struct BuildManifestBase<'a> {
    residual: &'a crate::ir::Graph,
    graph_text: &'a [u8],
    source: &'a [u8],
    budgets: &'a crate::check::Budgets,
    frontend_limits: noble_contracts::Limits,
    max_bytes: usize,
}

#[derive(Clone, Copy)]
struct BuildManifestInput<'a> {
    base: BuildManifestBase<'a>,
    epoch: BuildEpoch<'a>,
}

fn build_manifest_with_epoch(input: BuildManifestInput<'_>) -> Option<alloc::vec::Vec<u8>> {
    let BuildManifestInput { base, epoch } = input;
    let BuildManifestBase {
        residual,
        graph_text,
        source,
        budgets,
        frontend_limits,
        max_bytes,
    } = base;
    let flag_bytes = b"noble.ngraph.program-build-flags.v1"
        .len()
        .checked_add(3usize.checked_mul(8)?)?
        .checked_add(epoch.backend_flags.len())?
        .checked_add(144)?
        .checked_add(16)?;
    let compiler_bytes = b"noble.ngraph.program-compiler-epoch.v1"
        .len()
        .checked_add(6usize.checked_mul(8)?)?
        .checked_add(epoch.compiler_epoch.len())?
        .checked_add(crate::ir::IR_VERSION.len())?
        .checked_add(crate::ir::SEMANTIC_REVISION.len())?
        .checked_add(crate::ir::FACT_RULE_REVISION.len())?
        .checked_add(super::ACCOUNTING_REVISION.len())?
        .checked_add(crate::compiled::COMPILER_REVISION.len())?;
    let estimated = BUILD_DOMAIN
        .len()
        .checked_add(10usize.checked_mul(8)?)?
        .checked_add(graph_text.len())?
        .checked_add(source.len())?
        .checked_add(epoch.target.len())?
        .checked_add(epoch.representation.len())?
        .checked_add(epoch.backend.len())?
        .checked_add(flag_bytes)?
        .checked_add(epoch.runtime.len())?
        .checked_add(epoch.abi_revision.len())?
        .checked_add(8usize.checked_add(residual.slots.len().checked_mul(2)?)?)?
        .checked_add(compiler_bytes)?;
    if estimated > max_bytes {
        return None;
    }
    let abi = interface_bytes(residual)?;
    let budgets = budget_bytes(budgets);
    let source_limits = source_limit_bytes(frontend_limits);
    let flags = encode_parts(
        b"noble.ngraph.program-build-flags.v1",
        &[epoch.backend_flags, &budgets, &source_limits],
        max_bytes,
    )?;
    let compiler_epoch = encode_parts(
        b"noble.ngraph.program-compiler-epoch.v1",
        &[
            epoch.compiler_epoch,
            crate::ir::IR_VERSION.as_bytes(),
            crate::ir::SEMANTIC_REVISION.as_bytes(),
            crate::ir::FACT_RULE_REVISION.as_bytes(),
            super::ACCOUNTING_REVISION.as_bytes(),
            crate::compiled::COMPILER_REVISION.as_bytes(),
        ],
        max_bytes,
    )?;
    encode_parts(
        BUILD_DOMAIN,
        &[
            graph_text,
            source,
            epoch.target,
            epoch.representation,
            epoch.backend,
            &flags,
            epoch.runtime,
            epoch.abi_revision,
            &abi,
            &compiler_epoch,
        ],
        max_bytes,
    )
}

fn interface_bytes(graph: &crate::ir::Graph) -> Option<alloc::vec::Vec<u8>> {
    let count = u64::try_from(graph.slots.len()).ok()?;
    let capacity_bytes = 8usize.checked_add(graph.slots.len().checked_mul(2)?)?;
    let mut bytes = alloc::vec::Vec::with_capacity(capacity_bytes);
    bytes.extend_from_slice(&count.to_le_bytes());
    for slot in &graph.slots {
        bytes.push(match slot.kind {
            crate::ir::SlotKind::Input => 1,
            crate::ir::SlotKind::Param => 2,
        });
        bytes.push(scalar_type_tag(slot.ty));
    }
    Some(bytes)
}

fn residual_interface_bytes(
    graph: &crate::ir::Graph,
    static_inputs: &[Option<i64>],
) -> Option<alloc::vec::Vec<u8>> {
    let inputs = graph
        .slots
        .iter()
        .filter(|slot| slot.kind == crate::ir::SlotKind::Input);
    let mut output =
        alloc::vec::Vec::with_capacity(8usize.checked_add(static_inputs.len().checked_mul(2)?)?);
    let mut dynamic = alloc::vec::Vec::with_capacity(static_inputs.len());
    let mut position = 0usize;
    for slot in inputs {
        if let Some(value) = static_inputs.get(position) {
            if value.is_none() {
                dynamic.push(scalar_type_tag(slot.ty));
            }
        } else {
            return None;
        }
        position = position.checked_add(1)?;
    }
    if position != static_inputs.len() {
        return None;
    }
    output.extend_from_slice(&u64::try_from(dynamic.len()).ok()?.to_le_bytes());
    output.extend_from_slice(&dynamic);
    Some(output)
}

const fn scalar_type_tag(ty: crate::ir::ScalarType) -> u8 {
    match ty {
        crate::ir::ScalarType::I64 => 1,
        crate::ir::ScalarType::Bool => 2,
        crate::ir::ScalarType::F32 => 3,
    }
}

fn static_map_bytes(static_inputs: &[Option<i64>]) -> Option<alloc::vec::Vec<u8>> {
    let capacity_bytes = static_map_len(static_inputs)?;
    let mut bytes = alloc::vec::Vec::with_capacity(capacity_bytes);
    bytes.extend_from_slice(&u64::try_from(static_inputs.len()).ok()?.to_le_bytes());
    for (position, value) in static_inputs.iter().enumerate() {
        bytes.extend_from_slice(&u64::try_from(position).ok()?.to_le_bytes());
        match value {
            Some(value) => {
                bytes.push(1);
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            None => bytes.push(0),
        }
    }
    Some(bytes)
}

fn static_map_len(static_inputs: &[Option<i64>]) -> Option<usize> {
    8usize
        .checked_add(static_inputs.len().checked_mul(9)?)?
        .checked_add(
            static_inputs
                .iter()
                .filter(|value| value.is_some())
                .count()
                .checked_mul(8)?,
        )
}

fn closure_bytes(closure: &[u64]) -> Option<alloc::vec::Vec<u8>> {
    let mut bytes =
        alloc::vec::Vec::with_capacity(8usize.checked_add(closure.len().checked_mul(8)?)?);
    bytes.extend_from_slice(&u64::try_from(closure.len()).ok()?.to_le_bytes());
    for item in closure {
        bytes.extend_from_slice(&item.to_le_bytes());
    }
    Some(bytes)
}

fn budget_bytes(budgets: &crate::check::Budgets) -> alloc::vec::Vec<u8> {
    let fields = [
        budgets.work_units,
        budgets.expanded_nodes,
        budgets.inline_depth,
        budgets.live_nodes,
        budgets.layers,
        budgets.inputs,
        budgets.outputs,
        budgets.affine_terms,
        budgets.table_length_entries,
        budgets.table_entries,
        budgets.facts,
        budgets.graph_bytes,
        budgets.snapshot_bytes,
        budgets.forward_scratch,
        budgets.backward_scratch,
        budgets.optimizer_steps,
        budgets.residual_nodes,
        budgets.variants,
    ];
    let mut bytes = alloc::vec::Vec::with_capacity(fields.len() * 8);
    for field in fields {
        bytes.extend_from_slice(&field.to_le_bytes());
    }
    bytes
}

fn source_limit_bytes(limits: noble_contracts::Limits) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[0..4].copy_from_slice(&limits.bytes.to_le_bytes());
    bytes[4..8].copy_from_slice(&limits.nodes.to_le_bytes());
    bytes[8..12].copy_from_slice(&limits.depth.to_le_bytes());
    bytes[12..16].copy_from_slice(&limits.work.to_le_bytes());
    bytes
}

fn artifact_digest(
    ids: &dyn IdentityScheme,
    graph_text: &[u8],
    source: &[u8],
) -> Option<crate::ir::Digest> {
    let mut hasher = ids.start();
    hasher.update(ARTIFACT_DOMAIN);
    for part in [graph_text, source] {
        let length = u64::try_from(part.len()).ok()?.to_le_bytes();
        hasher.update(&length);
        hasher.update(part);
    }
    Some(hasher.finish())
}

fn report_manifest(report: &super::Report, max_bytes: usize) -> Option<alloc::vec::Vec<u8>> {
    let static_len = static_map_len(&report.static_inputs)?;
    let dynamic_len = 8usize.checked_add(report.dynamic_inputs.len())?;
    let lengths = [
        report.accounting.len(),
        report.original_graph.len(),
        report.residual_graph.len(),
        static_len,
        dynamic_len,
        8,
        8,
        8,
        8,
        8,
    ];
    if encoded_parts_len(b"noble.ngraph.specialization-report.v1", &lengths)? > max_bytes {
        return None;
    }
    let static_inputs = static_map_bytes(&report.static_inputs)?;
    let mut dynamic_inputs =
        alloc::vec::Vec::with_capacity(report.dynamic_inputs.len().checked_add(8)?);
    dynamic_inputs.extend_from_slice(
        &u64::try_from(report.dynamic_inputs.len())
            .ok()?
            .to_le_bytes(),
    );
    for ty in &report.dynamic_inputs {
        dynamic_inputs.push(scalar_type_tag(*ty));
    }
    encode_parts(
        b"noble.ngraph.specialization-report.v1",
        &[
            report.accounting.as_bytes(),
            &report.original_graph,
            &report.residual_graph,
            &static_inputs,
            &dynamic_inputs,
            &report.original_nodes.to_le_bytes(),
            &report.folded_nodes.to_le_bytes(),
            &report.pruned_nodes.to_le_bytes(),
            &report.residual_nodes.to_le_bytes(),
            &report.variants.to_le_bytes(),
        ],
        max_bytes,
    )
}

fn entry_bytes(entry: &Entry) -> Option<usize> {
    let report_vector_bytes = entry
        .report
        .static_inputs
        .capacity()
        .checked_mul(core::mem::size_of::<Option<i64>>())?
        .checked_add(
            entry
                .report
                .dynamic_inputs
                .capacity()
                .checked_mul(core::mem::size_of::<crate::ir::ScalarType>())?,
        )?;
    core::mem::size_of::<Entry>()
        .checked_add(entry.specialization_manifest.capacity())?
        .checked_add(entry.subject_manifest.capacity())?
        .checked_add(entry.program_manifest.capacity())?
        .checked_add(entry.graph_text.capacity())?
        .checked_add(entry.source.capacity())?
        .checked_add(report_vector_bytes)
}

fn exact_bytes(mut bytes: alloc::vec::Vec<u8>) -> alloc::vec::Vec<u8> {
    bytes.shrink_to_fit();
    bytes
}

#[cfg(test)]
#[path = "cache/tests.rs"]
mod tests;
