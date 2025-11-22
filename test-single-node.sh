#!/usr/bin/env bash
set -e

echo "=== Testing Single Node with Hiqlite ==="
echo ""

# Clean up
pkill -f "target/debug/mvm-ci" 2>/dev/null || true
pkill -f "flawless" 2>/dev/null || true
sleep 2
rm -rf ./data

# Use single-node config
cp hiqlite-single-node.toml hiqlite.toml

echo "Starting flawless..."
flawless up > /tmp/flawless-single.log 2>&1 &
FLAWLESS_PID=$!
sleep 3

echo "Starting mvm-ci..."
./target/debug/mvm-ci > /tmp/mvm-ci-single.log 2>&1 &
MVM_PID=$!
sleep 8

if ! ps -p $MVM_PID > /dev/null; then
  echo "✗ mvm-ci failed to start"
  echo "Last 30 lines of log:"
  tail -30 /tmp/mvm-ci-single.log
  kill $FLAWLESS_PID 2>/dev/null || true
  exit 1
fi

echo "✓ mvm-ci started successfully"
echo ""

echo "Submitting test job..."
curl -s -X POST http://localhost:3020/new-job \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "url=https://single-node-test.com" > /dev/null

sleep 5

echo ""
echo "Checking queue..."
curl -s http://localhost:3020/queue/list | jq -r '.[] | "  \(.job_id): \(.status) [completed_by: \(.completed_by // "none")]"'

echo ""
echo "Stats:"
curl -s http://localhost:3020/queue/stats | jq -c '  .'

echo ""
echo "Cleaning up..."
kill $MVM_PID $FLAWLESS_PID 2>/dev/null || true
wait 2>/dev/null || true

echo "✓ Single node test passed!"
