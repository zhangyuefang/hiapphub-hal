use hap_common::{hap_free_string, hap_module_init, hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Map, Value};

hap_module_init!("xml");
hap_free_string!();

fn xml_node_to_json(node: &roxmltree::Node) -> Value {
    if node.is_text() {
        return json!(node.text().unwrap_or(""));
    }
    if !node.is_element() {
        return Value::Null;
    }

    let mut obj = Map::new();
    let tag = node.tag_name().name().to_string();

    let mut attrs = Map::new();
    for attr in node.attributes() {
        attrs.insert(attr.name().to_string(), json!(attr.value()));
    }
    if !attrs.is_empty() {
        obj.insert("@attributes".to_string(), Value::Object(attrs));
    }

    let children: Vec<_> = node.children().filter(|c| c.is_element() || c.is_text()).collect();
    let text_children: Vec<_> = children.iter().filter(|c| c.is_text()).collect();
    let elem_children: Vec<_> = children.iter().filter(|c| c.is_element()).collect();

    if elem_children.is_empty() {
        let text = text_children.iter()
            .filter_map(|c| c.text())
            .collect::<String>();
        if obj.is_empty() {
            return json!({ tag: text });
        }
        obj.insert("#text".to_string(), json!(text));
        return json!({ tag: Value::Object(obj) });
    }

    let mut child_map: Map<String, Value> = Map::new();
    for child in &elem_children {
        let child_tag = child.tag_name().name().to_string();
        let child_val = xml_node_to_json(child);
        let inner = child_val.get(&child_tag).cloned().unwrap_or(child_val.clone());

        if let Some(existing) = child_map.get_mut(&child_tag) {
            if let Value::Array(arr) = existing {
                arr.push(inner);
            } else {
                let prev = existing.clone();
                *existing = json!([prev, inner]);
            }
        } else {
            child_map.insert(child_tag, inner);
        }
    }

    for (k, v) in child_map {
        obj.insert(k, v);
    }

    json!({ tag: Value::Object(obj) })
}

fn json_to_xml_inner(key: &str, value: &Value, indent: usize, indent_size: usize) -> String {
    let pad = " ".repeat(indent);
    match value {
        Value::Object(map) => {
            let mut attrs_str = String::new();
            let mut children_str = String::new();
            let mut text_content = None;

            for (k, v) in map {
                if k == "@attributes" {
                    if let Value::Object(attrs) = v {
                        for (ak, av) in attrs {
                            let val_str = match av { Value::String(s) => s.clone(), _ => av.to_string() };
                            attrs_str.push_str(&format!(r#" {ak}="{val_str}""#));
                        }
                    }
                } else if k == "#text" {
                    text_content = Some(match v { Value::String(s) => s.clone(), _ => v.to_string() });
                } else {
                    match v {
                        Value::Array(arr) => {
                            for item in arr {
                                children_str.push_str(&json_to_xml_inner(k, item, indent + indent_size, indent_size));
                            }
                        }
                        _ => children_str.push_str(&json_to_xml_inner(k, v, indent + indent_size, indent_size)),
                    }
                }
            }

            if let Some(ref text) = text_content {
                if children_str.is_empty() {
                    return format!("{pad}<{key}{attrs_str}>{text}</{key}>\n");
                }
            }
            if children_str.is_empty() && text_content.is_none() {
                format!("{pad}<{key}{attrs_str}/>\n")
            } else {
                format!("{pad}<{key}{attrs_str}>\n{children_str}{pad}</{key}>\n")
            }
        }
        Value::String(s) => format!("{pad}<{key}>{s}</{key}>\n"),
        Value::Number(n) => format!("{pad}<{key}>{n}</{key}>\n"),
        Value::Bool(b) => format!("{pad}<{key}>{b}</{key}>\n"),
        Value::Null => format!("{pad}<{key}/>\n"),
        Value::Array(arr) => {
            let mut result = String::new();
            for item in arr {
                result.push_str(&json_to_xml_inner(key, item, indent, indent_size));
            }
            result
        }
    }
}

// ---------- 1. parse ----------
#[derive(Deserialize)]
struct ParseParams { xml_string: String }
hap_fn!(hap_xml_parse, ParseParams, |p| {
    let doc = roxmltree::Document::parse(&p.xml_string)
        .map_err(|e| HapError::invalid_param(format!("XML parse failed: {e}")))?;
    let root = doc.root_element();
    Ok(xml_node_to_json(&root))
});

// ---------- 2. stringify ----------
#[derive(Deserialize)]
struct StringifyParams { object: Value, indent: Option<i32>, declaration: Option<bool>, encoding: Option<String> }
hap_fn!(hap_xml_stringify, StringifyParams, |p| {
    let indent = p.indent.unwrap_or(2) as usize;
    let enc = p.encoding.as_deref().unwrap_or("UTF-8");
    let add_decl = p.declaration.unwrap_or(true);
    let mut xml = String::new();
    if add_decl {
        xml.push_str(&format!(r#"<?xml version="1.0" encoding="{enc}"?>"#));
        xml.push('\n');
    }
    if let Value::Object(map) = &p.object {
        for (k, v) in map {
            xml.push_str(&json_to_xml_inner(k, v, 0, indent));
        }
    }
    Ok(json!(xml))
});

// ---------- 3. query_xpath ----------
#[derive(Deserialize)]
struct XPathParams { xml_string: String, xpath: String }
hap_fn!(hap_xml_query_xpath, XPathParams, |p| {
    let doc = roxmltree::Document::parse(&p.xml_string)
        .map_err(|e| HapError::invalid_param(format!("XML parse failed: {e}")))?;

    let parts: Vec<&str> = p.xpath.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();
    let root = doc.root_element();
    let mut nodes: Vec<roxmltree::Node> = if parts.first().map_or(false, |&p| p == root.tag_name().name()) {
        vec![root]
    } else {
        return Ok(json!(Vec::<String>::new()));
    };

    for part in parts.iter().skip(1) {
        let tag = part.trim();
        let mut next_nodes = Vec::new();
        for node in &nodes {
            for child in node.children() {
                if child.is_element() && child.tag_name().name() == tag {
                    next_nodes.push(child);
                }
            }
        }
        nodes = next_nodes;
    }

    let results: Vec<String> = nodes.iter().map(|n| {
        n.text().map(|t| t.to_string()).unwrap_or_else(|| {
            let mut s = String::new();
            for desc in n.descendants() {
                if let Some(t) = desc.text() { s.push_str(t); }
            }
            s
        })
    }).collect();
    Ok(json!(results))
});

// ---------- 4. validate ----------
#[derive(Deserialize)]
struct ValidateParams { xml_string: String, #[allow(dead_code)] xsd_string: Option<String> }
hap_fn!(hap_xml_validate, ValidateParams, |p| {
    match roxmltree::Document::parse(&p.xml_string) {
        Ok(_) => Ok(json!({ "valid": true, "errors": [] })),
        Err(e) => {
            let (line, col) = (0u32, 0u32);
            Ok(json!({ "valid": false, "errors": [{ "line": line, "column": col, "message": e.to_string() }] }))
        }
    }
});

// ---------- 5. read_file ----------
#[derive(Deserialize)]
struct ReadFileParams { path: String, #[allow(dead_code)] encoding: Option<String> }
hap_fn!(hap_xml_read_file, ReadFileParams, |p| {
    let content = std::fs::read_to_string(&p.path)?;
    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| HapError::invalid_param(format!("XML parse failed: {e}")))?;
    Ok(xml_node_to_json(&doc.root_element()))
});

// ---------- 6. write_file ----------
#[derive(Deserialize)]
struct WriteFileParams { path: String, object: Value, indent: Option<i32>, #[allow(dead_code)] encoding: Option<String> }
hap_fn!(hap_xml_write_file, WriteFileParams, |p| {
    let indent = p.indent.unwrap_or(2) as usize;
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    if let Value::Object(map) = &p.object {
        for (k, v) in map { xml.push_str(&json_to_xml_inner(k, v, 0, indent)); }
    }
    if let Some(parent) = std::path::Path::new(&p.path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p.path, &xml)?;
    Ok(json!(true))
});

// ---------- 7. transform_xslt ----------
#[derive(Deserialize)]
struct XsltParams { #[allow(dead_code)] xml_string: String, #[allow(dead_code)] xslt_string: String, #[allow(dead_code)] params: Option<Value> }
hap_fn!(hap_xml_transform_xslt, XsltParams, |_p| {
    Err(HapError::internal("XSLT transform not implemented (requires external XSLT engine)"))
});

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    fn call(func: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, json_str: &str) -> Value {
        let cs = CString::new(json_str).unwrap();
        let result = func(cs.as_ptr());
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { hap_free_string(result as *mut _) };
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_parse_simple() {
        let r = call(hap_xml_parse, r#"{"xml_string":"<root><name>Alice</name><age>30</age></root>"}"#);
        assert_eq!(r["root"]["name"], "Alice");
        assert_eq!(r["root"]["age"], "30");
    }

    #[test]
    fn test_parse_attributes() {
        let r = call(hap_xml_parse, r#"{"xml_string":"<item id=\"1\" type=\"book\">Hello</item>"}"#);
        assert_eq!(r["item"]["@attributes"]["id"], "1");
        assert_eq!(r["item"]["#text"], "Hello");
    }

    #[test]
    fn test_parse_nested() {
        let r = call(hap_xml_parse, r#"{"xml_string":"<root><items><item>A</item><item>B</item></items></root>"}"#);
        let items = &r["root"]["items"]["item"];
        assert!(items.is_array());
        assert_eq!(items.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_stringify() {
        let r = call(hap_xml_stringify, r#"{"object":{"root":{"name":"Alice","age":"30"}},"declaration":true}"#);
        let xml = r.as_str().unwrap();
        assert!(xml.contains("<?xml"));
        assert!(xml.contains("<root>"));
        assert!(xml.contains("<name>Alice</name>"));
    }

    #[test]
    fn test_stringify_no_declaration() {
        let r = call(hap_xml_stringify, r#"{"object":{"item":"test"},"declaration":false}"#);
        let xml = r.as_str().unwrap();
        assert!(!xml.contains("<?xml"));
        assert!(xml.contains("<item>test</item>"));
    }

    #[test]
    fn test_query_xpath() {
        let r = call(hap_xml_query_xpath, r#"{"xml_string":"<root><a><b>X</b><b>Y</b></a></root>","xpath":"/root/a/b"}"#);
        let arr = r.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0], "X");
        assert_eq!(arr[1], "Y");
    }

    #[test]
    fn test_validate_ok() {
        let r = call(hap_xml_validate, r#"{"xml_string":"<root><a>test</a></root>"}"#);
        assert_eq!(r["valid"], true);
    }

    #[test]
    fn test_validate_error() {
        let r = call(hap_xml_validate, r#"{"xml_string":"<root><a>test</b></root>"}"#);
        assert_eq!(r["valid"], false);
        assert!(!r["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_read_write_file() {
        let tmp = std::env::temp_dir().join("hap_xml_test.xml");
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        call(hap_xml_write_file, &format!(r#"{{"path":"{path}","object":{{"root":{{"name":"Test"}}}}}}"#));
        assert!(tmp.exists());
        let r = call(hap_xml_read_file, &format!(r#"{{"path":"{path}"}}"#));
        assert_eq!(r["root"]["name"], "Test");
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_xslt_not_implemented() {
        let r = call(hap_xml_transform_xslt, r#"{"xml_string":"<a/>","xslt_string":"<xsl/>"}"#);
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_describe() {
        let ptr = hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "xml");
        assert_eq!(v["functions"].as_array().unwrap().len(), 7);
        unsafe { hap_free_string(ptr as *mut _) };
    }
}
