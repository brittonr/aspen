# Export a concrete merge of this contract with manifest_ref/readers/bind_addr.
# The normal CLI also applies semantic address, duration, and grant admission.
let Prelude = import "nickel-contract-prelude.ncl" in
let Key = Prelude.contract (fun value => std.is_string value && std.string.is_match "^[0-9a-f]{64}$" value) in
let Ref = Prelude.contract (fun value => std.is_string value && std.string.is_match "^blake3:[0-9a-f]{64}$" value) in
{
  schema
    | Prelude.contract (Prelude.is_exact_string "molten.node-content.v1")
    | doc "Closed version of the bounded normal-node content contract."
    | default
    = "molten.node-content.v1",
  manifest_ref
    | Ref
    | doc "Exact canonical Molten manifest, not an archive digest or transport ticket.",
  readers
    | Array Key
    | Prelude.contract (fun value =>
      std.array.length value > 0
      && std.array.length value <= 16
      && Prelude.distinct_strings value
    )
    | doc "Finite local operator read grant. Public reader keys are not UCAN credentials.",
  bind_addr
    | Prelude.contracts.NonEmptyString
    | doc "Explicit non-wildcard IP socket address. Rust rejects invalid, multicast, and zero-port addresses.",
  tick_ms
    | Prelude.contract (fun value => Prelude.is_integer value && value >= 100 && value <= 1000)
    | doc "Normal daemon tick interval. The configured tick-wait budget cannot exceed 300 seconds."
    | default
    = 250,
}
