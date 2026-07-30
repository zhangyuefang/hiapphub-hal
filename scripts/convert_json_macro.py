#!/usr/bin/env python3
"""Convert json! macro modules to raw string literals with i18n."""
import re, os, json, sys
sys.path.insert(0, os.path.dirname(__file__))
from add_i18n import MODULE_OVERVIEWS, translate_desc, translate_param_desc

MODULES = ['clipboard','dialog','keychain','notification','power','shell-ext','shortcut','tray','websocket']

def extract_json_block(content):
    start = content.find('let desc = json!(')
    if start < 0:
        return None, None, None
    brace_start = content.index('{', start + len('let desc = json!('))
    brace_count = 0
    end = brace_start
    for i, c in enumerate(content[brace_start:], brace_start):
        if c == '{': brace_count += 1
        elif c == '}': brace_count -= 1
        if brace_count == 0:
            end = i + 1
            break
    stmt_end = content.index(');', end) + 2
    full_line_end = content.index('\n', stmt_end) if '\n' in content[stmt_end:] else len(content)
    
    next_line = content.find('str_to_c(', stmt_end)
    if next_line >= 0:
        line_end = content.index('\n', next_line) if '\n' in content[next_line:] else len(content)
        stmt_end = line_end
    
    json_str = content[brace_start:end]
    cleaned = re.sub(r',\s*([}\]])', r'\1', json_str)
    return cleaned, start, stmt_end

def add_i18n_to_desc(desc, mod_name):
    short = mod_name.replace('hap-mod-', '')
    
    if short in MODULE_OVERVIEWS:
        ov = MODULE_OVERVIEWS[short]
        desc['overview'] = ov['en-US']
        desc['overviews'] = ov
    
    if desc.get('descriptions') and 'zh-CN' in desc['descriptions']:
        pass
    elif desc.get('description'):
        zh = translate_desc(desc['description'])
        desc['descriptions'] = {"zh-CN": zh, "en-US": desc['description']}
    
    for fn in desc.get('functions', []):
        fn_en = fn.get('description', '') or ''
        if fn_en and not fn.get('descriptions'):
            fn['descriptions'] = {"zh-CN": translate_desc(fn_en), "en-US": fn_en}
        
        for p in fn.get('params', []):
            p_en = p.get('desc', '') or ''
            if p_en and not p.get('descs'):
                p['descs'] = {"zh-CN": translate_param_desc(p_en), "en-US": p_en}
        
        ret = fn.get('returns', {})
        r_en = ret.get('desc', '') or ''
        if r_en and not ret.get('descs'):
            ret['descs'] = {"zh-CN": translate_param_desc(r_en), "en-US": r_en}
    
    return desc

def process_module(mod_name):
    path = os.path.join(os.path.dirname(__file__), '..', f'hap-mod-{mod_name}', 'src', 'lib.rs')
    if not os.path.exists(path):
        print(f"  {mod_name}: file not found")
        return False
    
    with open(path) as f:
        content = f.read()
    
    json_str, start, end = extract_json_block(content)
    if json_str is None:
        print(f"  {mod_name}: no json! block found")
        return False
    
    desc = json.loads(json_str)
    desc = add_i18n_to_desc(desc, f'hap-mod-{mod_name}')
    
    new_json = json.dumps(desc, ensure_ascii=False, indent=2)
    
    has_hash = '"#' in new_json
    if has_hash:
        raw_literal = f'r##"{new_json}"##'
    else:
        raw_literal = f'r#"{new_json}"#'
    
    replacement = f'    str_to_c({raw_literal})'
    
    new_content = content[:start] + replacement + content[end:]
    
    if 'use serde_json::json;' in new_content:
        other_json_usage = False
        rest = new_content.replace('use serde_json::json;', '', 1)
        if 'json!' in rest.split('#[cfg(test)]')[0] if '#[cfg(test)]' in rest else rest:
            other_json_usage = True
        if not other_json_usage:
            new_content = new_content.replace('use serde_json::json;\n', '')
    
    with open(path, 'w') as f:
        f.write(new_content)
    
    print(f"  {mod_name}: OK, {len(desc.get('functions',[]))} functions")
    return True

def main():
    for m in MODULES:
        process_module(m)

if __name__ == '__main__':
    main()
