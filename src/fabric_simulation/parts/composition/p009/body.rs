
// Run judgment, counter cohorts, and novelty-guided exploration.

/// The protocol evidence, invariant results, and decision of a finished run.
struct ReferenceRunVerdict {
    protocol: ProtocolRunEvidence,
    invariant_results: Vec<InvariantResult>,
    decision: SimulationDecision,
}

/// Judge a finished run: the independent oracle decides every extension
/// invariant, and a protocol failure fails the run.
// r[impl molten.fabric_simulation.oracle_independence]
fn judge_reference_run(
    prepared: &PreparedReferenceWorld,
    trace: ProtocolTrace,
    observations: &[Observation],
    context: &ProtocolRunContext<'_>,
) -> crate::error::Result<ReferenceRunVerdict> {
    let protocol = evaluate_protocol_trace(prepared, trace, context)?;
    let plain_observations = observations.iter().map(|item| item.observation.clone()).collect::<Vec<_>>();
    let invariant_results = evaluate_invariants(
        &prepared.world.admitted.manifest.invariants,
        &plain_observations,
        &protocol.evidence.safety,
    );
    let is_protocol_failed = protocol.status.verdict == "fail";
    let decision = if invariant_results.iter().all(|result| result.passed) && !is_protocol_failed {
        SimulationDecision::Pass
    } else {
        SimulationDecision::InvariantFailed
    };
    Ok(ReferenceRunVerdict {
        protocol,
        invariant_results,
        decision,
    })
}

impl ProtocolRunEvidence {
    /// The identities a counter comparison must share with this run.
    // r[impl molten.fabric_simulation.protocol_cost]
    pub fn counter_cohort(
        &self,
        profile: &molten_core::protocol_oracle::ProtocolOracleProfile,
    ) -> molten_core::protocol_oracle::CounterCohort {
        let schema = profile.counters.iter().map(|counter| counter.name.as_str()).collect::<Vec<_>>().join("\n");
        let bounds = &profile.bounds;
        molten_core::protocol_oracle::CounterCohort {
            world_ref: self.evidence.run.world_ref.clone(),
            runtime_ref: self.evidence.run.runtime_ref.clone(),
            scheduler_ref: self.evidence.run.scheduler_ref.clone(),
            workload_ref: self.evidence.run.workload_ref.clone(),
            fault_ref: self.evidence.run.fault_ref.clone(),
            counter_schema_ref: blake3_ref(schema.as_bytes()),
            bounds_ref: blake3_ref(
                format!(
                    "{}:{}:{}:{}:{}",
                    bounds.max_records,
                    bounds.max_state_bytes,
                    bounds.max_total_bytes,
                    bounds.max_participants,
                    bounds.max_counters
                )
                .as_bytes(),
            ),
        }
    }
}

/// The novelty a seeded run of a world reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedNovelty {
    pub seed: u64,
    pub run_ref: String,
    pub score: molten_core::protocol_oracle::NoveltyScore,
}

/// Run a world under each seed and score the protocol state each schedule reached.
///
/// The scores guide schedule exploration: a seed whose run reaches no new
/// protocol state adds nothing a previous seed did not already exercise.
// r[impl molten.fabric_simulation.protocol_novelty]
pub fn explore_reference_seeds(manifest: &SimulatedWorldManifest, seeds: &[u64]) -> crate::error::Result<Vec<SeedNovelty>> {
    let mut guidance = molten_core::protocol_oracle::NoveltyGuidance::new();
    let mut scores = Vec::with_capacity(seeds.len());
    for seed in seeds {
        let fixture = run_reference_world(manifest, *seed)?;
        scores.push(SeedNovelty {
            seed: *seed,
            run_ref: fixture.run.run_ref,
            score: guidance.record(&fixture.protocol.evidence.novelty_refs),
        });
    }
    Ok(scores)
}
