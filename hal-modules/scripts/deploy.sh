#!/bin/bash
set -e
cd "$(dirname "$0")/.."

SHELL_DATA_DIR="$HOME/.hiapphub"
LIB_DIR="$SHELL_DATA_DIR/lib"

echo "=== HAL Modules Deploy ==="
echo "Building release..."
cargo build --workspace --release 2>&1

echo ""
echo "Deploying to $LIB_DIR..."
mkdir -p "$LIB_DIR"

DEPLOYED=0
for lib in target/release/libhap_mod_*.dylib target/release/libhap_mod_*.so; do
    [ -f "$lib" ] || continue
    BASENAME=$(basename "$lib")
    # libhap_mod_encoding.dylib -> hap-mod-encoding.hal
    HAL_NAME=$(echo "$BASENAME" | sed 's/^lib//; s/\.dylib$/.hal/; s/\.so$/.hal/; s/_/-/g')
    cp "$lib" "$LIB_DIR/$HAL_NAME"
    codesign -f -s - "$LIB_DIR/$HAL_NAME" 2>/dev/null || true
    SIZE=$(du -h "$LIB_DIR/$HAL_NAME" | cut -f1)
    echo "  $HAL_NAME ($SIZE)"
    DEPLOYED=$((DEPLOYED + 1))
done

echo ""
echo "Deployed $DEPLOYED modules to $LIB_DIR"
echo ""
ls -lh "$LIB_DIR"/*.hal 2>/dev/null | awk '{printf "  %-40s %s\n", $NF, $5}'
TOTAL=$(du -sh "$LIB_DIR" | cut -f1)
echo ""
echo "Total: $TOTAL"
