const MAX_OBSERVED_SURFACE_ENTRIES: usize = 128;

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
) -> super::super::model::ComponentResult<super::surface::DeclaredObservation> {
    let mut imports = Vec::with_capacity(MAX_OBSERVED_SURFACE_ENTRIES);
    let mut exports = Vec::with_capacity(MAX_OBSERVED_SURFACE_ENTRIES);
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        match payload.map_err(parse_denial)? {
            wasmparser::Payload::ComponentImportSection(section) => {
                imports.extend(declared_names(section)?);
            }
            wasmparser::Payload::ComponentExportSection(section) => {
                exports.extend(exported_names(section)?);
            }
            _ => {}
        }
    }
    validate_observed_bounds(&imports, "import")?;
    validate_observed_bounds(&exports, "export")?;
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
    profile: &super::super::model::ComponentRuntimeProfile,
    manifest: ManifestSource<'_>,
    component_ref: &str,
    component_bytes: &[u8],
) -> super::super::model::ComponentResult<AdmissionRecord> {
    let manifest = match manifest {
        ManifestSource::Provided(manifest) => manifest,
        ManifestSource::Missing => {
            return Err(super::super::model::ComponentDenial::new(
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

fn declared_names(
    section: wasmparser::ComponentImportSectionReader<'_>,
) -> super::super::model::ComponentResult<Vec<String>> {
    let mut names = Vec::with_capacity(MAX_OBSERVED_SURFACE_ENTRIES);
    for import in section {
        names.push(import.map_err(parse_denial)?.name.0.to_string());
    }
    Ok(names)
}

fn exported_names(
    section: wasmparser::ComponentExportSectionReader<'_>,
) -> super::super::model::ComponentResult<Vec<String>> {
    let mut names = Vec::with_capacity(MAX_OBSERVED_SURFACE_ENTRIES);
    for export in section {
        names.push(export.map_err(parse_denial)?.name.0.to_string());
    }
    Ok(names)
}

fn validate_observed_bounds(entries: &[String], label: &str) -> super::super::model::ComponentResult<()> {
    if entries.len() > MAX_OBSERVED_SURFACE_ENTRIES {
        return Err(super::super::model::ComponentDenial::new(format!(
            "observed component {label} surface exceeds the declared bound"
        )));
    }
    Ok(())
}

fn parse_denial(error: wasmparser::BinaryReaderError) -> super::super::model::ComponentDenial {
    super::super::model::ComponentDenial::classified(
        super::super::model::ComponentDenialClass::ComponentAdmissionDenial,
        format!("component surface extraction failed: {error}"),
    )
}
