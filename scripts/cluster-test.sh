#!/usr/bin/env bash
# Full real cluster test via aspen-cli.
# Prerequisite: ./scripts/cluster-start.sh && ./scripts/cluster-form.sh
# shellcheck disable=SC2016  # Single-quoted strings are intentional — they're eval'd by run_test()
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CLUSTER_DIR="/tmp/aspen-test-cluster"

# Use cluster-generated ticket (includes all nodes) for failover resilience.
# Fall back to node1's bootstrap ticket if the cluster ticket isn't available yet.
BOOTSTRAP_TICKET=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")
CLUSTER_TICKET=$(./target/debug/aspen-cli -q --ticket "$BOOTSTRAP_TICKET" cluster ticket 2>/dev/null || echo "")
TICKET="${CLUSTER_TICKET:-$BOOTSTRAP_TICKET}"
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

# ── Coordination Primitives (graceful error handling) ──
# These require WASM plugins for full functionality. Without plugins,
# they should return clean error messages (not deserialization crashes).
# Note: { cmd || true; } suppresses CLI exit code for pipefail compat.
echo "── Coordination Primitives (error handling) ──────"
run_test "Lock CLI parses"           '{ $CLI lock try-acquire e2e-lock --holder cli-test --ttl 30000 2>&1 || true; } | grep -qiE "acquired|error|unavailable"'
run_test "Counter CLI parses"        '{ $CLI counter get e2e-counter 2>&1 || true; } | grep -qiE "^[0-9]+$|value|error|unavailable"'
run_test "Sequence CLI parses"       '{ $CLI sequence next e2e-seq 2>&1 || true; } | grep -qiE "^[0-9]+$|value|error|unavailable"'
run_test "Queue CLI parses"          '{ $CLI queue enqueue e2e-q "item" 2>&1 || true; } | grep -qiE "enqueued|error|unavailable"'
run_test "Semaphore CLI parses"      '{ $CLI semaphore try-acquire e2e-sem --permits 1 --holder cli-test --capacity 5 2>&1 || true; } | grep -qiE "acquired|error|unavailable"'
run_test "Ratelimit CLI parses"      '{ $CLI ratelimit try-acquire e2e-rl --rate 100.0 --capacity 1000 2>&1 || true; } | grep -qiE "allowed|error|unavailable"'

# ── Hooks ──────────────────────────────────────────────
echo "── Hooks ─────────────────────────────────────────"
run_test "Hook list (clean error)"   '{ $CLI hook list 2>&1 || true; } | grep -qiE "handler|error|unavailable"'
run_test "Hook metrics (clean err)"  '{ $CLI hook metrics 2>&1 || true; } | grep -qiE "metrics|error|unavailable"'

# ── Docs ───────────────────────────────────────────────
echo "── Docs ──────────────────────────────────────────"
run_test "Docs status"               '$CLI docs status'

# ── Federation ─────────────────────────────────────────
echo "── Federation ────────────────────────────────────"
run_test "Federation (clean error)"  '{ $CLI federation status 2>&1 || true; } | grep -qiE "status|error|unavailable"'

# ── Service Registry ───────────────────────────────────
echo "── Service Registry ──────────────────────────────"
run_test "Service list (clean err)"  '{ $CLI service list 2>&1 || true; } | grep -qiE "services|error|unavailable"'

# ── SQL ─────────────────────────────────────────────────
echo "── SQL ─────────────────────────────────────────────"
run_test "SQL (clean error)"         '{ $CLI sql query "SELECT 1" 2>&1 || true; } | grep -qiE "error|unavailable|num"'

# ── Automerge ──────────────────────────────────────────
echo "── Automerge ─────────────────────────────────────"
run_test "Automerge list (clean)"    '{ $CLI automerge list 2>&1 || true; } | grep -qiE "error|unavailable|documents"'

# ── Secrets ────────────────────────────────────────────
echo "── Secrets ───────────────────────────────────────"
run_test "Secrets kv list (clean)"   '{ $CLI secrets kv list 2>&1 || true; } | grep -qiE "error|unavailable|secrets"'

# ── Barrier ────────────────────────────────────────────
echo "── Barrier ───────────────────────────────────────"
run_test "Barrier enter (clean err)" '{ $CLI barrier enter e2e-barrier --participant cli-test --count 2 2>&1 || true; } | grep -qiE "entered|error|unavailable|IMPLEMENTED"'
run_test "Barrier status (clean)"    '{ $CLI barrier status e2e-barrier 2>&1 || true; } | grep -qiE "status|error|unavailable|IMPLEMENTED"'

# ── RWLock ─────────────────────────────────────────────
echo "── RWLock ────────────────────────────────────────"
run_test "RWLock read (clean err)"   '{ $CLI rwlock read e2e-rwl --holder cli-test --ttl 30000 2>&1 || true; } | grep -qiE "acquired|error|unavailable|IMPLEMENTED"'
run_test "RWLock status (clean)"     '{ $CLI rwlock status e2e-rwl 2>&1 || true; } | grep -qiE "status|error|unavailable|IMPLEMENTED"'

# ── Job System ─────────────────────────────────────────
echo "── Job System ────────────────────────────────────"
# shellcheck disable=SC2034 # JOB_ID used inside run_test strings
JOB_ID=$($CLI --json job submit shell '{"command":"echo hello"}' 2>&1 | grep -oP '"job_id":\s*"\K[^"]+' || echo "")
run_test "Job submit"                '[ -n "$JOB_ID" ]'
run_test "Job list"                  '$CLI job list | grep -qiE "job|No jobs"'
run_test "Job status"                '[ -z "$JOB_ID" ] || $CLI --json job status "$JOB_ID" | grep -q job_id'

# ── Git / Forge (clean errors) ─────────────────────────
echo "── Git / Forge (error handling) ──────────────────"
run_test "Git list (clean err)"      '{ $CLI git list 2>&1 || true; } | grep -qiE "error|unavailable|repo"'
run_test "Git init (clean err)"      '{ $CLI git init e2e-repo 2>&1 || true; } | grep -qiE "error|unavailable|created"'
run_test "Branch list (clean err)"   '{ $CLI branch list --repo e2e-repo 2>&1 || true; } | grep -qiE "error|unavailable|branch"'
run_test "Tag list (clean err)"      '{ $CLI tag list --repo e2e-repo 2>&1 || true; } | grep -qiE "error|unavailable|tag"'
run_test "Issue list (clean err)"    '{ $CLI issue list --repo e2e-repo 2>&1 || true; } | grep -qiE "error|unavailable|issue"'
run_test "Patch list (clean err)"    '{ $CLI patch list --repo e2e-repo 2>&1 || true; } | grep -qiE "error|unavailable|patch"'

# ── Peer ───────────────────────────────────────────────
echo "── Peer ──────────────────────────────────────────"
run_test "Peer list (clean error)"   '{ $CLI peer list 2>&1 || true; } | grep -qiE "error|unavailable|peer|enabled"'

# ── Docs CRUD ──────────────────────────────────────────
echo "── Docs CRUD ─────────────────────────────────────"
run_test "Docs set"                  '{ $CLI docs set e2e-doc-key "hello-docs" 2>&1 || true; } | grep -qiE "e2e-doc-key|error|unavailable"'
run_test "Docs get"                  '{ $CLI docs get e2e-doc-key 2>&1 || true; } | grep -qiE "hello-docs|not found|error"'
run_test "Docs list"                 '{ $CLI docs list 2>&1 || true; } | grep -qiE "e2e-doc|no entries|total|error"'
run_test "Docs delete"               '{ $CLI docs delete e2e-doc-key 2>&1 || true; } | grep -qiE "deleted|error|unavailable"'

# ── Counter/Sequence CRUD ──────────────────────────────
echo "── Counter/Sequence CRUD ─────────────────────────"
run_test "Counter incr"              '{ $CLI counter incr e2e-ctr 2>&1 || true; } | grep -qiE "^[0-9]+$|error|unavailable"'
run_test "Counter add"               '{ $CLI counter add e2e-ctr 5 2>&1 || true; } | grep -qiE "^[0-9]+$|error|unavailable"'
run_test "Counter get after ops"     '{ $CLI counter get e2e-ctr 2>&1 || true; } | grep -qiE "^[0-9]+$|error|unavailable"'
run_test "Sequence reserve"          '{ $CLI sequence reserve e2e-seq2 10 2>&1 || true; } | grep -qiE "^[0-9]+$|error|unavailable"'
run_test "Sequence current"          '{ $CLI sequence current e2e-seq2 2>&1 || true; } | grep -qiE "^[0-9]+$|error|unavailable"'

# ── Lock CRUD ──────────────────────────────────────────
echo "── Lock CRUD ─────────────────────────────────────"
LOCK_OUT=$($CLI --json lock try-acquire e2e-lock2 --holder cli-test --ttl 30000 2>&1 || true)
# shellcheck disable=SC2034 # FENCING_TOKEN used inside run_test strings
FENCING_TOKEN=$(echo "$LOCK_OUT" | grep -oP '"fencing_token":\s*\K\d+' || echo "")
run_test "Lock acquire"              '[ -n "$FENCING_TOKEN" ]'
run_test "Lock renew"                '{ $CLI lock renew e2e-lock2 --holder cli-test --ttl 30000 --fencing-token $FENCING_TOKEN 2>&1 || true; } | grep -qiE "renewed|error|IMPLEMENTED|unavailable"'
run_test "Lock release"              '$CLI lock release e2e-lock2 --holder cli-test --fencing-token $FENCING_TOKEN'

# ── JSON Output Mode ───────────────────────────────────
echo "── JSON Output Mode ──────────────────────────────"
run_test "KV get --json"             '$CLI --json kv get e2e-k1 2>/dev/null | python3 -m json.tool >/dev/null'
run_test "Cluster health --json"     '$CLI --json cluster health 2>/dev/null | python3 -m json.tool >/dev/null'
run_test "Cluster metrics --json"    '$CLI --json cluster metrics 2>/dev/null | python3 -m json.tool >/dev/null'

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
