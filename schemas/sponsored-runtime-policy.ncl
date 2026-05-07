# Aspen sponsored runtime policy contracts.
#
# Nickel owns human-authored sponsorship policy/config shape only. Rust keeps
# runtime authorization, quota accounting, metering, and distributed state
# transitions.

let contains = fun values value =>
  std.array.fold_left (fun found item => found || item == value) false values in

let non_empty = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_string value && std.string.length value > 0 then
      'Ok
    else
      'Error { message = "%{field} must be a non-empty string" }
  ) in

let enum_string = fun field allowed =>
  std.contract.from_validator (fun value =>
    if std.is_string value && contains allowed value then
      'Ok
    else
      'Error { message = "%{field} must be one of %{std.serialize 'Json allowed}" }
  ) in

let positive_number = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_number value && value > 0 then
      'Ok
    else
      'Error { message = "%{field} must be a positive number" }
  ) in

let non_negative_number = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_number value && value >= 0 then
      'Ok
    else
      'Error { message = "%{field} must be a non-negative number" }
  ) in

let positive_integer = fun field =>
  std.contract.from_validator (fun value =>
    if std.is_number value && value >= 1 then
      'Ok
    else
      'Error { message = "%{field} must be an integer-like positive number" }
  ) in

let secret_free_ref = fun field =>
  std.contract.from_validator (fun value =>
    let lowered = if std.is_string value then std.string.lowercase value else "" in
    let forbidden = ["password=", "secret=", "token=", "bearer ", "private_key", "-----begin"] in
    let has_forbidden = std.array.fold_left (fun found marker => found || std.string.contains marker lowered) false forbidden in
    if std.is_string value && std.string.length value > 0 && !has_forbidden then
      'Ok
    else
      'Error { message = "%{field} must be an opaque reference and must not contain raw secret material" }
  ) in

let PrincipalKind = enum_string "principal.kind" [
  "sponsor",
  "beneficiary",
  "provider",
  "workload",
  "service",
  "node",
  "plugin",
] in

let ResourceClass = enum_string "resource.class" [
  "cpu",
  "memory",
  "disk",
  "network",
  "gpu",
] in

let SettlementKind = enum_string "settlement.kind" [
  "invoice",
  "transaction",
  "purchase-order",
  "voucher",
  "offline-contract",
  "private-reference",
] in

let IsolationClass = enum_string "isolation.class" [
  "native-built-in",
  "wasm",
  "hyperlight",
  "oci",
  "microvm",
] in

let PrincipalRef = {
  id | non_empty "principal.id",
  kind | PrincipalKind,
} in

let SponsorPrincipalRef = {
  id | non_empty "sponsor.id",
  kind | enum_string "sponsor.kind" ["sponsor"],
} in
let BeneficiaryPrincipalRef = {
  id | non_empty "beneficiary.id",
  kind | enum_string "beneficiary.kind" ["beneficiary"],
} in
let ProviderPrincipalRef = {
  id | non_empty "provider.id",
  kind | enum_string "provider.kind" ["provider"],
} in
let WorkloadPrincipalRef = {
  id | non_empty "workload.id",
  kind | enum_string "workload.kind" ["workload"],
} in
let ServicePrincipalRef = {
  id | non_empty "service.id",
  kind | enum_string "service.kind" ["service"],
} in

let ResourceLimit = {
  class | ResourceClass,
  limit | positive_number "resource.limit",
  unit | non_empty "resource.unit",
} in

let SettlementRef = {
  kind | SettlementKind,
  reference | secret_free_ref "settlement.reference",
} in

let ResourceClassCatalog = {
  id | non_empty "catalog.id",
  resources | Array ResourceLimit,
} in

let ProviderOffer = {
  id | non_empty "provider_offer.id",
  provider | ProviderPrincipalRef,
  catalog_ref | non_empty "provider_offer.catalog_ref",
  isolation_classes | Array IsolationClass,
  settlement_kinds | Array SettlementKind,
  max_concurrent | positive_integer "provider_offer.max_concurrent" | default = 1,
} in

let SponsorPolicy = {
  id | non_empty "sponsor_policy.id",
  sponsor | SponsorPrincipalRef,
  beneficiaries | Array BeneficiaryPrincipalRef,
  settlement_refs | Array SettlementRef | default = [],
  max_grant_seconds | positive_integer "sponsor_policy.max_grant_seconds",
  revocation_ref | secret_free_ref "sponsor_policy.revocation_ref",
} in

let AdmissionProfile = {
  id | non_empty "admission_profile.id",
  provider_offer_ref | non_empty "admission_profile.provider_offer_ref",
  sponsor_policy_ref | non_empty "admission_profile.sponsor_policy_ref",
  workload | WorkloadPrincipalRef,
  service | ServicePrincipalRef,
  requested | Array ResourceLimit,
  estimated_cost | non_negative_number "admission_profile.estimated_cost" | default = 0,
} in

let SponsorshipPolicyBundle = {
  schema_version | String | default = "1",
  catalogs | Array ResourceClassCatalog,
  provider_offers | Array ProviderOffer,
  sponsor_policies | Array SponsorPolicy,
  admission_profiles | Array AdmissionProfile,
} in

let PrincipalRefContract = PrincipalRef in
let SponsorPrincipalRefContract = SponsorPrincipalRef in
let BeneficiaryPrincipalRefContract = BeneficiaryPrincipalRef in
let ProviderPrincipalRefContract = ProviderPrincipalRef in
let WorkloadPrincipalRefContract = WorkloadPrincipalRef in
let ServicePrincipalRefContract = ServicePrincipalRef in
let ResourceLimitContract = ResourceLimit in
let SettlementRefContract = SettlementRef in
let ResourceClassCatalogContract = ResourceClassCatalog in
let ProviderOfferContract = ProviderOffer in
let SponsorPolicyContract = SponsorPolicy in
let AdmissionProfileContract = AdmissionProfile in
let SponsorshipPolicyBundleContract = SponsorshipPolicyBundle in

{
  PrincipalRef = PrincipalRefContract,
  SponsorPrincipalRef = SponsorPrincipalRefContract,
  BeneficiaryPrincipalRef = BeneficiaryPrincipalRefContract,
  ProviderPrincipalRef = ProviderPrincipalRefContract,
  WorkloadPrincipalRef = WorkloadPrincipalRefContract,
  ServicePrincipalRef = ServicePrincipalRefContract,
  ResourceLimit = ResourceLimitContract,
  SettlementRef = SettlementRefContract,
  ResourceClassCatalog = ResourceClassCatalogContract,
  ProviderOffer = ProviderOfferContract,
  SponsorPolicy = SponsorPolicyContract,
  AdmissionProfile = AdmissionProfileContract,
  SponsorshipPolicyBundle = SponsorshipPolicyBundleContract,

  example | SponsorshipPolicyBundle = {
    catalogs = [{
      id = "small-ci",
      resources = [
        { class = "cpu", limit = 4, unit = "vcpu" },
        { class = "memory", limit = 8192, unit = "MiB" },
      ],
    }],
    provider_offers = [{
      id = "provider-ci-small",
      provider = { id = "provider/node-a", kind = "provider" },
      catalog_ref = "small-ci",
      isolation_classes = ["wasm", "oci"],
      settlement_kinds = ["invoice", "voucher"],
      max_concurrent = 8,
    }],
    sponsor_policies = [{
      id = "sponsor-team-a",
      sponsor = { id = "org/team-a", kind = "sponsor" },
      beneficiaries = [{ id = "user/alice", kind = "beneficiary" }],
      settlement_refs = [{ kind = "invoice", reference = "invoice://2026-05/team-a" }],
      max_grant_seconds = 3600,
      revocation_ref = "raft://sponsorship/revocations/team-a",
    }],
    admission_profiles = [{
      id = "ci-build-small",
      provider_offer_ref = "provider-ci-small",
      sponsor_policy_ref = "sponsor-team-a",
      workload = { id = "workload/ci/build", kind = "workload" },
      service = { id = "service/forge-ci", kind = "service" },
      requested = [{ class = "cpu", limit = 2, unit = "vcpu" }],
      estimated_cost = 10,
    }],
  },
}
