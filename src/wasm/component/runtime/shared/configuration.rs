pub(super) fn build(
    profile: &super::super::super::model::RuntimeProfile,
) -> super::super::super::model::Result<hardened_wasmtime_shell::BoundedRuntime> {
    let features = &profile.features;
    let determinism = &profile.determinism;
    let policy = hardened_wasmtime_runtime::feature::policy::admit(
        hardened_wasmtime_runtime::feature::policy::WasmtimeFeatures {
            bulk_memory: features.bulk_memory,
            component_model: features.component_model,
            multi_value: features.multi_value,
            reference_types: features.reference_types,
            simd: features.simd,
            tail_call: features.tail_call,
            wide_arithmetic: features.wide_arithmetic,
            memory64: features.memory64,
            multi_memory: features.multi_memory,
            threads: features.threads,
            gc: features.gc,
            exceptions: features.exceptions,
            relaxed_simd: features.relaxed_simd,
            custom_page_sizes: features.custom_page_sizes,
            extended_const: features.extended_const,
            function_references: features.function_references,
            component_async: features.component_async,
        },
        hardened_wasmtime_runtime::feature::policy::Determinism {
            fuel_interruption: determinism.fuel_interruption,
            nan_canonicalization: determinism.nan_canonicalization,
            relaxed_simd_deterministic: determinism.relaxed_simd_deterministic,
            memory_growth_fixed: determinism.memory_growth == "fixed",
            table_growth_fixed: determinism.table_growth == "fixed",
        },
    )
    .map_err(|error| {
        super::super::super::model::Denial::classified(
            super::super::super::model::DenialClass::ProfileDenial,
            format!("shared feature admission denied: {error:?}"),
        )
    })?;
    let bounds = bounds(&profile.resources)?;
    hardened_wasmtime_shell::BoundedRuntime::build(&policy, bounds, hardened_wasmtime_runtime::Strategy::Cranelift)
        .map_err(|error| {
            super::super::super::model::Denial::classified(
                super::super::super::model::DenialClass::ProfileDenial,
                format!("shared runtime construction denied: {error:?}"),
            )
        })
}

fn bounds(
    resources: &super::super::super::model::ResourceLimits,
) -> super::super::super::model::Result<hardened_wasmtime_runtime::resource::admission::RuntimeBounds> {
    let page_bytes = hardened_wasmtime_runtime::resource::admission::WASM_PAGE_BYTES;
    if !resources.max_memory_bytes.is_multiple_of(page_bytes) {
        return Err(resource_denial("shared memory bound must be page aligned"));
    }
    Ok(hardened_wasmtime_runtime::resource::admission::RuntimeBounds {
        fuel: resources.fuel,
        max_stack_bytes: resources.max_stack_bytes,
        max_memory_pages: bound(
            resources
                .max_memory_bytes
                .checked_div(page_bytes)
                .ok_or_else(|| resource_denial("shared page width is zero"))?,
        )?,
        max_tables: bound(resources.max_tables)?,
        max_table_elements: bound(resources.max_table_elements)?,
        max_instances: bound(resources.max_instances)?,
        max_memories: bound(resources.max_memories)?,
        max_input_bytes: resources.max_hostcall_bytes,
        max_output_bytes: resources.max_result_bytes,
        max_component_bytes: resources.max_component_bytes,
        max_wit_bytes: resources.max_wit_bytes,
        max_imports: bound(resources.max_imports)?,
        max_exports: bound(resources.max_exports)?,
        max_concurrency: bound(resources.max_concurrency)?,
    })
}

fn bound(value: u64) -> super::super::super::model::Result<u32> {
    u32::try_from(value).map_err(|_| resource_denial("shared resource bound exceeds u32"))
}

fn resource_denial(detail: &str) -> super::super::super::model::Denial {
    super::super::super::model::Denial::classified(super::super::super::model::DenialClass::ResourceDenial, detail)
}

pub(super) fn facts(
    request: &super::super::shell::ExecutionRequest<'_>,
) -> hardened_wasmtime_runtime::artifact::shape::MaterializationFacts {
    let function = |name: &String| hardened_wasmtime_runtime::artifact::shape::InterfaceFunction {
        qualified_name: hardened_wasmtime_shell::qualified_name(&[name.as_str()]),
        // This exact WIT identity is already admitted by Molten, not inferred upstream.
        schema_identity: request.profile.wit.source_ref.clone(),
    };
    hardened_wasmtime_runtime::artifact::shape::MaterializationFacts {
        artifact_identity: blake3::hash(request.source.component_bytes()).to_hex().to_string(),
        class: hardened_wasmtime_runtime::artifact::shape::Class::Component,
        world: hardened_wasmtime_runtime::artifact::shape::WorldIdentity {
            package: request.profile.wit.package.clone(),
            world: request.facts.declared_world.clone(),
        },
        imports: request.facts.imports.iter().map(function).collect(),
        exports: request.facts.exports.iter().map(function).collect(),
    }
}

pub(super) fn prepare_denial(error: hardened_wasmtime_shell::PrepareError) -> super::super::super::model::Denial {
    let class = match error {
        hardened_wasmtime_shell::PrepareError::CompileFailed => {
            super::super::super::model::DenialClass::ComponentCompilationDenied
        }
        hardened_wasmtime_shell::PrepareError::DeadlineDenied => {
            super::super::super::model::DenialClass::ResourceDenial
        }
        hardened_wasmtime_shell::PrepareError::ArtifactTooLarge
        | hardened_wasmtime_shell::PrepareError::ArtifactMismatch
        | hardened_wasmtime_shell::PrepareError::ImportAdmission(_)
        | hardened_wasmtime_shell::PrepareError::ImportMismatch => {
            super::super::super::model::DenialClass::ComponentAdmissionDenial
        }
    };
    super::super::super::model::Denial::classified(class, format!("shared preparation denied: {error:?}"))
}
