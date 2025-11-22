#!/usr/bin/env bash

echo "=== Testing Distributed Load Balancing Across All Nodes ==="
echo ""

echo "Submitting 12 jobs slowly (1 per 3 seconds) to allow fair distribution..."
for i in {1..12}; do
  curl -X POST http://localhost:3020/new-job \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "url=distributed-job-${i}.com" \
    -s > /dev/null
  echo "  ✓ Submitted job $i"
  sleep 3
done

echo ""
echo "Waiting 10 more seconds for final jobs to complete..."
sleep 10

echo ""
echo "=== Distribution Results ==="
echo ""
echo "Jobs by completing node:"
curl -s http://localhost:3020/queue/list | \
  jq -r '.[] | select(.payload.url | startswith("distributed-job")) | .completed_by' | \
  sort | uniq -c

echo ""
echo "Node identities:"
echo "  Node1 (port 3020): $(curl -s http://localhost:3020/iroh/info | jq -r '.endpoint_id')"
echo "  Node2 (port 3021): $(curl -s http://localhost:3021/iroh/info | jq -r '.endpoint_id')"
echo "  Node3 (port 3022): $(curl -s http://localhost:3022/iroh/info | jq -r '.endpoint_id')"
