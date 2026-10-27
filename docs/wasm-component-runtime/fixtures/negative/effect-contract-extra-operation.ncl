let Contracts = import "../../effect-contracts.ncl" in
(Contracts.default & {
  closed_operations | force = [
    { effect_id = "blob.get", operation = "get", capability = "hostcall:blob-get" },
    { effect_id = "blob.put", operation = "put", capability = "hostcall:blob-put" },
  ],
}) | Contracts.Export
