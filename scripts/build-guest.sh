#!/usr/bin/env bash
#
# Build a guest binary for Hyperlight VM execution.
#
# Usage: ./scripts/build-guest.sh <worker-name>
# Example: ./scripts/build-guest.sh echo-worker

set -e

WORKER_NAME=$1

if [ -z "$WORKER_NAME" ]; then
    echo "Usage: $0 <worker-name>"
    echo "Available workers:"
    echo "  - echo-worker"
    echo "  - data-processor"
    exit 1
fi

WORKER_DIR="examples/vm-jobs/$WORKER_NAME"

if [ ! -d "$WORKER_DIR" ]; then
    echo "Error: Worker directory '$WORKER_DIR' not found"
    exit 1
fi

echo "Building guest binary: $WORKER_NAME"

# Create output directory
mkdir -p target/guest-binaries

# Build the guest binary
# Using musl target for static linking
cd "$WORKER_DIR"

echo "Building with cargo..."
cargo build --release --target x86_64-unknown-linux-musl 2>/dev/null || {
    echo "musl target not installed, trying with default target..."
    cargo build --release
}

# Find the built binary
if [ -f "target/x86_64-unknown-linux-musl/release/$WORKER_NAME" ]; then
    BINARY_PATH="target/x86_64-unknown-linux-musl/release/$WORKER_NAME"
elif [ -f "target/release/$WORKER_NAME" ]; then
    BINARY_PATH="target/release/$WORKER_NAME"
else
    echo "Error: Could not find built binary"
    exit 1
fi

# Copy to guest-binaries directory
cp "$BINARY_PATH" "../../../target/guest-binaries/$WORKER_NAME"

# Return to original directory
cd ../../..

echo "✓ Guest binary built: target/guest-binaries/$WORKER_NAME"

# Show binary info
ls -lh "target/guest-binaries/$WORKER_NAME"
file "target/guest-binaries/$WORKER_NAME" 2>/dev/null || true
