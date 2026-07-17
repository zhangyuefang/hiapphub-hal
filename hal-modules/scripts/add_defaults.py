#!/usr/bin/env python3
"""Extract default values from Rust code and add to descriptors."""
import re, os, json

BASE = os.path.join(os.path.dirname(__file__), '..')

def extract_defaults_from_code(funcs_content):
    """Extract default values from unwrap_or patterns."""
    defaults = {}
    
    current_fn = None
    for line in funcs_content.split('\n'):
        fn_match = re.search(r'hap_fn!\s*\(\s*(\w+)', line)
        if fn_match:
            current_fn = fn_match.group(1)
        
        if current_fn:
            for m in re.finditer(r'p\.(\w+)\.unwrap_or\((\d+(?:\.\d+)?)\)', line):
                defaults.setdefault(current_fn, {})[m.group(1)] = m.group(2)
            for m in re.finditer(r'p\.(\w+)\.unwrap_or\("([^"]*)"\s*\.to_string\(\)\)', line):
                defaults.setdefault(current_fn, {})[m.group(1)] = f'"{m.group(2)}"'
            for m in re.finditer(r'p\.(\w+)\.unwrap_or\((true|false)\)', line):
                defaults.setdefault(current_fn, {})[m.group(1)] = m.group(2)
            for m in re.finditer(r'p\.(\w+)\.unwrap_or_default\(\)', line):
                defaults.setdefault(current_fn, {})[m.group(1)] = '""'
            for m in re.finditer(r'p\.(\w+)\.clone\(\)\.unwrap_or\("([^"]*)"\s*\.to_string\(\)\)', line):
                defaults.setdefault(current_fn, {})[m.group(1)] = f'"{m.group(2)}"'
            for m in re.finditer(r'p\.(\w+)\.clone\(\)\.unwrap_or_default\(\)', line):
                defaults.setdefault(current_fn, {})[m.group(1)] = '""'
            for m in re.finditer(r'p\.(\w+)\.unwrap_or\((\d+)\s+as\s+\w+\)', line):
                defaults.setdefault(current_fn, {})[m.group(1)] = m.group(2)
    
    return defaults

def process():
    total_added = 0
    
    for mod_dir in sorted(os.listdir(BASE)):
        if not mod_dir.startswith('hap-mod-'):
            continue
        
        mod_name = mod_dir.replace('hap-mod-', '')
        lib_path = os.path.join(BASE, mod_dir, 'src', 'lib.rs')
        funcs_path = os.path.join(BASE, mod_dir, 'src', 'funcs.rs')
        
        if not os.path.exists(funcs_path):
            continue
        
        with open(lib_path) as f:
            lib_content = f.read()
        with open(funcs_path) as f:
            funcs_content = f.read()
        
        json_match = re.search(r'r##?"(\{[\s\S]*?\})"##?', lib_content)
        if not json_match:
            continue
        
        desc = json.loads(json_match.group(1))
        defaults = extract_defaults_from_code(funcs_content)
        modified = False
        count = 0
        
        for fn in desc.get('functions', []):
            symbol = fn.get('symbol', '')
            if symbol not in defaults:
                continue
            
            fn_defaults = defaults[symbol]
            for param in fn.get('params', []):
                if param['name'] in fn_defaults and not param.get('default_value'):
                    val = fn_defaults[param['name']]
                    if val == '""':
                        continue
                    param['default_value'] = val
                    modified = True
                    count += 1
        
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
            
            print(f"  {mod_name}: {count} default values added")
            total_added += count
        else:
            pass
    
    print(f"\nTotal: {total_added} default values added")

if __name__ == '__main__':
    process()
