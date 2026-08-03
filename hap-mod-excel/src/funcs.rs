use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, atomic::{AtomicU64, Ordering}};

struct BookEntry { path: String, wb: rust_xlsxwriter::Workbook }

fn parse_cell_ref(cell: &str) -> Result<(u32, u16), HapError> {
    let cell = cell.to_uppercase();
    let col_end = cell.find(|c: char| c.is_ascii_digit()).unwrap_or(cell.len());
    let col_str = &cell[..col_end];
    let row_str = &cell[col_end..];
    if col_str.is_empty() || row_str.is_empty() {
        return Err(HapError::invalid_param("invalid cell reference"));
    }
    let mut col: u16 = 0;
    for c in col_str.chars() {
        col = col * 26 + (c as u16 - b'A' as u16 + 1);
    }
    col -= 1;
    let row: u32 = row_str.parse::<u32>().map_err(|_| HapError::invalid_param("invalid row number"))? - 1;
    Ok((row, col))
}

static BOOK_MAP: LazyLock<Mutex<HashMap<String, BookEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static BOOK_COUNTER: AtomicU64 = AtomicU64::new(1);
fn next_book_id() -> String { format!("xls_{}", BOOK_COUNTER.fetch_add(1, Ordering::Relaxed)) }

fn cell_to_json(c: &calamine::Data) -> Value {
    match c {
        calamine::Data::Empty => json!(null),
        calamine::Data::String(s) => json!(s),
        calamine::Data::Float(f) => json!(f),
        calamine::Data::Int(n) => json!(n),
        calamine::Data::Bool(b) => json!(b),
        calamine::Data::DateTime(dt) => json!(dt.to_string()),
        calamine::Data::DateTimeIso(s) => json!(s),
        calamine::Data::DurationIso(s) => json!(s),
        calamine::Data::Error(e) => json!(format!("{:?}", e)),
    }
}

fn cell_to_string(c: &calamine::Data) -> String {
    match c {
        calamine::Data::String(s) => s.clone(),
        calamine::Data::Float(f) => f.to_string(),
        calamine::Data::Int(n) => n.to_string(),
        calamine::Data::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn open_xlsx(path: &str) -> Result<calamine::Xlsx<std::io::BufReader<std::fs::File>>, HapError> {
    calamine::open_workbook::<calamine::Xlsx<_>, _>(path)
        .map_err(|e| HapError::internal(e.to_string()))
}

// ---------- read ----------
#[derive(Deserialize)] pub struct ReadParams { pub path: String, #[allow(dead_code)] pub sheet: Option<Value>, #[allow(dead_code)] pub range: Option<String>, #[allow(dead_code)] pub header_row: Option<i32> }
hap_fn!(hap_excel_read, ReadParams, |p| {
    use calamine::Reader;
    let mut wb = open_xlsx(&p.path)?;
    let sheets = wb.sheet_names().to_vec();
    let sheet_name = sheets.first().cloned().unwrap_or_default();
    let range = wb.worksheet_range(&sheet_name).map_err(|e: calamine::XlsxError| HapError::internal(e.to_string()))?;
    let mut rows_out: Vec<Vec<Value>> = Vec::new();
    let mut headers: Vec<String> = Vec::new();
    for (i, row) in range.rows().enumerate() {
        let vals: Vec<Value> = row.iter().map(cell_to_json).collect();
        if i == 0 {
            headers = vals.iter().map(|v: &Value| v.as_str().unwrap_or("").to_string()).collect();
        } else {
            rows_out.push(vals);
        }
    }
    Ok(json!({"headers": headers, "rows": rows_out, "sheet_name": sheet_name, "total_rows": rows_out.len()}))
});

// ---------- write ----------
#[derive(Deserialize)] pub struct WriteParams { pub path: String, pub rows: Vec<Vec<Value>>, #[allow(dead_code)] pub headers: Option<Vec<String>>, #[allow(dead_code)] pub sheet: Option<String> }
hap_fn!(hap_excel_write, WriteParams, |p| {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    let mut row_idx = 0u32;
    if let Some(ref hdrs) = p.headers {
        for (c, h) in hdrs.iter().enumerate() {
            let _ = ws.write_string(row_idx, c as u16, h);
        }
        row_idx += 1;
    }
    for row in &p.rows {
        for (c, val) in row.iter().enumerate() {
            match val {
                Value::String(s) => { let _ = ws.write_string(row_idx, c as u16, s); },
                Value::Number(n) => { let _ = ws.write_number(row_idx, c as u16, n.as_f64().unwrap_or(0.0)); },
                Value::Bool(b) => { let _ = ws.write_boolean(row_idx, c as u16, *b); },
                _ => { let _ = ws.write_string(row_idx, c as u16, val.to_string()); },
            }
        }
        row_idx += 1;
    }
    wb.save(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!({"written_rows": p.rows.len()}))
});

// ---------- create ----------
#[derive(Deserialize)] pub struct CreateParams { pub path: String }
hap_fn!(hap_excel_create, CreateParams, |p| {
    let mut wb = rust_xlsxwriter::Workbook::new();
    wb.add_worksheet();
    let id = next_book_id();
    BOOK_MAP.lock().unwrap().insert(id.clone(), BookEntry { path: p.path.clone(), wb });
    Ok(json!({"book_id": id}))
});

// ---------- open ----------
#[derive(Deserialize)] pub struct OpenParams { pub path: String }
hap_fn!(hap_excel_open, OpenParams, |p| {
    let mut wb = rust_xlsxwriter::Workbook::new();
    wb.add_worksheet();
    let id = next_book_id();
    BOOK_MAP.lock().unwrap().insert(id.clone(), BookEntry { path: p.path.clone(), wb });
    Ok(json!({"book_id": id}))
});

// ---------- save ----------
#[derive(Deserialize)] pub struct SaveParams { pub book_id: String }
hap_fn!(hap_excel_save, SaveParams, |p| {
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    entry.wb.save(&entry.path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- save_as ----------
#[derive(Deserialize)] pub struct SaveAsParams { pub book_id: String, pub output_path: String }
hap_fn!(hap_excel_save_as, SaveAsParams, |p| {
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    entry.wb.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

// ---------- close ----------
#[derive(Deserialize)] pub struct CloseParams { pub book_id: String }
hap_fn!(hap_excel_close, CloseParams, |p| {
    BOOK_MAP.lock().unwrap().remove(&p.book_id);
    Ok(json!(true))
});

// ---------- list_sheets ----------
#[derive(Deserialize)] pub struct ListSheetsParams { pub path: String }
hap_fn!(hap_excel_list_sheets, ListSheetsParams, |p| {
    use calamine::Reader;
    let wb = open_xlsx(&p.path)?;
    let sheets: Vec<Value> = wb.sheet_names().iter().map(|n| json!({"name": n, "is_visible": true})).collect();
    Ok(json!(sheets))
});

// ---------- to_csv ----------
#[derive(Deserialize)] pub struct ToCsvParams { pub path: String, #[allow(dead_code)] pub sheet: Option<Value>, pub output_path: String, #[allow(dead_code)] pub delimiter: Option<String> }
hap_fn!(hap_excel_to_csv, ToCsvParams, |p| {
    use calamine::Reader;
    let mut wb = open_xlsx(&p.path)?;
    let sheets = wb.sheet_names().to_vec();
    let name = sheets.first().cloned().unwrap_or_default();
    let range = wb.worksheet_range(&name).map_err(|e: calamine::XlsxError| HapError::internal(e.to_string()))?;
    let delim = p.delimiter.as_deref().unwrap_or(",");
    let mut out = String::new();
    for row in range.rows() {
        let line: Vec<String> = row.iter().map(cell_to_string).collect();
        out.push_str(&line.join(delim));
        out.push('\n');
    }
    std::fs::write(&p.output_path, &out)?;
    Ok(json!(true))
});

// ---------- to_json ----------
#[derive(Deserialize)] pub struct ToJsonParams { pub path: String, #[allow(dead_code)] pub sheet: Option<Value>, #[allow(dead_code)] pub header_row: Option<i32> }
hap_fn!(hap_excel_to_json, ToJsonParams, |p| {
    use calamine::Reader;
    let mut wb = open_xlsx(&p.path)?;
    let sheets = wb.sheet_names().to_vec();
    let name = sheets.first().cloned().unwrap_or_default();
    let range = wb.worksheet_range(&name).map_err(|e: calamine::XlsxError| HapError::internal(e.to_string()))?;
    let mut headers: Vec<String> = Vec::new();
    let mut result: Vec<Value> = Vec::new();
    for (i, row) in range.rows().enumerate() {
        if i == 0 {
            headers = row.iter().map(cell_to_string).collect();
        } else {
            let mut obj = serde_json::Map::new();
            for (j, cell) in row.iter().enumerate() {
                let key = headers.get(j).cloned().unwrap_or_else(|| format!("col_{}", j));
                obj.insert(key, cell_to_json(cell));
            }
            result.push(Value::Object(obj));
        }
    }
    Ok(json!(result))
});

// ---------- from_csv ----------
#[derive(Deserialize)] pub struct FromCsvParams { pub csv_path: String, pub output_path: String, #[allow(dead_code)] pub sheet_name: Option<String>, #[allow(dead_code)] pub delimiter: Option<String> }
hap_fn!(hap_excel_from_csv, FromCsvParams, |p| {
    let content = std::fs::read_to_string(&p.csv_path)?;
    let delim = p.delimiter.as_deref().unwrap_or(",");
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    for (row_idx, line) in content.lines().enumerate() {
        for (col_idx, field) in line.split(delim).enumerate() {
            if let Ok(n) = field.parse::<f64>() { let _ = ws.write_number(row_idx as u32, col_idx as u16, n); }
            else { let _ = ws.write_string(row_idx as u32, col_idx as u16, field); }
        }
    }
    wb.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});


#[derive(Deserialize)] pub struct SheetNameParams { pub book_id: String, #[allow(dead_code)] pub name: String }
hap_fn!(hap_excel_add_sheet, SheetNameParams, |p| {
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.add_worksheet();
    let _ = ws.set_name(&p.name);
    Ok(json!(true))
});
hap_fn!(hap_excel_delete_sheet, SheetNameParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "rust_xlsxwriter does not support deleting worksheets"))
});

#[derive(Deserialize)] pub struct RenameSheetParams { #[allow(dead_code)] pub book_id: String, #[allow(dead_code)] pub old_name: String, #[allow(dead_code)] pub new_name: String }
hap_fn!(hap_excel_rename_sheet, RenameSheetParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "rust_xlsxwriter does not support renaming existing worksheets"))
});

#[derive(Deserialize)] pub struct CellParams { #[allow(dead_code)] pub book_id: String, #[allow(dead_code)] pub sheet: String, #[allow(dead_code)] pub cell: String }
hap_fn!(hap_excel_get_cell, CellParams, |_p| { Ok(json!({"value": null, "type": "empty"})) });

#[derive(Deserialize)] pub struct SetCellParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub cell: String, pub value: Value }
hap_fn!(hap_excel_set_cell, SetCellParams, |p| {
    let (row, col) = parse_cell_ref(&p.cell)?;
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    match &p.value {
        Value::String(s) => { ws.write_string(row, col, s).map_err(|e| HapError::internal(e.to_string()))?; },
        Value::Number(n) => { ws.write_number(row, col, n.as_f64().unwrap_or(0.0)).map_err(|e| HapError::internal(e.to_string()))?; },
        Value::Bool(b) => { ws.write_boolean(row, col, *b).map_err(|e| HapError::internal(e.to_string()))?; },
        _ => { ws.write_string(row, col, p.value.to_string()).map_err(|e| HapError::internal(e.to_string()))?; },
    }
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct RangeParams { #[allow(dead_code)] pub book_id: String, #[allow(dead_code)] pub sheet: String, #[allow(dead_code)] pub range: String }
hap_fn!(hap_excel_get_range, RangeParams, |_p| { Ok(json!({"values": [], "rows": 0, "cols": 0})) });

#[derive(Deserialize)] pub struct SetRangeParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub start_cell: String, pub values: Vec<Vec<Value>> }
hap_fn!(hap_excel_set_range, SetRangeParams, |p| {
    let (start_row, start_col) = parse_cell_ref(&p.start_cell)?;
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    let mut count = 0u32;
    for (ri, row) in p.values.iter().enumerate() {
        for (ci, val) in row.iter().enumerate() {
            let r = start_row + ri as u32;
            let c = start_col + ci as u16;
            match val {
                Value::String(s) => { ws.write_string(r, c, s).map_err(|e| HapError::internal(e.to_string()))?; },
                Value::Number(n) => { ws.write_number(r, c, n.as_f64().unwrap_or(0.0)).map_err(|e| HapError::internal(e.to_string()))?; },
                Value::Bool(b) => { ws.write_boolean(r, c, *b).map_err(|e| HapError::internal(e.to_string()))?; },
                Value::Null => {},
                _ => { ws.write_string(r, c, val.to_string()).map_err(|e| HapError::internal(e.to_string()))?; },
            }
            count += 1;
        }
    }
    Ok(json!({"cells_written": count}))
});

#[derive(Deserialize)] pub struct SetStyleParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub range: String, pub style: Value }
hap_fn!(hap_excel_set_style, SetStyleParams, |p| {
    let parts: Vec<&str> = p.range.split(':').collect();
    let (r1, c1) = parse_cell_ref(parts[0])?;
    let (r2, c2) = if parts.len() > 1 { parse_cell_ref(parts[1])? } else { (r1, c1) };

    let mut fmt = rust_xlsxwriter::Format::new();
    if let Some(bold) = p.style.get("bold").and_then(|v| v.as_bool()) {
        if bold { fmt = fmt.set_bold(); }
    }
    if let Some(italic) = p.style.get("italic").and_then(|v| v.as_bool()) {
        if italic { fmt = fmt.set_italic(); }
    }
    if let Some(size) = p.style.get("font_size").and_then(|v| v.as_f64()) {
        fmt = fmt.set_font_size(size);
    }
    if let Some(color) = p.style.get("font_color").and_then(|v| v.as_str()) {
        if let Ok(c) = u32::from_str_radix(color.trim_start_matches('#'), 16) {
            fmt = fmt.set_font_color(rust_xlsxwriter::Color::RGB(c));
        }
    }
    if let Some(bg) = p.style.get("bg_color").and_then(|v| v.as_str()) {
        if let Ok(c) = u32::from_str_radix(bg.trim_start_matches('#'), 16) {
            fmt = fmt.set_background_color(rust_xlsxwriter::Color::RGB(c));
        }
    }
    if let Some(align) = p.style.get("align").and_then(|v| v.as_str()) {
        let ha = match align {
            "center" => rust_xlsxwriter::FormatAlign::Center,
            "right" => rust_xlsxwriter::FormatAlign::Right,
            _ => rust_xlsxwriter::FormatAlign::Left,
        };
        fmt = fmt.set_align(ha);
    }
    if let Some(wrap) = p.style.get("wrap").and_then(|v| v.as_bool()) {
        if wrap { fmt = fmt.set_text_wrap(); }
    }

    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    for r in r1..=r2 {
        for c in c1..=c2 {
            ws.write_blank(r, c, &fmt).map_err(|e| HapError::internal(e.to_string()))?;
        }
    }
    Ok(json!(true))
});

hap_fn!(hap_excel_merge_cells, RangeParams, |p| {
    let parts: Vec<&str> = p.range.split(':').collect();
    if parts.len() != 2 { return Err(HapError::invalid_param("range format must be A1:B2")); }
    let (r1, c1) = parse_cell_ref(parts[0])?;
    let (r2, c2) = parse_cell_ref(parts[1])?;
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    let fmt = rust_xlsxwriter::Format::default();
    ws.merge_range(r1, c1, r2, c2, "", &fmt).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});
hap_fn!(hap_excel_unmerge_cells, RangeParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "rust_xlsxwriter does not support unmerging cells"))
});

#[derive(Deserialize)] pub struct ColWidthParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub column: String, pub width: f64 }
hap_fn!(hap_excel_set_column_width, ColWidthParams, |p| {
    let col_upper = p.column.to_uppercase();
    let mut col: u16 = 0;
    for c in col_upper.chars() { col = col * 26 + (c as u16 - b'A' as u16 + 1); }
    col -= 1;
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    ws.set_column_width(col, p.width).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct RowHeightParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub row: i32, pub height: f64 }
hap_fn!(hap_excel_set_row_height, RowHeightParams, |p| {
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    ws.set_row_height((p.row - 1) as u32, p.height).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

hap_fn!(hap_excel_auto_filter, RangeParams, |p| {
    let parts: Vec<&str> = p.range.split(':').collect();
    if parts.len() != 2 { return Err(HapError::invalid_param("range format must be A1:B2")); }
    let (r1, c1) = parse_cell_ref(parts[0])?;
    let (r2, c2) = parse_cell_ref(parts[1])?;
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    ws.autofilter(r1, c1, r2, c2).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct FreezePanesParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub row: i32, pub col: i32 }
hap_fn!(hap_excel_freeze_panes, FreezePanesParams, |p| {
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    ws.set_freeze_panes(p.row as u32, p.col as u16).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct ProtectParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub password: Option<String> }
hap_fn!(hap_excel_protect_sheet, ProtectParams, |p| {
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    let prot = rust_xlsxwriter::ProtectionOptions::default();
    ws.protect_with_options(&prot);
    if let Some(ref pw) = p.password { ws.protect_with_password(pw); }
    Ok(json!(true))
});
hap_fn!(hap_excel_unprotect_sheet, ProtectParams, |p| {
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    let _ = ws.unprotect_range(0, 0, 1048575, 16383);
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct SetFormulaParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub cell: String, pub formula: String }
hap_fn!(hap_excel_set_formula, SetFormulaParams, |p| {
    let (row, col) = parse_cell_ref(&p.cell)?;
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    ws.write_formula(row, col, rust_xlsxwriter::Formula::new(&p.formula)).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct AddImageParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub image_path: String, pub cell: String }
hap_fn!(hap_excel_add_image, AddImageParams, |p| {
    let (row, col) = parse_cell_ref(&p.cell)?;
    let img = rust_xlsxwriter::Image::new(&p.image_path).map_err(|e| HapError::internal(e.to_string()))?;
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    ws.insert_image(row, col, &img).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct CopySheetParams { #[allow(dead_code)] pub book_id: String, #[allow(dead_code)] pub source_sheet: String, #[allow(dead_code)] pub new_name: String }
hap_fn!(hap_excel_copy_sheet, CopySheetParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "rust_xlsxwriter does not support copying worksheets"))
});

#[derive(Deserialize)] pub struct AutoFitParams { pub book_id: String, #[allow(dead_code)] pub sheet: String }
hap_fn!(hap_excel_auto_fit_columns, AutoFitParams, |p| {
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    ws.autofit();
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct InsertRowParams { #[allow(dead_code)] pub book_id: String, #[allow(dead_code)] pub sheet: String, #[allow(dead_code)] pub row: i32, #[allow(dead_code)] pub count: Option<i32> }
hap_fn!(hap_excel_insert_row, InsertRowParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "rust_xlsxwriter does not support inserting rows"))
});

#[derive(Deserialize)] pub struct InsertColParams { #[allow(dead_code)] pub book_id: String, #[allow(dead_code)] pub sheet: String, #[allow(dead_code)] pub column: String, #[allow(dead_code)] pub count: Option<i32> }
hap_fn!(hap_excel_insert_column, InsertColParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "rust_xlsxwriter does not support inserting columns"))
});

#[derive(Deserialize)] pub struct DeleteRowParams { #[allow(dead_code)] pub book_id: String, #[allow(dead_code)] pub sheet: String, #[allow(dead_code)] pub row: i32, #[allow(dead_code)] pub count: Option<i32> }
hap_fn!(hap_excel_delete_row, DeleteRowParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "rust_xlsxwriter does not support deleting rows"))
});

#[derive(Deserialize)] pub struct DeleteColParams { #[allow(dead_code)] pub book_id: String, #[allow(dead_code)] pub sheet: String, #[allow(dead_code)] pub column: String, #[allow(dead_code)] pub count: Option<i32> }
hap_fn!(hap_excel_delete_column, DeleteColParams, |_p| {
    Err(HapError::new("NOT_IMPLEMENTED", "rust_xlsxwriter does not support deleting columns"))
});

#[derive(Deserialize)] pub struct AddChartParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub r#type: String, pub data_range: String, #[allow(dead_code)] pub title: Option<String>, #[allow(dead_code)] pub position: Option<String> }
hap_fn!(hap_excel_add_chart, AddChartParams, |p| {
    use rust_xlsxwriter::{Chart, ChartType, ChartSeries};
    let chart_type = match p.r#type.as_str() {
        "bar" => ChartType::Bar,
        "column" => ChartType::Column,
        "line" => ChartType::Line,
        "pie" => ChartType::Pie,
        "scatter" => ChartType::Scatter,
        "area" => ChartType::Area,
        "doughnut" => ChartType::Doughnut,
        _ => return Err(HapError::invalid_param("unsupported chart type")),
    };
    let mut chart = Chart::new(chart_type);
    if let Some(ref t) = p.title { chart.title().set_name(t); }
    let mut series = ChartSeries::new();
    series.set_values(&p.data_range);
    chart.push_series(&series);
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    let pos = p.position.as_deref().unwrap_or("E1");
    let (pr, pc) = parse_cell_ref(pos)?;
    ws.insert_chart(pr, pc, &chart).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct CondFmtParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub range: String, pub rule: Value }
hap_fn!(hap_excel_set_conditional_format, CondFmtParams, |p| {
    let parts: Vec<&str> = p.range.split(':').collect();
    let (r1, c1) = parse_cell_ref(parts[0])?;
    let (r2, c2) = if parts.len() > 1 { parse_cell_ref(parts[1])? } else { (r1, c1) };
    let rule_type = p.rule.get("type").and_then(|v| v.as_str()).unwrap_or("cell_value");
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    match rule_type {
        "data_bar" => {
            let cond = rust_xlsxwriter::ConditionalFormatDataBar::new();
            ws.add_conditional_format(r1, c1, r2, c2, &cond).map_err(|e| HapError::internal(e.to_string()))?;
        },
        "color_scale" => {
            let cond = rust_xlsxwriter::ConditionalFormat2ColorScale::new();
            ws.add_conditional_format(r1, c1, r2, c2, &cond).map_err(|e| HapError::internal(e.to_string()))?;
        },
        _ => {
            let op = p.rule.get("operator").and_then(|v| v.as_str()).unwrap_or("greater");
            let vals = p.rule.get("values").and_then(|v| v.as_array());
            let v1 = vals.and_then(|a| a.first()).and_then(|v| v.as_f64()).unwrap_or(0.0);
            use rust_xlsxwriter::ConditionalFormatCellRule;
            let rule = match op {
                "less" => ConditionalFormatCellRule::LessThan(v1),
                "equal" => ConditionalFormatCellRule::EqualTo(v1),
                "not_equal" => ConditionalFormatCellRule::NotEqualTo(v1),
                "between" => {
                    let v2 = vals.and_then(|a| a.get(1)).and_then(|v| v.as_f64()).unwrap_or(v1);
                    ConditionalFormatCellRule::Between(v1, v2)
                },
                _ => ConditionalFormatCellRule::GreaterThan(v1),
            };
            let mut cond = rust_xlsxwriter::ConditionalFormatCell::new().set_rule(rule);
            if let Some(fmt_obj) = p.rule.get("format") {
                let mut fmt = rust_xlsxwriter::Format::new();
                if let Some(bg) = fmt_obj.get("bg_color").and_then(|v| v.as_str()) {
                    if let Ok(c) = u32::from_str_radix(bg.trim_start_matches('#'), 16) {
                        fmt = fmt.set_background_color(rust_xlsxwriter::Color::RGB(c));
                    }
                }
                if let Some(fc) = fmt_obj.get("font_color").and_then(|v| v.as_str()) {
                    if let Ok(c) = u32::from_str_radix(fc.trim_start_matches('#'), 16) {
                        fmt = fmt.set_font_color(rust_xlsxwriter::Color::RGB(c));
                    }
                }
                if let Some(true) = fmt_obj.get("bold").and_then(|v| v.as_bool()) {
                    fmt = fmt.set_bold();
                }
                cond = cond.set_format(fmt);
            }
            ws.add_conditional_format(r1, c1, r2, c2, &cond).map_err(|e| HapError::internal(e.to_string()))?;
        }
    }
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct DataValParams { pub book_id: String, #[allow(dead_code)] pub sheet: String, pub range: String, pub r#type: String, #[allow(dead_code)] pub values: Option<Vec<String>>, #[allow(dead_code)] pub min: Option<Value>, #[allow(dead_code)] pub max: Option<Value>, #[allow(dead_code)] pub error_message: Option<String> }
hap_fn!(hap_excel_add_data_validation, DataValParams, |p| {
    let parts: Vec<&str> = p.range.split(':').collect();
    let (r1, c1) = parse_cell_ref(parts[0])?;
    let (r2, c2) = if parts.len() > 1 { parse_cell_ref(parts[1])? } else { (r1, c1) };
    let mut map = BOOK_MAP.lock().unwrap();
    let entry = map.get_mut(&p.book_id).ok_or_else(|| HapError::invalid_param("invalid book_id"))?;
    let ws = entry.wb.worksheet_from_index(0).map_err(|e| HapError::internal(e.to_string()))?;
    let dv = match p.r#type.as_str() {
        "list" => {
            let items: Vec<String> = p.values.unwrap_or_default();
            let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
            rust_xlsxwriter::DataValidation::new().allow_list_strings(&refs)
                .map_err(|e| HapError::internal(e.to_string()))?
        },
        "whole" => {
            let mn = p.min.as_ref().and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
            let mx = p.max.as_ref().and_then(|v| v.as_f64()).unwrap_or(100.0) as i32;
            rust_xlsxwriter::DataValidation::new()
                .allow_whole_number(rust_xlsxwriter::DataValidationRule::Between(mn, mx))
        },
        "decimal" => {
            let mn = p.min.as_ref().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let mx = p.max.as_ref().and_then(|v| v.as_f64()).unwrap_or(100.0);
            rust_xlsxwriter::DataValidation::new()
                .allow_decimal_number(rust_xlsxwriter::DataValidationRule::Between(mn, mx))
        },
        "text_length" => {
            let mn = p.min.as_ref().and_then(|v| v.as_f64()).unwrap_or(0.0) as u32;
            let mx = p.max.as_ref().and_then(|v| v.as_f64()).unwrap_or(255.0) as u32;
            rust_xlsxwriter::DataValidation::new()
                .allow_text_length(rust_xlsxwriter::DataValidationRule::Between(mn, mx))
        },
        _ => return Err(HapError::invalid_param(format!("unsupported validation type: {}", p.r#type))),
    };
    let dv = if let Some(ref msg) = p.error_message {
        dv.set_error_message(msg).map_err(|e| HapError::internal(e.to_string()))?
    } else { dv };
    ws.add_data_validation(r1, c1, r2, c2, &dv).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

hap_fn!(hap_excel_list_open, Value, |_p| {
    let map = BOOK_MAP.lock().unwrap();
    let list: Vec<Value> = map.iter().map(|(id, e)| json!({"book_id": id, "path": e.path})).collect();
    Ok(json!(list))
});
