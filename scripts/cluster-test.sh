#!/usr/bin/env bash
# Full real cluster test via aspen-cli.
# Prerequisite: ./scripts/cluster-start.sh && ./scripts/cluster-form.sh
# shellcheck disable=SC2016  # Single-quoted strings are intentional — they're eval'd by run_test()
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CLUSTER_DIR="/tmp/aspen-test-cluster"

TICKET=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")
TICKET2=$(cat "$CLUSTER_DIR/node2/cluster-ticket.txt")
TICKET3=$(cat "$CLUSTER_DIR/node3/cluster-ticket.txt")
CLI="./target/debug/aspen-cli -q --ticket $TICKET"

PASS=0; FAIL=0; TOTAL=0

run_test() {
  local name="$1"; shift; TOTAL=$((TOTAL + 1))
  # shellcheck disable=SC2294
  if OUTPUT=$(eval "$@" 2>&1); then
    echo "  ✅ [$TOTAL] $name"; PASS=$((PASS + 1))
  else
    echo "  ❌ [$TOTAL] $name"
    echo "       → $(echo "$OUTPUT" | head -2)"
    FAIL=$((FAIL + 1))
  fi
}

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  FULL REAL CLUSTER TEST — 3-NODE RAFT OVER IROH QUIC P2P  ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# ── Cluster Management ──────────────────────────────────
echo "── Cluster Management ──────────────────────────────"
run_test "Health check"              '$CLI cluster health | grep -q healthy'
run_test "3 voters"                  'test "$($CLI cluster status | grep -c Y)" -ge 3'
run_test "Leader elected"            '$CLI cluster metrics | grep -q Leader'
run_test "Cluster ticket"            '$CLI cluster ticket | grep -q aspen'
run_test "Prometheus metrics"        '$CLI cluster prometheus | grep -q raft'

# ── Key-Value Store ─────────────────────────────────────
echo "── Key-Value Store ─────────────────────────────────"
run_test "KV write"                  '$CLI kv set e2e-k1 "hello" | grep -q OK'
run_test "KV read"                   '$CLI kv get e2e-k1 | grep -q hello'
run_test "KV write 2nd"             '$CLI kv set e2e-k2 "second" | grep -q OK'
run_test "KV write 3rd"             '$CLI kv set e2e-k3 "third" | grep -q OK'
run_test "KV scan"                   'test "$($CLI kv scan e2e-k | grep -c e2e-k)" -ge 3'
run_test "KV batch write"           '$CLI kv batch-write e2e-bw-a=alpha e2e-bw-b=bravo e2e-bw-c=charlie | grep -qi "success\|3"'
run_test "KV batch read"            '$CLI kv batch-read e2e-bw-a e2e-bw-b e2e-bw-c | grep -q alpha'
run_test "KV CAS"                    '$CLI kv cas e2e-k1 --expected hello --new-value updated | grep -q OK'
run_test "KV read after CAS"        '$CLI kv get e2e-k1 | grep -q updated'
run_test "KV CAS conflict"          '! $CLI kv cas e2e-k1 --expected wrong --new-value fail 2>/dev/null'
run_test "KV delete"                 '$CLI kv delete e2e-k2 | grep -q Deleted'
run_test "KV read deleted"          '$CLI kv get e2e-k2 | grep -qi "not found\|false"'
run_test "KV CAD"                    '$CLI kv cad e2e-k3 --expected third | grep -q Deleted'

# ── Leases ──────────────────────────────────────────────
echo "── Leases ──────────────────────────────────────────"
# shellcheck disable=SC2034
LEASE_ID=$($CLI --json lease grant 120 2>&1 | grep -oP '"lease_id":\s*\K\d+')
run_test "Lease grant"               '[ -n "$LEASE_ID" ]'
run_test "Lease TTL"                 '$CLI lease ttl $LEASE_ID'
run_test "Lease keepalive"           '$CLI lease keepalive $LEASE_ID'
run_test "Lease list"                '$CLI lease list'
run_test "Lease revoke"              '$CLI lease revoke $LEASE_ID'

# ── Blob Storage ────────────────────────────────────────
echo "── Blob Storage ────────────────────────────────────"
# shellcheck disable=SC2034
BLOB_HASH=$($CLI --json blob add --data "e2e-blob-$(date +%s)" 2>&1 \
  | grep -oP '"hash":\s*"\K[^"]+')
run_test "Blob add"                  '[ -n "$BLOB_HASH" ]'
run_test "Blob has"                  '$CLI --json blob has $BLOB_HASH | grep -qi true'
run_test "Blob get"                  '$CLI blob get $BLOB_HASH | grep -q e2e-blob'
run_test "Blob list"                 '$CLI blob list'
run_test "Blob delete"               '$CLI blob delete $BLOB_HASH --force'

# ── Multi-Node Replication ──────────────────────────────
echo "── Multi-Node Replication ──────────────────────────"
$CLI kv set e2e-repl "replicated" >/dev/null 2>&1; sleep 1
run_test "Node 2 health"            "./target/debug/aspen-cli -q --ticket $TICKET2 cluster health | grep -q healthy"
run_test "Node 3 health"            "./target/debug/aspen-cli -q --ticket $TICKET3 cluster health | grep -q healthy"
run_test "Node 2 is follower"       "./target/debug/aspen-cli -q --ticket $TICKET2 cluster metrics | grep -q Follower"
run_test "Node 3 is follower"       "./target/debug/aspen-cli -q --ticket $TICKET3 cluster metrics | grep -q Follower"
run_test "KV read replicated"       '$CLI kv get e2e-repl | grep -q replicated'

# ── Verify Suite ────────────────────────────────────────
echo "── Verify Suite ────────────────────────────────────"
run_test "Verify KV"                 '$CLI verify kv | grep -q PASS'
run_test "Verify blob"               '$CLI verify blob | grep -qi "PASS\|verified"'
run_test "Verify all"                '$CLI verify all --continue-on-error | grep -qi passed'

# ── Snapshot & Maintenance ──────────────────────────────
echo "── Snapshot & Maintenance ──────────────────────────"
run_test "Raft snapshot"             '$CLI cluster snapshot'
run_test "Checkpoint WAL"            '$CLI cluster checkpoint-wal'

# ── Cleanup ─────────────────────────────────────────────
for k in e2e-k1 e2e-k2 e2e-k3 e2e-repl e2e-bw-a e2e-bw-b e2e-bw-c; do
  $CLI kv delete "$k" >/dev/null 2>&1 || true
done

# ── Summary ─────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════"
printf "  ✅ Passed: %d   ❌ Failed: %d   📊 Total: %d\n" $PASS $FAIL $TOTAL
if [ $FAIL -eq 0 ]; then
  echo "  🎉 ALL TESTS PASSED"
else
  echo "  ⚠️  $FAIL test(s) FAILED"
fi
echo "═══════════════════════════════════════════════════"
exit $FAIL
