use super::super::model::ComponentDenial;
use super::super::model::ComponentDenialClass;
use super::super::model::ComponentResult;
use super::super::model::GrowthStrategy;
use super::ComponentArtifactFacts;
use super::ComponentGrowthFacts;

const WASM_PAGE_BYTES: u64 = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservedResources {
    memory: ComponentGrowthFacts,
    table: ComponentGrowthFacts,
    instances: u64,
    memories: u64,
    tables: u64,
}

pub(crate) fn verify_component_artifact_facts(bytes: &[u8], expected: &ComponentArtifactFacts) -> ComponentResult<()> {
    validate_feature_cohort(bytes)?;
    let observed = inspect_resources(bytes)?;
    if observed.memory != expected.memory {
        return Err(ComponentDenial::new("component memory declaration differs from admitted materialization facts"));
    }
    if observed.table != expected.table {
        return Err(ComponentDenial::new("component table declaration differs from admitted materialization facts"));
    }
    if observed.instances != expected.instances {
        return Err(ComponentDenial::new("component instance count differs from admitted materialization facts"));
    }
    if observed.memories != expected.memories {
        return Err(ComponentDenial::new("component memory count differs from admitted materialization facts"));
    }
    if observed.tables != expected.tables {
        return Err(ComponentDenial::new("component table count differs from admitted materialization facts"));
    }
    Ok(())
}

fn validate_feature_cohort(bytes: &[u8]) -> ComponentResult<()> {
    let features = wasmparser::WasmFeatures::WASM1
        | wasmparser::WasmFeatures::REFERENCE_TYPES
        | wasmparser::WasmFeatures::MULTI_VALUE
        | wasmparser::WasmFeatures::BULK_MEMORY
        | wasmparser::WasmFeatures::SIMD
        | wasmparser::WasmFeatures::COMPONENT_MODEL;
    wasmparser::Validator::new_with_features(features).validate_all(bytes).map_err(|error| {
        ComponentDenial::classified(
            ComponentDenialClass::ProfileDenial,
            format!("component feature-cohort validation failed: {error}"),
        )
    })?;
    Ok(())
}

fn inspect_resources(bytes: &[u8]) -> ComponentResult<ObservedResources> {
    let mut observed = ObservedResources {
        memory: empty_growth(),
        table: empty_growth(),
        instances: 0,
        memories: 0,
        tables: 0,
    };
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        match payload.map_err(parse_denial)? {
            wasmparser::Payload::ImportSection(section) => inspect_imports(section, &mut observed)?,
            wasmparser::Payload::MemorySection(section) => {
                for memory in section {
                    observe_memory(memory.map_err(parse_denial)?, &mut observed)?;
                }
            }
            wasmparser::Payload::TableSection(section) => {
                for table in section {
                    observe_table(table.map_err(parse_denial)?.ty, &mut observed)?;
                }
            }
            wasmparser::Payload::InstanceSection(section) => {
                observed.instances = checked_add("core instance", observed.instances, u64::from(section.count()))?;
            }
            wasmparser::Payload::ComponentInstanceSection(section) => {
                observed.instances = checked_add("component instance", observed.instances, u64::from(section.count()))?;
            }
            _ => {}
        }
    }
    Ok(observed)
}

fn inspect_imports(
    section: wasmparser::ImportSectionReader<'_>,
    observed: &mut ObservedResources,
) -> ComponentResult<()> {
    for import in section {
        match import.map_err(parse_denial)?.ty {
            wasmparser::TypeRef::Memory(memory) => observe_memory(memory, observed)?,
            wasmparser::TypeRef::Table(table) => observe_table(table, observed)?,
            _ => {}
        }
    }
    Ok(())
}

fn observe_memory(memory: wasmparser::MemoryType, observed: &mut ObservedResources) -> ComponentResult<()> {
    if memory.memory64 || memory.shared || memory.page_size_log2.is_some() {
        return Err(ComponentDenial::new(
            "component memory uses a disabled memory64, shared, or custom-page-size feature",
        ));
    }
    if memory.maximum != Some(memory.initial) {
        return Err(ComponentDenial::new("component memory declaration permits nondeterministic growth"));
    }
    let initial = memory
        .initial
        .checked_mul(WASM_PAGE_BYTES)
        .ok_or_else(|| ComponentDenial::new("component memory byte bound overflowed"))?;
    observed.memory.initial = observed.memory.initial.max(initial);
    observed.memory.maximum = Some(observed.memory.initial);
    observed.memories = checked_add("memory", observed.memories, 1)?;
    Ok(())
}

fn observe_table(table: wasmparser::TableType, observed: &mut ObservedResources) -> ComponentResult<()> {
    if table.table64 || table.shared {
        return Err(ComponentDenial::new("component table uses a disabled table64 or shared-table feature"));
    }
    if table.maximum != Some(table.initial) {
        return Err(ComponentDenial::new("component table declaration permits nondeterministic growth"));
    }
    observed.table.initial = observed.table.initial.max(table.initial);
    observed.table.maximum = Some(observed.table.initial);
    observed.tables = checked_add("table", observed.tables, 1)?;
    Ok(())
}

fn checked_add(label: &str, left: u64, right: u64) -> ComponentResult<u64> {
    left.checked_add(right)
        .ok_or_else(|| ComponentDenial::new(format!("component {label} count overflowed")))
}

fn empty_growth() -> ComponentGrowthFacts {
    ComponentGrowthFacts {
        initial: 0,
        maximum: Some(0),
        strategy: GrowthStrategy::Fixed,
    }
}

fn parse_denial(error: wasmparser::BinaryReaderError) -> ComponentDenial {
    ComponentDenial::classified(
        ComponentDenialClass::ResourceDenial,
        format!("component resource inspection failed: {error}"),
    )
}
