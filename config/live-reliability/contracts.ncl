let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let Bounded = fun limit => Prelude.contract (fun value =>
  std.is_number value && std.number.is_integer value && value > 0 && value <= limit) in
let Ref = Prelude.contract (fun value => std.is_string value
  && std.string.is_match "^blake3:[0-9a-f]{64}$" value) in
let Integer = Prelude.contract (fun value => std.is_number value && std.number.is_integer value
  && value >= -9223372036854775808 && value <= 9223372036854775807) in
let RegisterProfile = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.live-register-profile.v1"),
  group | Prelude.contracts.StableId,
  service | Prelude.contracts.StableId,
  keys | Prelude.contracts.NonEmptyUniqueStringArray,
  initial_value | Prelude.contract (fun value => value == null),
  values | Array Integer,
  generator_ref | Ref,
  adapter_ref | Ref,
  model_ref | Ref,
  outcome_ref | Ref,
  seed | Prelude.contract (fun value => std.is_number value && std.number.is_integer value
    && value >= 0 && value <= 18446744073709551615),
  read_weight | Bounded 4294967295,
  write_weight | Bounded 4294967295,
  clients | Prelude.contracts.NonEmptyUniqueStringArray,
  endpoints | Prelude.contracts.NonEmptyUniqueStringArray,
  concurrency | Bounded 64,
  operations | Bounded 4096,
  control_operations | Bounded 4096,
  consistency_model | Prelude.contract (Prelude.is_exact_string "register"),
  recovery | {
    timeout_ms | Bounded 3600000,
    stable_membership_observations | Bounded 1024,
    require_all_endpoint_reads | Prelude.contract (fun value => value == true),
  },
  evidence | {
    max_attempts_per_operation | Bounded 16,
    max_events | Bounded 131072,
    max_uncertain | Prelude.contract (fun value => std.is_number value && std.number.is_integer value && value >= 0),
    max_bytes | Bounded 134217728,
    checker_states | Bounded 1000000,
    checker_branches | Bounded 10000000,
    checker_memo_bytes | Bounded 134217728,
    checker_reductions | Bounded 1000000,
  },
}, Prelude.contract (fun profile =>
  std.array.length profile.keys == 1
  && std.array.length profile.values > 0 && std.array.length profile.values <= 256
  && std.array.length profile.values == std.array.length (std.array.sort_dedup std.number.compare profile.values)
  && std.array.length profile.clients <= 64 && std.array.length profile.endpoints <= 64
  && profile.concurrency <= std.array.length profile.clients
  && profile.control_operations + std.array.length profile.endpoints <= profile.operations
  && profile.read_weight + profile.write_weight <= 4294967295
  && profile.evidence.max_events >= 2 * profile.operations * profile.evidence.max_attempts_per_operation
  && profile.evidence.max_uncertain <= profile.operations * profile.evidence.max_attempts_per_operation
)] in
let FaultMilliseconds = Bounded 120000 in
let FaultProfile = fun endpoints mechanism => std.contract.Sequence [{
  fault | mechanism,
  heal_deadline_ms | FaultMilliseconds,
  recovery_deadline_ms | FaultMilliseconds,
  stable_membership_ms | FaultMilliseconds,
  final_read_deadline_ms | FaultMilliseconds,
}, Prelude.contract (fun profile =>
  std.is_array endpoints
  && std.array.length endpoints >= 3 && std.array.length endpoints <= 9
  && std.array.all std.is_string endpoints
  && std.array.length (std.array.sort_dedup std.string.compare endpoints) == std.array.length endpoints
  && profile.stable_membership_ms <= profile.recovery_deadline_ms
)] in
{
  Profile = RegisterProfile,
  NoFault = fun endpoints => FaultProfile endpoints {
    kind | Prelude.contract (Prelude.is_exact_string "no-fault"),
  },
  ProcessRestart = fun endpoints => std.contract.Sequence [
    FaultProfile endpoints {
      kind | Prelude.contract (Prelude.is_exact_string "process-restart"),
      endpoint | String,
      down_ms | FaultMilliseconds,
    },
    Prelude.contract (fun profile =>
      std.array.elem profile.fault.endpoint endpoints
      && profile.fault.down_ms <= profile.heal_deadline_ms),
  ],
  TemporaryPartition = fun endpoints => std.contract.Sequence [
    FaultProfile endpoints {
      kind | Prelude.contract (Prelude.is_exact_string "temporary-partition"),
      left | Prelude.contracts.NonEmptyUniqueStringArray,
      right | Prelude.contracts.NonEmptyUniqueStringArray,
      duration_ms | FaultMilliseconds,
    },
    Prelude.contract (fun profile =>
      let sides = profile.fault.left @ profile.fault.right in
      profile.fault.duration_ms <= profile.heal_deadline_ms
      && std.array.length sides == std.array.length endpoints
      && std.array.all (fun endpoint => std.array.elem endpoint sides) endpoints),
  ],
}
