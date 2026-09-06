# These five rows were present in Molten's authored policy at 9ae893f31.
# Its committed projection still references them in receipt_contracts, but omits
# their receipt_schemas rows. Restore the rows; never delete their references.
[
  {
    command = "evidence function-object validate",
    top_level_keys = ["command", "family", "issues", "object_hash", "schema", "valid", "verdict"],
    issue_keys = ["actual", "code", "expected", "field_path"],
    hash_fields = ["object_hash"],
  },
  {
    command = "evidence name-pointer validate",
    top_level_keys = ["command", "family", "issues", "pointer_hash", "schema", "valid", "verdict"],
    issue_keys = ["actual", "code", "expected", "field_path"],
    hash_fields = ["pointer_hash"],
  },
  {
    command = "evidence export-bundle validate",
    top_level_keys = ["bundle_hash", "command", "family", "issues", "schema", "valid", "verdict"],
    issue_keys = ["actual", "code", "expected", "field_path"],
    hash_fields = ["bundle_hash"],
  },
  {
    command = "evidence downstream-pilot validate",
    top_level_keys = ["command", "family", "issues", "pilot_hash", "schema", "valid", "verdict"],
    issue_keys = ["actual", "code", "expected", "field_path"],
    hash_fields = ["pilot_hash"],
  },
  {
    command = "evidence circuit-primitive validate",
    top_level_keys = ["command", "family", "graph_hash", "issues", "schema", "valid", "verdict"],
    issue_keys = ["actual", "code", "expected", "field_path"],
    hash_fields = ["graph_hash"],
  },
]
