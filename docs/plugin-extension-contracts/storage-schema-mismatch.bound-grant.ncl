let BoundGrant = import "bound-grant.ncl" in
let StorageContract = import "storage.ncl" in
((import "storage.grant.ncl") & {
  input_schema_ref | force = "blake3:babababababababababababababababababababababababababababababababa",
}) | BoundGrant.ForContract StorageContract
