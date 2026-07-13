let Contracts = import "contracts.ncl" in
let get = fun field fallback args => std.record.get_or field fallback args in
{
  make = fun args => {
    schema_id = Contracts.schema_id,
    schema_version = Contracts.schema_version,
    source_language = Contracts.source_language,
    profile_ref = get "profile_ref" "blake3:1111111111111111111111111111111111111111111111111111111111111111" args,
    production = get "production" true args,
    fixture_only = get "fixture_only" false args,
    algorithm = get "algorithm" Contracts.production_algorithm args,
    entropy = {
      source_kind = get "entropy_source_kind" "os-csprng" args,
      source_ref = get "entropy_source_ref" "blake3:2222222222222222222222222222222222222222222222222222222222222222" args,
    },
    backend = {
      class = get "backend_class" "capability-file" args,
      backend_ref = get "backend_ref" "blake3:3333333333333333333333333333333333333333333333333333333333333333" args,
      capability_rooted = get "capability_rooted" true args,
      owner_only_permissions = get "owner_only_permissions" true args,
    },
    purposes = get "purposes" Contracts.purposes args,
    purpose_separation = get "purpose_separation" true args,
    domain = {
      id = "molten.crypto.signature",
      version = "v1",
    },
    rotation = {
      generation_fenced = true,
      revocation_required = true,
      overlap_verification_only = get "overlap_verification_only" true args,
    },
    redaction = {
      private_bytes = true,
      backend_locator = true,
    },
    non_claims = get "non_claims" Contracts.non_claims args,
  } | Contracts.Export,
}
