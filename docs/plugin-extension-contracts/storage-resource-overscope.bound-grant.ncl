let BoundGrant = import "bound-grant.ncl" in
let StorageContract = import "storage.ncl" in
((import "storage.grant.ncl") & {
  resource_scope | force = "blake3:cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
}) | BoundGrant.ForContract StorageContract
