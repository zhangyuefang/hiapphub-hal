#!/usr/bin/env python3
"""批量改进所有支持库的函数描述、参数描述和返回值描述"""
import json
import os
import glob

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Format: { module_name: { fn_name: { "desc": {"zh-CN":..,"en-US":..}, "params": {param_name: {"zh-CN":..,"en-US":..}}, "returns": {"zh-CN":..,"en-US":..} } } }
IMPROVEMENTS = {
    "hap-mod-image": {
        "info": {"params": {"path": {"zh-CN": "图片文件的路径", "en-US": "Path to the image file"}}, "returns": {"zh-CN": "包含宽高、格式、色彩模式等的图像信息对象", "en-US": "Image info object with dimensions, format, color mode"}},
        "resize": {"desc": {"zh-CN": "按指定的目标宽高缩放图片文件，可选择不同的插值算法", "en-US": "Resize image to specified dimensions with selectable interpolation"}, "params": {"path": {"zh-CN": "源图片文件路径", "en-US": "Source image path"}, "width": {"zh-CN": "目标宽度（像素）", "en-US": "Target width in pixels"}, "height": {"zh-CN": "目标高度（像素）", "en-US": "Target height in pixels"}}, "returns": {"zh-CN": "操作是否成功", "en-US": "Whether operation succeeded"}},
        "crop": {"desc": {"zh-CN": "从图片中裁剪出指定坐标和尺寸的矩形区域", "en-US": "Crop a rectangular region from the image at specified coordinates"}, "params": {"path": {"zh-CN": "源图片文件路径", "en-US": "Source image path"}, "x": {"zh-CN": "裁剪起始X坐标（像素）", "en-US": "Crop start X coordinate"}, "y": {"zh-CN": "裁剪起始Y坐标（像素）", "en-US": "Crop start Y coordinate"}}, "returns": {"zh-CN": "操作是否成功", "en-US": "Whether operation succeeded"}},
        "rotate": {"params": {"path": {"zh-CN": "源图片文件路径", "en-US": "Source image path"}, "degrees": {"zh-CN": "旋转角度（顺时针，支持90/180/270或任意值）", "en-US": "Rotation degrees (clockwise)"}, "output": {"zh-CN": "输出文件路径（不指定则覆盖原文件）", "en-US": "Output path (overwrites source if not specified)"}}, "returns": {"zh-CN": "操作是否成功", "en-US": "Whether operation succeeded"}},
        "flip": {"desc": {"zh-CN": "将图片沿水平轴或垂直轴翻转", "en-US": "Flip image along horizontal or vertical axis"}, "params": {"direction": {"zh-CN": "翻转方向：h=水平翻转，v=垂直翻转", "en-US": "Flip direction: h=horizontal, v=vertical"}}},
        "convert": {"params": {"path": {"zh-CN": "源图片文件路径", "en-US": "Source image path"}, "format": {"zh-CN": "目标格式（png/jpeg/webp/bmp/gif）", "en-US": "Target format (png/jpeg/webp/bmp/gif)"}}},
        "compress": {"params": {"path": {"zh-CN": "要压缩的图片文件路径", "en-US": "Image file to compress"}, "quality": {"zh-CN": "压缩质量（1-100，越低压缩率越高）", "en-US": "Quality (1-100, lower = more compression)"}}},
        "watermark": {"params": {"path": {"zh-CN": "要添加水印的图片路径", "en-US": "Image to add watermark to"}, "text": {"zh-CN": "水印文字内容", "en-US": "Watermark text content"}}},
        "thumbnail": {"params": {"path": {"zh-CN": "源图片文件路径", "en-US": "Source image path"}, "max_size": {"zh-CN": "缩略图最大边长（像素）", "en-US": "Max thumbnail dimension in pixels"}}},
    },
    "hap-mod-fs": {
        "read_text_file": {"desc": {"zh-CN": "读取文本文件的完整内容并返回字符串", "en-US": "Read complete text file content and return as string"}, "params": {"path": {"zh-CN": "要读取的文本文件路径", "en-US": "Path to the text file to read"}}, "returns": {"zh-CN": "文件的完整文本内容", "en-US": "Complete text content of the file"}},
        "write_text_file": {"desc": {"zh-CN": "将文本内容写入文件，如果文件已存在则覆盖", "en-US": "Write text content to file, overwriting if exists"}, "params": {"path": {"zh-CN": "要写入的目标文件路径", "en-US": "Target file path to write"}, "content": {"zh-CN": "要写入的文本内容", "en-US": "Text content to write"}}, "returns": {"zh-CN": "写入是否成功", "en-US": "Whether write succeeded"}},
        "append_text_file": {"desc": {"zh-CN": "在文件末尾追加文本内容，不覆盖已有内容", "en-US": "Append text content to end of file without overwriting"}, "params": {"path": {"zh-CN": "要追加内容的文件路径", "en-US": "File path to append to"}, "content": {"zh-CN": "要追加的文本内容", "en-US": "Text content to append"}}, "returns": {"zh-CN": "追加是否成功", "en-US": "Whether append succeeded"}},
        "read_binary": {"params": {"path": {"zh-CN": "要读取的二进制文件路径", "en-US": "Path to binary file to read"}}, "returns": {"zh-CN": "文件内容的 Base64 编码字符串", "en-US": "Base64 encoded string of file content"}},
        "write_binary": {"desc": {"zh-CN": "将 Base64 编码的数据解码后写入二进制文件", "en-US": "Decode Base64 data and write to binary file"}, "params": {"path": {"zh-CN": "要写入的目标文件路径", "en-US": "Target file path"}, "data": {"zh-CN": "Base64 编码的二进制数据", "en-US": "Base64 encoded binary data"}}, "returns": {"zh-CN": "写入是否成功", "en-US": "Whether write succeeded"}},
        "copy_file": {"desc": {"zh-CN": "复制文件到目标路径，保留文件属性和权限", "en-US": "Copy file to target path, preserving attributes"}, "params": {"source": {"zh-CN": "源文件路径", "en-US": "Source file path"}, "dest": {"zh-CN": "目标文件路径", "en-US": "Destination file path"}}},
        "move_file": {"desc": {"zh-CN": "移动或重命名文件到新路径", "en-US": "Move or rename file to new path"}, "params": {"source": {"zh-CN": "源文件路径", "en-US": "Source file path"}, "dest": {"zh-CN": "目标文件路径", "en-US": "Destination file path"}}},
        "delete_file": {"desc": {"zh-CN": "永久删除指定的文件（不可恢复，不放入回收站）", "en-US": "Permanently delete file (not recoverable, not sent to trash)"}, "params": {"path": {"zh-CN": "要删除的文件路径", "en-US": "File path to delete"}}},
        "exists": {"desc": {"zh-CN": "检查文件或目录在指定路径是否存在", "en-US": "Check if file or directory exists at path"}, "params": {"path": {"zh-CN": "要检查的文件或目录路径", "en-US": "Path to check"}}, "returns": {"zh-CN": "路径是否存在（true/false）", "en-US": "Whether path exists"}},
        "stat": {"desc": {"zh-CN": "获取文件或目录的详细元信息（大小、创建时间、修改时间、权限等）", "en-US": "Get detailed file/directory metadata (size, times, permissions)"}, "params": {"path": {"zh-CN": "要查询的文件或目录路径", "en-US": "Path to query"}}},
        "mkdir": {"desc": {"zh-CN": "创建目录，如果父目录不存在则递归创建整个路径", "en-US": "Create directory, recursively creating parent dirs if needed"}, "params": {"path": {"zh-CN": "要创建的目录路径", "en-US": "Directory path to create"}}},
        "list_dir": {"desc": {"zh-CN": "列出指定目录下的所有文件和子目录名称", "en-US": "List all files and subdirectories in a directory"}, "params": {"path": {"zh-CN": "要列出内容的目录路径", "en-US": "Directory path to list"}}},
    },
    "hap-mod-crypto": {
        "hash": {"desc": {"zh-CN": "使用指定算法计算字符串数据的哈希摘要值", "en-US": "Compute hash digest of string data with specified algorithm"}, "params": {"algorithm": {"zh-CN": "哈希算法名称（md5/sha1/sha256/sha512）", "en-US": "Hash algorithm (md5/sha1/sha256/sha512)"}, "data": {"zh-CN": "要计算哈希的字符串数据", "en-US": "String data to hash"}}, "returns": {"zh-CN": "十六进制编码的哈希值字符串", "en-US": "Hex-encoded hash string"}},
        "hash_file": {"desc": {"zh-CN": "计算文件内容的哈希摘要值（适用于大文件）", "en-US": "Compute hash digest of file content (suitable for large files)"}, "params": {"algorithm": {"zh-CN": "哈希算法名称", "en-US": "Hash algorithm"}, "path": {"zh-CN": "要计算哈希的文件路径", "en-US": "File path to hash"}}, "returns": {"zh-CN": "十六进制编码的文件哈希值", "en-US": "Hex-encoded file hash"}},
        "hmac": {"desc": {"zh-CN": "使用密钥和指定算法计算 HMAC 消息认证码", "en-US": "Compute HMAC message authentication code with key and algorithm"}, "params": {"algorithm": {"zh-CN": "HMAC 算法（sha256/sha512）", "en-US": "HMAC algorithm"}, "key": {"zh-CN": "HMAC 密钥字符串", "en-US": "HMAC key string"}, "data": {"zh-CN": "要认证的数据", "en-US": "Data to authenticate"}}, "returns": {"zh-CN": "十六进制编码的 HMAC 值", "en-US": "Hex-encoded HMAC value"}},
        "random_bytes": {"desc": {"zh-CN": "生成指定长度的密码学安全随机字节序列", "en-US": "Generate cryptographically secure random bytes"}, "params": {"length": {"zh-CN": "要生成的随机字节数量", "en-US": "Number of random bytes to generate"}}, "returns": {"zh-CN": "十六进制编码的随机字节串", "en-US": "Hex-encoded random bytes"}},
        "generate_uuid": {"desc": {"zh-CN": "生成一个符合 RFC 4122 标准的 UUID v4 随机标识符", "en-US": "Generate RFC 4122 compliant UUID v4 random identifier"}, "returns": {"zh-CN": "UUID 字符串（如 550e8400-e29b-41d4-a716-446655440000）", "en-US": "UUID string"}},
    },
    "hap-mod-audio": {
        "play": {"desc": {"zh-CN": "播放本地音频文件，支持调节音量和播放完成回调", "en-US": "Play local audio file with volume control and completion callback"}, "params": {"source": {"zh-CN": "本地音频文件路径", "en-US": "Local audio file path"}, "volume": {"zh-CN": "播放音量（0.0-1.0，1.0为最大音量）", "en-US": "Volume level (0.0-1.0)"}, "callback_id": {"zh-CN": "播放完成时的回调标识", "en-US": "Callback ID for playback completion"}}},
        "play_url": {"desc": {"zh-CN": "播放网络音频流URL，支持HTTP/HTTPS音频资源", "en-US": "Play audio from network URL (HTTP/HTTPS)"}, "params": {"url": {"zh-CN": "音频资源的网络URL地址", "en-US": "Audio resource URL"}}},
        "pause": {"desc": {"zh-CN": "暂停当前正在播放的音频（可通过 resume 恢复）", "en-US": "Pause currently playing audio (can be resumed)"}, "params": {"player_id": {"zh-CN": "要暂停的播放器实例ID", "en-US": "Player instance ID to pause"}}},
    },
    "hap-mod-excel": {
        "open": {"desc": {"zh-CN": "打开一个Excel文件（.xlsx/.xls），获取工作簿操作句柄", "en-US": "Open Excel file (.xlsx/.xls) and get workbook handle"}, "params": {"path": {"zh-CN": "Excel文件路径", "en-US": "Excel file path"}}},
        "create": {"desc": {"zh-CN": "创建一个新的空白Excel工作簿文件", "en-US": "Create a new empty Excel workbook"}, "params": {"path": {"zh-CN": "新文件的保存路径", "en-US": "Path for new file"}}},
        "save": {"desc": {"zh-CN": "保存对工作簿的所有修改到磁盘", "en-US": "Save all workbook changes to disk"}},
        "close": {"desc": {"zh-CN": "关闭工作簿并释放资源（未保存的修改将丢失）", "en-US": "Close workbook and release resources (unsaved changes lost)"}},
    },
    "hap-mod-net": {
        "tcp_connect": {"desc": {"zh-CN": "建立到远程主机的TCP连接，返回连接句柄", "en-US": "Establish TCP connection to remote host"}, "params": {"host": {"zh-CN": "远程主机地址（IP或域名）", "en-US": "Remote host (IP or domain)"}, "port": {"zh-CN": "远程端口号", "en-US": "Remote port number"}}},
        "tcp_send": {"desc": {"zh-CN": "通过已建立的TCP连接发送数据", "en-US": "Send data over established TCP connection"}},
        "tcp_close": {"desc": {"zh-CN": "关闭TCP连接并释放资源", "en-US": "Close TCP connection and release resources"}},
        "udp_bind": {"desc": {"zh-CN": "创建UDP套接字并绑定到本地端口", "en-US": "Create UDP socket and bind to local port"}},
        "udp_send": {"desc": {"zh-CN": "通过UDP发送数据报到指定地址和端口", "en-US": "Send UDP datagram to specified address and port"}},
        "dns_lookup": {"desc": {"zh-CN": "解析域名获取对应的IP地址列表", "en-US": "Resolve domain name to list of IP addresses"}, "params": {"host": {"zh-CN": "要解析的域名", "en-US": "Domain to resolve"}}},
    },
    "hap-mod-sqlite": {
        "open": {"desc": {"zh-CN": "打开或创建一个SQLite数据库文件，获取数据库连接句柄", "en-US": "Open or create SQLite database file and get connection handle"}, "params": {"path": {"zh-CN": "数据库文件路径（不存在则自动创建）", "en-US": "Database file path (created if not exists)"}}},
        "close": {"desc": {"zh-CN": "关闭数据库连接并释放所有相关资源", "en-US": "Close database connection and release resources"}},
        "execute": {"desc": {"zh-CN": "执行SQL语句（INSERT/UPDATE/DELETE/DDL），返回受影响行数", "en-US": "Execute SQL statement, return affected row count"}, "params": {"sql": {"zh-CN": "要执行的SQL语句", "en-US": "SQL statement to execute"}}},
        "query": {"desc": {"zh-CN": "执行SQL查询并返回结果集的所有行", "en-US": "Execute SQL query and return all result rows"}, "params": {"sql": {"zh-CN": "SELECT查询语句", "en-US": "SELECT query"}}},
    },
    "hap-mod-storage": {
        "get": {"desc": {"zh-CN": "根据键名从持久化存储中读取对应的值", "en-US": "Read value from persistent storage by key"}, "params": {"key": {"zh-CN": "存储键名", "en-US": "Storage key"}}},
        "set": {"desc": {"zh-CN": "将键值对写入持久化存储（已存在则覆盖）", "en-US": "Write key-value pair to persistent storage (overwrites if exists)"}, "params": {"key": {"zh-CN": "存储键名", "en-US": "Storage key"}, "value": {"zh-CN": "要存储的值（支持字符串、数字、布尔、对象）", "en-US": "Value to store (string/number/boolean/object)"}}},
        "remove": {"desc": {"zh-CN": "从存储中删除指定键名的数据", "en-US": "Remove data for specified key from storage"}, "params": {"key": {"zh-CN": "要删除的键名", "en-US": "Key to remove"}}},
        "has": {"desc": {"zh-CN": "检查存储中是否存在指定的键", "en-US": "Check if specified key exists in storage"}},
        "keys": {"desc": {"zh-CN": "获取存储中所有键名的列表", "en-US": "Get list of all keys in storage"}},
        "clear": {"desc": {"zh-CN": "清空当前命名空间下的所有存储数据", "en-US": "Clear all stored data in current namespace"}},
    },
    "hap-mod-dialog": {
        "alert": {"desc": {"zh-CN": "显示一个包含消息和确定按钮的提示对话框", "en-US": "Show alert dialog with message and OK button"}, "params": {"title": {"zh-CN": "对话框标题", "en-US": "Dialog title"}, "message": {"zh-CN": "要显示的消息内容", "en-US": "Message to display"}}},
        "confirm": {"desc": {"zh-CN": "显示带有确认和取消按钮的确认对话框，返回用户选择", "en-US": "Show confirm dialog with OK/Cancel, return user choice"}, "params": {"title": {"zh-CN": "对话框标题", "en-US": "Dialog title"}, "message": {"zh-CN": "要显示的确认消息", "en-US": "Confirmation message"}}},
        "prompt": {"desc": {"zh-CN": "显示带有文本输入框的对话框，让用户输入内容", "en-US": "Show dialog with text input field for user input"}, "params": {"title": {"zh-CN": "对话框标题", "en-US": "Dialog title"}, "message": {"zh-CN": "输入提示消息", "en-US": "Input prompt message"}}},
        "open_file": {"desc": {"zh-CN": "打开系统文件选择对话框，让用户选择一个文件", "en-US": "Open system file picker dialog to select a file"}},
        "open_files": {"desc": {"zh-CN": "打开系统文件选择对话框，允许用户选择多个文件", "en-US": "Open file picker allowing multiple file selection"}},
        "open_folder": {"desc": {"zh-CN": "打开系统文件夹选择对话框，让用户选择一个目录", "en-US": "Open folder picker dialog to select a directory"}},
        "save_file": {"desc": {"zh-CN": "打开系统保存文件对话框，让用户选择保存位置", "en-US": "Open save file dialog to choose save location"}},
    },
    "hap-mod-clipboard": {
        "read_text": {"desc": {"zh-CN": "从系统剪贴板读取文本内容", "en-US": "Read text content from system clipboard"}, "returns": {"zh-CN": "剪贴板中的文本字符串", "en-US": "Text string from clipboard"}},
        "write_text": {"desc": {"zh-CN": "将文本内容写入系统剪贴板（覆盖原有内容）", "en-US": "Write text content to system clipboard (replaces existing)"}, "params": {"text": {"zh-CN": "要写入剪贴板的文本内容", "en-US": "Text to write to clipboard"}}},
        "clear": {"desc": {"zh-CN": "清空系统剪贴板的所有内容", "en-US": "Clear all content from system clipboard"}},
        "read_image": {"desc": {"zh-CN": "从剪贴板读取图片数据（Base64编码PNG）", "en-US": "Read image data from clipboard as Base64 PNG"}},
        "write_image": {"desc": {"zh-CN": "将图片文件或Base64数据写入剪贴板", "en-US": "Write image file or Base64 data to clipboard"}},
    },
    "hap-mod-process": {
        "list": {"desc": {"zh-CN": "获取系统中所有正在运行的进程列表（PID、名称、CPU、内存等）", "en-US": "Get list of all running processes (PID, name, CPU, memory)"}, "returns": {"zh-CN": "进程信息数组", "en-US": "Array of process info"}},
        "get_info": {"desc": {"zh-CN": "获取指定PID进程的详细运行信息", "en-US": "Get detailed info for process with specified PID"}, "params": {"pid": {"zh-CN": "进程ID", "en-US": "Process ID"}}},
        "kill": {"desc": {"zh-CN": "向指定PID的进程发送终止信号", "en-US": "Send kill signal to process with specified PID"}, "params": {"pid": {"zh-CN": "要终止的进程ID", "en-US": "Process ID to kill"}}},
        "spawn": {"desc": {"zh-CN": "启动一个新的子进程并返回其PID", "en-US": "Spawn a new child process and return its PID"}, "params": {"command": {"zh-CN": "要执行的命令", "en-US": "Command to execute"}, "args": {"zh-CN": "命令参数列表", "en-US": "Command arguments"}}},
        "exec": {"desc": {"zh-CN": "执行命令并等待完成，返回标准输出和退出码", "en-US": "Execute command, wait for completion, return stdout and exit code"}, "params": {"command": {"zh-CN": "要执行的命令", "en-US": "Command to execute"}}},
    },
    "hap-mod-system": {
        "os_info": {"desc": {"zh-CN": "获取操作系统的详细信息（名称、版本、架构等）", "en-US": "Get OS details (name, version, architecture)"}, "returns": {"zh-CN": "操作系统信息对象", "en-US": "OS info object"}},
        "cpu_info": {"desc": {"zh-CN": "获取CPU的硬件信息（型号、核心数、频率等）", "en-US": "Get CPU hardware info (model, cores, frequency)"}, "returns": {"zh-CN": "CPU信息对象", "en-US": "CPU info object"}},
        "memory_info": {"desc": {"zh-CN": "获取系统内存使用情况（总量、已用、可用）", "en-US": "Get memory usage (total, used, available)"}, "returns": {"zh-CN": "内存信息对象", "en-US": "Memory info object"}},
        "disk_info": {"desc": {"zh-CN": "获取所有磁盘分区的容量和使用情况", "en-US": "Get capacity and usage for all disk partitions"}, "returns": {"zh-CN": "磁盘分区信息数组", "en-US": "Array of disk partition info"}},
        "hostname": {"desc": {"zh-CN": "获取当前计算机的主机名", "en-US": "Get current computer hostname"}, "returns": {"zh-CN": "主机名字符串", "en-US": "Hostname string"}},
        "username": {"desc": {"zh-CN": "获取当前登录用户的用户名", "en-US": "Get current logged-in username"}, "returns": {"zh-CN": "用户名字符串", "en-US": "Username string"}},
    },
}


def apply_improvements():
    manifests = sorted(glob.glob(os.path.join(BASE, "hap-mod-*/manifest.json")))
    updated = 0

    for mpath in manifests:
        mod_dir = os.path.basename(os.path.dirname(mpath))
        improvements = IMPROVEMENTS.get(mod_dir)
        if not improvements:
            continue

        with open(mpath, 'r', encoding='utf-8') as f:
            manifest = json.load(f)

        changed = False
        for fn in manifest.get("functions", []):
            fn_imp = improvements.get(fn["name"])
            if not fn_imp:
                continue

            # Update function description
            if "desc" in fn_imp:
                if fn.get("descriptions", {}).get("zh-CN") != fn_imp["desc"].get("zh-CN"):
                    fn["descriptions"] = fn_imp["desc"]
                    fn["description"] = fn_imp["desc"].get("en-US", "")
                    changed = True

            # Update parameter descriptions
            if "params" in fn_imp:
                for param in fn.get("params", []):
                    if param["name"] in fn_imp["params"]:
                        new_descs = fn_imp["params"][param["name"]]
                        if param.get("descs", {}).get("zh-CN") != new_descs.get("zh-CN"):
                            param["descs"] = new_descs
                            param["desc"] = new_descs.get("en-US", "")
                            changed = True

            # Update return descriptions
            if "returns" in fn_imp:
                returns = fn.get("returns", {})
                new_descs = fn_imp["returns"]
                if returns.get("descs", {}).get("zh-CN") != new_descs.get("zh-CN"):
                    returns["descs"] = new_descs
                    returns["desc"] = new_descs.get("en-US", "")
                    fn["returns"] = returns
                    changed = True

        if changed:
            with open(mpath, 'w', encoding='utf-8') as f:
                json.dump(manifest, f, ensure_ascii=False, indent=2)
                f.write('\n')
            updated += 1
            print(f"✓ {mod_dir}")

    print(f"\n更新了 {updated} 个 manifest 的描述")


if __name__ == "__main__":
    apply_improvements()
