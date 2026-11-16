# Authoring contracts for synthetic compute-capacity inputs, not runtime admission.
let Prelude = import "../../docs/nickel-contract-prelude.ncl" in
let Id = Prelude.contract (fun value => Prelude.is_stable_id value && std.string.length value <= 256) in
let Ref = Prelude.contracts.Blake3Ref in
let OptionalRef = Prelude.contract (fun value => value == null || Prelude.is_blake3_ref value) in
let QualificationRefs = std.contract.Sequence [Prelude.contract (Prelude.is_non_empty_bounded_array 128), Array Ref, Prelude.contracts.UniqueStringArray] in
let Positive = Prelude.contracts.PositiveInteger in
let NonNegative = Prelude.contracts.NonNegativeInteger in
let OptionalId = Prelude.contract (fun value => value == null || (Prelude.is_stable_id value && std.string.length value <= 256)) in
let OptionalQuantity = Prelude.contract (fun value => value == null || Prelude.is_non_negative_integer value) in
let Health = Prelude.contract (Prelude.is_one_of ["healthy", "degraded", "unavailable", "unknown"]) in
let Granularity = Prelude.contract (Prelude.is_one_of ["chip", "mmio-bundle", "tray", "mesh", "partition", "alias"]) in
let PoolKind = Prelude.contract (Prelude.is_one_of ["dedicated-memory-bytes", "unified-memory-bytes", "chips"]) in
let AllocationKind = Prelude.contract (Prelude.is_one_of ["execution", "serving-deployment"]) in
let LabelAuthority = Prelude.contract (Prelude.is_one_of ["advisory", "measured", "operator-declared", "authoritative"]) in
let BoundedIds = std.contract.Sequence [Prelude.contract (Prelude.is_bounded_array 128), Array Id, Prelude.contracts.UniqueStringArray] in
let Pool = std.contract.Sequence [{
  id | Id,
  kind | PoolKind,
  authority_id | Id,
  total | NonNegative,
  allocatable | NonNegative,
  observed_used | OptionalQuantity,
  observed_free | OptionalQuantity,
  is_observation_debited | Bool,
}, Prelude.contract (fun pool =>
  pool.allocatable <= pool.total
  && (pool.observed_used == null || pool.observed_used <= pool.total)
  && (pool.observed_free == null || pool.observed_free <= pool.total)
  && (pool.observed_used == null || pool.observed_free == null
      || pool.observed_used + pool.observed_free <= pool.total)
)] in
let View = {
  id | Id,
  device_id | Id,
  partition_id | OptionalId,
  granularity | Granularity,
  backing_pools | std.contract.Sequence [Prelude.contract (Prelude.is_non_empty_bounded_array 128), Array Id, Prelude.contracts.UniqueStringArray],
  is_grouping_enforced | Bool,
} in
let Compatibility = {
  backend | OptionalId,
  runtime | OptionalId,
  architecture | OptionalId,
  topology | OptionalId,
  qualification_refs | QualificationRefs,
} in
# r[impl molten.compute_capacity.offers]
# r[impl molten.compute_capacity.backing_pools]
let OfferContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.compute.offer.v1"),
  host_id | Id,
  host_incarnation | Id,
  device_incarnation | Id,
  provider_profile_ref | Ref,
  source_profile_ref | Ref,
  revision | Positive,
  topology_generation | Positive,
  observed_at | NonNegative,
  observation_charge_cursor | NonNegative,
  expires_at | Positive,
  health | Health,
  label_authority | LabelAuthority,
  compatibility | Compatibility,
  pools | std.contract.Sequence [Prelude.contract (Prelude.is_non_empty_bounded_array 128), Array Pool],
  views | std.contract.Sequence [Prelude.contract (Prelude.is_non_empty_bounded_array 128), Array View],
}, Prelude.contract (fun offer =>
  offer.expires_at > offer.observed_at
  && Prelude.non_empty_distinct_by (fun pool => pool.id) offer.pools
  && Prelude.non_empty_distinct_by (fun view => view.id) offer.views
  && std.array.all (fun view =>
    std.array.all (fun pool_id => std.array.any (fun pool => pool.id == pool_id) offer.pools) view.backing_pools
  ) offer.views
)] in
let Claim = { pool_id | Id, quantity | Positive } in
let RequiredCompatibility = {
  backend | OptionalId,
  runtime | OptionalId,
  architecture | OptionalId,
  topology | OptionalId,
} in
let Grant = {
  grant_ref | Ref,
  consumer_id | Id,
  is_valid | Bool,
  is_revoked | Bool,
  expires_at | Positive,
} in
# This shape describes a request. An admitted owner must recheck current facts and persist an intent.
# r[impl molten.compute_capacity.reservation_authority]
let ReservationContract = std.contract.Sequence [{
  host_id | Id,
  host_incarnation | Id,
  device_incarnation | Id,
  offer_revision | Positive,
  topology_generation | Positive,
  now | NonNegative,
  view_id | Id,
  granularity | Granularity,
  compatibility | RequiredCompatibility,
  minimum_label_authority | LabelAuthority,
  consumer_id | Id,
  operation_id | Id,
  attempt_id | Id,
  owner_generation | Positive,
  grant_ref | Ref,
  grant | Grant,
  claims | std.contract.Sequence [Prelude.contract (Prelude.is_non_empty_bounded_array 128), Array Claim],
  serving_allocation_id | OptionalId,
  kind | AllocationKind,
}, Prelude.contract (fun request =>
  Prelude.non_empty_distinct_by (fun claim => claim.pool_id) request.claims
  && request.grant.grant_ref == request.grant_ref
  && request.grant.consumer_id == request.consumer_id
  && request.grant.is_valid && !request.grant.is_revoked
  && request.grant.expires_at > request.now
  && (request.kind == "execution" || request.serving_allocation_id == null)
)] in
# Persisted reservation record, distinct from the candidate request above.
let AllocationContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.compute.reservation.v1"),
  id | Id,
  host_id | Id,
  host_incarnation | Id,
  device_incarnation | Id,
  offer_revision | Positive,
  topology_generation | Positive,
  charge_cursor | Positive,
  valid_until | Positive,
  authority_ids | std.contract.Sequence [Prelude.contract (Prelude.is_non_empty_bounded_array 128), Array Id],
  view_id | Id,
  granularity | Granularity,
  consumer_id | Id,
  operation_id | Id,
  attempt_id | Id,
  owner_generation | Positive,
  grant_ref | Ref,
  claims | std.contract.Sequence [Prelude.contract (Prelude.is_non_empty_bounded_array 128), Array Claim],
  serving_allocation_id | OptionalId,
  kind | AllocationKind,
  status | Prelude.contract (Prelude.is_one_of ["intent", "active", "quarantined", "released"]),
  enforcement | Prelude.contract (Prelude.is_one_of ["logical-fence-only", "backend-acknowledged", "backend-fenced"]),
  activation_backend_ref | OptionalRef,
}, Prelude.contract (fun allocation =>
  allocation.id == allocation.operation_id
  && Prelude.non_empty_distinct_by (fun claim => claim.pool_id) allocation.claims
  && (allocation.kind == "execution" || allocation.serving_allocation_id == null)
)] in
let Step = { request | ReservationContract, expected_allocation_id | Id } in
let has_pool = fun offer pool_id => std.array.any (fun pool => pool.id == pool_id) offer.pools in
let matching_view = fun offer request =>
  std.array.any (fun view =>
    view.id == request.view_id
    && view.granularity == request.granularity
    && view.is_grouping_enforced
    && std.array.all (fun pool_id =>
      std.array.any (fun claim => claim.pool_id == pool_id) request.claims
    ) view.backing_pools
    && std.array.all (fun claim => std.array.elem claim.pool_id view.backing_pools) request.claims
  ) offer.views in
let authority_rank = fun level =>
  if level == "authoritative" then 4
  else if level == "operator-declared" then 3
  else if level == "measured" then 2
  else 1 in
let matching_offer = fun offers request =>
  std.array.any (fun offer =>
    offer.host_id == request.host_id
    && offer.host_incarnation == request.host_incarnation
    && offer.device_incarnation == request.device_incarnation
    && offer.revision == request.offer_revision
    && offer.topology_generation == request.topology_generation
    && request.now >= offer.observed_at && request.now < offer.expires_at
    && offer.health == "healthy"
    && authority_rank offer.label_authority >= authority_rank request.minimum_label_authority
    && matching_view offer request
    && std.array.all (fun claim =>
      has_pool offer claim.pool_id
      && std.array.any (fun pool =>
        pool.id == claim.pool_id
        && (pool.observed_used != null || pool.observed_free != null)
        && claim.quantity <= pool.allocatable
        && (pool.observed_free == null || claim.quantity <= pool.observed_free)
        && (pool.observed_used == null || claim.quantity <= pool.total - pool.observed_used)
      ) offer.pools
    ) request.claims
    && (request.compatibility.backend == null || request.compatibility.backend == offer.compatibility.backend)
    && (request.compatibility.runtime == null || request.compatibility.runtime == offer.compatibility.runtime)
    && (request.compatibility.architecture == null || request.compatibility.architecture == offer.compatibility.architecture)
    && (request.compatibility.topology == null || request.compatibility.topology == offer.compatibility.topology)
  ) offers in
# r[impl molten.compute_capacity.fixtures]
let ConsumerContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.compute.synthetic-consumer.v1"),
  profile_id | Id,
  offers | std.contract.Sequence [Prelude.contract (Prelude.is_non_empty_bounded_array 128), Array OfferContract],
  steps | std.contract.Sequence [Prelude.contract (Prelude.is_non_empty_bounded_array 128), Array Step],
  synthetic_only | Prelude.contract (fun value => value == true),
}, Prelude.contract (fun consumer =>
  Prelude.non_empty_distinct_by (fun offer => offer.host_id) consumer.offers
  && Prelude.non_empty_distinct_by (fun step => step.expected_allocation_id) consumer.steps
  && std.array.all (fun step =>
    step.expected_allocation_id == step.request.operation_id
    && matching_offer consumer.offers step.request
    && (step.request.serving_allocation_id == null || std.array.any (fun serving =>
      serving.expected_allocation_id == step.request.serving_allocation_id
      && serving.request.kind == "serving-deployment"
      && serving.request.host_id != step.request.host_id
    ) consumer.steps)
  ) consumer.steps
)] in
let RequiredNonClaims = [
  "synthetic-offers-are-not-live-provider-qualification",
  "grouping-labels-do-not-prove-tt-topology-or-enforcement",
  "allocation-intents-are-not-backend-enforcement",
  "serving-references-do-not-double-charge-resident-pools",
  "offers-do-not-prove-model-readiness-or-inference-throughput",
  "release-requires-matching-owner-and-affirmative-backend-evidence",
] in
let AllocationReadback = std.contract.Sequence [{
  id | Id,
  consumer_id | Id,
  owner_generation | Positive,
  granularity | Granularity,
  status | Prelude.contract (Prelude.is_one_of ["intent", "active", "quarantined", "released"]),
  is_logically_fenced | Bool,
  enforcement | Prelude.contract (Prelude.is_one_of ["logical-fence-only", "backend-acknowledged", "backend-fenced"]),
}, Prelude.contract (fun allocation => allocation.is_logically_fenced == (allocation.status != "released"))] in
let ReadbackContract = std.contract.Sequence [{
  host_id | Id,
  revision | Positive,
  is_fresh | Bool,
  health | Health,
  qualification_refs | QualificationRefs,
  allocations | std.contract.Sequence [Prelude.contract (Prelude.is_bounded_array 128), Array AllocationReadback],
}, Prelude.contract (fun readback => Prelude.distinct_by (fun allocation => allocation.id) readback.allocations)] in
# r[impl molten.compute_capacity.readback]
let ProfileContract = std.contract.Sequence [{
  schema | Prelude.contract (Prelude.is_exact_string "molten.compute.capacity-profile.v1"),
  profile_id | Id,
  profile_version | Prelude.contract (Prelude.is_exact_integer 1),
  maximum_items | Prelude.contract (Prelude.is_exact_integer 128),
  maximum_text_bytes | Prelude.contract (Prelude.is_exact_integer 256),
  time_domain | Prelude.contract (Prelude.is_exact_string "logical"),
  observation_absence_means_zero | Prelude.contract (fun value => value == false),
  live_qualification | Prelude.contract (fun value => value == false),
  backend_enforcement | Prelude.contract (fun value => value == false),
  non_claims | Prelude.contract (fun values => values == RequiredNonClaims),
}, Prelude.contract (fun profile => profile.profile_id != "")] in
{
  Profile = ProfileContract,
  Offer = OfferContract,
  Reservation = ReservationContract,
  Allocation = AllocationContract,
  Consumer = ConsumerContract,
  Readback = ReadbackContract,
  required_non_claims = RequiredNonClaims,
}
