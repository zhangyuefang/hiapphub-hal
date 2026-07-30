#!/usr/bin/env python3
"""自动为所有短描述的函数生成详细描述"""
import json
import os
import glob

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Module context mapping for auto-generation
MODULE_CONTEXT = {
    "hap-mod-archive": ("压缩包", "archive"),
    "hap-mod-audio": ("音频", "audio"),
    "hap-mod-barcode": ("条码/二维码", "barcode/QR code"),
    "hap-mod-bluetooth": ("蓝牙", "Bluetooth"),
    "hap-mod-browser": ("浏览器", "browser"),
    "hap-mod-clipboard": ("剪贴板", "clipboard"),
    "hap-mod-crypto": ("加密", "crypto"),
    "hap-mod-csv": ("CSV", "CSV"),
    "hap-mod-datetime": ("日期时间", "datetime"),
    "hap-mod-dialog": ("对话框", "dialog"),
    "hap-mod-email": ("邮件", "email"),
    "hap-mod-encoding": ("编码", "encoding"),
    "hap-mod-excel": ("Excel", "Excel"),
    "hap-mod-fs": ("文件系统", "filesystem"),
    "hap-mod-http": ("HTTP", "HTTP"),
    "hap-mod-image": ("图片", "image"),
    "hap-mod-input": ("输入模拟", "input simulation"),
    "hap-mod-keychain": ("钥匙串/密钥存储", "keychain"),
    "hap-mod-log": ("日志", "logging"),
    "hap-mod-net": ("网络", "network"),
    "hap-mod-notification": ("系统通知", "notification"),
    "hap-mod-ocr": ("OCR文字识别", "OCR"),
    "hap-mod-pdf": ("PDF文档", "PDF"),
    "hap-mod-power": ("电源/电池", "power"),
    "hap-mod-process": ("进程", "process"),
    "hap-mod-scheduler": ("定时任务", "scheduler"),
    "hap-mod-screen": ("屏幕", "screen"),
    "hap-mod-serial": ("串口通信", "serial port"),
    "hap-mod-shell-ext": ("系统Shell", "shell"),
    "hap-mod-shortcut": ("全局快捷键", "global shortcut"),
    "hap-mod-sqlite": ("SQLite数据库", "SQLite"),
    "hap-mod-storage": ("持久化存储", "storage"),
    "hap-mod-system": ("系统信息", "system"),
    "hap-mod-tray": ("系统托盘", "system tray"),
    "hap-mod-usb": ("USB设备", "USB"),
    "hap-mod-websocket": ("WebSocket", "WebSocket"),
    "hap-mod-window": ("窗口管理", "window"),
    "hap-mod-xml": ("XML", "XML"),
}

# Function name → detailed description generators
# key patterns: verb_object or just verb
VERB_MAP_ZH = {
    "read": "读取", "write": "写入", "get": "获取", "set": "设置",
    "list": "列出", "create": "创建", "delete": "删除", "remove": "移除",
    "open": "打开", "close": "关闭", "send": "发送", "receive": "接收",
    "start": "启动", "stop": "停止", "pause": "暂停", "resume": "恢复",
    "connect": "连接", "disconnect": "断开", "save": "保存", "load": "加载",
    "copy": "复制", "move": "移动", "rename": "重命名",
    "find": "查找", "search": "搜索", "filter": "过滤", "sort": "排序",
    "show": "显示", "hide": "隐藏", "enable": "启用", "disable": "禁用",
    "add": "添加", "update": "更新", "clear": "清空", "reset": "重置",
    "check": "检查", "validate": "验证", "test": "测试",
    "encode": "编码", "decode": "解码", "encrypt": "加密", "decrypt": "解密",
    "compress": "压缩", "decompress": "解压", "extract": "提取",
    "parse": "解析", "format": "格式化", "convert": "转换",
    "query": "查询", "execute": "执行", "count": "计数",
    "register": "注册", "unregister": "注销",
    "subscribe": "订阅", "unsubscribe": "取消订阅",
    "export": "导出", "import": "导入",
    "lock": "锁定", "unlock": "解锁",
    "scan": "扫描", "detect": "检测", "recognize": "识别",
    "capture": "截取", "record": "录制",
    "play": "播放", "stop": "停止",
    "download": "下载", "upload": "上传",
    "attach": "附加", "detach": "分离",
}

# Common parameter description improvements
PARAM_DESC_MAP = {
    "path": {"zh-CN": "文件路径", "en-US": "File path"},
    "source": {"zh-CN": "源文件路径", "en-US": "Source file path"},
    "dest": {"zh-CN": "目标路径", "en-US": "Destination path"},
    "destination": {"zh-CN": "目标路径", "en-US": "Destination path"},
    "output": {"zh-CN": "输出文件路径", "en-US": "Output file path"},
    "output_path": {"zh-CN": "输出文件保存路径", "en-US": "Output file save path"},
    "input": {"zh-CN": "输入数据", "en-US": "Input data"},
    "content": {"zh-CN": "文本内容", "en-US": "Text content"},
    "text": {"zh-CN": "文本字符串", "en-US": "Text string"},
    "data": {"zh-CN": "数据内容", "en-US": "Data content"},
    "url": {"zh-CN": "URL地址", "en-US": "URL address"},
    "host": {"zh-CN": "主机地址（IP或域名）", "en-US": "Host (IP or domain)"},
    "port": {"zh-CN": "端口号", "en-US": "Port number"},
    "timeout": {"zh-CN": "超时时间（毫秒）", "en-US": "Timeout in milliseconds"},
    "timeout_ms": {"zh-CN": "超时时间（毫秒）", "en-US": "Timeout in milliseconds"},
    "callback_id": {"zh-CN": "异步回调标识符", "en-US": "Async callback identifier"},
    "format": {"zh-CN": "格式类型", "en-US": "Format type"},
    "encoding": {"zh-CN": "字符编码（如utf-8/gbk）", "en-US": "Character encoding (e.g. utf-8/gbk)"},
    "key": {"zh-CN": "键名", "en-US": "Key name"},
    "value": {"zh-CN": "值", "en-US": "Value"},
    "name": {"zh-CN": "名称", "en-US": "Name"},
    "id": {"zh-CN": "唯一标识符", "en-US": "Unique identifier"},
    "index": {"zh-CN": "索引位置（从0开始）", "en-US": "Index position (0-based)"},
    "query": {"zh-CN": "查询条件", "en-US": "Query condition"},
    "sql": {"zh-CN": "SQL语句", "en-US": "SQL statement"},
    "password": {"zh-CN": "密码", "en-US": "Password"},
    "username": {"zh-CN": "用户名", "en-US": "Username"},
    "title": {"zh-CN": "标题", "en-US": "Title"},
    "message": {"zh-CN": "消息内容", "en-US": "Message content"},
    "width": {"zh-CN": "宽度（像素）", "en-US": "Width in pixels"},
    "height": {"zh-CN": "高度（像素）", "en-US": "Height in pixels"},
    "x": {"zh-CN": "X坐标", "en-US": "X coordinate"},
    "y": {"zh-CN": "Y坐标", "en-US": "Y coordinate"},
    "recursive": {"zh-CN": "是否递归处理子目录", "en-US": "Whether to process subdirectories recursively"},
    "overwrite": {"zh-CN": "如果目标已存在是否覆盖", "en-US": "Whether to overwrite if target exists"},
    "selector": {"zh-CN": "CSS选择器", "en-US": "CSS selector"},
    "expression": {"zh-CN": "表达式", "en-US": "Expression"},
    "algorithm": {"zh-CN": "算法名称", "en-US": "Algorithm name"},
    "quality": {"zh-CN": "质量值（0-100）", "en-US": "Quality value (0-100)"},
    "volume": {"zh-CN": "音量（0.0-1.0）", "en-US": "Volume (0.0-1.0)"},
    "duration": {"zh-CN": "持续时间", "en-US": "Duration"},
    "delay_ms": {"zh-CN": "延迟时间（毫秒）", "en-US": "Delay in milliseconds"},
    "interval_ms": {"zh-CN": "间隔时间（毫秒）", "en-US": "Interval in milliseconds"},
    "limit": {"zh-CN": "最大数量限制", "en-US": "Maximum count limit"},
    "offset": {"zh-CN": "起始偏移量", "en-US": "Starting offset"},
    "page": {"zh-CN": "页码", "en-US": "Page number"},
    "filter": {"zh-CN": "过滤条件", "en-US": "Filter condition"},
    "pattern": {"zh-CN": "匹配模式", "en-US": "Match pattern"},
    "level": {"zh-CN": "级别", "en-US": "Level"},
    "options": {"zh-CN": "配置选项", "en-US": "Configuration options"},
    "config": {"zh-CN": "配置参数", "en-US": "Configuration parameters"},
    "headers": {"zh-CN": "HTTP请求头", "en-US": "HTTP headers"},
    "body": {"zh-CN": "请求体内容", "en-US": "Request body content"},
    "method": {"zh-CN": "HTTP方法（GET/POST/PUT/DELETE等）", "en-US": "HTTP method"},
    "save_path": {"zh-CN": "保存文件的目标路径", "en-US": "Target path to save file"},
}


def generate_fn_desc(fn_name, mod_name, params):
    """Based on function name and module context, generate a detailed description"""
    ctx_zh, ctx_en = MODULE_CONTEXT.get(mod_name, ("", ""))

    # Try to break fn_name into verb + object
    parts = fn_name.split("_")

    # Special patterns
    if fn_name.startswith("is_") or fn_name.startswith("has_"):
        obj = "_".join(parts[1:]).replace("_", " ")
        return {
            "zh-CN": f"检查{ctx_zh}是否{obj}",
            "en-US": f"Check if {ctx_en} {obj}",
        }

    if fn_name.startswith("get_"):
        obj = "_".join(parts[1:]).replace("_", " ")
        return {
            "zh-CN": f"获取{ctx_zh}的{obj}信息",
            "en-US": f"Get {obj} info of {ctx_en}",
        }

    if fn_name.startswith("set_"):
        obj = "_".join(parts[1:]).replace("_", " ")
        return {
            "zh-CN": f"设置{ctx_zh}的{obj}属性",
            "en-US": f"Set {obj} property of {ctx_en}",
        }

    if fn_name.startswith("on_"):
        event = "_".join(parts[1:]).replace("_", " ")
        return {
            "zh-CN": f"监听{ctx_zh}的{event}事件",
            "en-US": f"Listen for {event} event on {ctx_en}",
        }

    if fn_name.startswith("off_"):
        event = "_".join(parts[1:]).replace("_", " ")
        return {
            "zh-CN": f"取消监听{ctx_zh}的{event}事件",
            "en-US": f"Stop listening for {event} event on {ctx_en}",
        }

    # Generic verb mapping
    verb = parts[0] if parts else ""
    obj = " ".join(parts[1:]) if len(parts) > 1 else ""

    zh_verb = VERB_MAP_ZH.get(verb, verb)
    if obj:
        obj_zh = obj.replace("_", " ")
        return {
            "zh-CN": f"{zh_verb}{ctx_zh}的{obj_zh}",
            "en-US": f"{verb.capitalize()} {obj} of {ctx_en}",
        }
    else:
        return {
            "zh-CN": f"{zh_verb}{ctx_zh}",
            "en-US": f"{verb.capitalize()} {ctx_en}",
        }


def improve_param_desc(param_name, current_desc):
    """Improve parameter description if it's too short"""
    if param_name in PARAM_DESC_MAP:
        return PARAM_DESC_MAP[param_name]
    return None


def process_all():
    manifests = sorted(glob.glob(os.path.join(BASE, "hap-mod-*/manifest.json")))
    total_improved = 0

    for mpath in manifests:
        mod_dir = os.path.basename(os.path.dirname(mpath))
        with open(mpath, 'r', encoding='utf-8') as f:
            manifest = json.load(f)

        changed = False
        for fn in manifest.get("functions", []):
            # Improve function description if too short
            desc_zh = fn.get("descriptions", {}).get("zh-CN", fn.get("description", ""))
            if len(desc_zh) < 10:
                new_desc = generate_fn_desc(fn["name"], mod_dir, fn.get("params", []))
                # Only update if the generated desc is actually better
                if len(new_desc["zh-CN"]) > len(desc_zh):
                    fn["descriptions"] = new_desc
                    fn["description"] = new_desc["en-US"]
                    changed = True
                    total_improved += 1

            # Improve parameter descriptions
            for param in fn.get("params", []):
                param_desc = param.get("descs", {}).get("zh-CN", param.get("desc", ""))
                if len(param_desc) <= 3:  # Single character descs like "路径", "内容"
                    new_param_desc = improve_param_desc(param["name"], param_desc)
                    if new_param_desc:
                        param["descs"] = new_param_desc
                        param["desc"] = new_param_desc["en-US"]
                        changed = True

            # Improve return description
            returns = fn.get("returns", {})
            ret_desc = returns.get("descs", {}).get("zh-CN", returns.get("desc", ""))
            if ret_desc and len(ret_desc) <= 3:
                # Generic return improvements
                ret_type = returns.get("type", "")
                if ret_type == "boolean":
                    returns["descs"] = {"zh-CN": "操作是否成功", "en-US": "Whether operation succeeded"}
                    returns["desc"] = "Whether operation succeeded"
                    fn["returns"] = returns
                    changed = True
                elif ret_type == "string":
                    returns["descs"] = {"zh-CN": "返回的字符串结果", "en-US": "Returned string result"}
                    returns["desc"] = "Returned string result"
                    fn["returns"] = returns
                    changed = True

        if changed:
            with open(mpath, 'w', encoding='utf-8') as f:
                json.dump(manifest, f, ensure_ascii=False, indent=2)
                f.write('\n')
            print(f"✓ {mod_dir}")

    print(f"\n共改进 {total_improved} 个函数描述")


if __name__ == "__main__":
    process_all()
