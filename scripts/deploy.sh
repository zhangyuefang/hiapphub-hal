#!/bin/bash
set -e
cd "$(dirname "$0")/.."

# Detect OS and set paths accordingly
case "$(uname -s)" in
    Darwin)
        SHELL_DATA_DIR="$HOME/.hiapphub"
        LIB_EXT="dylib"
        SIGN_CMD="codesign -f -s -"
        ;;
    Linux)
        SHELL_DATA_DIR="$HOME/.hiapphub"
        LIB_EXT="so"
        SIGN_CMD=""
        ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
        SHELL_DATA_DIR="$APPDATA/hiapphub"
        LIB_EXT="dll"
        SIGN_CMD=""
        ;;
    *)
        echo "Unsupported OS: $(uname -s)"
        exit 1
        ;;
esac

LIB_DIR="$SHELL_DATA_DIR/lib"

echo "=== HAL Modules Deploy ==="
echo "OS: $(uname -s), Lib dir: $LIB_DIR"
echo "Building release..."
cargo build --workspace --release 2>&1

echo ""
echo "Deploying to $LIB_DIR..."
mkdir -p "$LIB_DIR"

# Clean up any legacy underscore-named .hal files to prevent duplicates
for old in "$LIB_DIR"/hap_mod_*.hal; do
    [ -f "$old" ] && rm -f "$old" && echo "  Removed legacy: $(basename "$old")"
done

DEPLOYED=0
for lib in target/release/libhap_mod_*.$LIB_EXT target/release/hap_mod_*.$LIB_EXT; do
    [ -f "$lib" ] || continue
    BASENAME=$(basename "$lib")
    # libhap_mod_encoding.dylib -> hap-mod-encoding.hal
    # hap_mod_encoding.dll -> hap-mod-encoding.hal (Windows no lib prefix)
    HAL_NAME=$(echo "$BASENAME" | sed "s/^lib//; s/\.$LIB_EXT$/.hal/; s/_/-/g")
    cp "$lib" "$LIB_DIR/$HAL_NAME"
    if [ -n "$SIGN_CMD" ]; then
        $SIGN_CMD "$LIB_DIR/$HAL_NAME" 2>/dev/null || true
    fi
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
