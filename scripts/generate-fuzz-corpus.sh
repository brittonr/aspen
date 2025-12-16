#!/usr/bin/env bash
# Generate seed corpus for fuzz targets
# Usage: ./scripts/generate-fuzz-corpus.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "Generating fuzz corpus seeds..."
echo ""

# Run the corpus generator binary
if command -v nix &> /dev/null; then
    nix develop .#fuzz --command cargo run --bin generate_fuzz_corpus --features fuzzing
else
    cargo run --bin generate_fuzz_corpus --features fuzzing
fi

echo ""
echo "Corpus generation complete!"
echo ""
echo "Corpus directories:"
find fuzz/corpus -mindepth 1 -maxdepth 1 -type d | while read -r dir; do
    count=$(find "$dir" -type f | wc -l)
    printf "  %-30s %d files\n" "$(basename "$dir")" "$count"
done
