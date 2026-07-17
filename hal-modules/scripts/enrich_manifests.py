#!/usr/bin/env python3
"""Batch enrich all module manifests with group/types/constants/events/async/platform fields."""

import json
import os

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

GROUPS = {
    "hap-mod-net": {
        "tcp_": "tcp", "udp_": "udp", "dns_": "dns",
        "ping": "general", "local_ip": "general", "interfaces": "general",
        "is_online": "general", "port_available": "general", "public_ip": "general",
        "mac_address": "general", "wifi_info": "general", "ssl_info": "general",
        "speed_test": "general", "traceroute": "general", "wake_on_lan": "general",
        "find_available_port": "general", "on_network_change": "general",
        "off_network_change": "general",
    },
    "hap-mod-audio": {
        "play": "playback", "play_url": "playback", "pause": "playback",
        "resume": "playback", "stop": "playback", "stop_all": "playback",
        "set_volume": "playback", "set_speed": "playback", "get_position": "playback",
        "get_duration": "playback", "seek": "playback", "is_playing": "playback",
        "get_state": "playback", "list_players": "playback",
        "list_devices": "device", "set_device": "device", "beep": "device",
        "get_system_volume": "system", "set_system_volume": "system",
        "is_muted": "system", "set_muted": "system",
        "on_device_change": "system", "off_device_change": "system",
        "record_start": "record", "record_pause": "record",
        "record_resume": "record", "record_stop": "record", "list_recorders": "record",
        "file_info": "edit", "convert": "edit", "trim": "edit",
        "concat": "edit", "get_metadata": "edit", "set_metadata": "edit",
        "set_album_art": "edit", "normalize": "edit", "get_waveform": "edit",
        "split": "edit", "fade": "edit", "mix": "edit",
    },
    "hap-mod-crypto": {
        "hash": "hash", "hash_file": "hash", "hmac": "hash", "crc32": "hash",
        "encrypt": "symmetric", "decrypt": "symmetric",
        "encrypt_with_password": "symmetric", "decrypt_with_password": "symmetric",
        "encrypt_file": "symmetric", "decrypt_file": "symmetric",
        "generate_key": "symmetric", "derive_key": "symmetric",
        "generate_keypair": "asymmetric", "sign": "asymmetric", "verify": "asymmetric",
        "rsa_encrypt": "asymmetric", "rsa_decrypt": "asymmetric",
        "x509_info": "asymmetric", "pem_to_der": "asymmetric", "der_to_pem": "asymmetric",
        "random_bytes": "utility", "generate_uuid": "utility",
        "constant_time_eq": "utility", "random_string": "utility",
        "generate_password": "utility",
        "bcrypt_hash": "utility", "bcrypt_verify": "utility",
        "generate_totp": "utility", "verify_totp": "utility",
        "generate_totp_secret": "utility",
    },
    "hap-mod-bluetooth": {
        "scan_start": "scan", "scan_stop": "scan",
        "connect": "connection", "disconnect": "connection", "is_connected": "connection",
        "discover_services": "data", "read_characteristic": "data",
        "write_characteristic": "data", "subscribe": "data", "unsubscribe": "data",
    },
    "hap-mod-fs": {
        "read_text_file": "file", "write_text_file": "file", "append_text_file": "file",
        "read_binary": "file", "write_binary": "file", "read_binary_range": "file",
        "read_text_lines": "file", "write_atomic": "file", "truncate": "file",
        "exists": "info", "stat": "info", "is_dir": "info", "is_file": "info",
        "file_size": "info", "real_path": "info", "file_name": "info",
        "extension": "info", "file_type": "info", "line_count": "info",
        "normalize_path": "info", "dir_size": "info", "compare": "info",
        "join_path": "info", "parent_path": "info", "checksum": "info",
        "list_dir": "dir", "glob": "dir", "mkdir": "dir",
        "copy": "operate", "copy_dir": "operate", "move": "operate",
        "remove": "operate", "symlink": "operate", "read_link": "operate",
        "hard_link": "operate", "touch": "operate", "set_permissions": "operate",
        "temp_file": "operate", "temp_dir": "operate",
        "search_content": "search",
        "watch": "watch", "unwatch": "watch", "list_watchers": "watch",
        "lock_file": "lock", "unlock_file": "lock", "list_locks": "lock",
        "disk_usage": "system",
    },
    "hap-mod-image": {
        "info": "basic", "resize": "basic", "crop": "basic", "rotate": "basic",
        "flip": "basic", "convert": "basic", "compress": "basic", "thumbnail": "basic",
        "to_base64": "basic", "from_base64": "basic", "exif": "basic",
        "strip_exif": "basic", "auto_orient": "basic", "pad": "basic",
        "concat": "basic", "trim": "basic", "round_corners": "basic",
        "create_blank": "draw", "text": "draw", "watermark_text": "draw",
        "watermark_image": "draw", "overlay": "draw",
        "draw_rect": "draw", "draw_circle": "draw", "draw_line": "draw",
        "draw_arrow": "draw", "mask": "draw",
        "blur": "filter", "sharpen": "filter", "grayscale": "filter",
        "adjust": "filter", "invert": "filter", "sepia": "filter",
        "opacity": "filter", "pixelate": "filter",
        "extract_colors": "analyze", "compare": "analyze",
        "get_pixel": "analyze", "histogram": "analyze",
        "gif_from_frames": "gif", "split_gif": "gif", "gif_info": "gif",
    },
    "hap-mod-input": {
        "key_press": "keyboard", "key_down": "keyboard", "key_up": "keyboard",
        "type_text": "keyboard", "hotkey": "keyboard",
        "mouse_move": "mouse", "mouse_click": "mouse", "mouse_down": "mouse",
        "mouse_up": "mouse", "mouse_drag": "mouse", "scroll": "mouse",
        "get_mouse_position": "mouse",
    },
    "hap-mod-email": {
        "send": "send",
        "fetch": "receive", "list_folders": "receive", "mark_read": "receive",
        "delete": "receive", "download_attachment": "receive",
    },
    "hap-mod-excel": {
        "read": "file", "write": "file", "open": "file", "create": "file",
        "save": "file", "save_as": "file", "close": "file", "list_open": "file",
        "to_csv": "file", "to_json": "file", "from_csv": "file",
        "add_sheet": "sheet", "delete_sheet": "sheet", "rename_sheet": "sheet",
        "list_sheets": "sheet", "copy_sheet": "sheet", "protect_sheet": "sheet",
        "unprotect_sheet": "sheet",
        "get_cell": "cell", "set_cell": "cell", "get_range": "cell",
        "set_range": "cell", "set_formula": "cell",
        "insert_row": "cell", "insert_column": "cell",
        "delete_row": "cell", "delete_column": "cell",
        "set_style": "format", "merge_cells": "format", "unmerge_cells": "format",
        "set_column_width": "format", "set_row_height": "format",
        "auto_filter": "format", "freeze_panes": "format",
        "auto_fit_columns": "format", "set_conditional_format": "format",
        "add_data_validation": "format",
        "add_image": "content", "add_chart": "content",
    },
    "hap-mod-pdf": {
        "info": "file", "open": "file", "create": "file", "save": "file",
        "close": "file", "merge": "file", "split": "file", "list_open": "file",
        "html_to_pdf": "file",
        "add_page": "content", "add_text": "content", "add_image": "content",
        "add_line": "content", "add_rect": "content", "add_table": "content",
        "add_link": "content", "add_page_numbers": "content",
        "add_header": "content", "add_footer": "content", "register_font": "content",
        "add_watermark": "content", "stamp_image": "content",
        "insert_page": "page", "delete_page": "page", "reorder_pages": "page",
        "rotate_page": "page", "page_dimensions": "page",
        "extract_text": "extract", "to_images": "extract", "extract_images": "extract",
        "set_password": "security", "remove_password": "security", "flatten": "security",
        "get_bookmarks": "navigate", "add_bookmark": "navigate",
        "get_form_fields": "form", "fill_form": "form",
        "get_annotations": "annotate", "add_annotation": "annotate",
        "remove_annotation": "annotate",
    },
}

TYPES = {
    "hap-mod-http": [
        {
            "name": "HttpFile",
            "descriptions": {"zh-CN": "上传文件结构", "en-US": "File upload structure"},
            "fields": [
                {"name": "name", "type": "string", "descs": {"zh-CN": "表单字段名", "en-US": "Form field name"}},
                {"name": "filename", "type": "string", "descs": {"zh-CN": "文件名", "en-US": "File name"}},
                {"name": "content_type", "type": "string", "optional": True, "default_value": "application/octet-stream", "descs": {"zh-CN": "MIME类型", "en-US": "MIME type"}},
                {"name": "data", "type": "string", "descs": {"zh-CN": "Base64编码的文件内容", "en-US": "Base64 encoded content"}}
            ]
        },
        {
            "name": "HttpResponse",
            "descriptions": {"zh-CN": "HTTP响应结构", "en-US": "HTTP response structure"},
            "fields": [
                {"name": "status", "type": "number", "descs": {"zh-CN": "状态码", "en-US": "Status code"}},
                {"name": "headers", "type": "object", "descs": {"zh-CN": "响应头", "en-US": "Response headers"}},
                {"name": "body", "type": "string", "descs": {"zh-CN": "响应体", "en-US": "Response body"}}
            ]
        }
    ],
    "hap-mod-email": [
        {
            "name": "EmailAttachment",
            "descriptions": {"zh-CN": "邮件附件结构", "en-US": "Email attachment"},
            "fields": [
                {"name": "filename", "type": "string", "descs": {"zh-CN": "文件名", "en-US": "File name"}},
                {"name": "content_type", "type": "string", "descs": {"zh-CN": "MIME类型", "en-US": "MIME type"}},
                {"name": "data", "type": "string", "descs": {"zh-CN": "Base64内容", "en-US": "Base64 content"}}
            ]
        }
    ],
    "hap-mod-image": [
        {
            "name": "Rect",
            "descriptions": {"zh-CN": "矩形区域", "en-US": "Rectangle region"},
            "fields": [
                {"name": "x", "type": "number", "descs": {"zh-CN": "X坐标", "en-US": "X coordinate"}},
                {"name": "y", "type": "number", "descs": {"zh-CN": "Y坐标", "en-US": "Y coordinate"}},
                {"name": "width", "type": "number", "descs": {"zh-CN": "宽度", "en-US": "Width"}},
                {"name": "height", "type": "number", "descs": {"zh-CN": "高度", "en-US": "Height"}}
            ]
        },
        {
            "name": "Color",
            "descriptions": {"zh-CN": "颜色值", "en-US": "Color value"},
            "fields": [
                {"name": "r", "type": "number", "descs": {"zh-CN": "红色(0-255)", "en-US": "Red(0-255)"}},
                {"name": "g", "type": "number", "descs": {"zh-CN": "绿色(0-255)", "en-US": "Green(0-255)"}},
                {"name": "b", "type": "number", "descs": {"zh-CN": "蓝色(0-255)", "en-US": "Blue(0-255)"}},
                {"name": "a", "type": "number", "optional": True, "default_value": "255", "descs": {"zh-CN": "透明度(0-255)", "en-US": "Alpha(0-255)"}}
            ]
        }
    ],
    "hap-mod-fs": [
        {
            "name": "FileStat",
            "descriptions": {"zh-CN": "文件状态信息", "en-US": "File status info"},
            "fields": [
                {"name": "size", "type": "number", "descs": {"zh-CN": "文件大小(字节)", "en-US": "File size(bytes)"}},
                {"name": "is_dir", "type": "boolean", "descs": {"zh-CN": "是否目录", "en-US": "Is directory"}},
                {"name": "is_file", "type": "boolean", "descs": {"zh-CN": "是否文件", "en-US": "Is file"}},
                {"name": "created", "type": "number", "descs": {"zh-CN": "创建时间戳(ms)", "en-US": "Created timestamp(ms)"}},
                {"name": "modified", "type": "number", "descs": {"zh-CN": "修改时间戳(ms)", "en-US": "Modified timestamp(ms)"}},
                {"name": "readonly", "type": "boolean", "descs": {"zh-CN": "是否只读", "en-US": "Is readonly"}}
            ]
        },
        {
            "name": "WatchEvent",
            "descriptions": {"zh-CN": "文件监听事件", "en-US": "File watch event"},
            "fields": [
                {"name": "path", "type": "string", "descs": {"zh-CN": "变更文件路径", "en-US": "Changed file path"}},
                {"name": "event", "type": "string", "descs": {"zh-CN": "事件类型(create/modify/delete/rename)", "en-US": "Event type"}},
                {"name": "timestamp", "type": "number", "descs": {"zh-CN": "时间戳(ms)", "en-US": "Timestamp(ms)"}}
            ]
        }
    ],
    "hap-mod-serial": [
        {
            "name": "PortConfig",
            "descriptions": {"zh-CN": "串口配置", "en-US": "Serial port config"},
            "fields": [
                {"name": "baud_rate", "type": "number", "descs": {"zh-CN": "波特率", "en-US": "Baud rate"}},
                {"name": "data_bits", "type": "number", "optional": True, "default_value": "8", "descs": {"zh-CN": "数据位", "en-US": "Data bits"}},
                {"name": "stop_bits", "type": "number", "optional": True, "default_value": "1", "descs": {"zh-CN": "停止位", "en-US": "Stop bits"}},
                {"name": "parity", "type": "string", "optional": True, "default_value": "none", "descs": {"zh-CN": "校验(none/odd/even)", "en-US": "Parity"}}
            ]
        }
    ],
    "hap-mod-bluetooth": [
        {
            "name": "BleDevice",
            "descriptions": {"zh-CN": "蓝牙设备信息", "en-US": "BLE device info"},
            "fields": [
                {"name": "id", "type": "string", "descs": {"zh-CN": "设备ID", "en-US": "Device ID"}},
                {"name": "name", "type": "string", "descs": {"zh-CN": "设备名称", "en-US": "Device name"}},
                {"name": "rssi", "type": "number", "descs": {"zh-CN": "信号强度", "en-US": "Signal strength"}}
            ]
        }
    ],
    "hap-mod-browser": [
        {
            "name": "Cookie",
            "descriptions": {"zh-CN": "Cookie结构", "en-US": "Cookie structure"},
            "fields": [
                {"name": "name", "type": "string", "descs": {"zh-CN": "名称", "en-US": "Name"}},
                {"name": "value", "type": "string", "descs": {"zh-CN": "值", "en-US": "Value"}},
                {"name": "domain", "type": "string", "optional": True, "descs": {"zh-CN": "域名", "en-US": "Domain"}},
                {"name": "path", "type": "string", "optional": True, "descs": {"zh-CN": "路径", "en-US": "Path"}},
                {"name": "expires", "type": "number", "optional": True, "descs": {"zh-CN": "过期时间戳", "en-US": "Expiry timestamp"}}
            ]
        }
    ],
    "hap-mod-usb": [
        {
            "name": "UsbDevice",
            "descriptions": {"zh-CN": "USB设备信息", "en-US": "USB device info"},
            "fields": [
                {"name": "vendor_id", "type": "number", "descs": {"zh-CN": "厂商ID", "en-US": "Vendor ID"}},
                {"name": "product_id", "type": "number", "descs": {"zh-CN": "产品ID", "en-US": "Product ID"}},
                {"name": "serial_number", "type": "string", "optional": True, "descs": {"zh-CN": "序列号", "en-US": "Serial number"}},
                {"name": "manufacturer", "type": "string", "optional": True, "descs": {"zh-CN": "制造商", "en-US": "Manufacturer"}}
            ]
        }
    ],
    "hap-mod-sqlite": [
        {
            "name": "ColumnInfo",
            "descriptions": {"zh-CN": "列信息", "en-US": "Column info"},
            "fields": [
                {"name": "name", "type": "string", "descs": {"zh-CN": "列名", "en-US": "Column name"}},
                {"name": "type", "type": "string", "descs": {"zh-CN": "数据类型", "en-US": "Data type"}},
                {"name": "nullable", "type": "boolean", "descs": {"zh-CN": "是否可空", "en-US": "Is nullable"}},
                {"name": "primary_key", "type": "boolean", "descs": {"zh-CN": "是否主键", "en-US": "Is primary key"}}
            ]
        }
    ],
    "hap-mod-net": [
        {
            "name": "NetworkInterface",
            "descriptions": {"zh-CN": "网络接口信息", "en-US": "Network interface info"},
            "fields": [
                {"name": "name", "type": "string", "descs": {"zh-CN": "接口名", "en-US": "Interface name"}},
                {"name": "ip", "type": "string", "descs": {"zh-CN": "IP地址", "en-US": "IP address"}},
                {"name": "mac", "type": "string", "descs": {"zh-CN": "MAC地址", "en-US": "MAC address"}},
                {"name": "is_up", "type": "boolean", "descs": {"zh-CN": "是否启用", "en-US": "Is active"}}
            ]
        }
    ],
    "hap-mod-audio": [
        {
            "name": "AudioDevice",
            "descriptions": {"zh-CN": "音频设备信息", "en-US": "Audio device info"},
            "fields": [
                {"name": "id", "type": "string", "descs": {"zh-CN": "设备ID", "en-US": "Device ID"}},
                {"name": "name", "type": "string", "descs": {"zh-CN": "设备名称", "en-US": "Device name"}},
                {"name": "is_default", "type": "boolean", "descs": {"zh-CN": "是否默认", "en-US": "Is default"}},
                {"name": "is_input", "type": "boolean", "descs": {"zh-CN": "是否输入设备", "en-US": "Is input device"}}
            ]
        }
    ],
}

CONSTANTS = {
    "hap-mod-crypto": [
        {"name": "HASH_MD5", "value": "md5", "type": "string", "group": "hash", "descs": {"zh-CN": "MD5哈希", "en-US": "MD5 hash"}},
        {"name": "HASH_SHA256", "value": "sha256", "type": "string", "group": "hash", "descs": {"zh-CN": "SHA-256哈希", "en-US": "SHA-256 hash"}},
        {"name": "HASH_SHA512", "value": "sha512", "type": "string", "group": "hash", "descs": {"zh-CN": "SHA-512哈希", "en-US": "SHA-512 hash"}},
        {"name": "ALG_AES_128_CBC", "value": "aes-128-cbc", "type": "string", "group": "algorithm", "descs": {"zh-CN": "AES-128-CBC加密", "en-US": "AES-128-CBC encryption"}},
        {"name": "ALG_AES_256_CBC", "value": "aes-256-cbc", "type": "string", "group": "algorithm", "descs": {"zh-CN": "AES-256-CBC加密", "en-US": "AES-256-CBC encryption"}},
        {"name": "ALG_AES_256_GCM", "value": "aes-256-gcm", "type": "string", "group": "algorithm", "descs": {"zh-CN": "AES-256-GCM加密", "en-US": "AES-256-GCM encryption"}},
        {"name": "ALG_RSA_2048", "value": "rsa-2048", "type": "string", "group": "algorithm", "descs": {"zh-CN": "RSA 2048位", "en-US": "RSA 2048-bit"}},
        {"name": "ALG_RSA_4096", "value": "rsa-4096", "type": "string", "group": "algorithm", "descs": {"zh-CN": "RSA 4096位", "en-US": "RSA 4096-bit"}},
    ],
    "hap-mod-http": [
        {"name": "METHOD_GET", "value": "GET", "type": "string", "group": "method", "descs": {"zh-CN": "GET方法", "en-US": "GET method"}},
        {"name": "METHOD_POST", "value": "POST", "type": "string", "group": "method", "descs": {"zh-CN": "POST方法", "en-US": "POST method"}},
        {"name": "METHOD_PUT", "value": "PUT", "type": "string", "group": "method", "descs": {"zh-CN": "PUT方法", "en-US": "PUT method"}},
        {"name": "METHOD_DELETE", "value": "DELETE", "type": "string", "group": "method", "descs": {"zh-CN": "DELETE方法", "en-US": "DELETE method"}},
        {"name": "METHOD_PATCH", "value": "PATCH", "type": "string", "group": "method", "descs": {"zh-CN": "PATCH方法", "en-US": "PATCH method"}},
        {"name": "STATUS_OK", "value": 200, "type": "number", "group": "status", "descs": {"zh-CN": "成功", "en-US": "OK"}},
        {"name": "STATUS_CREATED", "value": 201, "type": "number", "group": "status", "descs": {"zh-CN": "已创建", "en-US": "Created"}},
        {"name": "STATUS_BAD_REQUEST", "value": 400, "type": "number", "group": "status", "descs": {"zh-CN": "请求错误", "en-US": "Bad Request"}},
        {"name": "STATUS_UNAUTHORIZED", "value": 401, "type": "number", "group": "status", "descs": {"zh-CN": "未授权", "en-US": "Unauthorized"}},
        {"name": "STATUS_NOT_FOUND", "value": 404, "type": "number", "group": "status", "descs": {"zh-CN": "未找到", "en-US": "Not Found"}},
        {"name": "STATUS_SERVER_ERROR", "value": 500, "type": "number", "group": "status", "descs": {"zh-CN": "服务器错误", "en-US": "Server Error"}},
    ],
    "hap-mod-encoding": [
        {"name": "UTF8", "value": "utf-8", "type": "string", "group": "charset", "descs": {"zh-CN": "UTF-8编码", "en-US": "UTF-8 encoding"}},
        {"name": "GBK", "value": "gbk", "type": "string", "group": "charset", "descs": {"zh-CN": "GBK编码", "en-US": "GBK encoding"}},
        {"name": "GB2312", "value": "gb2312", "type": "string", "group": "charset", "descs": {"zh-CN": "GB2312编码", "en-US": "GB2312 encoding"}},
        {"name": "BIG5", "value": "big5", "type": "string", "group": "charset", "descs": {"zh-CN": "BIG5编码", "en-US": "BIG5 encoding"}},
        {"name": "SHIFT_JIS", "value": "shift-jis", "type": "string", "group": "charset", "descs": {"zh-CN": "Shift-JIS编码", "en-US": "Shift-JIS encoding"}},
        {"name": "ISO_8859_1", "value": "iso-8859-1", "type": "string", "group": "charset", "descs": {"zh-CN": "ISO-8859-1编码", "en-US": "ISO-8859-1 encoding"}},
    ],
    "hap-mod-log": [
        {"name": "LEVEL_TRACE", "value": "trace", "type": "string", "group": "level", "descs": {"zh-CN": "跟踪级别", "en-US": "Trace level"}},
        {"name": "LEVEL_DEBUG", "value": "debug", "type": "string", "group": "level", "descs": {"zh-CN": "调试级别", "en-US": "Debug level"}},
        {"name": "LEVEL_INFO", "value": "info", "type": "string", "group": "level", "descs": {"zh-CN": "信息级别", "en-US": "Info level"}},
        {"name": "LEVEL_WARN", "value": "warn", "type": "string", "group": "level", "descs": {"zh-CN": "警告级别", "en-US": "Warning level"}},
        {"name": "LEVEL_ERROR", "value": "error", "type": "string", "group": "level", "descs": {"zh-CN": "错误级别", "en-US": "Error level"}},
    ],
    "hap-mod-serial": [
        {"name": "BAUD_9600", "value": 9600, "type": "number", "group": "baud", "descs": {"zh-CN": "9600波特率", "en-US": "9600 baud"}},
        {"name": "BAUD_19200", "value": 19200, "type": "number", "group": "baud", "descs": {"zh-CN": "19200波特率", "en-US": "19200 baud"}},
        {"name": "BAUD_38400", "value": 38400, "type": "number", "group": "baud", "descs": {"zh-CN": "38400波特率", "en-US": "38400 baud"}},
        {"name": "BAUD_57600", "value": 57600, "type": "number", "group": "baud", "descs": {"zh-CN": "57600波特率", "en-US": "57600 baud"}},
        {"name": "BAUD_115200", "value": 115200, "type": "number", "group": "baud", "descs": {"zh-CN": "115200波特率", "en-US": "115200 baud"}},
        {"name": "PARITY_NONE", "value": "none", "type": "string", "group": "parity", "descs": {"zh-CN": "无校验", "en-US": "No parity"}},
        {"name": "PARITY_ODD", "value": "odd", "type": "string", "group": "parity", "descs": {"zh-CN": "奇校验", "en-US": "Odd parity"}},
        {"name": "PARITY_EVEN", "value": "even", "type": "string", "group": "parity", "descs": {"zh-CN": "偶校验", "en-US": "Even parity"}},
    ],
    "hap-mod-image": [
        {"name": "FORMAT_PNG", "value": "png", "type": "string", "group": "format", "descs": {"zh-CN": "PNG格式", "en-US": "PNG format"}},
        {"name": "FORMAT_JPEG", "value": "jpeg", "type": "string", "group": "format", "descs": {"zh-CN": "JPEG格式", "en-US": "JPEG format"}},
        {"name": "FORMAT_WEBP", "value": "webp", "type": "string", "group": "format", "descs": {"zh-CN": "WebP格式", "en-US": "WebP format"}},
        {"name": "FORMAT_GIF", "value": "gif", "type": "string", "group": "format", "descs": {"zh-CN": "GIF格式", "en-US": "GIF format"}},
        {"name": "FORMAT_BMP", "value": "bmp", "type": "string", "group": "format", "descs": {"zh-CN": "BMP格式", "en-US": "BMP format"}},
    ],
    "hap-mod-input": [
        {"name": "KEY_ENTER", "value": "Return", "type": "string", "group": "key", "descs": {"zh-CN": "回车键", "en-US": "Enter key"}},
        {"name": "KEY_ESCAPE", "value": "Escape", "type": "string", "group": "key", "descs": {"zh-CN": "ESC键", "en-US": "Escape key"}},
        {"name": "KEY_TAB", "value": "Tab", "type": "string", "group": "key", "descs": {"zh-CN": "Tab键", "en-US": "Tab key"}},
        {"name": "KEY_BACKSPACE", "value": "Backspace", "type": "string", "group": "key", "descs": {"zh-CN": "退格键", "en-US": "Backspace key"}},
        {"name": "KEY_DELETE", "value": "Delete", "type": "string", "group": "key", "descs": {"zh-CN": "删除键", "en-US": "Delete key"}},
        {"name": "KEY_SPACE", "value": "Space", "type": "string", "group": "key", "descs": {"zh-CN": "空格键", "en-US": "Space key"}},
        {"name": "KEY_UP", "value": "Up", "type": "string", "group": "key", "descs": {"zh-CN": "上方向键", "en-US": "Up arrow"}},
        {"name": "KEY_DOWN", "value": "Down", "type": "string", "group": "key", "descs": {"zh-CN": "下方向键", "en-US": "Down arrow"}},
        {"name": "KEY_LEFT", "value": "Left", "type": "string", "group": "key", "descs": {"zh-CN": "左方向键", "en-US": "Left arrow"}},
        {"name": "KEY_RIGHT", "value": "Right", "type": "string", "group": "key", "descs": {"zh-CN": "右方向键", "en-US": "Right arrow"}},
        {"name": "MOD_CTRL", "value": "Control", "type": "string", "group": "modifier", "descs": {"zh-CN": "Ctrl修饰键", "en-US": "Ctrl modifier"}},
        {"name": "MOD_ALT", "value": "Alt", "type": "string", "group": "modifier", "descs": {"zh-CN": "Alt修饰键", "en-US": "Alt modifier"}},
        {"name": "MOD_SHIFT", "value": "Shift", "type": "string", "group": "modifier", "descs": {"zh-CN": "Shift修饰键", "en-US": "Shift modifier"}},
        {"name": "MOD_META", "value": "Meta", "type": "string", "group": "modifier", "descs": {"zh-CN": "Meta/Command修饰键", "en-US": "Meta/Command modifier"}},
    ],
    "hap-mod-audio": [
        {"name": "FORMAT_MP3", "value": "mp3", "type": "string", "group": "format", "descs": {"zh-CN": "MP3格式", "en-US": "MP3 format"}},
        {"name": "FORMAT_WAV", "value": "wav", "type": "string", "group": "format", "descs": {"zh-CN": "WAV格式", "en-US": "WAV format"}},
        {"name": "FORMAT_FLAC", "value": "flac", "type": "string", "group": "format", "descs": {"zh-CN": "FLAC格式", "en-US": "FLAC format"}},
        {"name": "FORMAT_OGG", "value": "ogg", "type": "string", "group": "format", "descs": {"zh-CN": "OGG格式", "en-US": "OGG format"}},
        {"name": "FORMAT_AAC", "value": "aac", "type": "string", "group": "format", "descs": {"zh-CN": "AAC格式", "en-US": "AAC format"}},
    ],
}

EVENTS = {
    "hap-mod-websocket": [
        {"name": "message", "descriptions": {"zh-CN": "收到消息", "en-US": "Message received"}, "payload": [
            {"name": "conn_id", "type": "string", "descs": {"zh-CN": "连接ID", "en-US": "Connection ID"}},
            {"name": "data", "type": "string", "descs": {"zh-CN": "消息内容", "en-US": "Message data"}},
            {"name": "is_binary", "type": "boolean", "descs": {"zh-CN": "是否二进制", "en-US": "Is binary"}}
        ]},
        {"name": "close", "descriptions": {"zh-CN": "连接关闭", "en-US": "Connection closed"}, "payload": [
            {"name": "conn_id", "type": "string", "descs": {"zh-CN": "连接ID", "en-US": "Connection ID"}},
            {"name": "code", "type": "number", "descs": {"zh-CN": "关闭码", "en-US": "Close code"}},
            {"name": "reason", "type": "string", "descs": {"zh-CN": "原因", "en-US": "Reason"}}
        ]},
        {"name": "error", "descriptions": {"zh-CN": "连接错误", "en-US": "Connection error"}, "payload": [
            {"name": "conn_id", "type": "string", "descs": {"zh-CN": "连接ID", "en-US": "Connection ID"}},
            {"name": "message", "type": "string", "descs": {"zh-CN": "错误信息", "en-US": "Error message"}}
        ]},
        {"name": "server_client_connect", "descriptions": {"zh-CN": "客户端连入(服务端)", "en-US": "Client connected(server)"}, "payload": [
            {"name": "server_id", "type": "string", "descs": {"zh-CN": "服务器ID", "en-US": "Server ID"}},
            {"name": "client_id", "type": "string", "descs": {"zh-CN": "客户端ID", "en-US": "Client ID"}}
        ]},
        {"name": "server_client_disconnect", "descriptions": {"zh-CN": "客户端断开(服务端)", "en-US": "Client disconnected(server)"}, "payload": [
            {"name": "server_id", "type": "string", "descs": {"zh-CN": "服务器ID", "en-US": "Server ID"}},
            {"name": "client_id", "type": "string", "descs": {"zh-CN": "客户端ID", "en-US": "Client ID"}}
        ]},
    ],
    "hap-mod-fs": [
        {"name": "change", "descriptions": {"zh-CN": "文件变更事件", "en-US": "File change event"}, "payload": [
            {"name": "path", "type": "string", "descs": {"zh-CN": "变更路径", "en-US": "Changed path"}},
            {"name": "event", "type": "string", "descs": {"zh-CN": "事件类型(create/modify/delete/rename)", "en-US": "Event type"}},
        ]},
    ],
    "hap-mod-audio": [
        {"name": "device_change", "descriptions": {"zh-CN": "音频设备变更", "en-US": "Audio device changed"}, "payload": [
            {"name": "type", "type": "string", "descs": {"zh-CN": "变更类型(added/removed)", "en-US": "Change type"}}
        ]},
        {"name": "playback_ended", "descriptions": {"zh-CN": "播放结束", "en-US": "Playback ended"}, "payload": [
            {"name": "player_id", "type": "string", "descs": {"zh-CN": "播放器ID", "en-US": "Player ID"}}
        ]},
    ],
    "hap-mod-power": [
        {"name": "power_change", "descriptions": {"zh-CN": "电源状态变更", "en-US": "Power state changed"}, "payload": [
            {"name": "is_on_battery", "type": "boolean", "descs": {"zh-CN": "是否使用电池", "en-US": "Is on battery"}},
            {"name": "level", "type": "number", "descs": {"zh-CN": "电量百分比", "en-US": "Battery level %"}}
        ]},
    ],
    "hap-mod-bluetooth": [
        {"name": "device_found", "descriptions": {"zh-CN": "发现设备", "en-US": "Device found"}, "payload": [
            {"name": "id", "type": "string", "descs": {"zh-CN": "设备ID", "en-US": "Device ID"}},
            {"name": "name", "type": "string", "descs": {"zh-CN": "设备名", "en-US": "Device name"}},
            {"name": "rssi", "type": "number", "descs": {"zh-CN": "信号强度", "en-US": "RSSI"}}
        ]},
        {"name": "disconnected", "descriptions": {"zh-CN": "设备断开", "en-US": "Device disconnected"}, "payload": [
            {"name": "id", "type": "string", "descs": {"zh-CN": "设备ID", "en-US": "Device ID"}}
        ]},
        {"name": "characteristic_changed", "descriptions": {"zh-CN": "特征值变化", "en-US": "Characteristic changed"}, "payload": [
            {"name": "device_id", "type": "string", "descs": {"zh-CN": "设备ID", "en-US": "Device ID"}},
            {"name": "service_uuid", "type": "string", "descs": {"zh-CN": "服务UUID", "en-US": "Service UUID"}},
            {"name": "char_uuid", "type": "string", "descs": {"zh-CN": "特征UUID", "en-US": "Characteristic UUID"}},
            {"name": "value", "type": "string", "descs": {"zh-CN": "值(Base64)", "en-US": "Value(Base64)"}}
        ]},
    ],
    "hap-mod-serial": [
        {"name": "port_change", "descriptions": {"zh-CN": "串口设备变更", "en-US": "Port device changed"}, "payload": [
            {"name": "type", "type": "string", "descs": {"zh-CN": "变更类型(added/removed)", "en-US": "Change type"}},
            {"name": "port", "type": "string", "descs": {"zh-CN": "端口名", "en-US": "Port name"}}
        ]},
    ],
    "hap-mod-net": [
        {"name": "network_change", "descriptions": {"zh-CN": "网络状态变更", "en-US": "Network state changed"}, "payload": [
            {"name": "is_online", "type": "boolean", "descs": {"zh-CN": "是否在线", "en-US": "Is online"}}
        ]},
    ],
    "hap-mod-storage": [
        {"name": "change", "descriptions": {"zh-CN": "存储值变更", "en-US": "Storage value changed"}, "payload": [
            {"name": "key", "type": "string", "descs": {"zh-CN": "键名", "en-US": "Key"}},
            {"name": "old_value", "type": "string", "optional": True, "descs": {"zh-CN": "旧值", "en-US": "Old value"}},
            {"name": "new_value", "type": "string", "optional": True, "descs": {"zh-CN": "新值", "en-US": "New value"}}
        ]},
    ],
    "hap-mod-clipboard": [
        {"name": "change", "descriptions": {"zh-CN": "剪贴板内容变更", "en-US": "Clipboard changed"}, "payload": [
            {"name": "formats", "type": "array", "descs": {"zh-CN": "可用格式列表", "en-US": "Available formats"}}
        ]},
    ],
    "hap-mod-system": [
        {"name": "theme_change", "descriptions": {"zh-CN": "系统主题变更", "en-US": "System theme changed"}, "payload": [
            {"name": "theme", "type": "string", "descs": {"zh-CN": "新主题(light/dark)", "en-US": "New theme(light/dark)"}}
        ]},
    ],
}

ASYNC_FUNCTIONS = {
    "hap-mod-websocket": ["connect"],
    "hap-mod-http": ["request", "get", "post", "put", "patch", "delete", "head", "download", "upload", "post_form", "post_json", "get_json", "put_json", "patch_json", "download_resume"],
    "hap-mod-browser": ["launch", "connect", "navigate", "evaluate", "click", "type_text", "screenshot", "wait_for_selector", "pdf"],
    "hap-mod-audio": ["play", "play_url", "record_start", "convert", "trim", "concat", "normalize", "split", "fade", "mix"],
    "hap-mod-bluetooth": ["scan_start", "connect", "discover_services", "read_characteristic", "write_characteristic"],
    "hap-mod-net": ["tcp_connect", "tcp_send", "tcp_recv", "tcp_listen", "tcp_accept", "udp_recv", "dns_lookup", "ping", "speed_test", "traceroute"],
    "hap-mod-process": ["exec", "spawn", "wait"],
    "hap-mod-ocr": ["recognize", "recognize_region", "recognize_base64", "recognize_screen_region"],
    "hap-mod-email": ["send", "fetch", "list_folders", "mark_read", "delete", "download_attachment"],
    "hap-mod-serial": ["read", "read_line", "read_until"],
}

PLATFORM_LIMITS = {
    "hap-mod-ocr": {"recognize": ["macos", "linux"], "recognize_region": ["macos", "linux"], "recognize_base64": ["macos", "linux"], "recognize_screen_region": ["macos"]},
    "hap-mod-window": {"*": ["macos"]},
}


def get_group(module_name, func_name):
    if module_name not in GROUPS:
        return None
    mapping = GROUPS[module_name]
    if func_name in mapping:
        return mapping[func_name]
    for prefix, group in mapping.items():
        if prefix.endswith("_") and func_name.startswith(prefix):
            return group
    return None


def process_module(module_dir):
    module_name = os.path.basename(module_dir)
    manifest_path = os.path.join(module_dir, "manifest.json")
    if not os.path.exists(manifest_path):
        return

    with open(manifest_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    changed = False

    for fn in data.get("functions", []):
        g = get_group(module_name, fn["name"])
        if g and fn.get("group") != g:
            fn["group"] = g
            changed = True

        if module_name in ASYNC_FUNCTIONS and fn["name"] in ASYNC_FUNCTIONS[module_name]:
            if not fn.get("async"):
                fn["async"] = True
                changed = True

        if module_name in PLATFORM_LIMITS:
            pl = PLATFORM_LIMITS[module_name]
            platforms = pl.get(fn["name"]) or pl.get("*")
            if platforms and fn.get("platform") != platforms:
                fn["platform"] = platforms
                changed = True

    if module_name in TYPES and "types" not in data:
        data["types"] = TYPES[module_name]
        changed = True

    if module_name in CONSTANTS and "constants" not in data:
        data["constants"] = CONSTANTS[module_name]
        changed = True

    if module_name in EVENTS and "events" not in data:
        data["events"] = EVENTS[module_name]
        changed = True

    if changed:
        with open(manifest_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"  Updated: {module_name}")
    else:
        print(f"  Skipped: {module_name} (no changes)")


def main():
    for entry in sorted(os.listdir(BASE)):
        if entry.startswith("hap-mod-"):
            process_module(os.path.join(BASE, entry))
    print("\nDone!")


if __name__ == "__main__":
    main()
