#!/usr/bin/env python3
"""Mark optional parameters in module descriptors based on code analysis."""
import re, os, json, sys
sys.path.insert(0, os.path.dirname(__file__))
from find_optional_params import find_optional_params

BASE = os.path.join(os.path.dirname(__file__), '..')

def get_default_values():
    """Extract default values from code for optional params."""
    defaults = {}
    for mod_dir in sorted(os.listdir(BASE)):
        if not mod_dir.startswith('hap-mod-'):
            continue
        funcs_path = os.path.join(BASE, mod_dir, 'src', 'funcs.rs')
        if not os.path.exists(funcs_path):
            continue
        with open(funcs_path) as f:
            content = f.read()
        mod = mod_dir.replace('hap-mod-', '')
        
        for m in re.finditer(r'p\.(\w+)\.unwrap_or\((\d+|"[^"]*"|true|false)\)', content):
            param, val = m.group(1), m.group(2)
            fn_match = re.search(r'hap_fn!\s*\(\s*(\w+)\s*,', content[:m.start()][::-1])
            if not fn_match:
                continue
            fn_name = fn_match.group(1)[::-1]
            key = f"{mod}.{fn_name}.{param}"
            defaults[key] = val.strip('"')
        
        for m in re.finditer(r'p\.(\w+)\.unwrap_or_default\(\)', content):
            param = m.group(1)
            fn_match = content[:m.start()].rfind('hap_fn!')
            if fn_match >= 0:
                fn_name_match = re.search(r'hap_fn!\s*\(\s*(\w+)', content[fn_match:])
                if fn_name_match:
                    key = f"{mod}.{fn_name_match.group(1)}.{param}"
                    defaults[key] = ""
    
    return defaults

def process():
    optionals = find_optional_params()
    defaults = get_default_values()
    
    fn_to_optionals = {}
    for fn_key, params in optionals.items():
        parts = fn_key.split('.')
        mod = parts[0]
        rust_fn = parts[1]
        fn_to_optionals[f"{mod}.{rust_fn}"] = set(params)
    
    for mod_dir in sorted(os.listdir(BASE)):
        if not mod_dir.startswith('hap-mod-'):
            continue
        lib_path = os.path.join(BASE, mod_dir, 'src', 'lib.rs')
        if not os.path.exists(lib_path):
            continue
        
        with open(lib_path) as f:
            content = f.read()
        
        json_match = re.search(r'r##?"(\{[\s\S]*?\})"##?', content)
        if not json_match:
            continue
        
        desc = json.loads(json_match.group(1))
        mod = mod_dir.replace('hap-mod-', '')
        modified = False
        
        for fn in desc.get('functions', []):
            symbol = fn.get('symbol', '')
            fn_key = f"{mod}.{symbol}"
            
            if fn_key not in fn_to_optionals:
                continue
            
            opt_params = fn_to_optionals[fn_key]
            
            for param in fn.get('params', []):
                if param['name'] in opt_params:
                    if not param.get('optional'):
                        param['optional'] = True
                        modified = True
                    def_key = f"{mod}.{symbol}.{param['name']}"
                    if def_key in defaults and defaults[def_key]:
                        param['default_value'] = defaults[def_key]
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
            
            new_content = content[:json_match.start()] + new_block + content[json_match.end():]
            with open(lib_path, 'w') as f:
                f.write(new_content)
            
            opt_count = sum(1 for fn in desc['functions'] for p in fn.get('params', []) if p.get('optional'))
            print(f"  {mod}: marked {opt_count} optional params")
        else:
            print(f"  {mod}: no changes")

if __name__ == '__main__':
    process()
