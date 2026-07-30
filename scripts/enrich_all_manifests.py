#!/usr/bin/env python3
"""批量为所有支持库 manifest 添加缺失的 types/constants/events/groups 并改进描述"""
import json
import os
import glob

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

ENRICHMENT = {
    "hap-mod-archive": {
        "groups": {
            "compress": ["compress_zip", "compress_gzip", "compress_tar", "compress_bz2", "compress_7z"],
            "decompress": ["decompress_zip", "decompress_gzip", "decompress_tar", "decompress_bz2", "decompress_7z"],
            "info": ["list_contents", "get_info", "extract_single", "test_integrity"],
        },
        "types": [
            {"name": "ArchiveEntry", "descriptions": {"zh-CN": "压缩包内的单个文件条目信息", "en-US": "Single file entry info within an archive"}, "fields": [
                {"name": "path", "type": "string", "descs": {"zh-CN": "条目在压缩包内的相对路径", "en-US": "Relative path within the archive"}},
                {"name": "size", "type": "number", "descs": {"zh-CN": "解压后的文件大小（字节）", "en-US": "Uncompressed file size in bytes"}},
                {"name": "compressed_size", "type": "number", "descs": {"zh-CN": "压缩后的大小（字节）", "en-US": "Compressed size in bytes"}},
                {"name": "is_dir", "type": "boolean", "descs": {"zh-CN": "是否为目录", "en-US": "Whether entry is a directory"}},
                {"name": "modified", "type": "string", "descs": {"zh-CN": "最后修改时间", "en-US": "Last modified timestamp"}},
            ]},
            {"name": "ArchiveInfo", "descriptions": {"zh-CN": "压缩包的整体信息", "en-US": "Overall archive information"}, "fields": [
                {"name": "format", "type": "string", "descs": {"zh-CN": "压缩格式（zip/gzip/tar/bz2/7z）", "en-US": "Compression format"}},
                {"name": "total_size", "type": "number", "descs": {"zh-CN": "压缩包总大小", "en-US": "Total archive size"}},
                {"name": "entry_count", "type": "number", "descs": {"zh-CN": "包含的条目总数", "en-US": "Total number of entries"}},
            ]},
        ],
        "constants": [
            {"name": "FORMAT_ZIP", "value": "zip", "type": "string", "descs": {"zh-CN": "ZIP 格式", "en-US": "ZIP format"}},
            {"name": "FORMAT_GZIP", "value": "gzip", "type": "string", "descs": {"zh-CN": "GZIP 格式", "en-US": "GZIP format"}},
            {"name": "FORMAT_TAR", "value": "tar", "type": "string", "descs": {"zh-CN": "TAR 格式", "en-US": "TAR format"}},
            {"name": "FORMAT_BZ2", "value": "bz2", "type": "string", "descs": {"zh-CN": "BZ2 格式", "en-US": "BZ2 format"}},
            {"name": "FORMAT_7Z", "value": "7z", "type": "string", "descs": {"zh-CN": "7Z 格式", "en-US": "7Z format"}},
        ],
    },
    "hap-mod-barcode": {
        "groups": {
            "generate": ["generate_qrcode", "generate_barcode", "generate_svg"],
            "scan": ["scan_image", "scan_screen", "scan_camera", "decode_base64"],
        },
        "types": [
            {"name": "BarcodeResult", "descriptions": {"zh-CN": "条码/二维码扫描识别结果", "en-US": "Barcode/QR code scan result"}, "fields": [
                {"name": "text", "type": "string", "descs": {"zh-CN": "识别到的文本内容", "en-US": "Decoded text content"}},
                {"name": "format", "type": "string", "descs": {"zh-CN": "条码格式（qrcode/ean13/code128等）", "en-US": "Barcode format (qrcode/ean13/code128 etc)"}},
                {"name": "confidence", "type": "number", "descs": {"zh-CN": "识别置信度（0-1）", "en-US": "Recognition confidence (0-1)"}},
            ]},
        ],
        "constants": [
            {"name": "FORMAT_QRCODE", "value": "qrcode", "type": "string", "descs": {"zh-CN": "二维码格式", "en-US": "QR Code format"}},
            {"name": "FORMAT_EAN13", "value": "ean13", "type": "string", "descs": {"zh-CN": "EAN-13 条码格式", "en-US": "EAN-13 barcode format"}},
            {"name": "FORMAT_CODE128", "value": "code128", "type": "string", "descs": {"zh-CN": "Code 128 条码格式", "en-US": "Code 128 barcode format"}},
        ],
    },
    "hap-mod-clipboard": {
        "groups": {
            "text": ["read_text", "write_text", "clear"],
            "image": ["read_image", "write_image", "has_image"],
            "file": ["read_files", "write_files", "has_files"],
            "rich": ["read_html", "write_html", "read_rtf", "write_rtf"],
            "history": ["get_history", "clear_history", "enable_history", "disable_history"],
            "monitor": ["start_monitor", "stop_monitor"],
        },
        "types": [
            {"name": "ClipboardContent", "descriptions": {"zh-CN": "剪贴板内容对象", "en-US": "Clipboard content object"}, "fields": [
                {"name": "type", "type": "string", "descs": {"zh-CN": "内容类型（text/image/files/html）", "en-US": "Content type"}},
                {"name": "data", "type": "string", "descs": {"zh-CN": "内容数据", "en-US": "Content data"}},
                {"name": "timestamp", "type": "string", "descs": {"zh-CN": "写入时间戳", "en-US": "Write timestamp"}},
            ]},
        ],
    },
    "hap-mod-csv": {
        "groups": {
            "read": ["read_file", "parse_string", "read_row", "get_headers"],
            "write": ["write_file", "stringify", "append_row"],
        },
        "types": [
            {"name": "CsvOptions", "descriptions": {"zh-CN": "CSV 解析/生成配置选项", "en-US": "CSV parsing/generation options"}, "fields": [
                {"name": "delimiter", "type": "string", "descs": {"zh-CN": "分隔符，默认逗号", "en-US": "Delimiter, default comma"}},
                {"name": "has_header", "type": "boolean", "descs": {"zh-CN": "首行是否为表头", "en-US": "Whether first row is header"}},
                {"name": "encoding", "type": "string", "descs": {"zh-CN": "文件编码（utf-8/gbk等）", "en-US": "File encoding"}},
            ]},
        ],
    },
    "hap-mod-dialog": {
        "groups": {
            "message": ["alert", "confirm", "prompt"],
            "file": ["open_file", "open_files", "open_folder", "save_file"],
            "input": ["input_text", "input_password", "input_number"],
            "picker": ["color_picker", "date_picker", "font_picker"],
            "progress": ["show_progress", "update_progress", "close_progress"],
        },
        "types": [
            {"name": "DialogResult", "descriptions": {"zh-CN": "对话框返回结果", "en-US": "Dialog result"}, "fields": [
                {"name": "confirmed", "type": "boolean", "descs": {"zh-CN": "用户是否点击确认", "en-US": "Whether user confirmed"}},
                {"name": "value", "type": "string", "descs": {"zh-CN": "用户输入的值", "en-US": "User input value"}},
            ]},
            {"name": "FileFilter", "descriptions": {"zh-CN": "文件选择器的过滤规则", "en-US": "File picker filter rule"}, "fields": [
                {"name": "name", "type": "string", "descs": {"zh-CN": "过滤器名称（如'图片文件'）", "en-US": "Filter name"}},
                {"name": "extensions", "type": "string[]", "descs": {"zh-CN": "允许的扩展名列表", "en-US": "Allowed extensions"}},
            ]},
        ],
    },
    "hap-mod-excel": {
        "types": [
            {"name": "CellValue", "descriptions": {"zh-CN": "单元格的值及其类型信息", "en-US": "Cell value and type info"}, "fields": [
                {"name": "value", "type": "string", "descs": {"zh-CN": "单元格显示值", "en-US": "Cell display value"}},
                {"name": "type", "type": "string", "descs": {"zh-CN": "值类型（string/number/boolean/date/formula）", "en-US": "Value type"}},
                {"name": "formula", "type": "string", "descs": {"zh-CN": "公式（如有）", "en-US": "Formula if any"}, "optional": True},
            ]},
            {"name": "SheetInfo", "descriptions": {"zh-CN": "工作表基本信息", "en-US": "Worksheet basic info"}, "fields": [
                {"name": "name", "type": "string", "descs": {"zh-CN": "工作表名称", "en-US": "Sheet name"}},
                {"name": "index", "type": "number", "descs": {"zh-CN": "工作表索引（从0开始）", "en-US": "Sheet index (0-based)"}},
                {"name": "rows", "type": "number", "descs": {"zh-CN": "行数", "en-US": "Row count"}},
                {"name": "cols", "type": "number", "descs": {"zh-CN": "列数", "en-US": "Column count"}},
            ]},
        ],
        "constants": [
            {"name": "TYPE_STRING", "value": "string", "type": "string", "descs": {"zh-CN": "字符串类型", "en-US": "String type"}},
            {"name": "TYPE_NUMBER", "value": "number", "type": "string", "descs": {"zh-CN": "数字类型", "en-US": "Number type"}},
            {"name": "TYPE_BOOLEAN", "value": "boolean", "type": "string", "descs": {"zh-CN": "布尔类型", "en-US": "Boolean type"}},
            {"name": "TYPE_DATE", "value": "date", "type": "string", "descs": {"zh-CN": "日期类型", "en-US": "Date type"}},
            {"name": "TYPE_FORMULA", "value": "formula", "type": "string", "descs": {"zh-CN": "公式类型", "en-US": "Formula type"}},
        ],
    },
    "hap-mod-keychain": {
        "groups": {
            "password": ["set_password", "get_password", "delete_password", "has_password"],
            "generic": ["set_item", "get_item", "delete_item", "list_items"],
            "manage": ["clear_service", "export_all", "import_all"],
        },
        "types": [
            {"name": "KeychainItem", "descriptions": {"zh-CN": "钥匙串存储条目", "en-US": "Keychain storage item"}, "fields": [
                {"name": "service", "type": "string", "descs": {"zh-CN": "服务标识", "en-US": "Service identifier"}},
                {"name": "account", "type": "string", "descs": {"zh-CN": "账户名", "en-US": "Account name"}},
                {"name": "label", "type": "string", "descs": {"zh-CN": "显示标签", "en-US": "Display label"}, "optional": True},
            ]},
        ],
    },
    "hap-mod-notification": {
        "groups": {
            "send": ["show", "show_with_actions", "schedule"],
            "manage": ["cancel", "cancel_all", "get_pending", "request_permission", "check_permission"],
        },
        "types": [
            {"name": "NotificationAction", "descriptions": {"zh-CN": "通知操作按钮定义", "en-US": "Notification action button"}, "fields": [
                {"name": "id", "type": "string", "descs": {"zh-CN": "操作标识符", "en-US": "Action identifier"}},
                {"name": "title", "type": "string", "descs": {"zh-CN": "按钮显示文本", "en-US": "Button display text"}},
            ]},
        ],
        "events": [
            {"name": "notification_clicked", "descriptions": {"zh-CN": "用户点击了通知", "en-US": "User clicked a notification"}, "payload": [
                {"name": "notification_id", "type": "string", "descs": {"zh-CN": "被点击的通知ID", "en-US": "Clicked notification ID"}},
                {"name": "action_id", "type": "string", "descs": {"zh-CN": "点击的操作按钮ID（如有）", "en-US": "Clicked action ID if any"}, "optional": True},
            ]},
        ],
    },
    "hap-mod-pdf": {
        "types": [
            {"name": "PdfPageInfo", "descriptions": {"zh-CN": "PDF 页面信息", "en-US": "PDF page information"}, "fields": [
                {"name": "index", "type": "number", "descs": {"zh-CN": "页码索引（从0开始）", "en-US": "Page index (0-based)"}},
                {"name": "width", "type": "number", "descs": {"zh-CN": "页面宽度（点）", "en-US": "Page width in points"}},
                {"name": "height", "type": "number", "descs": {"zh-CN": "页面高度（点）", "en-US": "Page height in points"}},
            ]},
            {"name": "PdfMetadata", "descriptions": {"zh-CN": "PDF 文档元数据", "en-US": "PDF document metadata"}, "fields": [
                {"name": "title", "type": "string", "descs": {"zh-CN": "文档标题", "en-US": "Document title"}},
                {"name": "author", "type": "string", "descs": {"zh-CN": "文档作者", "en-US": "Document author"}},
                {"name": "page_count", "type": "number", "descs": {"zh-CN": "总页数", "en-US": "Total page count"}},
                {"name": "encrypted", "type": "boolean", "descs": {"zh-CN": "是否加密", "en-US": "Whether encrypted"}},
            ]},
        ],
    },
    "hap-mod-shell-ext": {
        "groups": {
            "execute": ["run", "run_detached", "run_with_env", "run_pipe"],
            "env": ["get_env", "set_env", "remove_env", "list_env"],
            "path": ["which", "resolve_path", "expand_home"],
            "os": ["get_shell", "get_home_dir", "get_temp_dir", "get_cwd", "set_cwd"],
            "file_assoc": ["open_with_default", "get_default_app", "register_extension"],
        },
        "types": [
            {"name": "CommandResult", "descriptions": {"zh-CN": "命令执行结果", "en-US": "Command execution result"}, "fields": [
                {"name": "stdout", "type": "string", "descs": {"zh-CN": "标准输出内容", "en-US": "Standard output content"}},
                {"name": "stderr", "type": "string", "descs": {"zh-CN": "标准错误输出", "en-US": "Standard error output"}},
                {"name": "exit_code", "type": "number", "descs": {"zh-CN": "进程退出码", "en-US": "Process exit code"}},
            ]},
        ],
    },
    "hap-mod-shortcut": {
        "types": [
            {"name": "ShortcutInfo", "descriptions": {"zh-CN": "快捷键注册信息", "en-US": "Shortcut registration info"}, "fields": [
                {"name": "id", "type": "string", "descs": {"zh-CN": "快捷键唯一标识", "en-US": "Shortcut unique ID"}},
                {"name": "keys", "type": "string", "descs": {"zh-CN": "按键组合（如 Ctrl+Shift+A）", "en-US": "Key combination"}},
                {"name": "description", "type": "string", "descs": {"zh-CN": "快捷键描述", "en-US": "Shortcut description"}},
            ]},
        ],
        "events": [
            {"name": "shortcut_triggered", "descriptions": {"zh-CN": "全局快捷键被触发", "en-US": "Global shortcut triggered"}, "payload": [
                {"name": "id", "type": "string", "descs": {"zh-CN": "被触发的快捷键ID", "en-US": "Triggered shortcut ID"}},
                {"name": "keys", "type": "string", "descs": {"zh-CN": "按键组合", "en-US": "Key combination"}},
            ]},
        ],
    },
    "hap-mod-storage": {
        "groups": {
            "kv": ["get", "set", "remove", "has", "keys", "clear"],
            "namespace": ["create_namespace", "delete_namespace", "list_namespaces"],
            "batch": ["get_many", "set_many", "remove_many"],
            "export": ["export_json", "import_json", "get_size", "compact"],
            "watch": ["watch", "unwatch"],
        },
        "types": [
            {"name": "StorageEntry", "descriptions": {"zh-CN": "存储条目详情", "en-US": "Storage entry details"}, "fields": [
                {"name": "key", "type": "string", "descs": {"zh-CN": "存储键名", "en-US": "Storage key"}},
                {"name": "value", "type": "string", "descs": {"zh-CN": "存储值（JSON字符串）", "en-US": "Storage value (JSON string)"}},
                {"name": "size", "type": "number", "descs": {"zh-CN": "值的字节大小", "en-US": "Value size in bytes"}},
                {"name": "updated_at", "type": "string", "descs": {"zh-CN": "最后更新时间", "en-US": "Last update timestamp"}},
            ]},
        ],
    },
    "hap-mod-tray": {
        "groups": {
            "create": ["create", "destroy", "set_icon", "set_tooltip"],
            "menu": ["set_menu", "update_menu_item", "add_separator"],
            "badge": ["set_badge", "clear_badge"],
            "action": ["on_click", "on_double_click"],
        },
        "types": [
            {"name": "TrayMenuItem", "descriptions": {"zh-CN": "托盘菜单项定义", "en-US": "Tray menu item definition"}, "fields": [
                {"name": "id", "type": "string", "descs": {"zh-CN": "菜单项唯一标识", "en-US": "Menu item unique ID"}},
                {"name": "label", "type": "string", "descs": {"zh-CN": "显示文本", "en-US": "Display text"}},
                {"name": "enabled", "type": "boolean", "descs": {"zh-CN": "是否启用", "en-US": "Whether enabled"}},
                {"name": "checked", "type": "boolean", "descs": {"zh-CN": "是否选中（复选菜单）", "en-US": "Whether checked"}, "optional": True},
            ]},
        ],
        "events": [
            {"name": "tray_menu_clicked", "descriptions": {"zh-CN": "托盘菜单项被点击", "en-US": "Tray menu item clicked"}, "payload": [
                {"name": "item_id", "type": "string", "descs": {"zh-CN": "被点击的菜单项ID", "en-US": "Clicked menu item ID"}},
            ]},
            {"name": "tray_clicked", "descriptions": {"zh-CN": "托盘图标被点击", "en-US": "Tray icon clicked"}, "payload": [
                {"name": "x", "type": "number", "descs": {"zh-CN": "点击位置X坐标", "en-US": "Click X position"}},
                {"name": "y", "type": "number", "descs": {"zh-CN": "点击位置Y坐标", "en-US": "Click Y position"}},
            ]},
        ],
    },
    "hap-mod-xml": {
        "groups": {
            "parse": ["parse_string", "parse_file", "validate"],
            "query": ["xpath", "query_all", "get_attribute", "get_text"],
            "generate": ["to_string", "to_file", "create_element"],
        },
        "types": [
            {"name": "XmlNode", "descriptions": {"zh-CN": "XML 节点对象", "en-US": "XML node object"}, "fields": [
                {"name": "tag", "type": "string", "descs": {"zh-CN": "节点标签名", "en-US": "Node tag name"}},
                {"name": "attributes", "type": "object", "descs": {"zh-CN": "节点属性键值对", "en-US": "Node attributes"}},
                {"name": "text", "type": "string", "descs": {"zh-CN": "节点文本内容", "en-US": "Node text content"}, "optional": True},
                {"name": "children", "type": "XmlNode[]", "descs": {"zh-CN": "子节点列表", "en-US": "Child nodes"}, "optional": True},
            ]},
        ],
    },
    "hap-mod-log": {
        "groups": {
            "write": ["debug", "info", "warn", "error", "trace"],
            "config": ["set_level", "set_file", "set_format", "set_max_size"],
            "manage": ["flush", "rotate", "clear"],
        },
    },
}

# 函数描述增强映射 (module_name -> { fn_name -> { "zh-CN": ..., "en-US": ... } })
DESC_ENRICH = {
    "hap-mod-image": {
        "info": {"zh-CN": "获取指定图片文件的基本信息（宽高、格式、色彩模式等）", "en-US": "Get basic information of an image file (dimensions, format, color mode, etc.)"},
        "resize": {"zh-CN": "按指定宽高缩放图片，支持多种插值算法", "en-US": "Resize image to specified dimensions with interpolation options"},
        "crop": {"zh-CN": "裁剪图片的指定矩形区域", "en-US": "Crop a rectangular region from the image"},
        "rotate": {"zh-CN": "将图片旋转指定角度（支持90/180/270或任意角度）", "en-US": "Rotate image by specified degrees"},
        "flip": {"zh-CN": "水平或垂直翻转图片", "en-US": "Flip image horizontally or vertically"},
        "convert": {"zh-CN": "将图片转换为其他格式（PNG/JPEG/WebP/BMP等）", "en-US": "Convert image to another format"},
        "compress": {"zh-CN": "压缩图片文件大小，可指定目标质量或最大文件尺寸", "en-US": "Compress image file size with quality or max size target"},
        "watermark": {"zh-CN": "在图片上添加文字或图片水印", "en-US": "Add text or image watermark to the image"},
        "thumbnail": {"zh-CN": "生成图片的缩略图（保持宽高比）", "en-US": "Generate a thumbnail preserving aspect ratio"},
        "merge": {"zh-CN": "将多张图片拼接合并为一张（横向或纵向）", "en-US": "Merge multiple images into one (horizontal or vertical)"},
        "blur": {"zh-CN": "对图片应用高斯模糊滤镜", "en-US": "Apply Gaussian blur filter to the image"},
        "sharpen": {"zh-CN": "对图片应用锐化滤镜以增强细节", "en-US": "Apply sharpen filter to enhance details"},
        "brightness": {"zh-CN": "调整图片的亮度值", "en-US": "Adjust image brightness"},
        "contrast": {"zh-CN": "调整图片的对比度", "en-US": "Adjust image contrast"},
        "grayscale": {"zh-CN": "将图片转换为灰度图", "en-US": "Convert image to grayscale"},
    },
    "hap-mod-fs": {
        "read_text": {"zh-CN": "读取文本文件的完整内容，支持指定编码", "en-US": "Read complete text file content with optional encoding"},
        "write_text": {"zh-CN": "将文本内容写入文件，如果文件不存在则创建", "en-US": "Write text content to file, creating if not exists"},
        "read_binary": {"zh-CN": "以二进制模式读取文件，返回 Base64 编码的内容", "en-US": "Read file in binary mode, returns Base64 encoded content"},
        "copy": {"zh-CN": "复制文件或目录到目标路径（支持递归复制目录）", "en-US": "Copy file or directory to target path (recursive for dirs)"},
        "move_file": {"zh-CN": "移动文件或目录到新路径（等同于重命名）", "en-US": "Move file or directory to new path (same as rename)"},
        "delete": {"zh-CN": "删除指定的文件或空目录", "en-US": "Delete specified file or empty directory"},
        "exists": {"zh-CN": "检查指定路径的文件或目录是否存在", "en-US": "Check if file or directory exists at the specified path"},
        "stat": {"zh-CN": "获取文件或目录的详细元数据信息（大小、权限、时间等）", "en-US": "Get detailed metadata of file or directory"},
        "mkdir": {"zh-CN": "创建目录，支持递归创建多级目录结构", "en-US": "Create directory, supports recursive creation"},
        "list_dir": {"zh-CN": "列出目录下的所有文件和子目录", "en-US": "List all files and subdirectories in a directory"},
    },
    "hap-mod-http": {
        "request": {"zh-CN": "发送自定义 HTTP 请求（支持 GET/POST/PUT/DELETE 等方法）", "en-US": "Send custom HTTP request with any method"},
        "get": {"zh-CN": "发送 HTTP GET 请求并返回响应内容", "en-US": "Send HTTP GET request and return response"},
        "post": {"zh-CN": "发送 HTTP POST 请求，支持 JSON/表单/文件上传等请求体", "en-US": "Send HTTP POST request with JSON/form/file upload body"},
        "download": {"zh-CN": "下载远程文件到本地指定路径，支持断点续传", "en-US": "Download remote file to local path with resume support"},
        "upload": {"zh-CN": "上传本地文件到远程服务器", "en-US": "Upload local file to remote server"},
    },
    "hap-mod-browser": {
        "launch": {"zh-CN": "启动一个新的浏览器实例（Chrome/Edge），可指定有头或无头模式", "en-US": "Launch new browser instance (Chrome/Edge), supports headless mode"},
        "connect": {"zh-CN": "通过 WebSocket URL 连接到已运行的浏览器实例的调试端口", "en-US": "Connect to running browser instance via WebSocket debug URL"},
        "close": {"zh-CN": "安全关闭浏览器实例并释放所有相关资源和页面连接", "en-US": "Safely close browser instance and release all resources"},
        "new_page": {"zh-CN": "在浏览器中创建一个新标签页并可选导航到指定URL", "en-US": "Create new browser tab with optional URL navigation"},
        "navigate": {"zh-CN": "导航页面到指定 URL 并等待页面加载完成", "en-US": "Navigate page to URL and wait for load completion"},
        "evaluate": {"zh-CN": "在页面中执行 JavaScript 表达式并返回计算结果", "en-US": "Execute JavaScript expression in page and return result"},
        "click": {"zh-CN": "通过 CSS 选择器定位元素并模拟鼠标点击操作", "en-US": "Locate element by CSS selector and simulate mouse click"},
        "type_text": {"zh-CN": "在指定输入元素中逐字输入文本内容", "en-US": "Type text character by character into specified input element"},
        "screenshot": {"zh-CN": "截取当前页面的屏幕截图，可保存为文件或返回 Base64", "en-US": "Capture page screenshot, save to file or return as Base64"},
        "get_html": {"zh-CN": "获取页面或指定元素的 HTML 源代码", "en-US": "Get HTML source of page or specified element"},
        "wait_for_selector": {"zh-CN": "等待指定 CSS 选择器的元素出现在页面 DOM 中", "en-US": "Wait for element matching CSS selector to appear in DOM"},
        "get_cookies": {"zh-CN": "获取当前页面的所有 Cookie 信息", "en-US": "Get all cookies for the current page"},
        "set_cookies": {"zh-CN": "批量设置页面的 Cookie", "en-US": "Set multiple cookies for the page"},
        "pdf": {"zh-CN": "将当前页面导出为 PDF 文件", "en-US": "Export current page as PDF file"},
        "list_pages": {"zh-CN": "列出浏览器实例中所有打开的标签页", "en-US": "List all open tabs in the browser instance"},
        "close_page": {"zh-CN": "关闭指定的浏览器标签页", "en-US": "Close specified browser tab"},
        "select": {"zh-CN": "在下拉选择框中选择指定的选项值", "en-US": "Select specified option value in dropdown"},
        "query_selector": {"zh-CN": "查询页面中匹配 CSS 选择器的元素属性信息", "en-US": "Query element attributes matching CSS selector"},
    },
    "hap-mod-input": {
        "key_press": {"zh-CN": "模拟按下并释放一个键盘按键，支持组合修饰键", "en-US": "Simulate key press and release with optional modifier keys"},
        "key_down": {"zh-CN": "模拟按住一个键盘按键（不释放）", "en-US": "Simulate holding down a key (without releasing)"},
        "key_up": {"zh-CN": "释放之前按住的键盘按键", "en-US": "Release a previously held key"},
        "type_text": {"zh-CN": "模拟逐字输入一段文本，支持设置每字符间的延迟", "en-US": "Simulate typing text character by character with optional delay"},
        "mouse_move": {"zh-CN": "将鼠标移动到屏幕指定坐标位置，支持平滑移动", "en-US": "Move mouse to specified screen coordinates with smooth option"},
        "mouse_click": {"zh-CN": "在指定位置执行鼠标点击操作（支持左/中/右键和多次点击）", "en-US": "Click mouse at position (supports left/middle/right and multi-click)"},
        "mouse_drag": {"zh-CN": "模拟从起点到终点的鼠标拖拽操作", "en-US": "Simulate mouse drag from start to end position"},
        "scroll": {"zh-CN": "模拟鼠标滚轮滚动操作（支持垂直和水平方向）", "en-US": "Simulate mouse wheel scroll (vertical and horizontal)"},
        "get_mouse_position": {"zh-CN": "获取鼠标当前在屏幕上的坐标位置", "en-US": "Get current mouse cursor position on screen"},
        "hotkey": {"zh-CN": "模拟同时按下多个按键的快捷键组合", "en-US": "Simulate pressing multiple keys simultaneously as hotkey"},
    },
    "hap-mod-window": {
        "list": {"zh-CN": "获取系统中当前所有窗口的列表信息", "en-US": "Get list of all current windows in the system"},
        "get_active": {"zh-CN": "获取当前处于前台活动状态的窗口信息", "en-US": "Get the currently active foreground window"},
        "focus": {"zh-CN": "将指定窗口切换到前台并获取焦点", "en-US": "Bring specified window to foreground and focus"},
        "move_to": {"zh-CN": "将窗口移动到屏幕指定的坐标位置", "en-US": "Move window to specified screen coordinates"},
        "resize": {"zh-CN": "调整窗口的宽度和高度尺寸", "en-US": "Resize window to specified width and height"},
        "minimize": {"zh-CN": "将窗口最小化到任务栏/Dock", "en-US": "Minimize window to taskbar/Dock"},
        "maximize": {"zh-CN": "将窗口最大化占满整个屏幕", "en-US": "Maximize window to fill the screen"},
        "restore": {"zh-CN": "将最小化的窗口恢复到正常显示状态", "en-US": "Restore minimized window to normal state"},
        "close": {"zh-CN": "关闭指定的窗口", "en-US": "Close the specified window"},
        "screenshot": {"zh-CN": "截取指定窗口的屏幕截图", "en-US": "Capture screenshot of specified window"},
        "get_bounds": {"zh-CN": "获取窗口的位置和尺寸信息（x/y/width/height）", "en-US": "Get window position and size (x/y/width/height)"},
        "set_topmost": {"zh-CN": "设置窗口是否始终保持在最顶层显示", "en-US": "Set whether window stays always on top"},
    },
    "hap-mod-usb": {
        "list_devices": {"zh-CN": "枚举系统中所有已连接的 USB 设备，可按厂商/产品ID过滤", "en-US": "Enumerate all connected USB devices with optional VID/PID filter"},
        "open": {"zh-CN": "打开指定的 USB 设备并声明接口，获取操作句柄", "en-US": "Open USB device, claim interface and get operation handle"},
        "close": {"zh-CN": "释放 USB 设备接口并关闭连接句柄", "en-US": "Release USB interface and close device handle"},
        "bulk_transfer_out": {"zh-CN": "向 USB 设备的批量传输端点写入数据", "en-US": "Write data to USB device bulk transfer endpoint"},
        "bulk_transfer_in": {"zh-CN": "从 USB 设备的批量传输端点读取数据", "en-US": "Read data from USB device bulk transfer endpoint"},
        "control_transfer": {"zh-CN": "执行 USB 控制传输（读取或写入设备控制信息）", "en-US": "Execute USB control transfer (read or write device control info)"},
        "get_device_info": {"zh-CN": "获取已打开 USB 设备的详细硬件信息", "en-US": "Get detailed hardware info of opened USB device"},
        "reset_device": {"zh-CN": "重置 USB 设备（等同于重新插拔）", "en-US": "Reset USB device (equivalent to re-plug)"},
    },
    "hap-mod-email": {
        "send": {"zh-CN": "通过 SMTP 协议发送电子邮件，支持HTML正文、抄送和附件", "en-US": "Send email via SMTP with HTML body, CC/BCC and attachments support"},
        "fetch": {"zh-CN": "通过 IMAP 协议获取邮箱中的邮件列表（支持只获取未读）", "en-US": "Fetch email list via IMAP (supports unseen only filter)"},
        "list_folders": {"zh-CN": "获取邮箱中所有文件夹（收件箱、已发送、草稿等）的列表", "en-US": "List all mailbox folders (Inbox, Sent, Drafts, etc.)"},
        "mark_read": {"zh-CN": "将指定邮件标记为已读状态", "en-US": "Mark specified email as read"},
        "delete": {"zh-CN": "从邮箱中永久删除指定的邮件", "en-US": "Permanently delete specified email from mailbox"},
        "download_attachment": {"zh-CN": "下载邮件中的指定附件并保存到本地路径", "en-US": "Download email attachment and save to local path"},
    },
    "hap-mod-ocr": {
        "recognize": {"zh-CN": "对指定图片文件执行光学字符识别（OCR），提取文字内容", "en-US": "Perform OCR on image file to extract text content"},
        "recognize_region": {"zh-CN": "对图片中指定矩形区域执行文字识别", "en-US": "Perform OCR on specified rectangular region of an image"},
        "recognize_base64": {"zh-CN": "对 Base64 编码的图片数据执行文字识别", "en-US": "Perform OCR on Base64 encoded image data"},
        "get_supported_languages": {"zh-CN": "获取当前系统 OCR 引擎支持的语言列表", "en-US": "Get list of languages supported by system OCR engine"},
        "recognize_screen_region": {"zh-CN": "截取屏幕指定区域并执行文字识别", "en-US": "Capture screen region and perform OCR"},
    },
    "hap-mod-scheduler": {
        "create_cron": {"zh-CN": "创建基于 Cron 表达式的定时任务（如每天凌晨3点执行）", "en-US": "Create scheduled task based on cron expression"},
        "create_interval": {"zh-CN": "创建按固定时间间隔重复执行的定时任务", "en-US": "Create task that repeats at fixed interval"},
        "create_timeout": {"zh-CN": "创建在指定延迟后执行一次的延时任务", "en-US": "Create one-shot task that fires after specified delay"},
        "cancel": {"zh-CN": "取消并删除一个已注册的定时任务", "en-US": "Cancel and remove a registered scheduled task"},
        "pause": {"zh-CN": "暂停一个定时任务（不删除，可恢复）", "en-US": "Pause a scheduled task (can be resumed later)"},
        "resume": {"zh-CN": "恢复一个已暂停的定时任务继续执行", "en-US": "Resume a previously paused scheduled task"},
        "list": {"zh-CN": "获取所有已注册的定时任务列表及其状态", "en-US": "List all registered scheduled tasks with their status"},
        "get_next_run": {"zh-CN": "计算指定 Cron 任务的下一次执行时间", "en-US": "Calculate next execution time for a cron task"},
    },
    "hap-mod-bluetooth": {
        "scan_start": {"zh-CN": "启动蓝牙低功耗（BLE）设备扫描，可按名称过滤", "en-US": "Start BLE device scanning with optional name filter"},
        "scan_stop": {"zh-CN": "停止正在进行的蓝牙设备扫描", "en-US": "Stop ongoing Bluetooth device scanning"},
        "connect": {"zh-CN": "连接到指定的 BLE 设备", "en-US": "Connect to specified BLE device"},
        "disconnect": {"zh-CN": "断开与指定 BLE 设备的连接", "en-US": "Disconnect from specified BLE device"},
        "discover_services": {"zh-CN": "发现已连接 BLE 设备上的所有 GATT 服务", "en-US": "Discover all GATT services on connected BLE device"},
        "read_characteristic": {"zh-CN": "读取 BLE 设备指定服务中的特征值数据", "en-US": "Read characteristic value from specified BLE service"},
        "write_characteristic": {"zh-CN": "向 BLE 设备的指定特征写入数据", "en-US": "Write data to specified BLE characteristic"},
        "subscribe": {"zh-CN": "订阅 BLE 特征值变化通知，实时接收数据更新", "en-US": "Subscribe to BLE characteristic notifications"},
        "unsubscribe": {"zh-CN": "取消订阅 BLE 特征值变化通知", "en-US": "Unsubscribe from BLE characteristic notifications"},
        "is_connected": {"zh-CN": "检查与指定 BLE 设备的连接是否仍然活跃", "en-US": "Check if connection to specified BLE device is still active"},
    },
}


def apply_enrichment():
    manifests = sorted(glob.glob(os.path.join(BASE, "hap-mod-*/manifest.json")))
    updated = 0

    for mpath in manifests:
        mod_dir = os.path.basename(os.path.dirname(mpath))
        with open(mpath, 'r', encoding='utf-8') as f:
            manifest = json.load(f)

        changed = False
        enrich = ENRICHMENT.get(mod_dir, {})

        # Add types
        if "types" in enrich and not manifest.get("types"):
            manifest["types"] = enrich["types"]
            changed = True

        # Add constants
        if "constants" in enrich and not manifest.get("constants"):
            manifest["constants"] = enrich["constants"]
            changed = True

        # Add events
        if "events" in enrich and not manifest.get("events"):
            manifest["events"] = enrich["events"]
            changed = True

        # Apply groups
        if "groups" in enrich:
            group_map = {}
            for group_name, fn_names in enrich["groups"].items():
                for fn_name in fn_names:
                    group_map[fn_name] = group_name

            for fn in manifest.get("functions", []):
                if fn["name"] in group_map and not fn.get("group"):
                    fn["group"] = group_map[fn["name"]]
                    changed = True

        # Apply description enrichment
        desc_enrich = DESC_ENRICH.get(mod_dir, {})
        for fn in manifest.get("functions", []):
            if fn["name"] in desc_enrich:
                new_descs = desc_enrich[fn["name"]]
                old_descs = fn.get("descriptions", {}) or {}
                if old_descs.get("zh-CN") != new_descs.get("zh-CN"):
                    fn["descriptions"] = new_descs
                    fn["description"] = new_descs.get("en-US", fn.get("description", ""))
                    changed = True

        if changed:
            with open(mpath, 'w', encoding='utf-8') as f:
                json.dump(manifest, f, ensure_ascii=False, indent=2)
                f.write('\n')
            updated += 1
            print(f"✓ {mod_dir}")

    print(f"\n更新了 {updated} 个 manifest")


if __name__ == "__main__":
    apply_enrichment()
