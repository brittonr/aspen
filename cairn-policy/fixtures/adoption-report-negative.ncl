let Adoption = import "../adoption-report.ncl" in
let HashA = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let Relationship = {
  relationship_id = "cairn-to-octet",
  upstream_repository = "onixresearch/cairn",
  upstream_change = "unify-change-readiness-and-next-actions",
  consumer_repository = "onixresearch/octet",
  required_revision = "62f872c",
  subject_id = "cairn-readiness-adoption",
  policy_id = "cairn-default",
} in
{
  snapshot | Adoption.contracts.Snapshot = {
    schema = "cairn.adoption.snapshot.v1",
    query = {
      scope_id = "negative-cairn-to-octet",
      relationships = [Relationship],
      # Negative control: named bounds must be positive integers.
      bounds = { max_relationships = 0, max_observations = 4096, max_provenance = 128 },
    },
    provenance = [{
      producer_id = "workspace-stack-deps-v2",
      schema = "onixresearch.stack-dependency-receipt.v1",
      source_revision = "22515fb7bc01d378312053d389d4bbdd55637a50",
      tool_identity = "stack-deps@22515fb7bc01d378312053d389d4bbdd55637a50",
      policy_identity = "workspace-dependency-policy@22515fb7bc01d378312053d389d4bbdd55637a50",
      observation_scope = "cairn-to-octet",
      receipt_hash = HashA,
    }],
    observations = [],
    coverage = {
      partial = false,
      selected_relationships = 1,
      observed_relationships = 0,
      omitted_relationships = [],
      bound_exhausted = [],
    },
  },
  policy = Adoption.policy,
}
