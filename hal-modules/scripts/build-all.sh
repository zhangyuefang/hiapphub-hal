#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "=== HAL Modules Build ==="
echo "Building all modules (cdylib + rlib)..."

cargo build --workspace --release 2>&1

echo ""
echo "=== cdylib 产物 ==="
for lib in target/release/*.dylib target/release/*.so target/release/*.dll 2>/dev/null; do
    [ -f "$lib" ] && echo "  $(basename "$lib") ($(du -h "$lib" | cut -f1))"
done

echo ""
echo "=== rlib 产物 ==="
for lib in target/release/*.rlib 2>/dev/null; do
    [ -f "$lib" ] && echo "  $(basename "$lib") ($(du -h "$lib" | cut -f1))"
done

echo ""
echo "Build complete! (cdylib for Shell dynamic loading, rlib for App Host static linking)"
