# HiAppHub HAL

HiAppHub 支持库（Hardware Abstraction Layer）模块集合，提供系统级 API。

每个模块为独立 Rust cdylib 动态库（`.hal`），通过 C ABI 自动注册到 Bridge。

## 模块列表

| 模块 | 说明 |
|---|---|
| `hap-mod-fs` | 文件系统 |
| `hap-mod-process` | 进程管理 |
| `hap-mod-dialog` | 原生对话框 |
| `hap-mod-crypto` | 加密/Hash |
| `hap-mod-http` | HTTP 请求 |
| `hap-mod-storage` | KV 存储 |
| `hap-mod-clipboard` | 剪贴板 |
| `hap-mod-tray` | 系统托盘 |
| `hap-mod-system` | 系统信息 |
| `hap-mod-app-manager` | 应用管理/OTA |
| `hap-mod-ipc-server` | IPC 服务器 |
| `hap-mod-devtools-server` | DevTools WS 服务 |
| 其他 30+ 模块 | audio/image/excel/csv/email/bluetooth... |

## 编译

```bash
cargo build --release
```

## 部署

```bash
chmod +x scripts/build-all.sh
./scripts/build-all.sh
./scripts/deploy.sh
```

## 开发新模块

```bash
npx create-hap add --type hal --name my-module
```

## 许可证

MIT
