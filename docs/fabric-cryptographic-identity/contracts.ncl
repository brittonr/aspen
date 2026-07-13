# Typed contract for Molten's cryptographic identity adapter profile.

let SchemaId = "molten.fabric.crypto-identity-profile.v1" in
let SchemaVersion = 1 in
let SourceLanguage = "nickel" in
let ProductionAlgorithm = "ed25519-iroh-v1" in
let FixtureAlgorithm = "blake3-local-fixture-v1" in
let RequiredPurposes = [
  "transport-endpoint",
  "federation-origin",
  "delegation",
  "evidence-signing",
  "authority",
] in
let RequiredNonClaims = [
  "verified-signature-does-not-grant-capability-authority",
  "public-identity-does-not-grant-membership",
  "signature-alone-does-not-prove-provenance",
  "signature-does-not-prove-payload-correctness",
  "profile-does-not-guarantee-backend-availability",
  "profile-does-not-imply-algorithm-agility",
] in
let ValidAlgorithms = [ProductionAlgorithm, FixtureAlgorithm] in
let ValidEntropySources = ["os-csprng", "deterministic-fixture"] in
let ValidBackends = ["capability-file", "managed-external", "deterministic-fixture"] in
let valid_ref = fun value =>
  std.is_string value
  && std.string.is_match "^blake3:[0-9a-f]{64}$" value in
let valid_profile = fun value =>
  std.is_record value
  && value.schema_id == SchemaId
  && value.schema_version == SchemaVersion
  && value.source_language == SourceLanguage
  && valid_ref value.profile_ref
  && valid_ref value.entropy.source_ref
  && valid_ref value.backend.backend_ref
  && std.array.elem value.algorithm ValidAlgorithms
  && std.array.elem value.entropy.source_kind ValidEntropySources
  && std.array.elem value.backend.class ValidBackends
  && value.purposes == RequiredPurposes
  && value.domain.id == "molten.crypto.signature"
  && value.domain.version == "v1"
  && value.rotation.generation_fenced
  && value.rotation.revocation_required
  && value.redaction.private_bytes
  && value.redaction.backend_locator
  && value.non_claims == RequiredNonClaims
  && (
    !value.production
    || (
      value.algorithm == ProductionAlgorithm
      && value.entropy.source_kind == "os-csprng"
      && value.backend.class != "deterministic-fixture"
      && value.backend.capability_rooted
      && value.backend.owner_only_permissions
      && value.purpose_separation
      && !value.fixture_only
    )
  )
  && (
    value.production
    || (
      value.algorithm == FixtureAlgorithm
      && value.entropy.source_kind == "deterministic-fixture"
      && value.backend.class == "deterministic-fixture"
      && value.fixture_only
    )
  ) in
let ExportContract = std.contract.from_predicate valid_profile in
{
  schema_id = SchemaId,
  schema_version = SchemaVersion,
  source_language = SourceLanguage,
  production_algorithm = ProductionAlgorithm,
  fixture_algorithm = FixtureAlgorithm,
  purposes = RequiredPurposes,
  non_claims = RequiredNonClaims,
  Export = ExportContract,
}
