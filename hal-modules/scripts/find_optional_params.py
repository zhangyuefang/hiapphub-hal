#!/usr/bin/env python3
"""Analyze function implementations to find optional parameters."""
import re, os, json

BASE = os.path.join(os.path.dirname(__file__), '..')

def find_optional_params():
    results = {}
    
    for mod_dir in sorted(os.listdir(BASE)):
        if not mod_dir.startswith('hap-mod-'):
            continue
        funcs_path = os.path.join(BASE, mod_dir, 'src', 'funcs.rs')
        if not os.path.exists(funcs_path):
            continue
        
        with open(funcs_path) as f:
            content = f.read()
        
        mod_name = mod_dir.replace('hap-mod-', '')
        
        fn_blocks = re.findall(
            r'hap_fn!\s*\(\s*(\w+)\s*,\s*(\w+)\s*,.*?\{([\s\S]*?)(?=\nhap_fn!|\n#\[cfg|\Z)',
            content
        )
        
        if not fn_blocks:
            fn_blocks = re.findall(
                r'pub\s+extern\s+"C"\s+fn\s+(\w+).*?\{([\s\S]*?)(?=\npub\s+extern|\n#\[cfg|\Z)',
                content
            )
        
        for block in fn_blocks:
            if len(block) == 3:
                fn_name, _struct_name, body = block
            else:
                fn_name, body = block
            
            optionals = set()
            for m in re.finditer(r'p\.(\w+)\.unwrap_or', body):
                optionals.add(m.group(1))
            for m in re.finditer(r'p\.(\w+)\.unwrap_or_default', body):
                optionals.add(m.group(1))
            for m in re.finditer(r'p\.(\w+)\.is_none\(\)', body):
                optionals.add(m.group(1))
            for m in re.finditer(r'p\.(\w+)\.is_some\(\)', body):
                optionals.add(m.group(1))
            for m in re.finditer(r'let\s+\w+\s*=\s*p\.(\w+)\.clone\(\)\.unwrap_or', body):
                optionals.add(m.group(1))
            for m in re.finditer(r'p\.(\w+)\.as_ref\(\)', body):
                optionals.add(m.group(1))
            for m in re.finditer(r'p\.(\w+)\.as_deref\(\)', body):
                optionals.add(m.group(1))
            for m in re.finditer(r'if\s+let\s+Some\(\w+\)\s*=\s*(?:&?p\.(\w+)|p\.(\w+))', body):
                name = m.group(1) or m.group(2)
                optionals.add(name)
            
            for m in re.finditer(r'Option<\w+>', body):
                pass
            
            if optionals:
                key = f"{mod_name}.{fn_name}"
                results[key] = sorted(optionals)
    
    return results

if __name__ == '__main__':
    results = find_optional_params()
    for fn, params in sorted(results.items()):
        print(f"{fn}: {', '.join(params)}")
    print(f"\nTotal: {len(results)} functions with optional params")
