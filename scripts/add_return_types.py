#!/usr/bin/env python3
"""Add return types for functions returning 'object'."""

import json
import os

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

RETURN_TYPES = {
    "hap-mod-http": {
        "_type_defs": [
            {"name": "HttpResponse", "descriptions": {"zh-CN": "HTTP响应", "en-US": "HTTP response"}, "fields": [
                {"name": "status", "type": "number", "descs": {"zh-CN": "状态码", "en-US": "Status code"}},
                {"name": "headers", "type": "object", "descs": {"zh-CN": "响应头", "en-US": "Response headers"}},
                {"name": "body", "type": "string", "descs": {"zh-CN": "响应体", "en-US": "Response body"}},
                {"name": "url", "type": "string", "descs": {"zh-CN": "最终URL(含重定向)", "en-US": "Final URL"}},
            ]},
            {"name": "DownloadResult", "descriptions": {"zh-CN": "下载结果", "en-US": "Download result"}, "fields": [
                {"name": "path", "type": "string", "descs": {"zh-CN": "保存路径", "en-US": "Save path"}},
                {"name": "size", "type": "number", "descs": {"zh-CN": "文件大小", "en-US": "File size"}},
                {"name": "status", "type": "number", "descs": {"zh-CN": "状态码", "en-US": "Status code"}},
            ]},
            {"name": "SseConnection", "descriptions": {"zh-CN": "SSE连接信息", "en-US": "SSE connection info"}, "fields": [
                {"name": "sse_id", "type": "string", "descs": {"zh-CN": "SSE连接ID", "en-US": "SSE connection ID"}},
            ]},
        ],
        "request": "HttpResponse", "get": "HttpResponse", "post": "HttpResponse",
        "put": "HttpResponse", "patch": "HttpResponse", "delete": "HttpResponse",
        "head": "HttpResponse", "upload": "HttpResponse",
        "post_form": "HttpResponse", "post_json": "HttpResponse",
        "get_json": "HttpResponse", "put_json": "HttpResponse", "patch_json": "HttpResponse",
        "download": "DownloadResult", "download_resume": "DownloadResult",
        "sse_connect": "SseConnection",
    },
    "hap-mod-websocket": {
        "_type_defs": [
            {"name": "WsConnection", "descriptions": {"zh-CN": "WebSocket连接信息", "en-US": "WebSocket connection info"}, "fields": [
                {"name": "conn_id", "type": "string", "descs": {"zh-CN": "连接ID", "en-US": "Connection ID"}},
                {"name": "url", "type": "string", "descs": {"zh-CN": "连接地址", "en-US": "Connection URL"}},
                {"name": "protocol", "type": "string", "optional": True, "descs": {"zh-CN": "协商的子协议", "en-US": "Negotiated sub-protocol"}},
            ]},
        ],
        "connect": "WsConnection",
    },
    "hap-mod-fs": {
        "_type_defs": [
            {"name": "FileStat", "descriptions": {"zh-CN": "文件状态", "en-US": "File status"}, "fields": [
                {"name": "size", "type": "number", "descs": {"zh-CN": "大小(字节)", "en-US": "Size(bytes)"}},
                {"name": "is_dir", "type": "boolean", "descs": {"zh-CN": "是否目录", "en-US": "Is directory"}},
                {"name": "is_file", "type": "boolean", "descs": {"zh-CN": "是否文件", "en-US": "Is file"}},
                {"name": "created", "type": "number", "descs": {"zh-CN": "创建时间戳(ms)", "en-US": "Created(ms)"}},
                {"name": "modified", "type": "number", "descs": {"zh-CN": "修改时间戳(ms)", "en-US": "Modified(ms)"}},
                {"name": "readonly", "type": "boolean", "descs": {"zh-CN": "是否只读", "en-US": "Is readonly"}},
            ]},
            {"name": "WatchHandle", "descriptions": {"zh-CN": "文件监听句柄", "en-US": "Watch handle"}, "fields": [
                {"name": "watch_id", "type": "string", "descs": {"zh-CN": "监听ID", "en-US": "Watch ID"}},
                {"name": "path", "type": "string", "descs": {"zh-CN": "监听路径", "en-US": "Watched path"}},
            ]},
            {"name": "DiskUsage", "descriptions": {"zh-CN": "磁盘使用情况", "en-US": "Disk usage"}, "fields": [
                {"name": "total", "type": "number", "descs": {"zh-CN": "总空间(字节)", "en-US": "Total(bytes)"}},
                {"name": "used", "type": "number", "descs": {"zh-CN": "已用空间", "en-US": "Used(bytes)"}},
                {"name": "free", "type": "number", "descs": {"zh-CN": "可用空间", "en-US": "Free(bytes)"}},
            ]},
            {"name": "LockHandle", "descriptions": {"zh-CN": "文件锁句柄", "en-US": "File lock handle"}, "fields": [
                {"name": "lock_id", "type": "string", "descs": {"zh-CN": "锁ID", "en-US": "Lock ID"}},
                {"name": "path", "type": "string", "descs": {"zh-CN": "锁定路径", "en-US": "Locked path"}},
            ]},
            {"name": "CompareResult", "descriptions": {"zh-CN": "文件比较结果", "en-US": "File compare result"}, "fields": [
                {"name": "identical", "type": "boolean", "descs": {"zh-CN": "是否相同", "en-US": "Is identical"}},
                {"name": "size_diff", "type": "number", "descs": {"zh-CN": "大小差异", "en-US": "Size difference"}},
            ]},
        ],
        "stat": "FileStat", "watch": "WatchHandle", "disk_usage": "DiskUsage",
        "lock_file": "LockHandle", "compare": "CompareResult",
    },
    "hap-mod-browser": {
        "_type_defs": [
            {"name": "BrowserInstance", "descriptions": {"zh-CN": "浏览器实例", "en-US": "Browser instance"}, "fields": [
                {"name": "browser_id", "type": "string", "descs": {"zh-CN": "浏览器ID", "en-US": "Browser ID"}},
                {"name": "ws_url", "type": "string", "descs": {"zh-CN": "WebSocket调试地址", "en-US": "WebSocket debug URL"}},
            ]},
            {"name": "PageInfo", "descriptions": {"zh-CN": "页面信息", "en-US": "Page info"}, "fields": [
                {"name": "page_id", "type": "string", "descs": {"zh-CN": "页面ID", "en-US": "Page ID"}},
                {"name": "url", "type": "string", "descs": {"zh-CN": "页面URL", "en-US": "Page URL"}},
                {"name": "title", "type": "string", "descs": {"zh-CN": "页面标题", "en-US": "Page title"}},
            ]},
            {"name": "ScreenshotResult", "descriptions": {"zh-CN": "截图结果", "en-US": "Screenshot result"}, "fields": [
                {"name": "data", "type": "string", "descs": {"zh-CN": "Base64图片数据", "en-US": "Base64 image data"}},
                {"name": "format", "type": "string", "descs": {"zh-CN": "图片格式(png)", "en-US": "Image format"}},
            ]},
            {"name": "ElementInfo", "descriptions": {"zh-CN": "DOM元素信息", "en-US": "DOM element info"}, "fields": [
                {"name": "tag", "type": "string", "descs": {"zh-CN": "标签名", "en-US": "Tag name"}},
                {"name": "text", "type": "string", "descs": {"zh-CN": "文本内容", "en-US": "Text content"}},
                {"name": "attributes", "type": "object", "descs": {"zh-CN": "属性映射", "en-US": "Attributes"}},
            ]},
        ],
        "launch": "BrowserInstance", "connect": "BrowserInstance",
        "new_page": "PageInfo", "navigate": "PageInfo",
        "screenshot": "ScreenshotResult", "pdf": "ScreenshotResult",
        "query_selector": "ElementInfo",
    },
    "hap-mod-crypto": {
        "_type_defs": [
            {"name": "EncryptResult", "descriptions": {"zh-CN": "加密结果", "en-US": "Encryption result"}, "fields": [
                {"name": "ciphertext", "type": "string", "descs": {"zh-CN": "密文(Base64)", "en-US": "Ciphertext(Base64)"}},
                {"name": "iv", "type": "string", "optional": True, "descs": {"zh-CN": "初始向量", "en-US": "Initialization vector"}},
                {"name": "tag", "type": "string", "optional": True, "descs": {"zh-CN": "认证标签(GCM)", "en-US": "Auth tag(GCM)"}},
            ]},
            {"name": "KeyPair", "descriptions": {"zh-CN": "密钥对", "en-US": "Key pair"}, "fields": [
                {"name": "public_key", "type": "string", "descs": {"zh-CN": "公钥(PEM)", "en-US": "Public key(PEM)"}},
                {"name": "private_key", "type": "string", "descs": {"zh-CN": "私钥(PEM)", "en-US": "Private key(PEM)"}},
            ]},
            {"name": "TotpResult", "descriptions": {"zh-CN": "TOTP结果", "en-US": "TOTP result"}, "fields": [
                {"name": "code", "type": "string", "descs": {"zh-CN": "验证码", "en-US": "Verification code"}},
                {"name": "remaining", "type": "number", "descs": {"zh-CN": "剩余秒数", "en-US": "Remaining seconds"}},
            ]},
            {"name": "TotpSecret", "descriptions": {"zh-CN": "TOTP密钥", "en-US": "TOTP secret"}, "fields": [
                {"name": "secret", "type": "string", "descs": {"zh-CN": "密钥(Base32)", "en-US": "Secret(Base32)"}},
                {"name": "url", "type": "string", "descs": {"zh-CN": "otpauth URL", "en-US": "otpauth URL"}},
            ]},
            {"name": "X509Info", "descriptions": {"zh-CN": "证书信息", "en-US": "Certificate info"}, "fields": [
                {"name": "subject", "type": "string", "descs": {"zh-CN": "主题", "en-US": "Subject"}},
                {"name": "issuer", "type": "string", "descs": {"zh-CN": "颁发者", "en-US": "Issuer"}},
                {"name": "not_before", "type": "string", "descs": {"zh-CN": "生效时间", "en-US": "Not before"}},
                {"name": "not_after", "type": "string", "descs": {"zh-CN": "过期时间", "en-US": "Not after"}},
            ]},
            {"name": "FileEncryptResult", "descriptions": {"zh-CN": "文件加密结果", "en-US": "File encrypt result"}, "fields": [
                {"name": "output_path", "type": "string", "descs": {"zh-CN": "输出路径", "en-US": "Output path"}},
                {"name": "size", "type": "number", "descs": {"zh-CN": "输出大小", "en-US": "Output size"}},
            ]},
        ],
        "encrypt": "EncryptResult", "generate_keypair": "KeyPair",
        "generate_totp": "TotpResult", "generate_totp_secret": "TotpSecret",
        "x509_info": "X509Info", "encrypt_file": "FileEncryptResult", "decrypt_file": "FileEncryptResult",
    },
    "hap-mod-system": {
        "_type_defs": [
            {"name": "OsInfo", "descriptions": {"zh-CN": "操作系统信息", "en-US": "OS info"}, "fields": [
                {"name": "os", "type": "string", "descs": {"zh-CN": "系统名", "en-US": "OS name"}},
                {"name": "version", "type": "string", "descs": {"zh-CN": "版本号", "en-US": "Version"}},
                {"name": "arch", "type": "string", "descs": {"zh-CN": "架构", "en-US": "Architecture"}},
                {"name": "kernel", "type": "string", "descs": {"zh-CN": "内核版本", "en-US": "Kernel version"}},
            ]},
            {"name": "CpuInfo", "descriptions": {"zh-CN": "CPU信息", "en-US": "CPU info"}, "fields": [
                {"name": "model", "type": "string", "descs": {"zh-CN": "型号", "en-US": "Model"}},
                {"name": "cores", "type": "number", "descs": {"zh-CN": "核心数", "en-US": "Cores"}},
                {"name": "frequency", "type": "number", "descs": {"zh-CN": "频率(MHz)", "en-US": "Frequency(MHz)"}},
            ]},
            {"name": "MemoryInfo", "descriptions": {"zh-CN": "内存信息", "en-US": "Memory info"}, "fields": [
                {"name": "total", "type": "number", "descs": {"zh-CN": "总内存(MB)", "en-US": "Total(MB)"}},
                {"name": "free", "type": "number", "descs": {"zh-CN": "可用(MB)", "en-US": "Free(MB)"}},
                {"name": "used", "type": "number", "descs": {"zh-CN": "已用(MB)", "en-US": "Used(MB)"}},
            ]},
            {"name": "GpuInfo", "descriptions": {"zh-CN": "GPU信息", "en-US": "GPU info"}, "fields": [
                {"name": "name", "type": "string", "descs": {"zh-CN": "名称", "en-US": "Name"}},
                {"name": "vendor", "type": "string", "descs": {"zh-CN": "厂商", "en-US": "Vendor"}},
                {"name": "memory", "type": "number", "optional": True, "descs": {"zh-CN": "显存(MB)", "en-US": "VRAM(MB)"}},
            ]},
            {"name": "ProxyConfig", "descriptions": {"zh-CN": "代理配置", "en-US": "Proxy config"}, "fields": [
                {"name": "http", "type": "string", "optional": True, "descs": {"zh-CN": "HTTP代理", "en-US": "HTTP proxy"}},
                {"name": "https", "type": "string", "optional": True, "descs": {"zh-CN": "HTTPS代理", "en-US": "HTTPS proxy"}},
                {"name": "no_proxy", "type": "string", "optional": True, "descs": {"zh-CN": "不代理列表", "en-US": "No proxy list"}},
            ]},
        ],
        "os_info": "OsInfo", "cpu_info": "CpuInfo", "memory_info": "MemoryInfo",
        "gpu_info": "GpuInfo", "get_proxy": "ProxyConfig",
    },
    "hap-mod-image": {
        "_type_defs": [
            {"name": "ImageInfo", "descriptions": {"zh-CN": "图片信息", "en-US": "Image info"}, "fields": [
                {"name": "width", "type": "number", "descs": {"zh-CN": "宽度(px)", "en-US": "Width(px)"}},
                {"name": "height", "type": "number", "descs": {"zh-CN": "高度(px)", "en-US": "Height(px)"}},
                {"name": "format", "type": "string", "descs": {"zh-CN": "格式", "en-US": "Format"}},
                {"name": "size", "type": "number", "descs": {"zh-CN": "文件大小", "en-US": "File size"}},
            ]},
            {"name": "ExifData", "descriptions": {"zh-CN": "EXIF数据", "en-US": "EXIF data"}, "fields": [
                {"name": "camera", "type": "string", "optional": True, "descs": {"zh-CN": "相机型号", "en-US": "Camera model"}},
                {"name": "datetime", "type": "string", "optional": True, "descs": {"zh-CN": "拍摄时间", "en-US": "Date taken"}},
                {"name": "gps", "type": "object", "optional": True, "descs": {"zh-CN": "GPS坐标", "en-US": "GPS coordinates"}},
            ]},
            {"name": "PixelColor", "descriptions": {"zh-CN": "像素颜色", "en-US": "Pixel color"}, "fields": [
                {"name": "r", "type": "number", "descs": {"zh-CN": "红(0-255)", "en-US": "Red(0-255)"}},
                {"name": "g", "type": "number", "descs": {"zh-CN": "绿(0-255)", "en-US": "Green(0-255)"}},
                {"name": "b", "type": "number", "descs": {"zh-CN": "蓝(0-255)", "en-US": "Blue(0-255)"}},
                {"name": "a", "type": "number", "descs": {"zh-CN": "透明度", "en-US": "Alpha"}},
            ]},
            {"name": "CompareResult", "descriptions": {"zh-CN": "图片对比结果", "en-US": "Image compare result"}, "fields": [
                {"name": "similarity", "type": "number", "descs": {"zh-CN": "相似度(0-1)", "en-US": "Similarity(0-1)"}},
                {"name": "diff_pixels", "type": "number", "descs": {"zh-CN": "差异像素数", "en-US": "Different pixels"}},
            ]},
            {"name": "GifInfo", "descriptions": {"zh-CN": "GIF信息", "en-US": "GIF info"}, "fields": [
                {"name": "frames", "type": "number", "descs": {"zh-CN": "帧数", "en-US": "Frame count"}},
                {"name": "width", "type": "number", "descs": {"zh-CN": "宽度", "en-US": "Width"}},
                {"name": "height", "type": "number", "descs": {"zh-CN": "高度", "en-US": "Height"}},
                {"name": "duration_ms", "type": "number", "descs": {"zh-CN": "总时长(ms)", "en-US": "Duration(ms)"}},
            ]},
        ],
        "info": "ImageInfo", "exif": "ExifData", "get_pixel": "PixelColor",
        "compare": "CompareResult", "gif_info": "GifInfo",
    },
    "hap-mod-audio": {
        "_type_defs": [
            {"name": "PlayHandle", "descriptions": {"zh-CN": "播放句柄", "en-US": "Play handle"}, "fields": [
                {"name": "player_id", "type": "string", "descs": {"zh-CN": "播放器ID", "en-US": "Player ID"}},
            ]},
            {"name": "RecordHandle", "descriptions": {"zh-CN": "录音句柄", "en-US": "Record handle"}, "fields": [
                {"name": "recorder_id", "type": "string", "descs": {"zh-CN": "录音器ID", "en-US": "Recorder ID"}},
            ]},
            {"name": "RecordResult", "descriptions": {"zh-CN": "录音结果", "en-US": "Record result"}, "fields": [
                {"name": "path", "type": "string", "descs": {"zh-CN": "文件路径", "en-US": "File path"}},
                {"name": "duration_ms", "type": "number", "descs": {"zh-CN": "时长(ms)", "en-US": "Duration(ms)"}},
                {"name": "size", "type": "number", "descs": {"zh-CN": "文件大小", "en-US": "File size"}},
            ]},
            {"name": "AudioFileInfo", "descriptions": {"zh-CN": "音频文件信息", "en-US": "Audio file info"}, "fields": [
                {"name": "duration_ms", "type": "number", "descs": {"zh-CN": "时长(ms)", "en-US": "Duration(ms)"}},
                {"name": "format", "type": "string", "descs": {"zh-CN": "格式", "en-US": "Format"}},
                {"name": "sample_rate", "type": "number", "descs": {"zh-CN": "采样率", "en-US": "Sample rate"}},
                {"name": "channels", "type": "number", "descs": {"zh-CN": "声道数", "en-US": "Channels"}},
                {"name": "bitrate", "type": "number", "descs": {"zh-CN": "比特率", "en-US": "Bitrate"}},
            ]},
            {"name": "ConvertResult", "descriptions": {"zh-CN": "转换结果", "en-US": "Convert result"}, "fields": [
                {"name": "path", "type": "string", "descs": {"zh-CN": "输出路径", "en-US": "Output path"}},
                {"name": "size", "type": "number", "descs": {"zh-CN": "文件大小", "en-US": "File size"}},
            ]},
        ],
        "play": "PlayHandle", "play_url": "PlayHandle",
        "record_start": "RecordHandle", "record_stop": "RecordResult",
        "file_info": "AudioFileInfo",
        "convert": "ConvertResult", "trim": "ConvertResult", "concat": "ConvertResult",
    },
    "hap-mod-net": {
        "_type_defs": [
            {"name": "TcpConnection", "descriptions": {"zh-CN": "TCP连接", "en-US": "TCP connection"}, "fields": [
                {"name": "conn_id", "type": "string", "descs": {"zh-CN": "连接ID", "en-US": "Connection ID"}},
                {"name": "local_addr", "type": "string", "descs": {"zh-CN": "本地地址", "en-US": "Local address"}},
                {"name": "remote_addr", "type": "string", "descs": {"zh-CN": "远端地址", "en-US": "Remote address"}},
            ]},
            {"name": "TcpServer", "descriptions": {"zh-CN": "TCP服务器", "en-US": "TCP server"}, "fields": [
                {"name": "server_id", "type": "string", "descs": {"zh-CN": "服务器ID", "en-US": "Server ID"}},
                {"name": "addr", "type": "string", "descs": {"zh-CN": "监听地址", "en-US": "Listen address"}},
            ]},
            {"name": "UdpSocket", "descriptions": {"zh-CN": "UDP套接字", "en-US": "UDP socket"}, "fields": [
                {"name": "socket_id", "type": "string", "descs": {"zh-CN": "套接字ID", "en-US": "Socket ID"}},
                {"name": "local_addr", "type": "string", "descs": {"zh-CN": "本地地址", "en-US": "Local address"}},
            ]},
            {"name": "UdpMessage", "descriptions": {"zh-CN": "UDP消息", "en-US": "UDP message"}, "fields": [
                {"name": "data", "type": "string", "descs": {"zh-CN": "数据内容", "en-US": "Data"}},
                {"name": "from", "type": "string", "descs": {"zh-CN": "来源地址", "en-US": "Source address"}},
            ]},
            {"name": "PingResult", "descriptions": {"zh-CN": "Ping结果", "en-US": "Ping result"}, "fields": [
                {"name": "host", "type": "string", "descs": {"zh-CN": "目标主机", "en-US": "Target host"}},
                {"name": "time_ms", "type": "number", "descs": {"zh-CN": "延迟(ms)", "en-US": "Latency(ms)"}},
                {"name": "ttl", "type": "number", "descs": {"zh-CN": "TTL", "en-US": "TTL"}},
            ]},
            {"name": "WifiInfo", "descriptions": {"zh-CN": "WiFi信息", "en-US": "WiFi info"}, "fields": [
                {"name": "ssid", "type": "string", "descs": {"zh-CN": "网络名称", "en-US": "Network name"}},
                {"name": "signal", "type": "number", "descs": {"zh-CN": "信号强度", "en-US": "Signal strength"}},
                {"name": "security", "type": "string", "descs": {"zh-CN": "安全类型", "en-US": "Security type"}},
            ]},
            {"name": "SslInfo", "descriptions": {"zh-CN": "SSL证书信息", "en-US": "SSL certificate info"}, "fields": [
                {"name": "issuer", "type": "string", "descs": {"zh-CN": "颁发者", "en-US": "Issuer"}},
                {"name": "subject", "type": "string", "descs": {"zh-CN": "主题", "en-US": "Subject"}},
                {"name": "expires", "type": "string", "descs": {"zh-CN": "过期时间", "en-US": "Expiry date"}},
                {"name": "valid", "type": "boolean", "descs": {"zh-CN": "是否有效", "en-US": "Is valid"}},
            ]},
            {"name": "SpeedResult", "descriptions": {"zh-CN": "测速结果", "en-US": "Speed test result"}, "fields": [
                {"name": "download_mbps", "type": "number", "descs": {"zh-CN": "下载速度(Mbps)", "en-US": "Download(Mbps)"}},
                {"name": "upload_mbps", "type": "number", "descs": {"zh-CN": "上传速度(Mbps)", "en-US": "Upload(Mbps)"}},
                {"name": "ping_ms", "type": "number", "descs": {"zh-CN": "延迟(ms)", "en-US": "Ping(ms)"}},
            ]},
        ],
        "tcp_connect": "TcpConnection", "tcp_listen": "TcpServer",
        "tcp_accept": "TcpConnection", "udp_bind": "UdpSocket",
        "udp_recv": "UdpMessage", "ping": "PingResult",
        "wifi_info": "WifiInfo", "ssl_info": "SslInfo", "speed_test": "SpeedResult",
    },
    "hap-mod-process": {
        "_type_defs": [
            {"name": "ExecResult", "descriptions": {"zh-CN": "执行结果", "en-US": "Execution result"}, "fields": [
                {"name": "stdout", "type": "string", "descs": {"zh-CN": "标准输出", "en-US": "Standard output"}},
                {"name": "stderr", "type": "string", "descs": {"zh-CN": "标准错误", "en-US": "Standard error"}},
                {"name": "exit_code", "type": "number", "descs": {"zh-CN": "退出码", "en-US": "Exit code"}},
            ]},
            {"name": "SpawnHandle", "descriptions": {"zh-CN": "进程句柄", "en-US": "Process handle"}, "fields": [
                {"name": "pid", "type": "number", "descs": {"zh-CN": "进程ID", "en-US": "Process ID"}},
            ]},
            {"name": "ProcessUsage", "descriptions": {"zh-CN": "进程资源使用", "en-US": "Process usage"}, "fields": [
                {"name": "cpu_percent", "type": "number", "descs": {"zh-CN": "CPU使用率(%)", "en-US": "CPU usage(%)"}},
                {"name": "memory_mb", "type": "number", "descs": {"zh-CN": "内存(MB)", "en-US": "Memory(MB)"}},
            ]},
        ],
        "exec": "ExecResult", "spawn": "SpawnHandle", "wait": "ExecResult",
        "self_usage": "ProcessUsage",
    },
    "hap-mod-screen": {
        "_type_defs": [
            {"name": "CaptureResult", "descriptions": {"zh-CN": "截屏结果", "en-US": "Capture result"}, "fields": [
                {"name": "data", "type": "string", "descs": {"zh-CN": "Base64图片", "en-US": "Base64 image"}},
                {"name": "width", "type": "number", "descs": {"zh-CN": "宽度", "en-US": "Width"}},
                {"name": "height", "type": "number", "descs": {"zh-CN": "高度", "en-US": "Height"}},
            ]},
            {"name": "DisplayInfo", "descriptions": {"zh-CN": "显示器信息", "en-US": "Display info"}, "fields": [
                {"name": "width", "type": "number", "descs": {"zh-CN": "宽度", "en-US": "Width"}},
                {"name": "height", "type": "number", "descs": {"zh-CN": "高度", "en-US": "Height"}},
                {"name": "scale", "type": "number", "descs": {"zh-CN": "缩放比例", "en-US": "Scale factor"}},
                {"name": "is_primary", "type": "boolean", "descs": {"zh-CN": "是否主显示器", "en-US": "Is primary"}},
            ]},
            {"name": "CursorPosition", "descriptions": {"zh-CN": "光标位置", "en-US": "Cursor position"}, "fields": [
                {"name": "x", "type": "number", "descs": {"zh-CN": "X坐标", "en-US": "X"}},
                {"name": "y", "type": "number", "descs": {"zh-CN": "Y坐标", "en-US": "Y"}},
            ]},
            {"name": "ScreenColor", "descriptions": {"zh-CN": "屏幕颜色", "en-US": "Screen color"}, "fields": [
                {"name": "r", "type": "number", "descs": {"zh-CN": "红", "en-US": "Red"}},
                {"name": "g", "type": "number", "descs": {"zh-CN": "绿", "en-US": "Green"}},
                {"name": "b", "type": "number", "descs": {"zh-CN": "蓝", "en-US": "Blue"}},
                {"name": "hex", "type": "string", "descs": {"zh-CN": "十六进制", "en-US": "Hex"}},
            ]},
            {"name": "WindowInfo", "descriptions": {"zh-CN": "窗口信息", "en-US": "Window info"}, "fields": [
                {"name": "id", "type": "string", "descs": {"zh-CN": "窗口ID", "en-US": "Window ID"}},
                {"name": "title", "type": "string", "descs": {"zh-CN": "标题", "en-US": "Title"}},
                {"name": "app", "type": "string", "descs": {"zh-CN": "应用名", "en-US": "App name"}},
            ]},
        ],
        "capture_full": "CaptureResult", "capture_region": "CaptureResult",
        "capture_window": "CaptureResult",
        "get_primary": "DisplayInfo", "get_cursor_pos": "CursorPosition",
        "color_at": "ScreenColor", "active_window": "WindowInfo",
    },
    "hap-mod-power": {
        "_type_defs": [
            {"name": "BatteryStatus", "descriptions": {"zh-CN": "电池状态", "en-US": "Battery status"}, "fields": [
                {"name": "level", "type": "number", "descs": {"zh-CN": "电量(%)", "en-US": "Level(%)"}},
                {"name": "charging", "type": "boolean", "descs": {"zh-CN": "是否充电", "en-US": "Is charging"}},
                {"name": "time_remaining", "type": "number", "optional": True, "descs": {"zh-CN": "剩余分钟", "en-US": "Minutes remaining"}},
            ]},
            {"name": "SleepLock", "descriptions": {"zh-CN": "休眠锁", "en-US": "Sleep lock"}, "fields": [
                {"name": "lock_id", "type": "string", "descs": {"zh-CN": "锁ID", "en-US": "Lock ID"}},
            ]},
        ],
        "battery_status": "BatteryStatus", "prevent_sleep": "SleepLock",
    },
    "hap-mod-input": {
        "_type_defs": [
            {"name": "MousePosition", "descriptions": {"zh-CN": "鼠标位置", "en-US": "Mouse position"}, "fields": [
                {"name": "x", "type": "number", "descs": {"zh-CN": "X坐标", "en-US": "X"}},
                {"name": "y", "type": "number", "descs": {"zh-CN": "Y坐标", "en-US": "Y"}},
            ]},
        ],
        "get_mouse_position": "MousePosition",
    },
    "hap-mod-window": {
        "_type_defs": [
            {"name": "WindowInfo", "descriptions": {"zh-CN": "窗口信息", "en-US": "Window info"}, "fields": [
                {"name": "id", "type": "string", "descs": {"zh-CN": "窗口ID", "en-US": "Window ID"}},
                {"name": "title", "type": "string", "descs": {"zh-CN": "标题", "en-US": "Title"}},
                {"name": "app", "type": "string", "descs": {"zh-CN": "应用名", "en-US": "App name"}},
                {"name": "x", "type": "number", "descs": {"zh-CN": "X位置", "en-US": "X position"}},
                {"name": "y", "type": "number", "descs": {"zh-CN": "Y位置", "en-US": "Y position"}},
                {"name": "width", "type": "number", "descs": {"zh-CN": "宽度", "en-US": "Width"}},
                {"name": "height", "type": "number", "descs": {"zh-CN": "高度", "en-US": "Height"}},
            ]},
            {"name": "WindowBounds", "descriptions": {"zh-CN": "窗口边界", "en-US": "Window bounds"}, "fields": [
                {"name": "x", "type": "number", "descs": {"zh-CN": "X", "en-US": "X"}},
                {"name": "y", "type": "number", "descs": {"zh-CN": "Y", "en-US": "Y"}},
                {"name": "width", "type": "number", "descs": {"zh-CN": "宽度", "en-US": "Width"}},
                {"name": "height", "type": "number", "descs": {"zh-CN": "高度", "en-US": "Height"}},
            ]},
        ],
        "get_active": "WindowInfo", "screenshot": "CaptureResult", "get_bounds": "WindowBounds",
    },
    "hap-mod-ocr": {
        "_type_defs": [
            {"name": "OcrResult", "descriptions": {"zh-CN": "OCR识别结果", "en-US": "OCR result"}, "fields": [
                {"name": "text", "type": "string", "descs": {"zh-CN": "识别文本", "en-US": "Recognized text"}},
                {"name": "confidence", "type": "number", "descs": {"zh-CN": "置信度(0-1)", "en-US": "Confidence(0-1)"}},
                {"name": "regions", "type": "array", "optional": True, "descs": {"zh-CN": "文字区域列表", "en-US": "Text regions"}},
            ]},
        ],
        "recognize": "OcrResult", "recognize_region": "OcrResult",
        "recognize_base64": "OcrResult", "recognize_screen_region": "OcrResult",
    },
    "hap-mod-sqlite": {
        "_type_defs": [
            {"name": "DbHandle", "descriptions": {"zh-CN": "数据库句柄", "en-US": "Database handle"}, "fields": [
                {"name": "db_id", "type": "string", "descs": {"zh-CN": "数据库ID", "en-US": "Database ID"}},
                {"name": "path", "type": "string", "descs": {"zh-CN": "文件路径", "en-US": "File path"}},
            ]},
            {"name": "ExecResult", "descriptions": {"zh-CN": "执行结果", "en-US": "Execution result"}, "fields": [
                {"name": "changes", "type": "number", "descs": {"zh-CN": "影响行数", "en-US": "Rows affected"}},
                {"name": "last_insert_id", "type": "number", "descs": {"zh-CN": "最后插入ID", "en-US": "Last insert ID"}},
            ]},
        ],
        "open": "DbHandle", "execute": "ExecResult", "batch_execute": "ExecResult",
    },
    "hap-mod-bluetooth": {
        "_type_defs": [
            {"name": "BleConnection", "descriptions": {"zh-CN": "蓝牙连接", "en-US": "BLE connection"}, "fields": [
                {"name": "device_id", "type": "string", "descs": {"zh-CN": "设备ID", "en-US": "Device ID"}},
                {"name": "name", "type": "string", "descs": {"zh-CN": "设备名", "en-US": "Device name"}},
            ]},
            {"name": "CharValue", "descriptions": {"zh-CN": "特征值", "en-US": "Characteristic value"}, "fields": [
                {"name": "value", "type": "string", "descs": {"zh-CN": "值(Base64)", "en-US": "Value(Base64)"}},
                {"name": "uuid", "type": "string", "descs": {"zh-CN": "特征UUID", "en-US": "Characteristic UUID"}},
            ]},
        ],
        "connect": "BleConnection", "read_characteristic": "CharValue",
    },
    "hap-mod-usb": {
        "_type_defs": [
            {"name": "UsbHandle", "descriptions": {"zh-CN": "USB设备句柄", "en-US": "USB device handle"}, "fields": [
                {"name": "device_id", "type": "string", "descs": {"zh-CN": "设备ID", "en-US": "Device ID"}},
                {"name": "vendor_id", "type": "number", "descs": {"zh-CN": "厂商ID", "en-US": "Vendor ID"}},
                {"name": "product_id", "type": "number", "descs": {"zh-CN": "产品ID", "en-US": "Product ID"}},
            ]},
            {"name": "TransferResult", "descriptions": {"zh-CN": "传输结果", "en-US": "Transfer result"}, "fields": [
                {"name": "bytes_transferred", "type": "number", "descs": {"zh-CN": "传输字节数", "en-US": "Bytes transferred"}},
                {"name": "data", "type": "string", "optional": True, "descs": {"zh-CN": "接收数据(Base64)", "en-US": "Received data(Base64)"}},
            ]},
        ],
        "open": "UsbHandle",
        "bulk_transfer_out": "TransferResult", "bulk_transfer_in": "TransferResult",
        "control_transfer": "TransferResult",
    },
    "hap-mod-serial": {
        "_type_defs": [
            {"name": "SerialHandle", "descriptions": {"zh-CN": "串口句柄", "en-US": "Serial port handle"}, "fields": [
                {"name": "port_id", "type": "string", "descs": {"zh-CN": "端口ID", "en-US": "Port ID"}},
                {"name": "name", "type": "string", "descs": {"zh-CN": "端口名", "en-US": "Port name"}},
            ]},
        ],
        "open": "SerialHandle",
    },
    "hap-mod-scheduler": {
        "_type_defs": [
            {"name": "TaskHandle", "descriptions": {"zh-CN": "定时任务句柄", "en-US": "Schedule handle"}, "fields": [
                {"name": "task_id", "type": "string", "descs": {"zh-CN": "任务ID", "en-US": "Task ID"}},
                {"name": "next_run", "type": "string", "optional": True, "descs": {"zh-CN": "下次执行时间", "en-US": "Next run time"}},
            ]},
            {"name": "NextRun", "descriptions": {"zh-CN": "下次执行信息", "en-US": "Next run info"}, "fields": [
                {"name": "time", "type": "string", "descs": {"zh-CN": "执行时间", "en-US": "Run time"}},
                {"name": "remaining_ms", "type": "number", "descs": {"zh-CN": "剩余毫秒", "en-US": "Remaining ms"}},
            ]},
        ],
        "create_cron": "TaskHandle", "create_interval": "TaskHandle",
        "create_timeout": "TaskHandle", "get_next_run": "NextRun",
    },
    "hap-mod-datetime": {
        "_type_defs": [
            {"name": "DateTime", "descriptions": {"zh-CN": "日期时间结构", "en-US": "DateTime structure"}, "fields": [
                {"name": "year", "type": "number", "descs": {"zh-CN": "年", "en-US": "Year"}},
                {"name": "month", "type": "number", "descs": {"zh-CN": "月", "en-US": "Month"}},
                {"name": "day", "type": "number", "descs": {"zh-CN": "日", "en-US": "Day"}},
                {"name": "hour", "type": "number", "descs": {"zh-CN": "时", "en-US": "Hour"}},
                {"name": "minute", "type": "number", "descs": {"zh-CN": "分", "en-US": "Minute"}},
                {"name": "second", "type": "number", "descs": {"zh-CN": "秒", "en-US": "Second"}},
                {"name": "timestamp", "type": "number", "descs": {"zh-CN": "Unix时间戳(ms)", "en-US": "Unix timestamp(ms)"}},
            ]},
        ],
        "now": "DateTime", "parse": "DateTime",
    },
}


def process_module(module_dir):
    module_name = os.path.basename(module_dir)
    if module_name not in RETURN_TYPES:
        return

    manifest_path = os.path.join(module_dir, "manifest.json")
    with open(manifest_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    config = RETURN_TYPES[module_name]
    type_defs = config.get("_type_defs", [])
    changed = False

    existing_types = {t["name"] for t in data.get("types", [])}
    for td in type_defs:
        if td["name"] not in existing_types:
            if "types" not in data:
                data["types"] = []
            data["types"].append(td)
            changed = True

    for fn in data.get("functions", []):
        if fn["name"] in config and fn.get("returns", {}).get("type") == "object":
            fn["returns"]["type"] = config[fn["name"]]
            changed = True

    if changed:
        with open(manifest_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"  Updated: {module_name}")
    else:
        print(f"  Skipped: {module_name}")


def main():
    for entry in sorted(os.listdir(BASE)):
        if entry.startswith("hap-mod-"):
            process_module(os.path.join(BASE, entry))
    print("\nDone!")


if __name__ == "__main__":
    main()
