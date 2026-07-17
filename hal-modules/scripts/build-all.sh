#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "=== HAL Modules Build ==="
echo "Building all 30 modules..."

cargo build --workspace --release 2>&1

echo ""
echo "=== Build Results ==="
for lib in target/release/*.dylib target/release/*.so target/release/*.dll 2>/dev/null; do
    [ -f "$lib" ] && echo "  $(basename "$lib") ($(du -h "$lib" | cut -f1))"
done

echo ""
echo "Build complete!"
