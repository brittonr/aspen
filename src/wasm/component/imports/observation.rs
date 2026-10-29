pub(super) const MAX_OBSERVED_SURFACE_ENTRIES: usize = 128;

/// Result of recording one declared-surface admission without instantiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRecord {
    pub observation: super::surface::DeclaredObservation,
    pub admission: super::surface::DeclaredVerdict,
    pub receipt: super::receipt::AdmissionEvidence,
}

/// Manifest availability as supplied by the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestSource<'a> {
    Provided(&'a super::surface::DeclaredManifest),
    Missing,
}

/// Extract the observed component import surface from exact artifact bytes.
///
/// The extraction records the pinned tool and verifier identities and never
/// compiles or instantiates the component.
pub fn observe_artifact_surface(
    world: &str,
    component_ref: &str,
    bytes: &[u8],
) -> super::super::model::Result<super::surface::DeclaredObservation> {
    wasmparser::Validator::new().validate_all(bytes).map_err(parse_denial)?;
    let mut types = super::bookkeeping::RootTypes::new();
    let mut depth = 0u32;
    let mut imports = Vec::with_capacity(MAX_OBSERVED_SURFACE_ENTRIES);
    let mut exports = Vec::with_capacity(MAX_OBSERVED_SURFACE_ENTRIES);
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        match payload.map_err(parse_denial)? {
            wasmparser::Payload::Version { .. } => {
                depth = depth
                    .checked_add(1)
                    .ok_or_else(|| super::super::model::Denial::new("component nesting overflowed"))?;
            }
            wasmparser::Payload::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| super::super::model::Denial::new("component nesting is invalid"))?;
            }
            wasmparser::Payload::ComponentTypeSection(section) if depth == 1 => types.observe(section)?,
            wasmparser::Payload::ComponentAliasSection(section) if depth == 1 => types.aliases(section)?,
            wasmparser::Payload::ComponentImportSection(section) if depth == 1 => {
                for import in section {
                    let import = import.map_err(parse_denial)?;
                    if types.import(world, &import)? {
                        push_name(&mut imports, import.name.0)?;
                    }
                }
            }
            wasmparser::Payload::ComponentExportSection(section) if depth == 1 => {
                for export in section {
                    let export = export.map_err(parse_denial)?;
                    if export.kind != wasmparser::ComponentExternalKind::Func {
                        return Err(super::super::model::Denial::new("component exports a non-function root item"));
                    }
                    push_name(&mut exports, export.name.0)?;
                }
            }
            _ => {}
        }
    }
    Ok(super::surface::DeclaredObservation {
        extraction_tool: super::expected_extraction_tool(),
        verifier: super::ADMISSION_VERIFIER.to_string(),
        component_ref: component_ref.to_string(),
        world: world.to_string(),
        imports: super::super::model::sorted_unique(&imports),
        exports: super::super::model::sorted_unique(&exports),
    })
}

/// Record one declared-surface admission over exact bytes.
///
/// This host-side rail reads the supplied artifact bytes, extracts the
/// observed surface, admits it against the declared manifest and world, and
/// binds the verdict in a canonical receipt. It never instantiates the
/// component.
pub fn record_surface_admission(
    profile: &super::super::model::RuntimeProfile,
    manifest: ManifestSource<'_>,
    component_ref: &str,
    component_bytes: &[u8],
) -> super::super::model::Result<AdmissionRecord> {
    let manifest = match manifest {
        ManifestSource::Provided(manifest) => manifest,
        ManifestSource::Missing => {
            return Err(super::super::model::Denial::new(
                "component import manifest is missing, partial, or unreadable",
            ));
        }
    };
    let world = super::surface::declared_world(profile);
    let observation = observe_artifact_surface(&world.world, component_ref, component_bytes)?;
    let admission = super::surface::admit_declared(manifest, &world, &observation)?;
    let receipt = super::receipt::build_admission_evidence(super::receipt::AdmissionInput {
        manifest_ref: admission.manifest_ref.clone(),
        wit_package: admission.wit_package.clone(),
        world: admission.world.clone(),
        wit_ref: admission.wit_ref.clone(),
        import_set_ref: admission.import_set_ref.clone(),
        extraction_tool: admission.extraction_tool.clone(),
        verifier: admission.verifier.clone(),
        is_admitted: admission.is_admitted,
        blockers: admission.blockers.clone(),
        labels: Vec::new(),
    })?;
    Ok(AdmissionRecord {
        observation,
        admission,
        receipt,
    })
}

fn push_name(names: &mut impl crate::bounded::VecSink<String>, name: &str) -> super::super::model::Result<()> {
    if names.item_count() >= MAX_OBSERVED_SURFACE_ENTRIES {
        return Err(super::super::model::Denial::new("component external surface exceeds its bound"));
    }
    names.push_item(name.to_string());
    Ok(())
}

pub(super) fn parse_denial(error: wasmparser::BinaryReaderError) -> super::super::model::Denial {
    super::super::model::Denial::classified(
        super::super::model::DenialClass::ComponentAdmissionDenial,
        format!("component surface extraction failed: {error}"),
    )
}
