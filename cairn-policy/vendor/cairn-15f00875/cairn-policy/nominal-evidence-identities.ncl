let Contracts = import "contracts.ncl" in
let PolicyId = "cairn.nominal-evidence-identities.v1" in
let RuntimeSchema = "cairn.nominal-evidence-identity-runtime.v1" in
let OctetSchema = "octet-nominal-domain-policy/v1" in
let DomainIdMaxBytes = 128 in
let ActorIdMaxBytes = 256 in
let Blake3RawHexBytes = 64 in
let Blake3LongestPrefixBytes = 7 in
let Blake3DisplayMaxBytes = Blake3RawHexBytes + Blake3LongestPrefixBytes in
let GitOidSha1HexBytes = 40 in
let Sha256HexBytes = 64 in
let CoreScope = "crates/cairn-core/src/verified" in
let WireScope = "crates/cairn-core/src/verified" in
let NonClaim = "nominal identities prove configured local syntax and role separation only; they do not prove evidence truth, authority, policy correctness, verifier soundness, mutation safety outside checked plans, or release eligibility" in
let domain = fun id kind hash_algorithm prefix byte_limit symbols => {
  domain_id = id,
  category = kind,
  algorithm = hash_algorithm,
  required_prefix = prefix,
  max_bytes = byte_limit,
  rust_symbols = symbols,
  core_scopes = [CoreScope],
  wire_compatibility_scopes = [WireScope],
  raw_representation = "string",
  wrapper_kind = "marker-generic",
  constructor_policy = "marker-only",
} in
let RuntimeDomains = [
  domain "actor-id" "domain-ref" "none" "actor:" ActorIdMaxBytes ["ActorId"],
  domain "archive-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["ArchiveDigest"],
  domain "archive-id" "domain-id" "none" "" DomainIdMaxBytes ["ArchiveId"],
  domain "artifact-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["ArtifactDigest"],
  domain "binary-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["BinaryDigest"],
  domain "chain-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["ChainDigest"],
  domain "change-id" "domain-id" "none" "" DomainIdMaxBytes ["ChangeId"],
  domain "content-root-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["ContentRootDigest"],
  domain "evidence-kind" "domain-id" "none" "" DomainIdMaxBytes ["EvidenceKind"],
  domain "git-oid-sha1" "imported-digest" "Git-OID-SHA-1" "" GitOidSha1HexBytes ["GitOidSha1Digest"],
  domain "graph-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["GraphDigest"],
  domain "input-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["InputDigest"],
  domain "ledger-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["LedgerDigest"],
  domain "mantle-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["MantleDigest"],
  domain "operation-id" "domain-id" "none" "operation:" DomainIdMaxBytes ["OperationId"],
  domain "pack-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["PackDigest"],
  domain "plan-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["PlanDigest"],
  domain "policy-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["PolicyDigest"],
  domain "policy-kind" "domain-id" "none" "" DomainIdMaxBytes ["PolicyKind"],
  domain "projection-role" "domain-id" "none" "" DomainIdMaxBytes ["ProjectionRole"],
  domain "receipt-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["ReceiptDigest"],
  domain "sha256-digest" "imported-digest" "SHA-256" "" Sha256HexBytes ["Sha256Digest"],
  domain "sidecar-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["SidecarDigest"],
  domain "source-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["SourceDigest"],
  domain "state-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["StateDigest"],
  domain "valence-digest" "blake3-digest" "BLAKE3" "" Blake3DisplayMaxBytes ["ValenceDigest"],
  domain "workspace-id" "domain-id" "none" "workspace:" DomainIdMaxBytes ["WorkspaceId"],
] in
let octet_domain = fun entry => {
  domain_id = entry.domain_id,
  symbols = entry.rust_symbols,
  raw_representation = entry.raw_representation,
  core_scopes = entry.core_scopes,
  compatibility_scopes = entry.wire_compatibility_scopes,
  compatibility_required = true,
  wrapper_kind = entry.wrapper_kind,
  constructor_policy = entry.constructor_policy,
} in
{
  policy_id = PolicyId,
  runtime = {
    schema_version = RuntimeSchema,
    policy_id = PolicyId,
    domains = RuntimeDomains,
  },
  octet = {
    schema_version = OctetSchema,
    policy_id = PolicyId,
    default_mode = "deny",
    heuristic_suffixes = ["Id", "Digest", "Hash", "Root"],
    domains = std.array.map octet_domain RuntimeDomains,
    non_claims = [NonClaim],
  },
  non_claims = [NonClaim],
} | Contracts.NominalIdentityPolicy
