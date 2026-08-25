let Domain = {
  name | String,
  family | [| 'entity, 'canonical |],
  rust_alias | String,
  migrated_scopes | Array String,
}
in
let Profile = {
  schema | String,
  maximum_entity_ref_bytes | Number,
  canonical_algorithm | String,
  domains | Array Domain,
  wire_authority | [| 'preserves |],
  constructor_non_claims | Array String,
}
in
let entity_ref_byte_limit = 256 in
let profile | Profile = {
  schema = "molten.nominal-reference-domains.v1",
  maximum_entity_ref_bytes = entity_ref_byte_limit,
  canonical_algorithm = "blake3",
  wire_authority = 'preserves,
  domains = [
    { name = "principal", family = 'entity, rust_alias = "PrincipalRef", migrated_scopes = ["authority-holder"] },
    { name = "node", family = 'entity, rust_alias = "NodeRef", migrated_scopes = ["node-control"] },
    { name = "actor", family = 'entity, rust_alias = "ActorRef", migrated_scopes = ["handler-admission"] },
    { name = "service", family = 'entity, rust_alias = "ServiceRef", migrated_scopes = ["handler-admission"] },
    { name = "session", family = 'entity, rust_alias = "SessionRef", migrated_scopes = ["authority", "effect-admission"] },
    { name = "authority-context", family = 'entity, rust_alias = "AuthorityContextRef", migrated_scopes = ["authority"] },
    { name = "delegation", family = 'canonical, rust_alias = "DelegationRef", migrated_scopes = ["capability-proofset"] },
    { name = "revocation", family = 'canonical, rust_alias = "RevocationRef", migrated_scopes = ["capability-proofset"] },
    { name = "key", family = 'canonical, rust_alias = "KeyRef", migrated_scopes = ["authority"] },
    { name = "policy", family = 'canonical, rust_alias = "PolicyRef", migrated_scopes = ["authority", "retention"] },
    { name = "resource", family = 'canonical, rust_alias = "ResourceRef", migrated_scopes = ["effect-admission", "node-control"] },
    { name = "evidence", family = 'canonical, rust_alias = "EvidenceRef", migrated_scopes = ["authority", "provenance", "replay"] },
    { name = "artifact", family = 'canonical, rust_alias = "ArtifactRef", migrated_scopes = ["artifact-binding", "retention", "cache"] },
    { name = "operation", family = 'entity, rust_alias = "OperationRef", migrated_scopes = ["effect-admission", "artifact-binding"] },
    { name = "receipt", family = 'canonical, rust_alias = "ReceiptRef", migrated_scopes = ["provenance", "replay"] },
  ],
  constructor_non_claims = [
    "typed-reference-is-not-authority",
    "syntax-is-not-freshness",
    "digest-is-not-evidence-truth",
    "artifact-possession-is-not-execution-authority",
  ],
}
in
profile
