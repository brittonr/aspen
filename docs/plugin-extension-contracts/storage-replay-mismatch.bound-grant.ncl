let BoundGrant = import "bound-grant.ncl" in
let StorageContract = import "storage.ncl" in
((import "storage.grant.ncl") & {
  replay_class | force = "deterministic",
}) | BoundGrant.ForContract StorageContract
