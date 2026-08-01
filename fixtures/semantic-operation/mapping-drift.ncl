let Contracts = import "contract.ncl" in
let Operation = {
  domain = "effect-operation",
  algorithm = "blake3",
  hex = "e89d7b8bf84b4c9c8564ec745109dcef80838bc99ca887edbbfd0d78e72e952b",
} in
{
  schema = "molten.semantic-operation-mapping.v1",
  kamacite_revision = "d76fe4abe543724d8fc0ac4b362187caf2e27622",
  operation = Operation,
  surfaces = {
    manifest = Operation,
    handler_binding = {
      domain = "effect-operation",
      algorithm = "blake3",
      hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    },
    handle = Operation,
    request = Operation,
    response = Operation,
    effect_log = Operation,
    adapter_import = Operation,
    remote_execution = Operation,
    runtime_receipt = Operation,
    replay_identity = Operation,
    evaluation_cache_key = Operation,
    job = Operation,
    upgrade_check = Operation,
  },
  expected = "deny",
} | Contracts.Mapping
