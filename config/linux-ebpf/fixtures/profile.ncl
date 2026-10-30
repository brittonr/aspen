# Synthetic authoring fixture, never an enabled host profile or source receipt.
let C = import "../contracts.ncl" in
let Ref = "blake3:0000000000000000000000000000000000000000000000000000000000000000" in
{
  schema = "molten.linux-ebpf-observation.profile.v1",
  profile_ref = Ref,
  binding = {
    machine = "synthetic-machine", generation = "synthetic-generation",
    pack = "onix:blake3:bpf-pack:0000000000000000000000000000000000000000000000000000000000000000",
    kernel = "onix:blake3:kernel-build:0000000000000000000000000000000000000000000000000000000000000000",
    program = Ref, object = Ref, btf = Ref,
    schema = "onixos.bpf-observation.window.v1", scope = Ref, accounting = Ref, adapter = Ref,
  },
  mappings = [{ signal = "process", descriptor_ref = Ref, aggregation = "sum", labels = [] }],
  max_records = 32, max_bytes = 4096, max_streams = 8,
  max_queue_records = 32, max_queue_bytes = 4096, max_series = 1, max_events = 1,
  window_ticks = 10, freshness_ticks = 5, required = true,
  loss_policy = "incomplete-on-any-loss-or-missing-accounting",
  event_detail = "status-only", redaction = "deny-unmapped-and-all-free-text", scope = "local-component",
  non_claims = C.non_claims,
} | C.Profile
