#!/bin/bash
# 构建 HiAppHub HAL 模块到鸿蒙 (aarch64-unknown-linux-ohos)
set -e

TARGET="aarch64-unknown-linux-ohos"
OHOS_MODULES=(
  hap-mod-storage
  hap-mod-encoding
  hap-mod-crypto
  hap-mod-fs
  hap-mod-csv
  hap-mod-datetime
  hap-mod-xml
  hap-mod-log
  hap-mod-archive
  hap-mod-app-manager
  hap-mod-clipboard
  hap-mod-process
  hap-mod-scheduler
  hap-mod-excel
  hap-mod-pdf
  hap-mod-http
  hap-mod-webserver
  hap-mod-websocket
  hap-mod-net
  hap-mod-email
  hap-mod-image
  hap-mod-barcode
  hap-mod-sqlite
)

PACKAGES=""
for mod in "${OHOS_MODULES[@]}"; do
  PACKAGES="$PACKAGES -p $mod"
done

echo "Building ${#OHOS_MODULES[@]} HAL modules for $TARGET..."
cargo build --target "$TARGET" --release $PACKAGES

echo "Done. Output:"
ls -la "target/$TARGET/release/"*.so 2>/dev/null
