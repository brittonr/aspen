# An operator must supply all identities after review of actual tool execution.
# This contract has no admitted default cohort and grants no startup authority.
let Prelude = import "nickel-contract-prelude.ncl" in
let Hash = Prelude.contract (fun value => std.is_string value && std.string.is_match "^[0-9a-f]{64}$" value) in
let Revision = Prelude.contract (fun value => std.is_string value && std.string.is_match "^[0-9a-f]{40}$" value) in
{
  schema
    | Prelude.contract (Prelude.is_exact_string "molten.node-startup-cohort.v1")
    | doc "Version of the exact operator cohort, not a startup grant."
    | default = "molten.node-startup-cohort.v1",
  descriptor_blake3 | Hash
    | doc "Independently approved digest of bundle.json. Never derive trust from the supplied bundle.",
  cohort
    | doc "Distinct source, executable, build compiler, and Octet identities."
    = {
      source_revision | Revision | doc "Exact reviewed Molten Git revision.",
      source_inventory_blake3 | Hash | doc "Digest of the complete approved source-inventory.json bytes.",
      executable_blake3 | Hash | doc "Digest of the running Molten executable, not the source or archive.",
      build_rustc_blake3 | Hash | doc "Observed digest of the compiler used to build Molten.",
      build_toolchain
        | Prelude.contract (Prelude.is_exact_string "nightly-2026-05-26")
        | doc "Repository build compiler cohort. It is distinct from the pinned Octet compiler."
        | default = "nightly-2026-05-26",
      octet_revision
        | Prelude.contract (Prelude.is_exact_string "c9b06bcf565c51d4a77d210e61b69ae51db9df25")
        | doc "Immutable Octet toolchain input for this version of the verifier."
        | default = "c9b06bcf565c51d4a77d210e61b69ae51db9df25",
      octet_cli_blake3 | Hash | doc "Observed digest of the Octet CLI used for the run.",
      octet_driver_blake3 | Hash | doc "Observed digest of the Dylint driver used for the run.",
      octet_lints_blake3 | Hash | doc "Observed digest of the pinned Octet lint library.",
      octet_rustc_blake3 | Hash | doc "Observed digest of Octet's compiler, separate from the Molten compiler.",
      octet_toolchain
        | Prelude.contract (Prelude.is_exact_string "nightly-2026-03-21-x86_64-unknown-linux-gnu")
        | doc "Compiler ABI declared by the pinned Octet cohort."
        | default = "nightly-2026-03-21-x86_64-unknown-linux-gnu",
    },
}
