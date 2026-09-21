pub const MANIFEST_SCHEMA: &str = "molten.wasm-import-manifest.v1";
const DECLARED_WORLD_IMPORTS: &[&str] = &[];
const DECLARED_WORLD_EXPORTS: &[&str] = &[super::super::admission::COMPONENT_INVOKE_EXPORT];
const MAX_MANIFEST_SURFACE_ENTRIES: usize = 128;

/// Declared complete import table for one admitted component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredManifest {
    pub schema_id: String,
    pub profile_id: String,
    pub wit_package: String,
    pub world: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub manifest_ref: String,
}

/// Input for building one declared import manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestInput {
    pub profile_id: String,
    pub wit_package: String,
    pub world: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

/// The exact interface surface of the declared WIT world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredWorld {
    pub profile_id: String,
    pub wit_package: String,
    pub world: String,
    pub wit_ref: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

/// Observed import-surface facts supplied by the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredObservation {
    pub extraction_tool: String,
    pub verifier: String,
    pub component_ref: String,
    pub world: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

/// Deterministic verdict over the declared and observed surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredVerdict {
    pub is_admitted: bool,
    pub manifest_ref: String,
    pub profile_id: String,
    pub wit_package: String,
    pub world: String,
    pub wit_ref: String,
    pub import_set_ref: String,
    pub export_set_ref: String,
    pub extraction_tool: String,
    pub verifier: String,
    pub blockers: Vec<String>,
}

/// Build the declared world surface pinned by the supported profile.
pub fn declared_world(profile: &super::super::model::ComponentRuntimeProfile) -> DeclaredWorld {
    DeclaredWorld {
        profile_id: profile.profile_id.clone(),
        wit_package: profile.wit.package.clone(),
        world: profile.wit.world.clone(),
        wit_ref: profile.wit.source_ref.clone(),
        imports: DECLARED_WORLD_IMPORTS.iter().map(|value| (*value).to_string()).collect(),
        exports: DECLARED_WORLD_EXPORTS.iter().map(|value| (*value).to_string()).collect(),
    }
}

/// Build a typed import manifest, validating shape and binding its identity.
pub fn build_manifest(input: ManifestInput) -> super::super::model::ComponentResult<DeclaredManifest> {
    let blockers = manifest_shape_blockers(&input);
    if !blockers.is_empty() {
        return Err(super::super::model::ComponentDenial::from_blockers(blockers));
    }
    let mut manifest = DeclaredManifest {
        schema_id: MANIFEST_SCHEMA.to_string(),
        profile_id: input.profile_id,
        wit_package: input.wit_package,
        world: input.world,
        imports: super::super::model::sorted_unique(&input.imports),
        exports: super::super::model::sorted_unique(&input.exports),
        manifest_ref: String::new(),
    };
    manifest.manifest_ref = manifest_identity(&manifest)?;
    Ok(manifest)
}

/// Canonical Preserves value of one declared import manifest.
pub fn manifest_value(manifest: &DeclaredManifest) -> preserves::IOValue {
    crate::preserves_rail::record("wasm-import-manifest-v1", vec![
        crate::preserves_rail::record("schema", vec![crate::preserves_rail::string(&manifest.schema_id)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(&manifest.profile_id)]),
        crate::preserves_rail::record("wit-package", vec![crate::preserves_rail::string(&manifest.wit_package)]),
        crate::preserves_rail::record("world", vec![crate::preserves_rail::string(&manifest.world)]),
        crate::preserves_rail::record("imports", vec![strings(&manifest.imports)]),
        crate::preserves_rail::record("exports", vec![strings(&manifest.exports)]),
    ])
}

/// Pure declared-surface admission over typed manifest, world, and observation facts.
pub fn admit_declared(
    manifest: &DeclaredManifest,
    world: &DeclaredWorld,
    observation: &DeclaredObservation,
) -> super::super::model::ComponentResult<DeclaredVerdict> {
    let mut blockers = world_drift_blockers(manifest, world);
    blockers.extend(observation_drift_blockers(manifest, observation));
    if manifest.manifest_ref != manifest_identity(manifest)? {
        blockers.push("component import manifest identity is stale or tampered".to_string());
    }
    let import_set_ref = super::super::model::content_ref(manifest.imports.join("\n").as_bytes());
    let export_set_ref = super::super::model::content_ref(manifest.exports.join("\n").as_bytes());
    Ok(DeclaredVerdict {
        is_admitted: blockers.is_empty(),
        manifest_ref: manifest.manifest_ref.clone(),
        profile_id: manifest.profile_id.clone(),
        wit_package: manifest.wit_package.clone(),
        world: manifest.world.clone(),
        wit_ref: world.wit_ref.clone(),
        import_set_ref,
        export_set_ref,
        extraction_tool: observation.extraction_tool.clone(),
        verifier: observation.verifier.clone(),
        blockers,
    })
}

/// Derive the plan-side observation from declared artifact facts.
pub fn facts_observation(
    component_ref: &str,
    facts: &super::super::admission::ComponentArtifactFacts,
) -> DeclaredObservation {
    DeclaredObservation {
        extraction_tool: super::expected_extraction_tool(),
        verifier: super::ADMISSION_VERIFIER.to_string(),
        component_ref: component_ref.to_string(),
        world: facts.declared_world.clone(),
        imports: super::super::model::sorted_unique(&facts.imports),
        exports: super::super::model::sorted_unique(&facts.exports),
    }
}

fn manifest_shape_blockers(input: &ManifestInput) -> Vec<String> {
    let fields = [
        ("profile", input.profile_id.trim()),
        ("wit-package", input.wit_package.trim()),
        ("world", input.world.trim()),
    ];
    let mut blockers = Vec::with_capacity(fields.len());
    for (label, field) in fields {
        if field.is_empty() {
            blockers.push(format!("component import manifest {label} is missing"));
        }
    }
    blockers.extend(entry_blockers("import", &input.imports));
    blockers.extend(entry_blockers("export", &input.exports));
    blockers
}

fn entry_blockers(label: &str, entries: &[String]) -> Vec<String> {
    let mut blockers = Vec::with_capacity(entries.len());
    if entries.len() > MAX_MANIFEST_SURFACE_ENTRIES {
        blockers.push(format!("component import manifest {label} surface exceeds the declared bound"));
    }
    if super::super::model::sorted_unique(entries) != entries {
        blockers.push(format!("component import manifest {label} surface must be sorted and unique"));
    }
    for entry in entries {
        if entry.trim().is_empty() {
            blockers.push(format!("component import manifest carries an empty {label} name"));
        }
    }
    blockers
}

fn world_drift_blockers(manifest: &DeclaredManifest, world: &DeclaredWorld) -> Vec<String> {
    let mut blockers = Vec::with_capacity(manifest.imports.len() + manifest.exports.len());
    if manifest.schema_id != MANIFEST_SCHEMA {
        blockers.push("component import manifest schema is not the admitted manifest schema".to_string());
    }
    if manifest.profile_id != world.profile_id {
        blockers.push("component import manifest profile is stale or mismatched".to_string());
    }
    if manifest.wit_package != world.wit_package || manifest.world != world.world {
        blockers.push("component import manifest drifts from the declared WIT world".to_string());
    }
    if manifest.imports != world.imports {
        blockers.push("component import manifest imports do not match the declared WIT world".to_string());
    }
    if manifest.exports != world.exports {
        blockers.push("component import manifest exports do not match the declared WIT world".to_string());
    }
    for surface in manifest.imports.iter().chain(manifest.exports.iter()) {
        if surface.starts_with(super::super::admission::WASI_IMPORT_PREFIX) {
            blockers.push(format!("ambient WASI surface {surface} is denied"));
        }
    }
    blockers
}

fn observation_drift_blockers(manifest: &DeclaredManifest, observation: &DeclaredObservation) -> Vec<String> {
    let mut blockers = Vec::with_capacity(observation.imports.len());
    if observation.world != manifest.world {
        blockers.push("observed component world drifts from the declared manifest world".to_string());
    }
    if observation.imports != manifest.imports {
        blockers.push("observed component imports drift from the declared manifest imports".to_string());
    }
    if observation.exports != manifest.exports {
        blockers.push("observed component exports drift from the declared manifest exports".to_string());
    }
    if observation.extraction_tool != super::expected_extraction_tool() {
        blockers.push("component import extraction tool identity is stale".to_string());
    }
    if observation.verifier != super::ADMISSION_VERIFIER {
        blockers.push("component import verifier identity is stale".to_string());
    }
    for import in &observation.imports {
        if import.starts_with(super::super::admission::WASI_IMPORT_PREFIX) {
            blockers.push(format!("ambient WASI import {import} is denied"));
        }
    }
    blockers
}

fn manifest_identity(manifest: &DeclaredManifest) -> super::super::model::ComponentResult<String> {
    crate::preserves_rail::canonical_hash(&manifest_value(manifest)).map_err(|error| {
        super::super::model::ComponentDenial::new(format!("component import manifest hashing failed: {error}"))
    })
}

fn strings(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}
