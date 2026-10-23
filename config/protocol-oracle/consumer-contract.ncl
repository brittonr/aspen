# r[impl molten.fabric_simulation.protocol_oracle_evidence]
# The immutable protocol-observation contract an external adapter, such as a
# later ChaosControl integration, consumes. It names the projection record,
# its canonical encoding, and its identity derivation; it depends on no
# workspace-relative product path.
let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let Contracts = import "contracts.ncl" in
let FieldKinds = [
  "string",
  "symbol",
  "unsigned",
  "content-ref",
  "optional-content-ref",
  "extension-value",
  "string-sequence",
]
in
let Field = {
  name | Prelude.contracts.NonEmptyString,
  kind | Prelude.contract (Prelude.is_one_of FieldKinds),
  is_evidential | Bool,
}
in
let ConsumerContract = {
  schema | Prelude.contract (Prelude.is_exact_string "molten.protocol-observation-contract.v1"),
  projection_schema | Prelude.contract (Prelude.is_exact_string "molten.protocol-projection.v1"),
  record_label | Prelude.contract (Prelude.is_exact_string "protocol-projection"),
  encoding | Prelude.contract (Prelude.is_exact_string "preserves-canonical-packed"),
  identity
    | {
      hash | Prelude.contract (Prelude.is_exact_string "blake3-derive-key"),
      projection_domain | Prelude.contracts.NonEmptyString,
      state_domain | Prelude.contracts.NonEmptyString,
      novelty_domain | Prelude.contracts.NonEmptyString,
      evidence_domain | Prelude.contracts.NonEmptyString,
    },
  fields | Array Field,
  transition_classes | Prelude.contracts.NonEmptyUniqueStringArray,
  durability_facts | Prelude.contracts.NonEmptyUniqueStringArray,
  completeness | Prelude.contracts.NonEmptyUniqueStringArray,
  non_claims | Prelude.contracts.NonEmptyUniqueStringArray,
}
in
let field = fun field_name field_kind evidential => { name = field_name, kind = field_kind, is_evidential = evidential } in
{
  schema = "molten.protocol-observation-contract.v1",
  projection_schema = "molten.protocol-projection.v1",
  record_label = "protocol-projection",
  encoding = "preserves-canonical-packed",
  identity = {
    hash = "blake3-derive-key",
    projection_domain = "molten.protocol-projection.v1",
    state_domain = "molten.protocol-state.v1",
    novelty_domain = "molten.protocol-novelty.v1",
    evidence_domain = "molten.protocol-oracle-evidence.v1",
  },
  fields = [
    field "schema" "string" true,
    field "protocol" "string" true,
    field "service" "symbol" true,
    field "participant" "string" true,
    field "generation" "unsigned" true,
    field "source-sequence" "unsigned" true,
    field "run-sequence" "unsigned" true,
    field "scheduler-choice-ref" "content-ref" true,
    field "transition-class" "symbol" true,
    field "logical-position" "unsigned" true,
    field "state" "extension-value" true,
    field "state-ref" "content-ref" true,
    field "physical-ref" "optional-content-ref" true,
    field "history-ref" "content-ref" true,
    field "durability" "symbol" true,
    field "progress" "unsigned" true,
    field "completeness" "symbol" true,
    field "reported-invariants" "string-sequence" false,
  ],
  transition_classes = ["initial", "applied", "conflict", "denied", "recovered"],
  durability_facts = ["volatile", "durable", "recovered"],
  completeness = ["complete", "partial"],
  non_claims = Contracts.required_non_claims,
} | ConsumerContract
