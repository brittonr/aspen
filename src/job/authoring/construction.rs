// Shared typed construction route (feature `job-dag-construction`).
//
// Authors build graphs with Choregraph's versioned `choregraph.graph-construction.v1` library.
// nickel-eval evaluates the captured entry/module map in its owned bounded worker, Choregraph
// elaborates the candidate against the Aspen-selected registry (`construction/registry.rs`), and
// the v3 projection lowers to a `selected-sequence-v2` JobDag (`construction/lower.rs`) through the
// normal parser. Lowering never executes a stage.

/// Registry target that a candidate's `finish` call must name with profile `selected-sequence-v2`.
pub const JOB_CONSTRUCTION_TARGET: &str = "molten.job-dag";
/// Logical module that holds the caller's parameters as one native Nickel data literal.
pub const JOB_CONSTRUCTION_PARAMETERS_MODULE: &str = "molten/parameters.ncl";
/// Construction-only join with two ordered inputs; it lowers onto two `sum-u64` input edges.
/// Summation is order-insensitive, so Aspen's lexical fan-in order cannot change its result.
pub const JOB_CONSTRUCTION_SUM_JOIN: &str = "sum2-u64";
const JOB_CONSTRUCTION_TOOLCHAIN: &str = concat!("molten-", env!("CARGO_PKG_VERSION"));
const JOB_CONSTRUCTION_MAX_AUTHOR_MODULES: usize = 64;
const JOB_CONSTRUCTION_SHIPPED_MODULES: usize = 3;
const JOB_CONSTRUCTION_INPUT_BYTES: usize = 2_097_152;
const JOB_CONSTRUCTION_NAME_DEPTH: usize = 16;
const JOB_CONSTRUCTION_RESOLUTION_EDGES: usize = 1_024;
const JOB_CONSTRUCTION_RESPONSE_BYTES: usize = 4_194_304;
const JOB_CONSTRUCTION_MEMORY_BYTES: usize = 536_870_912;
const JOB_CONSTRUCTION_EXECUTION_MILLIS: u64 = 3_000;
/// Unoptimized builds of the companion worker carry the whole Nickel core with debug information.
const JOB_CONSTRUCTION_MAX_WORKER_BYTES: u64 = 1_073_741_824;
/// Choregraph's reviewed ceilings for the dimensions Aspen's runtime bounds do not narrow.
const JOB_CONSTRUCTION_MAX_PORTS: usize = 16_384;
const JOB_CONSTRUCTION_MAX_EFFECTS: usize = 16_384;
const JOB_CONSTRUCTION_MAX_DEPTH: usize = 64;
const JOB_CONSTRUCTION_MAX_BOUNDARIES: usize = 16_384;
const JOB_CONSTRUCTION_MAX_IDENTIFIER_BYTES: usize = 256;
const JOB_CONSTRUCTION_MAX_OUTPUT_BYTES: usize = 8_388_608;

/// One author-supplied immutable Nickel module named by a logical identity, not a host path.
#[derive(Debug, Clone, Copy)]
pub struct ConstructionModule<'a> {
    pub logical_id: &'a str,
    pub source: &'a str,
}

/// Explicitly selected inputs for the shared construction route.
pub struct ConstructedNickelJobInput<'a> {
    /// Logical entry module; it imports `choregraph/graph-construction-v1.ncl` and exports the
    /// result of its `finish` call.
    pub entry: &'a str,
    pub modules: &'a [ConstructionModule<'a>],
    /// Available to authors as the captured module `molten/parameters.ncl`.
    pub parameters: &'a serde_json::Value,
    /// Output roots as fragment key paths from the root key to an operation key. Empty selects sinks.
    pub outputs: &'a [&'a [&'a str]],
    /// Exact absolute `molten-nickel-eval-worker` executable and its measured content ref.
    pub worker: &'a FilePath,
    pub expected_worker_ref: &'a str,
}

/// The admitted JobDag plus the inspectable elaboration that produced it. Aspen node `c<N>` is
/// projection node `N`; the projection and origins carry the scoped paths.
pub struct ConstructedNickelJob {
    pub dag: JobDag,
    pub provenance: IoValue,
    pub projection: choregraph_nickel::construction::model::projection::LatticeV3,
    pub origins: Vec<choregraph_nickel::construction::model::projection::Origin>,
    pub binding: choregraph_nickel::construction::association::CompilationBinding,
}

/// Evaluates captured construction source, elaborates it through Choregraph and admits the result
/// through the normal JobDag parser. Inspection/lowering never executes a workload stage.
// r[impl molten.job_dag.captured_authoring]
pub fn lower_constructed_nickel_job(input: ConstructedNickelJobInput<'_>) -> Result<ConstructedNickelJob> {
    let parameters = nickel_eval::serialize::render_native_value(input.parameters)
        .map_err(|_| MoltenError::invalid_harness("construction parameters are not native Nickel data"))?;
    let modules = construction_modules(&input, &parameters)?;
    let (worker_ref, evaluated) = evaluate_construction(&input, &modules)?;
    let evaluator_binding = serde_json::to_vec(&evaluated.binding)
        .map_err(|error| MoltenError::invalid_harness(format!("construction evaluator binding: {error}")))?;
    let registry = job_construction_registry()?;
    let lowerer_ref = construction_lowerer_ref()?;
    let compiled = compile_construction(CompilationRequest {
        input: &input,
        modules: &modules,
        json: &evaluated.json,
        registry: &registry,
        worker_ref: &worker_ref,
        evaluator_binding: &evaluator_binding,
        lowerer_ref: &lowerer_ref,
    })?;
    let dag = lower_construction_projection(&compiled.projection, input.outputs)?;
    let provenance = construction_provenance(ConstructionProvenance {
        binding: &compiled.binding,
        evaluator_binding: &evaluator_binding,
        worker_ref: &worker_ref,
        registry_ref: &registry.implementation,
        lowerer_ref: &lowerer_ref,
        dag: &dag,
        origins: &compiled.origins,
    })?;
    debug_assert_eq!(dag.nodes.len(), compiled.projection.nodes.len());
    Ok(ConstructedNickelJob {
        dag,
        provenance,
        projection: compiled.projection,
        origins: compiled.origins,
        binding: compiled.binding,
    })
}

/// Runs the admitted map in the selected worker, measured before and after evaluation.
fn evaluate_construction(
    input: &ConstructedNickelJobInput<'_>,
    modules: &[nickel_eval::captured::Module<'_>],
) -> Result<(String, nickel_eval::captured::Evaluated)> {
    let worker_ref = construction_worker_identity(input.worker)?;
    if worker_ref != input.expected_worker_ref {
        return Err(MoltenError::invalid_harness("construction worker differs from selected executable identity"));
    }
    let admitted = nickel_eval::captured::admit(nickel_eval::captured::Request {
        entry: input.entry,
        modules,
        cohort: nickel_eval::captured::reviewed_cohort(),
        policy: nickel_eval::captured::POLICY,
        limits: construction_evaluation_limits(),
    })
    .map_err(|error| MoltenError::invalid_harness(format!("captured construction admission rejected: {error}")))?;
    let evaluated = nickel_eval::captured::evaluate_isolated(&admitted, input.worker).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "captured construction evaluation rejected at {} ({error}); source locations unavailable",
            input.entry
        ))
    })?;
    if worker_ref != construction_worker_identity(input.worker)? {
        return Err(MoltenError::invalid_harness("construction worker changed during evaluation"));
    }
    Ok((worker_ref, evaluated))
}

struct CompilationRequest<'a> {
    input: &'a ConstructedNickelJobInput<'a>,
    modules: &'a [nickel_eval::captured::Module<'a>],
    json: &'a str,
    registry: &'a choregraph_nickel::construction::model::host::TargetRegistry,
    worker_ref: &'a str,
    evaluator_binding: &'a [u8],
    lowerer_ref: &'a str,
}

/// Elaborates the exported candidate with the real captured binding as Choregraph material.
fn compile_construction(request: CompilationRequest<'_>) -> Result<choregraph_nickel::construction::Output> {
    let captured: Vec<_> = request
        .modules
        .iter()
        .map(|module| choregraph_nickel::construction::association::CapturedModule {
            logical_id: module.name,
            bytes: module.source.as_bytes(),
        })
        .collect();
    let output_blake3 = blake3::hash(request.json.as_bytes()).to_hex().to_string();
    let materials = choregraph_nickel::construction::association::CompilationMaterials {
        modules: &captured,
        entry: request.input.entry,
        evaluator_identity: request.worker_ref,
        evaluator_profile: choregraph_nickel::construction::model::profile::CAPTURED_EVALUATOR_PROFILE,
        evaluator_binding: request.evaluator_binding,
        evaluated_output_blake3: &output_blake3,
        library: choregraph_nickel::construction::model::profile::LIBRARY_BYTES,
        contracts: choregraph_nickel::construction::model::profile::CONTRACTS_BYTES,
        lowerer_identity: request.lowerer_ref,
        toolchain_identity: JOB_CONSTRUCTION_TOOLCHAIN,
    };
    choregraph_nickel::construction::compile_json(
        request.json.as_bytes(),
        request.registry,
        &materials,
        construction_graph_limits(),
    )
    .map_err(|failure| {
        MoltenError::invalid_harness(format!("graph construction rejected at {} ({failure})", request.input.entry))
    })
}

/// Streams the selected worker executable through BLAKE3, so measurement memory stays constant.
/// This is integrity and currentness of the selected bytes, not executable authorization.
fn construction_worker_identity(path: &FilePath) -> Result<String> {
    if !path.is_absolute() {
        return Err(MoltenError::invalid_harness("construction worker path must be absolute"));
    }
    let file = std::fs::File::open(path).map_err(MoltenError::from)?;
    if file.metadata().map_err(MoltenError::from)?.len() > JOB_CONSTRUCTION_MAX_WORKER_BYTES {
        return Err(MoltenError::invalid_harness("construction worker executable exceeds byte bound"));
    }
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_reader(std::io::Read::take(file, JOB_CONSTRUCTION_MAX_WORKER_BYTES))
        .map_err(MoltenError::from)?;
    Ok(crate::preserves_rail::content_ref_from_blake3_hash(hasher.finalize()))
}

fn construction_modules<'a>(
    input: &ConstructedNickelJobInput<'a>,
    parameters: &'a str,
) -> Result<Vec<nickel_eval::captured::Module<'a>>> {
    ensure_count_at_most(input.modules.len(), JOB_CONSTRUCTION_MAX_AUTHOR_MODULES, "construction modules")?;
    let library_module = choregraph_nickel::construction::model::profile::LIBRARY_MODULE;
    let contracts_module = choregraph_nickel::construction::model::profile::CONTRACTS_MODULE;
    let shipped = [library_module, contracts_module, JOB_CONSTRUCTION_PARAMETERS_MODULE];
    let mut modules = Vec::with_capacity(input.modules.len() + JOB_CONSTRUCTION_SHIPPED_MODULES);
    let mut author_bytes = parameters.len();
    for module in input.modules {
        if shipped.contains(&module.logical_id) {
            return Err(MoltenError::invalid_harness("construction module reuses a shipped logical identity"));
        }
        author_bytes =
            checked_count_sum(author_bytes, module.source.len(), MAX_JOB_AUTHORING_BYTES, "construction source closure")?;
        modules.push(nickel_eval::captured::Module { name: module.logical_id, source: module.source });
    }
    let library = std::str::from_utf8(choregraph_nickel::construction::model::profile::LIBRARY_BYTES)
        .map_err(|_| MoltenError::invalid_harness("shipped construction library is not UTF-8"))?;
    let contracts = std::str::from_utf8(choregraph_nickel::construction::model::profile::CONTRACTS_BYTES)
        .map_err(|_| MoltenError::invalid_harness("shipped construction contracts are not UTF-8"))?;
    modules.push(nickel_eval::captured::Module { name: library_module, source: library });
    modules.push(nickel_eval::captured::Module { name: contracts_module, source: contracts });
    modules.push(nickel_eval::captured::Module { name: JOB_CONSTRUCTION_PARAMETERS_MODULE, source: parameters });
    debug_assert_eq!(modules.len(), input.modules.len() + JOB_CONSTRUCTION_SHIPPED_MODULES);
    Ok(modules)
}

fn construction_evaluation_limits() -> nickel_eval::captured::Limits {
    nickel_eval::captured::Limits {
        input_bytes: JOB_CONSTRUCTION_INPUT_BYTES,
        module_count: JOB_CONSTRUCTION_MAX_AUTHOR_MODULES + JOB_CONSTRUCTION_SHIPPED_MODULES,
        name_depth: JOB_CONSTRUCTION_NAME_DEPTH,
        resolution_edges: JOB_CONSTRUCTION_RESOLUTION_EDGES,
        output_bytes: MAX_JOB_AUTHORING_BYTES,
        response_bytes: JOB_CONSTRUCTION_RESPONSE_BYTES,
        memory_bytes: JOB_CONSTRUCTION_MEMORY_BYTES,
        execution_millis: JOB_CONSTRUCTION_EXECUTION_MILLIS,
    }
}

/// Choregraph's native ceilings, narrowed to the selected-sequence runtime bounds.
fn construction_graph_limits() -> choregraph_nickel::construction::model::profile::Limits {
    choregraph_nickel::construction::model::profile::Limits {
        max_candidate_bytes: MAX_JOB_AUTHORING_BYTES,
        max_nodes: MAX_JOB_NODES,
        max_ports: JOB_CONSTRUCTION_MAX_PORTS,
        max_edges: MAX_JOB_EDGES,
        max_effects: JOB_CONSTRUCTION_MAX_EFFECTS,
        max_instances: MAX_JOB_NODES,
        max_depth: JOB_CONSTRUCTION_MAX_DEPTH,
        max_boundaries: JOB_CONSTRUCTION_MAX_BOUNDARIES,
        max_identifier_bytes: JOB_CONSTRUCTION_MAX_IDENTIFIER_BYTES,
        max_origin_bytes: MAX_JOB_AUTHORING_BYTES,
        max_parameter_bytes: MAX_JOB_AUTHORING_BYTES,
        max_output_bytes: JOB_CONSTRUCTION_MAX_OUTPUT_BYTES,
    }
}

/// Identity of the route's lowering source: this file, its included registry and lowering, and the
/// selected-sequence admission it lowers through.
fn construction_lowerer_ref() -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("job-construction-lowerer-v1", vec![
        crate::preserves_rail::string(concat!(
            include_str!("construction.rs"),
            include_str!("construction/registry.rs"),
            include_str!("construction/lower.rs"),
            include_str!("../selected/registry.rs"),
            include_str!("../selected/registry/admission.rs"),
            include_str!("../selected/routing.rs"),
        )),
    ]))
}

include!("construction/registry.rs");
include!("construction/lower.rs");
