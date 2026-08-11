# r[verify molten.runtime_spine.preserves_boundary_profile.contract]
# r[impl molten.runtime_spine.preserves_boundary_profile.fixtures.positive]
let Contracts = import "contracts.ncl" in
let Ref = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let NonClaim = "canonical-boundary-identity-only: this profile does not prove transport liveness, actor authority correctness, replay completeness, or Valence Evidence IR acceptance" in
let row = fun family schema => {
  artifact_family = family,
  schema_label = schema,
  canonical_bytes_required = true,
  blake3_identity_field = "artifact_ref",
  adapter_owner = "preserves-adapter",
  core_dto = "typed-boundary-dto",
  allowed_consumers = ["runtime", "evidence", "release-review"],
  non_claims = [NonClaim],
  adapter_only = true,
} in
let measurement = fun family schema => {
  artifact_family = family,
  schema_label = schema,
  canonical_bytes = true,
  blake3_ref = Ref,
  consumer = "runtime",
  raw_preserves_core_coupling = false,
} in
let Rows = [
  row "node-control-envelope" "molten.node-control.envelope.v1",
  row "ticket" "molten.node-control.ticket.v1",
  row "workflow-bundle" "molten.node-control.workflow-bundle.v1",
  row "receipt" "molten.receipt.v1",
  row "evidence-envelope" "molten.stack-evidence.envelope.v1",
] in
{
  schema = "molten.preserves-boundary-profile.v1",
  rows = Rows,
  measurements = std.array.map (fun item => measurement item.artifact_family item.schema_label) Rows,
} | Contracts.Profile
