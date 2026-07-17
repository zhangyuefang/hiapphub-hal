use hap_common::{hap_free_string, hap_module_init, hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;

hap_module_init!("csv");
hap_free_string!();

fn build_reader(delimiter: Option<&str>, quote_char: Option<&str>, has_header: bool, trim: bool) -> csv::ReaderBuilder {
    let mut b = csv::ReaderBuilder::new();
    b.has_headers(has_header);
    if let Some(d) = delimiter {
        if let Some(ch) = d.as_bytes().first() { b.delimiter(*ch); }
    }
    if let Some(q) = quote_char {
        if let Some(ch) = q.as_bytes().first() { b.quote(*ch); }
    }
    if trim { b.trim(csv::Trim::All); }
    b
}

// ---------- 1. parse ----------
#[derive(Deserialize)]
struct ParseParams { content: String, delimiter: Option<String>, has_header: Option<bool>, quote_char: Option<String>, trim: Option<bool> }
hap_fn!(hap_csv_parse, ParseParams, |p| {
    let has_header = p.has_header.unwrap_or(true);
    let trim = p.trim.unwrap_or(false);
    let mut rdr = build_reader(p.delimiter.as_deref(), p.quote_char.as_deref(), has_header, trim)
        .from_reader(p.content.as_bytes());
    let headers: Vec<String> = if has_header {
        rdr.headers().map_err(|e| HapError::internal(e.to_string()))?.iter().map(|s| s.to_string()).collect()
    } else { vec![] };
    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| HapError::internal(e.to_string()))?;
        rows.push(record.iter().map(|s| json!(s)).collect::<Vec<_>>());
    }
    let count = rows.len();
    Ok(json!({ "headers": headers, "rows": rows, "row_count": count }))
});

// ---------- 2. stringify ----------
#[derive(Deserialize)]
struct StringifyParams { rows: Vec<Vec<String>>, headers: Option<Vec<String>>, delimiter: Option<String>, line_ending: Option<String> }
hap_fn!(hap_csv_stringify, StringifyParams, |p| {
    let delim = p.delimiter.as_deref().and_then(|d| d.as_bytes().first().copied()).unwrap_or(b',');
    let term = match p.line_ending.as_deref() {
        Some("crlf") => csv::Terminator::CRLF,
        _ => csv::Terminator::Any(b'\n'),
    };
    let mut wtr = csv::WriterBuilder::new().delimiter(delim).terminator(term).from_writer(vec![]);
    if let Some(ref h) = p.headers { wtr.write_record(h).map_err(|e| HapError::internal(e.to_string()))?; }
    for row in &p.rows { wtr.write_record(row).map_err(|e| HapError::internal(e.to_string()))?; }
    let data = wtr.into_inner().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(String::from_utf8_lossy(&data).into_owned()))
});

// ---------- 3. read_file ----------
#[derive(Deserialize)]
struct ReadFileParams { path: String, delimiter: Option<String>, has_header: Option<bool>, #[allow(dead_code)] encoding: Option<String>, quote_char: Option<String>, trim: Option<bool>, skip_rows: Option<i32>, limit: Option<i32> }
hap_fn!(hap_csv_read_file, ReadFileParams, |p| {
    let has_header = p.has_header.unwrap_or(true);
    let trim = p.trim.unwrap_or(false);
    let content = std::fs::read_to_string(&p.path)?;
    let mut rdr = build_reader(p.delimiter.as_deref(), p.quote_char.as_deref(), has_header, trim)
        .from_reader(content.as_bytes());
    let headers: Vec<String> = if has_header {
        rdr.headers().map_err(|e| HapError::internal(e.to_string()))?.iter().map(|s| s.to_string()).collect()
    } else { vec![] };
    let skip = p.skip_rows.unwrap_or(0) as usize;
    let limit = p.limit.map(|l| l as usize);
    let mut rows = Vec::new();
    let mut total = 0usize;
    for result in rdr.records() {
        let record = result.map_err(|e| HapError::internal(e.to_string()))?;
        total += 1;
        if total <= skip { continue; }
        if let Some(lim) = limit { if rows.len() >= lim { continue; } }
        rows.push(record.iter().map(|s| json!(s)).collect::<Vec<_>>());
    }
    Ok(json!({ "headers": headers, "rows": rows, "total": total }))
});

// ---------- 4. write_file ----------
#[derive(Deserialize)]
struct WriteFileParams { path: String, rows: Vec<Vec<String>>, headers: Option<Vec<String>>, delimiter: Option<String>, #[allow(dead_code)] encoding: Option<String>, append: Option<bool>, bom: Option<bool> }
hap_fn!(hap_csv_write_file, WriteFileParams, |p| {
    let delim = p.delimiter.as_deref().and_then(|d| d.as_bytes().first().copied()).unwrap_or(b',');
    let append = p.append.unwrap_or(false);
    if let Some(parent) = std::path::Path::new(&p.path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::OpenOptions::new().create(true).write(true).append(append).truncate(!append).open(&p.path)?;
    let mut buf: Box<dyn std::io::Write> = Box::new(file);
    if p.bom.unwrap_or(false) && !append {
        buf.write_all(&[0xEF, 0xBB, 0xBF])?;
    }
    let mut wtr = csv::WriterBuilder::new().delimiter(delim).from_writer(buf);
    if let Some(ref h) = p.headers { if !append { wtr.write_record(h).map_err(|e| HapError::internal(e.to_string()))?; } }
    for row in &p.rows { wtr.write_record(row).map_err(|e| HapError::internal(e.to_string()))?; }
    wtr.flush().map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!({ "written": p.rows.len() }))
});

// ---------- 5. count_rows ----------
#[derive(Deserialize)]
struct CountRowsParams { path: String, has_header: Option<bool>, quote_char: Option<String> }
hap_fn!(hap_csv_count_rows, CountRowsParams, |p| {
    let has_header = p.has_header.unwrap_or(true);
    let content = std::fs::read_to_string(&p.path)?;
    let mut rdr = build_reader(None, p.quote_char.as_deref(), has_header, false).from_reader(content.as_bytes());
    let count = rdr.records().count();
    Ok(json!(count))
});

// ---------- 6. get_headers ----------
#[derive(Deserialize)]
struct GetHeadersParams { path: String, delimiter: Option<String>, quote_char: Option<String> }
hap_fn!(hap_csv_get_headers, GetHeadersParams, |p| {
    let content = std::fs::read_to_string(&p.path)?;
    let mut rdr = build_reader(p.delimiter.as_deref(), p.quote_char.as_deref(), true, false).from_reader(content.as_bytes());
    let headers: Vec<String> = rdr.headers().map_err(|e| HapError::internal(e.to_string()))?.iter().map(|s| s.to_string()).collect();
    Ok(json!(headers))
});

// ---------- 7. read_stream ----------
#[derive(Deserialize)]
struct ReadStreamParams { path: String, offset: i32, limit: i32, delimiter: Option<String>, has_header: Option<bool>, #[allow(dead_code)] encoding: Option<String>, quote_char: Option<String>, trim: Option<bool> }
hap_fn!(hap_csv_read_stream, ReadStreamParams, |p| {
    let has_header = p.has_header.unwrap_or(true);
    let trim = p.trim.unwrap_or(false);
    let content = std::fs::read_to_string(&p.path)?;
    let mut rdr = build_reader(p.delimiter.as_deref(), p.quote_char.as_deref(), has_header, trim)
        .from_reader(content.as_bytes());
    let headers: Vec<String> = if has_header {
        rdr.headers().map_err(|e| HapError::internal(e.to_string()))?.iter().map(|s| s.to_string()).collect()
    } else { vec![] };
    let offset = p.offset as usize;
    let limit = p.limit as usize;
    let mut rows = Vec::new();
    let mut idx = 0usize;
    let mut has_more = false;
    for result in rdr.records() {
        let record = result.map_err(|e| HapError::internal(e.to_string()))?;
        if idx >= offset && rows.len() < limit {
            rows.push(record.iter().map(|s| json!(s)).collect::<Vec<_>>());
        } else if rows.len() >= limit {
            has_more = true;
            break;
        }
        idx += 1;
    }
    Ok(json!({ "headers": headers, "rows": rows, "has_more": has_more }))
});

#[no_mangle]
pub extern "C" fn hap_module_describe() -> *const std::os::raw::c_char {
    hap_common::ffi::str_to_c(include_str!("../manifest.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::ffi::{CStr, CString};

    fn call(func: extern "C" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_char, json: &str) -> Value {
        let cs = CString::new(json).unwrap();
        let result = func(cs.as_ptr());
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { hap_free_string(result as *mut _) };
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn test_parse() {
        let r = call(hap_csv_parse, r#"{"content":"name,age\nAlice,30\nBob,25"}"#);
        assert_eq!(r["headers"], json!(["name", "age"]));
        assert_eq!(r["row_count"], 2);
        assert_eq!(r["rows"][0][0], "Alice");
    }

    #[test]
    fn test_parse_no_header() {
        let r = call(hap_csv_parse, r#"{"content":"a,b\nc,d","has_header":false}"#);
        assert_eq!(r["headers"], json!([]));
        assert_eq!(r["row_count"], 2);
    }

    #[test]
    fn test_parse_custom_delimiter() {
        let r = call(hap_csv_parse, r#"{"content":"name\tage\nAlice\t30","delimiter":"\t"}"#);
        assert_eq!(r["rows"][0][1], "30");
    }

    #[test]
    fn test_stringify() {
        let r = call(hap_csv_stringify, r#"{"rows":[["Alice","30"],["Bob","25"]],"headers":["name","age"]}"#);
        let s = r.as_str().unwrap();
        assert!(s.contains("name,age"));
        assert!(s.contains("Alice,30"));
    }

    #[test]
    fn test_read_write_file() {
        let tmp = std::env::temp_dir().join("hap_csv_test.csv");
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        call(hap_csv_write_file, &format!(r#"{{"path":"{path}","rows":[["A","1"],["B","2"]],"headers":["col1","col2"]}}"#));
        let r = call(hap_csv_read_file, &format!(r#"{{"path":"{path}"}}"#));
        assert_eq!(r["headers"], json!(["col1", "col2"]));
        assert_eq!(r["rows"].as_array().unwrap().len(), 2);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_count_rows() {
        let tmp = std::env::temp_dir().join("hap_csv_count.csv");
        std::fs::write(&tmp, "h1,h2\na,b\nc,d\ne,f\n").unwrap();
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        let r = call(hap_csv_count_rows, &format!(r#"{{"path":"{path}"}}"#));
        assert_eq!(r.as_i64().unwrap(), 3);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_get_headers() {
        let tmp = std::env::temp_dir().join("hap_csv_headers.csv");
        std::fs::write(&tmp, "x,y,z\n1,2,3\n").unwrap();
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        let r = call(hap_csv_get_headers, &format!(r#"{{"path":"{path}"}}"#));
        assert_eq!(r, json!(["x", "y", "z"]));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_read_stream() {
        let tmp = std::env::temp_dir().join("hap_csv_stream.csv");
        std::fs::write(&tmp, "h\na\nb\nc\nd\ne\n").unwrap();
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        let r = call(hap_csv_read_stream, &format!(r#"{{"path":"{path}","offset":1,"limit":2}}"#));
        assert_eq!(r["rows"].as_array().unwrap().len(), 2);
        assert_eq!(r["has_more"], true);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_write_bom() {
        let tmp = std::env::temp_dir().join("hap_csv_bom.csv");
        let path = tmp.to_string_lossy().replace('\\', "\\\\");
        call(hap_csv_write_file, &format!(r#"{{"path":"{path}","rows":[["a","b"]],"bom":true}}"#));
        let bytes = std::fs::read(&tmp).unwrap();
        assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_describe() {
        let ptr = hap_module_describe();
        let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let v: Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["name"], "csv");
        assert_eq!(v["functions"].as_array().unwrap().len(), 7);
        unsafe { hap_free_string(ptr as *mut _) };
    }
}
