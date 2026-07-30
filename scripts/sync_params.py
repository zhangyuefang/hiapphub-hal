#!/usr/bin/env python3
"""Sync params from Rust structs to module descriptors, marking optional ones."""
import re, os, json, sys
sys.path.insert(0, os.path.dirname(__file__))
from add_i18n import translate_param_desc, PHRASE_MAP

BASE = os.path.join(os.path.dirname(__file__), '..')

RUST_TYPE_MAP = {
    'String': 'string', 'str': 'string', '&str': 'string',
    'i32': 'i32', 'i64': 'i64', 'u32': 'u32', 'u64': 'u64',
    'f32': 'f32', 'f64': 'f64',
    'bool': 'bool',
    'Vec<String>': 'array', 'Vec<u8>': 'array',
    'serde_json::Value': 'object', 'Value': 'object',
    'HashMap<String, String>': 'object', 'HashMap<String, serde_json::Value>': 'object',
}

PARAM_DESC_MAP = {
    'password': ('Password', '密码'),
    'encryption': ('Encryption method', '加密方式'),
    'callback_id': ('Callback ID', '回调 ID'),
    'overwrite': ('Overwrite existing', '是否覆盖已有文件'),
    'compression_level': ('Compression level', '压缩级别'),
    'level': ('Compression level', '压缩级别'),
    'method': ('Compression method', '压缩方法'),
    'compress': ('Compression algorithm', '压缩算法'),
    'volume': ('Volume 0-1', '音量 0-1'),
    'channels': ('Number of channels', '声道数'),
    'sample_rate': ('Sample rate Hz', '采样率(Hz)'),
    'frequency': ('Frequency Hz', '频率(Hz)'),
    'duration_ms': ('Duration in ms', '持续时间(毫秒)'),
    'fade_in_ms': ('Fade in ms', '淡入时长(毫秒)'),
    'fade_out_ms': ('Fade out ms', '淡出时长(毫秒)'),
    'samples': ('Number of samples', '采样数'),
    'metadata': ('Metadata tags', '元数据标签'),
    'error_level': ('Error correction level', '纠错等级'),
    'format': ('Output format', '输出格式'),
    'size': ('Image size px', '图像尺寸(像素)'),
    'height': ('Height px', '高度(像素)'),
    'width': ('Width px', '宽度(像素)'),
    'logo_size_ratio': ('Logo size ratio', 'Logo 大小比例'),
    'plain_text': ('Plain text fallback', '纯文本回退'),
    'formats': ('Format list', '格式列表'),
    'ok_label': ('OK button label', '确认按钮文本'),
    'cancel_label': ('Cancel button label', '取消按钮文本'),
    'default_date': ('Default date', '默认日期'),
    'default_time': ('Default time', '默认时间'),
    'default_value': ('Default value', '默认值'),
    'title': ('Dialog title', '对话框标题'),
    'message': ('Dialog message', '对话框消息'),
    'buttons': ('Button labels', '按钮标签列表'),
    'multiple': ('Allow multiple selection', '是否允许多选'),
    'cancellable': ('Show cancel button', '是否显示取消按钮'),
    'indeterminate': ('Indeterminate progress', '是否不确定进度'),
    'position': ('Position', '位置'),
    'values': ('Valid values', '有效值'),
    'min': ('Minimum value', '最小值'),
    'max': ('Maximum value', '最大值'),
    'rule': ('Condition rule', '条件规则'),
    'style': ('Style object', '样式对象'),
    'delimiter': ('Delimiter character', '分隔符'),
    'font_size': ('Font size pt', '字号(磅)'),
    'line_width': ('Line width', '线宽'),
    'height_mm': ('Height in mm', '高度(毫米)'),
    'width_mm': ('Width in mm', '宽度(毫米)'),
    'start_number': ('Start page number', '起始页码'),
    'fill': ('Fill color', '填充颜色'),
    'opacity': ('Opacity 0-1', '透明度 0-1'),
    'rotation': ('Rotation degrees', '旋转角度'),
    'scale': ('Scale factor', '缩放比例'),
    'page_start': ('Start page', '起始页'),
    'page_end': ('End page', '结束页'),
    'page_index': ('Page index', '页码索引'),
    'pages_per_file': ('Pages per file', '每文件页数'),
    'stdin': ('Standard input', '标准输入'),
    'timeout_ms': ('Timeout in ms', '超时(毫秒)'),
    'signal': ('Signal number', '信号编号'),
    'display_id': ('Display ID', '显示器 ID'),
    'quality': ('Image quality', '图像质量'),
    'buffer_type': ('Buffer type', '缓冲区类型'),
    'baud_rate': ('Baud rate', '波特率'),
    'data_bits': ('Data bits', '数据位'),
    'parity': ('Parity', '校验位'),
    'stop_bits': ('Stop bits', '停止位'),
    'encoding': ('Encoding', '编码'),
    'copies': ('Number of copies', '打印份数'),
    'duplex': ('Duplex printing', '双面打印'),
    'args': ('Launch arguments', '启动参数'),
    'enabled': ('Enable/disable', '启用/禁用'),
    'tx_type': ('Transaction type', '事务类型'),
    'params': ('Bind parameters', '绑定参数'),
    'with_header': ('Include header', '是否包含表头'),
    'has_header': ('Has header row', '是否有表头'),
    'on_conflict': ('On conflict action', '冲突处理'),
    'create': ('Create if not exists', '不存在时创建'),
    'readonly': ('Read only mode', '只读模式'),
    'storage_dir': ('Storage directory', '存储目录'),
    'prefix': ('Key prefix filter', '键前缀过滤'),
    'delta': ('Increment delta', '递增量'),
    'app_id': ('Application ID', '应用 ID'),
    'interval_ms': ('Interval in ms', '间隔(毫秒)'),
    'section': ('Settings section', '设置项'),
    'code': ('Close code', '关闭码'),
    'reason': ('Close reason', '关闭原因'),
}

def parse_struct_fields(content, struct_name):
    """Parse a Rust struct to get field names and types."""
    pattern = rf'pub\s+struct\s+{re.escape(struct_name)}\s*\{{([^}}]+)\}}'
    m = re.search(pattern, content)
    if not m:
        return []
    
    body = m.group(1)
    body = re.sub(r'#\[[\w(,="\s)*]+\]', '', body)
    
    fields = []
    for field_match in re.finditer(r'pub\s+(\w+)\s*:\s*(.+?)(?:,|$)', body):
        name = field_match.group(1)
        rust_type = field_match.group(2).strip()
        
        is_optional = rust_type.startswith('Option<')
        if is_optional:
            inner = re.match(r'Option<(.+)>', rust_type)
            if inner:
                rust_type = inner.group(1)
        
        js_type = RUST_TYPE_MAP.get(rust_type, 'string')
        
        fields.append({
            'name': name,
            'type': js_type,
            'optional': is_optional,
            'rust_type': rust_type,
        })
    
    return fields

def process_module(mod_name):
    lib_path = os.path.join(BASE, f'hap-mod-{mod_name}', 'src', 'lib.rs')
    funcs_path = os.path.join(BASE, f'hap-mod-{mod_name}', 'src', 'funcs.rs')
    
    if not os.path.exists(lib_path) or not os.path.exists(funcs_path):
        return False
    
    with open(lib_path) as f:
        lib_content = f.read()
    with open(funcs_path) as f:
        funcs_content = f.read()
    
    json_match = re.search(r'r##?"(\{[\s\S]*?\})"##?', lib_content)
    if not json_match:
        return False
    
    desc = json.loads(json_match.group(1))
    modified = False
    
    symbol_to_struct = {}
    for m in re.finditer(r'hap_fn!\s*\(\s*(\w+)\s*,\s*(\w+)', funcs_content):
        symbol_to_struct[m.group(1)] = m.group(2)
    
    for fn in desc.get('functions', []):
        symbol = fn.get('symbol', '')
        if symbol not in symbol_to_struct:
            continue
        
        struct_name = symbol_to_struct[symbol]
        struct_fields = parse_struct_fields(funcs_content, struct_name)
        if not struct_fields:
            continue
        
        existing_params = {p['name']: p for p in fn.get('params', [])}
        new_params = []
        
        for field in struct_fields:
            if field['name'] in existing_params:
                param = existing_params[field['name']]
                if field['optional'] and not param.get('optional'):
                    param['optional'] = True
                    modified = True
                new_params.append(param)
            else:
                en_desc, zh_desc = PARAM_DESC_MAP.get(field['name'], (field['name'].replace('_', ' ').title(), translate_param_desc(field['name'].replace('_', ' ').title())))
                param = {
                    'name': field['name'],
                    'type': field['type'],
                    'desc': en_desc,
                    'descs': {'zh-CN': zh_desc, 'en-US': en_desc},
                }
                if field['optional']:
                    param['optional'] = True
                new_params.append(param)
                modified = True
        
        if len(new_params) != len(fn.get('params', [])) or any(p.get('optional') for p in new_params):
            fn['params'] = new_params
            modified = True
    
    if modified:
        new_json = json.dumps(desc, ensure_ascii=False, indent=2)
        old_block = json_match.group(0)
        if old_block.startswith('r##"'):
            new_block = f'r##"{new_json}"##'
        elif '"#' in new_json:
            new_block = f'r##"{new_json}"##'
        else:
            new_block = f'r#"{new_json}"#'
        
        new_content = lib_content[:json_match.start()] + new_block + lib_content[json_match.end():]
        with open(lib_path, 'w') as f:
            f.write(new_content)
        
        total_params = sum(len(fn.get('params', [])) for fn in desc['functions'])
        opt_params = sum(1 for fn in desc['functions'] for p in fn.get('params', []) if p.get('optional'))
        print(f"  {mod_name}: {total_params} params total, {opt_params} optional")
        return True
    
    print(f"  {mod_name}: no changes needed")
    return False

def main():
    changed = 0
    for mod_dir in sorted(os.listdir(BASE)):
        if not mod_dir.startswith('hap-mod-'):
            continue
        mod_name = mod_dir.replace('hap-mod-', '')
        if process_module(mod_name):
            changed += 1
    print(f"\nUpdated: {changed} modules")

if __name__ == '__main__':
    main()
