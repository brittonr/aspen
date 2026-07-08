let BoundGrant = import "bound-grant.ncl" in
let StorageContract = import "storage.ncl" in
((import "storage.grant.ncl") & {
  operation | force = "storage.write",
}) | BoundGrant.ForContract StorageContract
