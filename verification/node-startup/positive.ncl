# Structural contract fixture only. These placeholder hashes approve no real bundle.
let Policy = import "../../docs/node-startup-cohort.ncl" in
let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" in
({
  descriptor_blake3 = hash,
  cohort = {
    source_revision = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    source_inventory_blake3 = hash,
    executable_blake3 = hash,
    build_rustc_blake3 = hash,
    octet_cli_blake3 = hash,
    octet_driver_blake3 = hash,
    octet_lints_blake3 = hash,
    octet_rustc_blake3 = hash,
  },
}) | Policy
