#!/usr/bin/env sh
# Verification script for storage formal specifications
#
# This script runs tests and optional Verus verification on the storage specifications.
# Uses Nix-packaged Verus or falls back to system Verus.
#
# Usage:
#   ./scripts/verify-storage.sh           # Run all tests (default)
#   ./scripts/verify-storage.sh --check   # Just check syntax (no proofs)
#   ./scripts/verify-storage.sh test      # Run spec unit tests only
#
# Note: The spec files in crates/aspen-raft/src/spec/ are hybrid files that:
# 1. Have Verus specifications documented in comments (for reference)
# 2. Have runtime stubs for testing within the crate
# 3. Are designed to work as Rust modules, not standalone Verus files
#
# For standalone Verus verification, specs would need to be extracted
# into self-contained .rs files without crate dependencies.

set -eu

cd "$(git rev-parse --show-toplevel)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Verus command - set during check_verus
VERUS_CMD=""

# Check if Verus is available
check_verus() {
    # First, try the verus command in PATH (e.g., from nix develop)
    if command -v verus >/dev/null 2>&1; then
        VERUS_CMD="verus"
        return 0
    fi

    # Try nix run if we're in a flake project
    if [ -f "flake.nix" ] && command -v nix >/dev/null 2>&1; then
        # Verify nix run .#verus works
        if nix run .#verus -- --version >/dev/null 2>&1; then
            VERUS_CMD="nix run .#verus --"
            printf "${GREEN}Verus available via: nix run .#verus${NC}\n"
            return 0
        fi
    fi

    printf "${YELLOW}Note: Verus not found in PATH${NC}\n"
    echo "  Use: nix run .#verus -- --version"
    echo "  Or:  nix develop (includes verus in PATH)"
    return 1
}

# Show Verus version
show_verus_version() {
    if [ -n "$VERUS_CMD" ]; then
        printf "${BLUE}Verus version:${NC}\n"
        $VERUS_CMD --version 2>&1 | grep -E "Version|Platform|Toolchain" | sed 's/^/  /'
        echo ""
    fi
}

# Run unit tests for spec modules
run_spec_tests() {
    printf "${YELLOW}=== Running spec unit tests ===${NC}\n"
    if cargo nextest run -p aspen-raft --filter-expr 'test(/spec::/)'; then
        printf "${GREEN}[PASS]${NC} Unit tests\n"
    else
        printf "${RED}[FAIL]${NC} Unit tests\n"
        return 1
    fi
}

# Main
main() {
    mode="${1:-all}"

    echo "Storage Verification Suite"
    echo "=========================="
    echo ""

    case "$mode" in
        --check|check)
            echo "Syntax check mode (no proofs)"
            cargo check -p aspen-raft
            ;;

        test|tests)
            run_spec_tests
            ;;

        verus-check)
            # Check Verus availability and show version
            if check_verus; then
                show_verus_version
                printf "${GREEN}Verus is ready for verification${NC}\n"
                echo ""
                echo "To verify a standalone Verus file:"
                echo "  $VERUS_CMD --crate-type=lib path/to/spec.rs"
            else
                exit 1
            fi
            ;;

        verus|verify)
            printf "${YELLOW}=== Running Verus formal verification ===${NC}\n"
            if command -v nix >/dev/null 2>&1; then
                if nix run .#verify-verus -- all; then
                    printf "${GREEN}[PASS]${NC} Verus verification\n"
                else
                    printf "${RED}[FAIL]${NC} Verus verification\n"
                    exit 1
                fi
            else
                printf "${YELLOW}[SKIP]${NC} Nix not available\n"
            fi
            ;;

        all|"")
            echo "Verifying all specifications..."
            echo ""

            # Check Verus availability (informational)
            check_verus || true
            if [ -n "$VERUS_CMD" ]; then
                show_verus_version
            fi

            # First, ensure it compiles
            printf "${YELLOW}=== Checking Rust compilation ===${NC}\n"
            if cargo check -p aspen-raft; then
                printf "${GREEN}[PASS]${NC} Compilation\n"
            else
                printf "${RED}[FAIL]${NC} Compilation\n"
                exit 1
            fi
            echo ""

            # Run unit tests (this tests the spec module invariants)
            run_spec_tests
            echo ""

            printf "${GREEN}=== All verifications passed ===${NC}\n"
            echo ""

            # Note about Verus verification
            if [ -n "$VERUS_CMD" ]; then
                printf "${BLUE}Note:${NC} The spec files contain Verus specifications in comments.\n"
                echo "To verify standalone Verus specs, use:"
                echo "  $VERUS_CMD --crate-type=lib path/to/standalone_spec.rs"
            fi
            ;;

        *)
            echo "Usage: $0 [all|check|test|verus-check|verus]"
            echo ""
            echo "Commands:"
            echo "  all          Run compilation check and unit tests (default)"
            echo "  check        Syntax check only (cargo check)"
            echo "  test         Run spec unit tests only"
            echo "  verus-check  Verify Verus is available and show version"
            echo "  verus        Run Verus formal verification on all specs"
            exit 1
            ;;
    esac
}

main "$@"
