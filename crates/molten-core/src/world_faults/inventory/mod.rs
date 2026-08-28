mod cases;
mod contracts;

use std::collections::BTreeSet;

use cases::expected_failure_cases;
use contracts::expected_contract;

use super::*;

// r[impl molten.world_faults.inventory]
pub fn standard_world_mutation_inventory() -> WorldMutationInventory {
    WorldMutationInventory {
        schema: WORLD_MUTATION_INVENTORY_SCHEMA,
        version: WORLD_MUTATION_INVENTORY_VERSION,
        rows: WorldMutationKind::ALL.into_iter().map(expected_contract).collect(),
    }
}

pub fn registered_world_mutation_names() -> Vec<String> {
    WorldMutationKind::ALL.into_iter().map(|mutation| mutation.as_str().to_string()).collect()
}

// r[impl molten.world_faults.inventory]
#[allow(
    tigerstyle::unbounded_collection_growth,
    reason = "the closed row and product-name sets are bounded by REQUIRED_WORLD_MUTATION_COUNT"
)]
pub fn validate_world_mutation_inventory(
    inventory: &WorldMutationInventory,
    product_mutations: &[String],
) -> Vec<WorldFaultIssue> {
    let mut issues = Vec::with_capacity(REQUIRED_WORLD_MUTATION_COUNT);
    if inventory.schema != WORLD_MUTATION_INVENTORY_SCHEMA {
        issues.push(WorldFaultIssue::SchemaMismatch("world-mutation-inventory"));
    }
    if inventory.version != WORLD_MUTATION_INVENTORY_VERSION {
        issues.push(WorldFaultIssue::InventoryVersionMismatch);
    }
    if inventory.rows.len() != REQUIRED_WORLD_MUTATION_COUNT {
        issues.push(WorldFaultIssue::InventoryRowCount {
            actual: inventory.rows.len(),
            expected: REQUIRED_WORLD_MUTATION_COUNT,
        });
    }

    let mut observed = BTreeSet::new();
    for row in &inventory.rows {
        if !observed.insert(row.mutation) {
            issues.push(WorldFaultIssue::DuplicateMutation(row.mutation));
            continue;
        }
        if row != &expected_contract(row.mutation) {
            issues.push(WorldFaultIssue::InventoryContractMismatch(row.mutation));
        }
        validate_required_contract(row, &mut issues);
    }
    for mutation in WorldMutationKind::ALL {
        if !observed.contains(&mutation) {
            issues.push(WorldFaultIssue::MissingMutation(mutation));
        }
    }

    let expected_names = registered_world_mutation_names().into_iter().collect::<BTreeSet<_>>();
    let product_names = product_mutations.iter().cloned().collect::<BTreeSet<_>>();
    for mutation in product_names.difference(&expected_names) {
        issues.push(WorldFaultIssue::UnknownProductMutation(mutation.clone()));
    }
    for mutation in expected_names.difference(&product_names) {
        issues.push(WorldFaultIssue::ProductMutationMissing(mutation.clone()));
    }
    issues.sort();
    issues.dedup();
    issues
}

#[allow(
    tigerstyle::unbounded_collection_growth,
    reason = "phase and negative-case loops are bounded by closed enum inventories"
)]
fn validate_required_contract(row: &WorldMutationContract, issues: &mut Vec<WorldFaultIssue>) {
    for phase in FaultPhase::ALL {
        if !row.required_phases.contains(&phase) {
            issues.push(WorldFaultIssue::MissingRequiredPhase {
                mutation: row.mutation,
                phase,
            });
        }
    }
    for required in expected_failure_cases(row.mutation) {
        if !row.required_cases.contains(&required) {
            issues.push(WorldFaultIssue::MissingRequiredFailureCase {
                mutation: row.mutation,
                case: required,
            });
        }
    }
    let is_witness_support_exact = if row.mutation == WorldMutationKind::Witness {
        row.support == MutationSupport::UnsupportedIndependentWitness
    } else {
        row.support == MutationSupport::Supported
    };
    if !is_witness_support_exact {
        issues.push(WorldFaultIssue::WitnessSupportOverclaim);
    }
}
