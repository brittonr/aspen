use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Mutex;

use molten_core::world_commit::SnapshotCohortRef;
use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_snapshot::*;
use preserves::IOValue;
use preserves::ValueImpl;

use crate::error::Failure;
use crate::error::Result;
use crate::runtime::RuntimeSnapshot;

pub const LOCAL_LOGICAL_RUNTIME_PROFILE: &str = "runtime-spine-committed-turn-v1";
pub const MAX_LOCAL_LOGICAL_IMAGE_BYTES: u64 = 16 * 1_048_576;

pub struct LogicalRuntimeImage {
    pub snapshot: RuntimeSnapshot,
    pub history: Vec<IOValue>,
    pub unresolved_effects: Vec<crate::runtime::RuntimeEvent>,
    pub policy_ref: String,
}

pub fn local_logical_profile_ref() -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("logical-runtime-profile-v1", vec![
        crate::preserves_rail::string(LOCAL_LOGICAL_RUNTIME_PROFILE),
    ]))
}

/// Observe the executing process, not an operator-supplied descriptor.
pub fn observe_local_logical_cohort() -> Result<SnapshotCohort> {
    // The executing image and declared ABI facts cannot change within this process.
    // Root configuration and current admission are deliberately not cached here.
    static COHORT: Mutex<Option<SnapshotCohort>> = Mutex::new(None);
    let mut cached = COHORT.lock().map_err(|_| Failure::invalid_harness("runtime cohort measurement lock poisoned"))?;
    if let Some(cohort) = cached.as_ref() {
        return Ok(cohort.clone());
    }
    let cohort = measure_local_logical_cohort()?;
    *cached = Some(cohort.clone());
    Ok(cohort)
}

fn measure_local_logical_cohort() -> Result<SnapshotCohort> {
    let mut executable = std::fs::File::open("/proc/self/exe").map_err(Failure::from)?;
    if executable.metadata().map_err(Failure::from)?.len() > 2_147_483_648 {
        return Err(Failure::invalid_harness("runtime executable exceeds measurement bound"));
    }
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 65_536];
    let mut measured = 0_u64;
    loop {
        let count = executable.read(&mut buffer).map_err(Failure::from)?;
        if count == 0 {
            break;
        }
        measured = measured
            .checked_add(count as u64)
            .ok_or_else(|| Failure::invalid_harness("runtime executable measurement overflow"))?;
        if measured > 2_147_483_648 {
            return Err(Failure::invalid_harness("runtime executable grew beyond measurement bound"));
        }
        hasher.update(&buffer[..count]);
    }
    let build = format!("blake3:{}", hasher.finalize().to_hex());
    let mut facts = Vec::with_capacity(LOGICAL_COHORT_FACTS.len());
    for kind in LOGICAL_COHORT_FACTS {
        let meaning = match kind {
            CohortFactKind::RuntimeBuild => build.as_str(),
            CohortFactKind::RuntimeAbi => "runtime-snapshot-v1",
            CohortFactKind::SchemaSet => "runtime-spine-image-v1/closed-component-projections-v1",
            CohortFactKind::HandlerSet => "molten-runtime-state/committed-turn",
            CohortFactKind::TaskModel => "runtime-facets/committed-no-inflight-turn",
            CohortFactKind::SchedulerProfile => "runtime-state/ordered-pending-message-set",
            CohortFactKind::TimeProfile => "runtime-state/logical-u64",
            CohortFactKind::EntropyProfile => "runtime-state/xorshift64-star/noncryptographic",
            CohortFactKind::EffectProfile => "runtime-state/effect-sequence/unresolved-no-dispatch",
            _ => return Err(Failure::invalid_harness("unsupported local runtime cohort fact")),
        };
        let identity =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("runtime-cohort-fact-v1", vec![
                crate::preserves_rail::string(kind.as_str()),
                crate::preserves_rail::string(meaning),
            ]))?;
        facts.push(CohortFact { kind: *kind, identity });
    }
    let cohort_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("runtime-cohort-v1", vec![
        crate::preserves_rail::sequence(
            facts
                .iter()
                .map(|fact| {
                    crate::preserves_rail::record("fact", vec![
                        crate::preserves_rail::string(fact.kind.as_str()),
                        crate::preserves_rail::string(&fact.identity),
                    ])
                })
                .collect(),
        ),
    ]))?;
    Ok(SnapshotCohort {
        cohort_ref: SnapshotCohortRef::new(cohort_ref)
            .map_err(|error| Failure::invalid_harness(format!("cohort: {error:?}")))?,
        facts,
    })
}

impl LogicalRuntimeImage {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let bytes =
            crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("runtime-spine-image-v1", vec![
                self.snapshot.to_value(),
                crate::preserves_rail::sequence(self.history.clone()),
                effects(&self.unresolved_effects),
                crate::preserves_rail::string(&self.policy_ref),
            ]))?;
        if bytes.len() as u64 > MAX_LOCAL_LOGICAL_IMAGE_BYTES {
            return Err(Failure::invalid_harness("runtime image exceeds byte bound"));
        }
        Ok(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() as u64 > MAX_LOCAL_LOGICAL_IMAGE_BYTES {
            return Err(Failure::invalid_harness("runtime image exceeds byte bound"));
        }
        let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
        let f = crate::preserves_rail::simple_record_fields(&decoded.value, "runtime-spine-image-v1", 4)?;
        let snapshot_value = crate::preserves_rail::value_to_iovalue(&f[0]);
        let image = Self {
            snapshot: RuntimeSnapshot::from_canonical_bytes(&crate::preserves_rail::canonical_bytes(&snapshot_value)?)?,
            history: parse_values(&f[1])?,
            unresolved_effects: parse_values(&f[2])?.as_slice().iter().map(parse_effect).collect::<Result<Vec<_>>>()?,
            policy_ref: crate::preserves_rail::required_string_field(&f[3], "image policy")?,
        };
        if image.canonical_bytes()? != bytes {
            return Err(Failure::invalid_harness("runtime image is not normalized"));
        }
        Ok(image)
    }

    fn validate(&self) -> Result<()> {
        self.snapshot.validate_for_restore()?;
        crate::preserves_rail::validate_content_ref(&self.policy_ref)?;
        if self.history.len() > 4096 || self.unresolved_effects.len() > 4096 {
            return Err(Failure::invalid_harness("runtime image exceeds resume bounds"));
        }
        let mut previous = None;
        for event in &self.unresolved_effects {
            let crate::runtime::RuntimeEvent::EffectRequest { sequence, .. } = event else {
                return Err(Failure::invalid_harness("unresolved runtime effect is not a request"));
            };
            parse_effect(&event.to_value())?;
            if *sequence >= self.snapshot.effect_sequence || previous.is_some_and(|prior| prior >= *sequence) {
                return Err(Failure::invalid_harness("unresolved effect sequence is crossed or non-normalized"));
            }
            previous = Some(*sequence);
        }
        Ok(())
    }

    pub(super) fn projections(&self, cohort: &SnapshotCohort) -> Result<BTreeMap<SnapshotComponentKind, Vec<u8>>> {
        let mut projections = BTreeMap::new();
        for kind in LOGICAL_COMPONENTS {
            if projections.len() >= LOGICAL_COMPONENTS.len() {
                return Err(Failure::invalid_harness("runtime component inventory exceeds profile bound"));
            }
            let value = match kind {
                SnapshotComponentKind::DurableState => {
                    projections.insert(*kind, self.canonical_bytes()?);
                    continue;
                }
                SnapshotComponentKind::Artifact => crate::preserves_rail::sequence(
                    cohort
                        .facts
                        .iter()
                        .filter(|fact| fact.kind == CohortFactKind::RuntimeBuild)
                        .map(|fact| crate::preserves_rail::string(&fact.identity))
                        .collect(),
                ),
                SnapshotComponentKind::Schema => {
                    crate::preserves_rail::string("runtime-snapshot-v1/runtime-spine-image-v1")
                }
                SnapshotComponentKind::Tasks => crate::preserves_rail::sequence(
                    self.snapshot.facets.values().map(|facet| facet.to_value()).collect(),
                ),
                SnapshotComponentKind::History => crate::preserves_rail::sequence(self.history.clone()),
                SnapshotComponentKind::Effects => crate::preserves_rail::record("effects", vec![
                    crate::preserves_rail::u64_value(self.snapshot.effect_sequence),
                    effects(&self.unresolved_effects),
                ]),
                SnapshotComponentKind::Scheduler => crate::preserves_rail::sequence(
                    self.snapshot.messages.iter().map(|message| message.to_value()).collect(),
                ),
                SnapshotComponentKind::Time => crate::preserves_rail::u64_value(self.snapshot.logical_time),
                SnapshotComponentKind::Entropy => crate::preserves_rail::u64_value(self.snapshot.rng_state),
                SnapshotComponentKind::RuntimeProfile => crate::preserves_rail::record("profile", vec![
                    crate::preserves_rail::string(local_logical_profile_ref()?),
                    crate::preserves_rail::string(cohort.cohort_ref.as_str()),
                ]),
                SnapshotComponentKind::Policy => crate::preserves_rail::string(&self.policy_ref),
                _ => return Err(Failure::invalid_harness("unsupported runtime image component")),
            };
            projections.insert(
                *kind,
                crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record(
                    "runtime-spine-component-v1",
                    vec![crate::preserves_rail::string(kind.as_str()), value],
                ))?,
            );
        }
        Ok(projections)
    }
}

pub(super) fn image_descriptor(
    image: &LogicalRuntimeImage,
    cohort: SnapshotCohort,
    commit: WorldCommitRef,
) -> Result<SnapshotDescriptor> {
    let components = image
        .projections(&cohort)?
        .into_iter()
        .map(|(kind, bytes)| {
            let identity = crate::preserves_rail::content_ref_from_bytes(&bytes);
            let root = kind
                .root_kind()
                .map(|root| WorldRootRef::parse(root, identity.clone()))
                .transpose()
                .map_err(|error| Failure::invalid_harness(format!("component root: {error:?}")))?;
            Ok(SnapshotComponent {
                kind,
                identity,
                root,
                owner: ComponentOwner::Molten,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(SnapshotDescriptor {
        class: SnapshotClass::Logical,
        commit_ref: commit,
        profile_ref: SnapshotProfileRef::new(local_logical_profile_ref()?)
            .map_err(|error| Failure::invalid_harness(format!("profile: {error:?}")))?,
        cohort,
        components,
        contains_live_handle: false,
        synchronization: None,
    })
}

fn effects(events: &[crate::runtime::RuntimeEvent]) -> IOValue {
    crate::preserves_rail::sequence(events.iter().map(|event| event.to_value()).collect())
}

fn parse_values(value: &preserves::Value<IOValue>) -> Result<Vec<IOValue>> {
    let values = value
        .collect_sequence()
        .ok_or_else(|| Failure::invalid_harness("expected complete image entries"))?;
    if values.len() > 4096 {
        return Err(Failure::invalid_harness("runtime image entry bound"));
    }
    Ok(values.as_slice().iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

fn parse_effect(value: &IOValue) -> Result<crate::runtime::RuntimeEvent> {
    use crate::runtime::RuntimeEffect;
    use crate::runtime::RuntimeEvent;
    let fields = value
        .collect_simple_record("runtime-event-effect-request-v1", None)
        .ok_or_else(|| Failure::invalid_harness("expected complete unresolved runtime effect request"))?;
    let arity = fields.len();
    if arity != 3 && arity != 4 {
        return Err(Failure::invalid_harness("unresolved effect request arity"));
    }
    let effect = match crate::preserves_rail::required_string_field(&fields[0], "effect")?.as_str() {
        "clock" if arity == 3 => RuntimeEffect::Clock,
        "random" if arity == 4 => RuntimeEffect::Random,
        _ => return Err(Failure::invalid_harness("unsupported unresolved effect profile")),
    };
    let number = |value: &preserves::Value<IOValue>| -> Result<u64> {
        value
            .as_u64()
            .ok_or_else(|| Failure::invalid_harness("expected effect sequence/upper u64"))?
            .map_err(|error| Failure::invalid_harness(format!("effect number: {error}")))
    };
    Ok(RuntimeEvent::EffectRequest {
        effect,
        actor: crate::preserves_rail::required_string_field(&fields[1], "effect actor")?,
        sequence: number(&fields[2])?,
        upper: if arity == 4 { Some(number(&fields[3])?) } else { None },
    })
}
