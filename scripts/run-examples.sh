#!/usr/bin/env sh
# Run all Aspen examples to verify functionality
set -e

echo "🚀 Running Aspen Examples"
echo "========================="

examples="basic_cluster kv_operations multi_node_cluster"

for example in $examples; do
    echo ""
    echo "📦 Running $example..."
    echo "---"

    # Run with timeout to prevent hanging
    if timeout 90 cargo run --example "$example" > /dev/null 2>&1; then
        echo "✅ $example completed successfully"
    else
        exit_code=$?
        echo "❌ $example failed with exit code $exit_code"
        exit 1
    fi
done

echo ""
echo "🎉 All examples completed successfully!"
echo ""
echo "To run examples individually:"
echo "  cargo run --example basic_cluster"
echo "  cargo run --example kv_operations"
echo "  cargo run --example multi_node_cluster"
