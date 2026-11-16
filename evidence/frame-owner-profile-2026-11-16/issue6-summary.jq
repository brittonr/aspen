def stats:
  sort as $s | {n: length, min: $s[0], median_lower: $s[((length - 1) / 2 | floor)],
    p95_nearest_rank: $s[((length * 0.95 | ceil) - 1)], max: $s[-1], mean: (add / length)};
if length != 46 then error("expected 46 complete real-Delivered rows") else . end
| if (map(select(.phase == "warmup")) | length) != 6 then error("expected six warmups") else . end
| if all(.[]; .checks.real_client_delivered and .checks.real_server_delivered
    and .checks.request_bound_ack_matches and .checks.zero_inflight_bytes
    and .checks.actual_ack_receipt_matches_canonical and .checks.actual_state_matches_canonical
    and .checks.clean_listener_cleanup and .checks.automatic_retry_count == 0
    and .actual_ack_transition_ref == .canonical_ack_transition_ref
    and .client_ack_ref == .payload_ref and .server_ack_ref == .payload_ref
    and .production_representative_runtime_reuse == false)
  then . else error("receipt/ACK/canonical evidence incomplete") end
| map(select(.phase == "measured"))
| if length != 40 then error("expected forty measured rows") else . end
| group_by(.payload_bytes)
| map(if length != 20 then error("expected twenty measured rows per size") else . end
  | . as $rows
  | {payload_bytes: .[0].payload_bytes, size_class: .[0].size_class, n: length,
      register_hash_ns: (map(.stages_ns.register_hash_ns) | stats),
      full_owner_held_ns: (map(.owner_held_ns) | stats),
      whole_operation_ns: (map(.whole_operation_ns) | stats),
      register_hash_owner_fraction_aggregate:
        ((map(.stages_ns.register_hash_ns) | add) / (map(.owner_held_ns) | add)),
      stage_distributions_ns: (reduce ($rows[0].stages_ns | keys[]) as $key
        ({}; .[$key] = ($rows | map(.stages_ns[$key]) | stats)))})
| {schema: "aspen.issue6.summary.v1", sizes: .,
    exceeds_ten_percent_at_both_sizes_diagnostic_only:
      (length == 2 and all(.[]; .register_hash_owner_fraction_aggregate > 0.10)),
    adoption_allowed: false,
    decision: "gate-closed-missing-production-representative-runtime-reuse-baseline",
    scope: "optimized-test-fixture native loopback; not Internet/hardware/release qualification",
    denominator: "actual external register_call_ns + route_call_ns; not stage sum or runtime-subtracted estimate"}
