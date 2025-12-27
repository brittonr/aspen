#!/usr/bin/env bash
# DNS Layer Demo Script
# Run this to see the DNS integration test with colorful output

set -euo pipefail

cd "$(dirname "$0")/.."

echo "==================================="
echo "  Aspen DNS Layer Demo"
echo "==================================="
echo ""
echo "This will run the DNS integration test showing:"
echo "  - A, AAAA, MX, TXT, SRV record creation"
echo "  - Wildcard DNS resolution"
echo "  - Zone management"
echo "  - Record scanning and deletion"
echo ""
echo "Press Enter to start..."
read -r

# Run the DNS integration test with colorful output
nix develop -c cargo nextest run \
    --run-ignored ignored-only \
    -E 'test(dns_crud_single_node)' \
    --no-capture

echo ""
echo "Demo complete!"
