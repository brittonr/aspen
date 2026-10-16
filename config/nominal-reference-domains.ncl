# Review inventory only. Octet executes its provider-owned embedded generated catalog;
# this file is not an additional runtime policy input.
let CatalogRow = {
  domain_id | String,
  symbols | Array String,
  raw_representation | String,
  core_scopes | Array String,
  compatibility_scopes | Array String,
  compatibility_required | Bool,
  wrapper_kind | String,
  constructor_policy | String,
}
in
let Domain = {
  name | String,
  family | [| 'entity, 'canonical |],
  rust_alias | String,
  catalog | CatalogRow,
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
let row = fun name row_symbols => {
  domain_id = "molten.nominal-reference." ++ name,
  symbols = row_symbols,
  raw_representation = "string",
  core_scopes = ["crates/molten-core/src/nominal.rs"],
  compatibility_scopes = ["src/authority/nominal.rs"],
  compatibility_required = true,
  wrapper_kind = "marker-generic",
  constructor_policy = "private-fallible",
}
in
let profile | Profile = {
  schema = "molten.nominal-reference-domains.v2",
  maximum_entity_ref_bytes = 256,
  canonical_algorithm = "blake3",
  wire_authority = 'preserves,
  domains = [
    {
      name = "principal", family = 'entity, rust_alias = "PrincipalRef",
      catalog = row "principal" ["PrincipalRef", "AuthorityReferenceSet::holder"],
    },
    {
      name = "node", family = 'entity, rust_alias = "NodeRef",
      catalog = row "node" ["NodeRef", "ExecutionReferenceSet::node"],
    },
    {
      name = "actor", family = 'entity, rust_alias = "ActorRef",
      catalog = row "actor" ["ActorRef", "ExecutionReferenceSet::actor"],
    },
    {
      name = "service", family = 'entity, rust_alias = "ServiceRef",
      catalog = row "service" ["ServiceRef", "ExecutionReferenceSet::service"],
    },
    {
      name = "session", family = 'entity, rust_alias = "SessionRef",
      catalog = row "session" ["SessionRef", "AuthorityReferenceSet::session", "ExecutionReferenceSet::session"],
    },
    {
      name = "authority-context", family = 'entity, rust_alias = "AuthorityContextRef",
      catalog = row "authority-context" ["AuthorityContextRef", "AuthorityReferenceSet::context"],
    },
    {
      name = "delegation", family = 'canonical, rust_alias = "DelegationRef",
      catalog = row "delegation" ["DelegationRef", "AuthorityReferenceSet::delegation"],
    },
    {
      name = "revocation", family = 'canonical, rust_alias = "RevocationRef",
      catalog = row "revocation" ["RevocationRef", "AuthorityReferenceSet::revocation"],
    },
    {
      name = "key", family = 'canonical, rust_alias = "KeyRef",
      catalog = row "key" ["KeyRef", "AuthorityReferenceSet::key"],
    },
    {
      name = "policy", family = 'canonical, rust_alias = "PolicyRef",
      catalog = row "policy" ["PolicyRef", "AuthorityReferenceSet::policy"],
    },
    {
      name = "resource", family = 'canonical, rust_alias = "ResourceRef",
      catalog = row "resource" ["ResourceRef", "AuthorityReferenceSet::resource", "ExecutionReferenceSet::resource"],
    },
    {
      name = "evidence", family = 'canonical, rust_alias = "EvidenceRef",
      catalog = row "evidence" ["EvidenceRef", "AuthorityReferenceSet::evidence", "ArtifactReferenceSet::evidence", "HistoricalReferenceSet::evidence"],
    },
    {
      name = "artifact", family = 'canonical, rust_alias = "ArtifactRef",
      catalog = row "artifact" ["ArtifactRef", "ArtifactReferenceSet::artifact", "HistoricalReferenceSet::artifact"],
    },
    {
      name = "operation", family = 'entity, rust_alias = "OperationRef",
      catalog = row "operation" ["OperationRef", "ExecutionReferenceSet::operation", "ArtifactReferenceSet::operation"],
    },
    {
      name = "receipt", family = 'canonical, rust_alias = "ReceiptRef",
      catalog = row "receipt" ["ReceiptRef", "ArtifactReferenceSet::receipt", "HistoricalReferenceSet::receipt"],
    },
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
