#!/usr/bin/env bash
# Run mutation testing on specified crates.
#
# Usage:
#   ./scripts/run-mutants.sh                          # Default: aspen-coordination verified/
#   ./scripts/run-mutants.sh aspen-core               # Specific crate
#   ./scripts/run-mutants.sh aspen-coordination --all  # All mutants (slow!)
#
# Results written to mutants.out/
# Surviving mutants indicate test gaps.

set -euo pipefail

CRATE="${1:-aspen-coordination}"
MODE="${2:-verified}"

echo "=== Mutation Testing: $CRATE (mode: $MODE) ==="

if [ "$MODE" = "--all" ]; then
    cargo mutants -p "$CRATE" --in-place --timeout 120 -- --lib
elif [ "$MODE" = "verified" ]; then
    FILE_FILTER="-f crates/$CRATE/src/verified/"
    # shellcheck disable=SC2086
    cargo mutants -p "$CRATE" --in-place --timeout 120 $FILE_FILTER -- --lib
else
    cargo mutants -p "$CRATE" --in-place --timeout 120 -f "$MODE" -- --lib
fi

echo ""
echo "=== Results ==="
echo "Caught:   $(wc -l < mutants.out/caught.txt)"
echo "Missed:   $(wc -l < mutants.out/missed.txt)"
echo "Timeout:  $(wc -l < mutants.out/timeout.txt)"
echo "Unviable: $(wc -l < mutants.out/unviable.txt)"

TOTAL=$(( $(wc -l < mutants.out/caught.txt) + $(wc -l < mutants.out/missed.txt) ))
if [ "$TOTAL" -gt 0 ]; then
    CAUGHT=$(wc -l < mutants.out/caught.txt)
    SCORE=$(( CAUGHT * 100 / TOTAL ))
    echo "Score:    ${SCORE}%"
fi

if [ -s mutants.out/missed.txt ]; then
    echo ""
    echo "=== Surviving Mutants (test gaps) ==="
    head -20 mutants.out/missed.txt
    MISSED=$(wc -l < mutants.out/missed.txt)
    if [ "$MISSED" -gt 20 ]; then
        echo "... and $((MISSED - 20)) more (see mutants.out/missed.txt)"
    fi
fi
