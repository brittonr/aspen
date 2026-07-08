let BoundGrant = import "bound-grant.ncl" in
let StorageContract = import "storage.ncl" in
((import "storage.grant.ncl") & {
  hostcall_descriptor_ref | force = "blake3:abababababababababababababababababababababababababababababababab",
}) | BoundGrant.ForContract StorageContract
