#!/bin/bash
# E2E test for hap-mod-automation
# 前提：Shell 已启动且 DevTools 正在运行（已打开项目）

set -e

PORT_FILE="$HOME/.hiapphub/devtools.port"
if [ ! -f "$PORT_FILE" ]; then
  echo "FAIL: $PORT_FILE not found"
  exit 1
fi

PORT=$(python3 -c "import json;print(json.load(open('$PORT_FILE'))['port'])")
TOKEN=$(python3 -c "import json;print(json.load(open('$PORT_FILE'))['token'])")
BASE="http://127.0.0.1:$PORT/api/v1"

echo "=== hap-mod-automation E2E Test ==="
echo "Port: $PORT"

# 1. 验证 API 可达
echo -n "[1/10] API reachable... "
APPS=$(curl -sf -H "Authorization: Bearer $TOKEN" "$BASE/apps")
echo "OK ($(echo $APPS | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))') apps)"

# 获取第一个 app
APP_ID=$(echo $APPS | python3 -c 'import sys,json;apps=json.load(sys.stdin);print(apps[0]["appId"] if apps else "")')
if [ -z "$APP_ID" ]; then
  echo "FAIL: no apps running"
  exit 1
fi
echo "   Target app: $APP_ID"
W="main"

# 2. eval
echo -n "[2/10] eval... "
RESULT=$(curl -sf -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"code":"document.title"}' "$BASE/apps/$APP_ID/windows/$W/eval")
echo "OK: $RESULT"

# 3. bounds
echo -n "[3/10] bounds... "
BOUNDS=$(curl -sf -H "Authorization: Bearer $TOKEN" "$BASE/apps/$APP_ID/windows/$W/bounds")
echo "OK: $BOUNDS"

# 4. dom query
echo -n "[4/10] dom query... "
DOM=$(curl -sf -H "Authorization: Bearer $TOKEN" "$BASE/apps/$APP_ID/windows/$W/dom/query?selector=body")
echo "OK: found=$(echo $DOM | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d.get("found",d.get("tagName","?")))')"

# 5. click (safe: click body)
echo -n "[5/10] click... "
CLICK=$(curl -sf -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"selector":"body"}' "$BASE/apps/$APP_ID/windows/$W/click")
echo "OK: $CLICK"

# 6. screenshot
echo -n "[6/10] screenshot... "
SS=$(curl -sf -H "Authorization: Bearer $TOKEN" "$BASE/apps/$APP_ID/windows/$W/screenshot")
SS_LEN=$(echo $SS | python3 -c 'import sys,json;d=json.load(sys.stdin);print(len(d.get("data","")))')
echo "OK: ${SS_LEN} chars base64"

# 7. performance
echo -n "[7/10] performance... "
PERF=$(curl -sf -H "Authorization: Bearer $TOKEN" "$BASE/apps/$APP_ID/windows/$W/performance")
echo "OK"

# 8. accessibility
echo -n "[8/10] accessibility... "
A11Y=$(curl -sf -H "Authorization: Bearer $TOKEN" "$BASE/apps/$APP_ID/windows/$W/accessibility")
echo "OK"

# 9. batch
echo -n "[9/10] batch... "
BATCH=$(curl -sf -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"steps":[{"action":"eval","code":"1+1"}],"stopOnError":true}' \
  "$BASE/apps/$APP_ID/windows/$W/batch")
echo "OK: $BATCH"

# 10. Rust module unit (native call via cargo test)
echo -n "[10/10] Rust unit tests... "
cd "$(dirname "$0")/.."
CARGO_OUT=$(cargo test -p hap-mod-automation 2>&1)
PASS_COUNT=$(echo "$CARGO_OUT" | grep "test result" | head -1 | grep -oE '[0-9]+ passed')
echo "OK: $PASS_COUNT"

echo ""
echo "=== ALL TESTS PASSED ==="
