#!/bin/sh
# verify-verus-sync.sh - Check for drift between production verified functions and Verus specs
#
# DEPRECATED: This shell script has been replaced by the aspen-verus-metrics Rust tool.
# Use `nix run .#verify-verus-sync` or `cargo run -p aspen-verus-metrics -- check` instead.
#
# The new tool provides:
# - AST-based parsing via syn for accurate function extraction
# - Configurable normalization with per-crate mapping files
# - Structured output (JSON/terminal) for CI integration
# - Coverage tracking and metrics
#
# This script is kept as a fallback for environments where the Rust tool isn't available.
#
# This script validates that:
# 1. Production functions in src/verified/*.rs match their Verus counterparts
# 2. Function signatures are consistent between both locations
# 3. Function BODIES are logically equivalent (normalizing syntax differences)
# 4. Key exec functions exported from verus/ have matching implementations
#
# Usage:
#   ./scripts/verify-verus-sync.sh [--verbose] [--crate <crate-name>] [--show-diff]
#
# Exit codes:
#   0 - All functions in sync
#   1 - Drift detected
#   2 - Script error

echo "WARNING: This shell script is deprecated. Use 'nix run .#verify-verus-sync' instead." >&2
echo "" >&2

set -e

VERBOSE=false
SHOW_DIFF=false
CRATE=""
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Parse arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --show-diff|-d)
            SHOW_DIFF=true
            shift
            ;;
        --crate|-c)
            CRATE="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: verify-verus-sync.sh [--verbose] [--crate <crate-name>] [--show-diff]"
            echo ""
            echo "Checks for drift between production verified functions and Verus specs."
            echo ""
            echo "Options:"
            echo "  --verbose, -v    Show detailed comparison output"
            echo "  --show-diff, -d  Show line-by-line differences when drift detected"
            echo "  --crate, -c      Check only the specified crate"
            echo "  --help, -h       Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 2
            ;;
    esac
done

log() {
    if [ "$VERBOSE" = true ]; then
        echo "$@"
    fi
}

# Crates with verified modules
VERIFIED_CRATES="aspen-coordination aspen-raft aspen-core aspen-transport aspen-cluster"

if [ -n "$CRATE" ]; then
    VERIFIED_CRATES="$CRATE"
fi

DRIFT_DETECTED=false

# Function to extract public function names from a Rust file
extract_pub_fns() {
    file="$1"
    # Match: pub fn name( or pub const fn name(
    grep -oE 'pub (const )?fn [a-zA-Z_][a-zA-Z0-9_]*' "$file" 2>/dev/null | \
        sed 's/pub const fn //' | \
        sed 's/pub fn //' | \
        sort -u
}


# ============================================================================
# Function Body Extraction and Comparison
# ============================================================================

# Extract function body from production Rust file
# Handles: pub fn name(...) -> Type { body }
# or: pub fn name(...) { body }
extract_prod_function_body() {
    file="$1"
    fn_name="$2"

    # Use awk to extract the function body
    # This handles multi-line functions and nested braces
    awk -v fn_name="$fn_name" '
    BEGIN { in_fn = 0; in_body = 0; brace_count = 0; found = 0; body = "" }

    # Match function start: pub fn name( or pub const fn name(
    /pub (const )?fn / && $0 ~ fn_name "\\(" {
        in_fn = 1
        found = 1
        # Check if opening brace is on this line
        if ($0 ~ /{/) {
            in_body = 1
            brace_count = 1
        }
        next
    }

    # Look for opening brace if not found yet
    in_fn == 1 && in_body == 0 {
        if ($0 ~ /{/) {
            in_body = 1
            brace_count = 1
        }
        next
    }

    in_fn == 1 && in_body == 1 {
        # Count braces
        line = $0
        open_braces = gsub(/{/, "{", line)
        line = $0
        close_braces = gsub(/}/, "}", line)

        new_brace_count = brace_count + open_braces - close_braces

        # Only include content while inside the function body
        # Skip the final closing brace
        if (new_brace_count >= 1) {
            body = body $0 "\n"
        } else if (brace_count >= 1 && close_braces > 0) {
            # Last line with closing brace - include content before the brace
            sub(/}[[:space:]]*$/, "", $0)
            if ($0 !~ /^[[:space:]]*$/) {
                body = body $0 "\n"
            }
        }

        brace_count = new_brace_count
        if (brace_count <= 0) {
            in_fn = 0
            in_body = 0
        }
    }

    END {
        if (found) {
            print body
        }
    }
    ' "$file"
}

# Extract function body from Verus file
# Handles: pub fn name(...) -> (result: Type) ensures ... { body }
extract_verus_function_body() {
    file="$1"
    fn_name="$2"

    # Use awk to extract the function body, skipping ensures/requires clauses
    awk -v fn_name="$fn_name" '
    BEGIN {
        in_fn = 0
        in_body = 0
        brace_count = 0
        found = 0
        body = ""
        skip_ensures = 0
    }

    # Match function start: pub fn name(
    # Exclude spec fn and proof fn
    /pub fn / && !/spec fn/ && !/proof fn/ && $0 ~ fn_name "\\(" {
        in_fn = 1
        found = 1
        skip_ensures = 1
        next
    }

    # Skip ensures/requires clauses until we hit the opening brace of body
    in_fn == 1 && skip_ensures == 1 {
        if ($0 ~ /^[[:space:]]*\{[[:space:]]*$/ || $0 ~ /\{[[:space:]]*$/) {
            # Found opening brace of function body
            skip_ensures = 0
            in_body = 1
            brace_count = 1
            next
        }
        next
    }

    # In function body
    in_fn == 1 && in_body == 1 {
        # Count braces
        line = $0
        open_braces = gsub(/{/, "{", line)
        line = $0
        close_braces = gsub(/}/, "}", line)

        # Append to body before updating brace count
        if (brace_count > 1 || (brace_count == 1 && close_braces == 0)) {
            body = body $0 "\n"
        } else if (brace_count == 1 && close_braces > 0) {
            # Last line - remove trailing }
            sub(/}[[:space:]]*$/, "", $0)
            if ($0 !~ /^[[:space:]]*$/) {
                body = body $0 "\n"
            }
        }

        brace_count = brace_count + open_braces - close_braces

        if (brace_count <= 0) {
            in_fn = 0
            in_body = 0
        }
    }

    END {
        if (found) {
            print body
        }
    }
    ' "$file"
}

# Normalize a function body for comparison
# - Remove comments
# - Normalize whitespace
# - Remove trailing semicolons from expressions
# - Normalize saturating_add/sub patterns
normalize_body() {
    body="$1"
    echo "$body" | \
        # Remove single-line comments
        sed 's|//.*||g' | \
        # Remove multi-line comments (simple case)
        sed 's|/\*.*\*/||g' | \
        # Remove all leading whitespace (indentation)
        sed 's/^[[:space:]]*//' | \
        # Normalize inline whitespace: collapse multiple spaces/tabs to single space
        tr -s ' \t' ' ' | \
        # Remove trailing whitespace
        sed 's/[[:space:]]*$//' | \
        # Remove empty lines
        grep -v '^$' | \
        # Remove standalone closing braces
        grep -v '^}$' | \
        # Remove type annotations like `as u64` (Verus sometimes adds these)
        sed 's/ as u64//g' | \
        sed 's/ as int//g' | \
        # Normalize match arm syntax
        sed 's/=>[[:space:]]*/=> /g' | \
        # Remove trailing commas in match arms
        sed 's/,[[:space:]]*$//' | \
        # Final trim
        cat
}

# Normalize code for semantic comparison
# This removes variable names and focuses on structure
normalize_for_semantic_comparison() {
    body="$1"
    echo "$body" | \
        # First apply basic normalization
        sed 's|//.*||g' | \
        sed 's|/\*.*\*/||g' | \
        sed 's/^[[:space:]]*//' | \
        tr -s ' \t' ' ' | \
        sed 's/[[:space:]]*$//' | \
        grep -v '^$' | \
        grep -v '^}$' | \
        # Remove type casts
        sed 's/ as u64//g' | \
        sed 's/ as int//g' | \
        # Normalize common patterns that differ between prod and verus
        # entry.fencing_token -> token (for Option<&LockEntry> vs Option<u64>)
        sed 's/entry\.fencing_token/token/g' | \
        sed 's/current_entry/current_token/g' | \
        # Normalize match binding names: Some(entry) -> Some(token)
        sed 's/Some(entry)/Some(token)/g' | \
        # Normalize match arm syntax
        sed 's/=>[[:space:]]*/=> /g' | \
        # Remove trailing commas
        sed 's/,[[:space:]]*$//' | \
        # Defensive .max(1) after saturating_add(1) is semantically same
        # since saturating_add(1) already returns >= 1 for any positive starting point
        sed 's/\.saturating_add(1)\.max(1)/.saturating_add(1)/g' | \
        cat
}

# Compare two function bodies and report differences
compare_function_bodies() {
    prod_body="$1"
    verus_body="$2"
    fn_name="$3"

    # Normalize both bodies
    norm_prod=$(normalize_body "$prod_body")
    norm_verus=$(normalize_body "$verus_body")

    # Compare normalized bodies
    if [ "$norm_prod" = "$norm_verus" ]; then
        return 0  # Match
    fi

    # Check for semantic equivalence with relaxed comparison
    # Remove all whitespace and compare
    compact_prod=$(echo "$norm_prod" | tr -d ' \t\n')
    compact_verus=$(echo "$norm_verus" | tr -d ' \t\n')

    if [ "$compact_prod" = "$compact_verus" ]; then
        return 0  # Match (whitespace difference only)
    fi

    # Try semantic comparison (normalize variable names)
    sem_prod=$(normalize_for_semantic_comparison "$prod_body")
    sem_verus=$(normalize_for_semantic_comparison "$verus_body")

    compact_sem_prod=$(echo "$sem_prod" | tr -d ' \t\n')
    compact_sem_verus=$(echo "$sem_verus" | tr -d ' \t\n')

    if [ "$compact_sem_prod" = "$compact_sem_verus" ]; then
        return 0  # Semantically equivalent
    fi

    # Bodies differ
    return 1
}

# Show detailed diff between two bodies
show_body_diff() {
    prod_body="$1"
    verus_body="$2"
    fn_name="$3"

    echo "    Differences in '$fn_name':"
    echo "    --- Production (src/verified/) ---"
    echo "$prod_body" | sed 's/^/    | /'
    echo "    --- Verus (verus/) ---"
    echo "$verus_body" | sed 's/^/    | /'
    echo ""
}

# ============================================================================
# Main Verification Logic
# ============================================================================

echo "=== Verus Sync Validation ==="
echo ""

for crate in $VERIFIED_CRATES; do
    crate_dir="$ROOT_DIR/crates/$crate"
    verified_dir="$crate_dir/src/verified"
    verus_dir="$crate_dir/verus"

    if [ ! -d "$verified_dir" ]; then
        log "Skipping $crate: no src/verified/ directory"
        continue
    fi

    if [ ! -d "$verus_dir" ]; then
        log "Skipping $crate: no verus/ directory"
        continue
    fi

    echo "Checking $crate..."
    CRATE_DRIFT=false

    # Get list of production verified files
    for verified_file in "$verified_dir"/*.rs; do
        [ -f "$verified_file" ] || continue
        basename=$(basename "$verified_file" .rs)

        # Skip mod.rs
        [ "$basename" = "mod" ] && continue

        log "  Checking $basename..."

        # Extract function names from production code
        prod_fns=$(extract_pub_fns "$verified_file")

        # Find corresponding verus spec files
        # Convention: lock.rs -> lock_state_spec.rs, lock_ops_spec.rs, etc.
        verus_files=""
        for pattern in "${basename}_spec.rs" "${basename}_state_spec.rs" "${basename}_ops_spec.rs"; do
            if [ -f "$verus_dir/$pattern" ]; then
                verus_files="$verus_files $verus_dir/$pattern"
            fi
        done

        # Also check for generic patterns
        for vf in "$verus_dir"/*_spec.rs "$verus_dir"/*.rs; do
            [ -f "$vf" ] || continue
            # Add if not already in list
            case "$verus_files" in
                *"$vf"*) ;;
                *) verus_files="$verus_files $vf" ;;
            esac
        done

        # Check each function
        for fn_name in $prod_fns; do
            # Skip common utility functions that may not have specs
            case "$fn_name" in
                new|default|from|into|clone|eq|hash|fmt)
                    continue
                    ;;
            esac

            # Find the function in verus files
            verus_file_with_fn=""
            for vf in $verus_files; do
                if grep -q "pub fn $fn_name" "$vf" 2>/dev/null; then
                    # Make sure it's an exec fn, not spec fn or proof fn
                    if grep -E "^\s*pub fn $fn_name\s*\(" "$vf" 2>/dev/null | grep -v 'spec fn' | grep -v 'proof fn' >/dev/null 2>&1; then
                        verus_file_with_fn="$vf"
                        break
                    fi
                fi
            done

            if [ -z "$verus_file_with_fn" ]; then
                # Not all functions need verus specs - this is informational
                log "    INFO: $fn_name has no Verus exec fn (may be intentional)"
                continue
            fi

            # Extract and compare function bodies
            prod_body=$(extract_prod_function_body "$verified_file" "$fn_name")
            verus_body=$(extract_verus_function_body "$verus_file_with_fn" "$fn_name")

            if [ -z "$prod_body" ]; then
                log "    WARN: Could not extract body for $fn_name from production"
                continue
            fi

            if [ -z "$verus_body" ]; then
                log "    WARN: Could not extract body for $fn_name from Verus"
                continue
            fi

            if compare_function_bodies "$prod_body" "$verus_body" "$fn_name"; then
                log "    OK: $fn_name bodies match"
            else
                echo "  DRIFT: Function '$fn_name' body differs between production and Verus"
                CRATE_DRIFT=true
                DRIFT_DETECTED=true

                if [ "$SHOW_DIFF" = true ]; then
                    show_body_diff "$prod_body" "$verus_body" "$fn_name"
                fi
            fi
        done
    done

    # Check for key exec functions in verus that should have production implementations
    CRITICAL_FNS=""

    case "$crate" in
        aspen-coordination)
            CRITICAL_FNS="is_lock_expired compute_lock_deadline remaining_ttl_ms compute_next_fencing_token"
            ;;
        aspen-raft)
            CRITICAL_FNS="compute_entry_hash verify_entry_hash"
            ;;
        aspen-core)
            CRITICAL_FNS=""
            ;;
    esac

    for fn_name in $CRITICAL_FNS; do
        # Check production implementation exists
        found_in_prod=false
        prod_file=""
        for verified_file in "$verified_dir"/*.rs; do
            [ -f "$verified_file" ] || continue
            if grep -q "pub fn $fn_name" "$verified_file" 2>/dev/null; then
                found_in_prod=true
                prod_file="$verified_file"
                break
            fi
        done

        if [ "$found_in_prod" = false ]; then
            echo "  DRIFT: Critical function '$fn_name' missing from src/verified/"
            CRATE_DRIFT=true
            DRIFT_DETECTED=true
            continue
        fi

        # Check verus spec exists
        found_in_verus=false
        verus_file=""
        for vf in "$verus_dir"/*.rs; do
            [ -f "$vf" ] || continue
            # Check for exec fn (not spec fn or proof fn)
            if grep -E "^\s*pub fn $fn_name\s*\(" "$vf" 2>/dev/null | grep -v 'spec fn' | grep -v 'proof fn' >/dev/null 2>&1; then
                found_in_verus=true
                verus_file="$vf"
                break
            fi
        done

        if [ "$found_in_verus" = false ]; then
            echo "  DRIFT: Critical function '$fn_name' missing from verus/"
            CRATE_DRIFT=true
            DRIFT_DETECTED=true
            continue
        fi

        # Compare bodies for critical functions
        if [ -n "$prod_file" ] && [ -n "$verus_file" ]; then
            prod_body=$(extract_prod_function_body "$prod_file" "$fn_name")
            verus_body=$(extract_verus_function_body "$verus_file" "$fn_name")

            if [ -n "$prod_body" ] && [ -n "$verus_body" ]; then
                if ! compare_function_bodies "$prod_body" "$verus_body" "$fn_name"; then
                    echo "  DRIFT: Critical function '$fn_name' body differs"
                    CRATE_DRIFT=true
                    DRIFT_DETECTED=true

                    if [ "$SHOW_DIFF" = true ]; then
                        show_body_diff "$prod_body" "$verus_body" "$fn_name"
                    fi
                else
                    log "    OK: Critical function $fn_name bodies match"
                fi
            fi
        fi
    done

    if [ "$CRATE_DRIFT" = false ]; then
        echo "  OK"
    fi
    echo ""
done

# Summary
echo "=== Summary ==="
if [ "$DRIFT_DETECTED" = true ]; then
    echo "DRIFT DETECTED - Production and Verus specs may be out of sync"
    echo ""
    echo "To fix drift:"
    echo "  1. Update src/verified/*.rs to match verus/*.rs logic"
    echo "  2. Or update verus/*.rs specs to match production code"
    echo "  3. Run 'nix run .#verify-verus' to verify specs are correct"
    echo ""
    echo "Use --show-diff to see detailed differences"
    exit 1
else
    echo "All verified functions appear to be in sync with Verus specs"
    exit 0
fi
