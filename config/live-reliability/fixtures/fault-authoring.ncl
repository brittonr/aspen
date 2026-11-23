let mk = import "../profile.ncl" in
let Faults = import "../faults.ncl" in
let Contracts = import "../contracts.ncl" in
# Synthetic references exercise authoring, not artifact or provider admission.
let ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let product = mk { generator_ref = ref, adapter_ref = ref, model_ref = ref, outcome_ref = ref } in
let nodes = product.endpoints in
let bounds = {
  heal_deadline_ms = 10,
  recovery_deadline_ms = 100,
  stable_membership_ms = 10,
  final_read_deadline_ms = 10,
} in
let minority = ["node-a:7180"] in
let majority = ["node-b:7180", "node-c:7180"] in
{
  accepted = {
    register = product,
    no_fault = Faults.no_fault nodes bounds,
    process_restart = Faults.process_restart nodes bounds {
      endpoint = "node-a:7180",
      down_ms = bounds.heal_deadline_ms,
    },
    temporary_partition = Faults.temporary_partition nodes bounds {
      left = minority,
      right = majority,
      duration_ms = bounds.heal_deadline_ms,
    },
  },
  denied = {
    restart_over_heal = Faults.process_restart nodes bounds { endpoint = "node-a:7180", down_ms = 11 },
    partition_over_heal = Faults.temporary_partition nodes bounds {
      left = minority, right = majority, duration_ms = 11,
    },
    restart_outside_cohort = Faults.process_restart nodes bounds { endpoint = "other:7180", down_ms = 1 },
    partition_overlap = Faults.temporary_partition nodes bounds {
      left = minority, right = ["node-a:7180", "node-c:7180"], duration_ms = 1,
    },
    partition_incomplete = Faults.temporary_partition nodes bounds {
      left = minority, right = ["node-b:7180"], duration_ms = 1,
    },
    short_recovery = Faults.no_fault nodes {
      heal_deadline_ms = 10, recovery_deadline_ms = 9, stable_membership_ms = 10, final_read_deadline_ms = 10,
    },
    unsupported = (bounds & { fault = { kind = "durability" } }) | Contracts.NoFault nodes,
    supplied_effect = Faults.process_restart nodes bounds {
      endpoint = "node-a:7180", down_ms = 1, effect_ref = ref,
    },
  },
}
