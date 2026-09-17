# Preserve the accepted policy and add the Molten root-crate trace source.
let EvidenceSource = {
  extensions | Array String,
  root | String,
  source_kind | String,
} in
let Profile = {
  id | String,
  evidence_sources | Array EvidenceSource,
  ..
} in
let Policy = {
  traceability_policy | {
    profiles | Array Profile,
    ..
  },
  ..
} in
fun source =>
  let base = source | Policy in
  base & {
    traceability_policy.profiles | force =
      std.array.map
        (fun profile =>
          if profile.id == "cairn-default" then
            profile & {
              evidence_sources | force = [{
                extensions = ["rs"],
                root = "src",
                source_kind = "rust_source",
              }] @ profile.evidence_sources,
            }
          else profile)
        base.traceability_policy.profiles,
  }
