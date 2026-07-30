#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "=== HAL Modules Test Suite ==="
echo "Testing all 30 modules..."
echo ""

TOTAL_PASS=0
TOTAL_FAIL=0
FAILED_MODULES=""

MODULES=(
    "hap-mod-encoding" "hap-mod-datetime" "hap-mod-log" "hap-mod-csv" "hap-mod-xml"
    "hap-mod-crypto" "hap-mod-system" "hap-mod-storage"
    "hap-mod-fs" "hap-mod-archive"
    "hap-mod-sqlite"
    "hap-mod-http" "hap-mod-websocket" "hap-mod-net"
    "hap-mod-clipboard" "hap-mod-dialog" "hap-mod-notification" "hap-mod-shortcut" "hap-mod-tray" "hap-mod-shell-ext"
    "hap-mod-image" "hap-mod-barcode" "hap-mod-audio"
    "hap-mod-pdf" "hap-mod-excel"
    "hap-mod-process" "hap-mod-screen" "hap-mod-serial" "hap-mod-power" "hap-mod-keychain"
)

for mod in "${MODULES[@]}"; do
    OUTPUT=$(cargo test -p "$mod" 2>&1)
    PASSED=$(echo "$OUTPUT" | grep "test result:" | sed 's/.*ok\. //' | sed 's/ passed.*//')
    FAILED=$(echo "$OUTPUT" | grep "test result:" | sed 's/.*; //' | sed 's/ failed.*//' | head -1)

    if echo "$OUTPUT" | grep -q "FAILED"; then
        TOTAL_FAIL=$((TOTAL_FAIL + 1))
        FAILED_MODULES="$FAILED_MODULES $mod"
        echo "  ❌ $mod: FAILED"
    else
        TOTAL_PASS=$((TOTAL_PASS + ${PASSED:-0}))
        echo "  ✅ $mod: ${PASSED:-0} passed"
    fi
done

echo ""
echo "=== Summary ==="
echo "Modules: ${#MODULES[@]}"
echo "Tests passed: $TOTAL_PASS"
echo "Failed modules: $TOTAL_FAIL"
[ -n "$FAILED_MODULES" ] && echo "Failed:$FAILED_MODULES"
echo ""

[ $TOTAL_FAIL -eq 0 ] && echo "All tests passed!" || exit 1
