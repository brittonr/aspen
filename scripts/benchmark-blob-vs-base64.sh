#!/bin/bash
set -e

# Benchmark: Blob Storage vs Base64 Encoding Performance
#
# Compares bandwidth usage and submission time between:
# 1. Old way: Base64-encoded binary in JSON payload
# 2. New way: Blob storage with hash reference

echo "========================================"
echo "  Blob Storage vs Base64 Benchmark"
echo "========================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create test binary (52KB - typical size of echo-worker)
BINARY_FILE="/tmp/test-vm-binary-$$.bin"
echo "Creating test binary (52KB)..."
dd if=/dev/urandom of="$BINARY_FILE" bs=1024 count=52 2>/dev/null
BINARY_SIZE=$(ls -l "$BINARY_FILE" | awk '{print $5}')
echo -e "${GREEN}✓${NC} Created test binary: $BINARY_SIZE bytes"

# Function to measure time
measure_time() {
    local start=$(date +%s%N)
    "$@"
    local end=$(date +%s%N)
    echo $(( (end - start) / 1000000 )) # Convert to milliseconds
}

# Test 1: Base64 encoding overhead
echo ""
echo -e "${YELLOW}Test 1: Base64 Encoding Overhead${NC}"
echo "────────────────────────────────────"

# Measure base64 encoding
BASE64_DATA=$(base64 < "$BINARY_FILE")
BASE64_SIZE=${#BASE64_DATA}
ENCODING_OVERHEAD=$(( (BASE64_SIZE - BINARY_SIZE) * 100 / BINARY_SIZE ))

echo "Original size:  $BINARY_SIZE bytes"
echo "Base64 size:   $BASE64_SIZE bytes"
echo -e "Overhead:      ${RED}+${ENCODING_OVERHEAD}%${NC} ($(( BASE64_SIZE - BINARY_SIZE )) extra bytes)"

# Create JSON payloads for comparison
JSON_BASE64=$(cat <<EOF
{
  "type": "NativeBinary",
  "binary": "$BASE64_DATA"
}
EOF
)

JSON_BLOB=$(cat <<EOF
{
  "type": "BlobBinary",
  "hash": "3b99a8d846f3d553e9c7c8d23f5c6a8b9e7d1a2f4c5e8b3d6a9f2e1c7b4d8a5c",
  "size": $BINARY_SIZE,
  "format": "elf"
}
EOF
)

JSON_BASE64_SIZE=${#JSON_BASE64}
JSON_BLOB_SIZE=${#JSON_BLOB}

echo ""
echo -e "${YELLOW}Test 2: Payload Size Comparison${NC}"
echo "────────────────────────────────────"
echo "Base64 JSON payload:  $JSON_BASE64_SIZE bytes"
echo "Blob ref JSON payload: $JSON_BLOB_SIZE bytes"
echo -e "Reduction:            ${GREEN}-$(( (JSON_BASE64_SIZE - JSON_BLOB_SIZE) * 100 / JSON_BASE64_SIZE ))%${NC}"

# Test 3: Multiple job submissions
echo ""
echo -e "${YELLOW}Test 3: Bandwidth for 100 Jobs${NC}"
echo "────────────────────────────────────"

# Calculate bandwidth for 100 jobs
JOBS=100
OLD_BANDWIDTH=$(( JSON_BASE64_SIZE * JOBS ))
NEW_BANDWIDTH=$(( BINARY_SIZE + JSON_BLOB_SIZE * JOBS )) # Upload once + references

echo "Old method (base64 each time):"
echo "  Total bandwidth: $(( OLD_BANDWIDTH / 1024 )) KB"
echo ""
echo "New method (blob + references):"
echo "  Initial upload:  $(( BINARY_SIZE / 1024 )) KB"
echo "  Job payloads:    $(( (JSON_BLOB_SIZE * JOBS) / 1024 )) KB"
echo "  Total bandwidth: $(( NEW_BANDWIDTH / 1024 )) KB"
echo ""
echo -e "Bandwidth savings: ${GREEN}-$(( (OLD_BANDWIDTH - NEW_BANDWIDTH) * 100 / OLD_BANDWIDTH ))%${NC}"

# Test 4: Deduplication benefits
echo ""
echo -e "${YELLOW}Test 4: Deduplication Benefits${NC}"
echo "────────────────────────────────────"

UNIQUE_BINARIES=10
echo "Scenario: $UNIQUE_BINARIES unique binaries, 10 jobs each"
echo ""
echo "Old method:"
echo "  Bandwidth: $(( OLD_BANDWIDTH * UNIQUE_BINARIES / 1024 )) KB"
echo ""
echo "New method:"
echo "  Binary uploads:  $(( (BINARY_SIZE * UNIQUE_BINARIES) / 1024 )) KB"
echo "  Job references:  $(( (JSON_BLOB_SIZE * JOBS * UNIQUE_BINARIES) / 1024 )) KB"
echo "  Total:          $(( ((BINARY_SIZE * UNIQUE_BINARIES) + (JSON_BLOB_SIZE * JOBS * UNIQUE_BINARIES)) / 1024 )) KB"

OLD_TOTAL=$(( OLD_BANDWIDTH * UNIQUE_BINARIES ))
NEW_TOTAL=$(( (BINARY_SIZE * UNIQUE_BINARIES) + (JSON_BLOB_SIZE * JOBS * UNIQUE_BINARIES) ))
SAVINGS=$(( (OLD_TOTAL - NEW_TOTAL) * 100 / OLD_TOTAL ))
echo ""
echo -e "Bandwidth savings: ${GREEN}-${SAVINGS}%${NC}"

# Summary
echo ""
echo "========================================"
echo -e "${GREEN}Summary${NC}"
echo "========================================"
echo ""
echo "✓ Base64 encoding adds ${ENCODING_OVERHEAD}% overhead"
echo "✓ Blob references are $(( (JSON_BASE64_SIZE - JSON_BLOB_SIZE) * 100 / JSON_BASE64_SIZE ))% smaller than base64 payloads"
echo "✓ For 100 jobs: ${GREEN}$(( (OLD_BANDWIDTH - NEW_BANDWIDTH) * 100 / OLD_BANDWIDTH ))% bandwidth reduction${NC}"
echo "✓ With deduplication: ${GREEN}${SAVINGS}% total savings${NC}"
echo ""
echo "Additional benefits not measured:"
echo "• P2P distribution (parallel downloads)"
echo "• Content-addressed integrity verification"
echo "• Automatic garbage collection"
echo "• Binary caching across nodes"

# Cleanup
rm -f "$BINARY_FILE"

echo ""
echo "Benchmark complete!"
