let Contracts = import "../contracts.ncl" in
let Base = import "../default.ncl" in
(Base & {
  receipt_contracts | force = Base.receipt_contracts @ [{
    family = "unknown-command-fixture",
    schema = "fixture.unknown-receipt.v1",
    command = "evidence unknown-command validate",
    ownership = "fixture",
    compatibility_class = "negative-fixture",
    encoding = "deterministic-json-v1",
    hash_field = "fixture_hash",
    hash_material = "fixture",
    non_claims = ["no_authority"],
  }],
}) | Contracts.Policy
