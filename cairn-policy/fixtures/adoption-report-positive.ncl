let Adoption = import "../adoption-report.ncl" in
let HashA = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
let HashB = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" in
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
      scope_id = "positive-cairn-to-octet",
      relationships = [Relationship],
      bounds = { max_relationships = 512, max_observations = 4096, max_provenance = 128 },
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
    observations = [
      {
        relationship = Relationship,
        stage = 'upstream_lifecycle,
        state = 'supported,
        producer_id = "cairn-lifecycle",
        producer_status = "archived",
        evidence_hashes = [HashB],
        observed_revision = "62f872c",
        message = "upstream archive observed",
      },
      {
        relationship = Relationship,
        stage = 'consumer_pin,
        state = 'supported,
        producer_id = "workspace-stack-deps-v2",
        producer_status = "matching",
        evidence_hashes = [HashB],
        observed_revision = "62f872c",
        message = "consumer pin matches required revision",
      },
      {
        relationship = Relationship,
        stage = 'consumer_compatibility,
        state = 'supported,
        producer_id = "consumer-evidence",
        producer_status = "compatible",
        evidence_hashes = [HashB],
        observed_revision = "62f872c",
        message = "consumer compatibility evidence supplied",
      },
      {
        relationship = Relationship,
        stage = 'integration_evidence,
        state = 'supported,
        producer_id = "stack-doctor",
        producer_status = "passed",
        evidence_hashes = [HashB],
        observed_revision = "62f872c",
        message = "current integration evidence supplied",
      },
    ],
    coverage = {
      partial = false,
      selected_relationships = 1,
      observed_relationships = 1,
      omitted_relationships = [],
      bound_exhausted = [],
    },
  },
  policy = Adoption.policy,
}
