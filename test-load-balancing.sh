#!/usr/bin/env bash

echo "=== Testing Distributed Load Balancing ==="
echo ""

echo "Submitting 6 jobs to the cluster..."
for i in {1..6}; do
  curl -X POST http://localhost:3020/new-job \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "url=test-job-${i}.com" \
    -s > /dev/null
  echo "  ✓ Submitted job $i"
  sleep 0.5
done

echo ""
echo "Waiting 20 seconds for jobs to be processed..."
sleep 20

echo ""
echo "=== Job Distribution Results ==="
curl -s http://localhost:3020/queue/list | jq -r '.[] | "\(.payload.url): claimed_by=\(.claimed_by // "none") completed_by=\(.completed_by // "none")"' 2>/dev/null

echo ""
echo "=== Node Statistics ==="
echo "Checking how many jobs each node processed..."
curl -s http://localhost:3020/queue/list | jq -r '.[].completed_by' 2>/dev/null | sort | uniq -c
