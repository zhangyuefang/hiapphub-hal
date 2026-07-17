use hap_common::{hap_fn, HapError};
use lopdf::dictionary;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, atomic::{AtomicU64, Ordering}};

fn append_content_dict(page_dict: &mut lopdf::Dictionary, content_id: lopdf::ObjectId) {
    let existing = page_dict.get(b"Contents").ok().cloned();
    match existing {
        Some(lopdf::Object::Array(mut arr)) => {
            arr.push(content_id.into());
            page_dict.set("Contents", arr);
        },
        Some(lopdf::Object::Reference(r)) => {
            page_dict.set("Contents", vec![r.into(), content_id.into()]);
        },
        _ => {
            page_dict.set("Contents", content_id);
        }
    }
}

fn append_content(doc: &mut lopdf::Document, page_obj_id: lopdf::ObjectId, content_id: lopdf::ObjectId) {
    if let Ok(page_dict) = doc.get_dictionary_mut(page_obj_id) {
        let existing = page_dict.get(b"Contents").ok().cloned();
        match existing {
            Some(lopdf::Object::Array(mut arr)) => {
                arr.push(content_id.into());
                page_dict.set("Contents", arr);
            },
            Some(lopdf::Object::Reference(r)) => {
                page_dict.set("Contents", vec![r.into(), content_id.into()]);
            },
            _ => {
                page_dict.set("Contents", content_id);
            }
        }
    }
}

static DOC_MAP: LazyLock<Mutex<HashMap<String, lopdf::Document>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static DOC_COUNTER: AtomicU64 = AtomicU64::new(1);
fn next_doc_id() -> String { format!("pdf_{}", DOC_COUNTER.fetch_add(1, Ordering::Relaxed)) }

// ---------- info ----------
#[derive(Deserialize)] pub struct InfoParams { pub path: String }
hap_fn!(hap_pdf_info, InfoParams, |p| {
    let doc = lopdf::Document::load(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    let pages = doc.get_pages().len() as i32;
    let size = std::fs::metadata(&p.path)?.len() as i64;
    Ok(json!({"pages": pages, "encrypted": doc.is_encrypted(), "file_size": size}))
});

// ---------- open ----------
#[derive(Deserialize)] pub struct OpenParams { pub path: String, #[allow(dead_code)] pub password: Option<String> }
hap_fn!(hap_pdf_open, OpenParams, |p| {
    let doc = lopdf::Document::load(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    let pages = doc.get_pages().len() as i32;
    let id = next_doc_id();
    DOC_MAP.lock().unwrap().insert(id.clone(), doc);
    Ok(json!({"doc_id": id, "pages": pages}))
});

// ---------- create ----------
#[derive(Deserialize)] pub struct CreateParams { #[allow(dead_code)] pub title: Option<String>, #[allow(dead_code)] pub author: Option<String>, #[allow(dead_code)] pub page_size: Option<Value> }
hap_fn!(hap_pdf_create, CreateParams, |_p| {
    let mut doc = lopdf::Document::with_version("1.5");
    let pages_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Pages",
        "Kids" => vec![],
        "Count" => lopdf::Object::Integer(0),
    });
    doc.add_object(lopdf::dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", doc.objects.keys().last().copied().unwrap());
    let id = next_doc_id();
    DOC_MAP.lock().unwrap().insert(id.clone(), doc);
    Ok(json!({"doc_id": id}))
});

// ---------- save ----------
#[derive(Deserialize)] pub struct SaveParams { pub doc_id: String, pub output_path: String }
hap_fn!(hap_pdf_save, SaveParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    let size = std::fs::metadata(&p.output_path)?.len() as i64;
    Ok(json!({"size": size}))
});

// ---------- close ----------
#[derive(Deserialize)] pub struct CloseParams { pub doc_id: String }
hap_fn!(hap_pdf_close, CloseParams, |p| {
    DOC_MAP.lock().unwrap().remove(&p.doc_id);
    Ok(json!(true))
});

// ---------- page_dimensions ----------
#[derive(Deserialize)] pub struct PageDimParams { pub path: String, pub page_index: Option<i32> }
hap_fn!(hap_pdf_page_dimensions, PageDimParams, |p| {
    let doc = lopdf::Document::load(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    let pages = doc.get_pages();
    let page_idx = p.page_index.unwrap_or(0) as u32 + 1;
    if let Some(&page_id) = pages.get(&page_idx) {
        if let Ok(page) = doc.get_dictionary(page_id) {
            if let Ok(media_box) = page.get(b"MediaBox") {
                if let Ok(arr) = media_box.as_array() {
                    if arr.len() == 4 {
                        let w = arr[2].as_float().unwrap_or(595.0);
                        let h = arr[3].as_float().unwrap_or(842.0);
                        let w_mm = w * 25.4 / 72.0;
                        let h_mm = h * 25.4 / 72.0;
                        let orient = if w > h { "landscape" } else { "portrait" };
                        return Ok(json!({"width_mm": w_mm, "height_mm": h_mm, "orientation": orient}));
                    }
                }
            }
        }
    }
    Ok(json!({"width_mm": 210.0, "height_mm": 297.0, "orientation": "portrait"}))
});

// ---------- extract_text ----------
#[derive(Deserialize)] pub struct ExtractTextParams { pub path: String, pub page_start: Option<i32>, pub page_end: Option<i32>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_pdf_extract_text, ExtractTextParams, |p| {
    let doc = lopdf::Document::load(&p.path).map_err(|e| HapError::internal(e.to_string()))?;
    let pages = doc.get_pages();
    let total = pages.len() as i32;
    let start = p.page_start.unwrap_or(0).max(0);
    let end = p.page_end.unwrap_or(total - 1).min(total - 1);
    let mut text = String::new();
    for page_num in (start + 1) as u32..=(end + 1) as u32 {
        if let Some(&page_id) = pages.get(&page_num) {
            if let Ok(content) = doc.get_page_content(page_id) {
                let content_str = String::from_utf8_lossy(&content);
                for line in content_str.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('(') && trimmed.contains(") Tj") {
                        if let Some(s) = trimmed.strip_prefix('(') {
                            if let Some(s) = s.strip_suffix(") Tj") {
                                text.push_str(s);
                                text.push('\n');
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(json!(text))
});

// ---------- merge ----------
#[derive(Deserialize)] pub struct MergeParams { pub input_paths: Vec<String>, pub output_path: String, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_pdf_merge, MergeParams, |p| {
    if p.input_paths.is_empty() { return Err(HapError::invalid_param("at least one input file required")); }
    let mut doc = lopdf::Document::load(&p.input_paths[0]).map_err(|e| HapError::internal(e.to_string()))?;
    for path in &p.input_paths[1..] {
        let other = lopdf::Document::load(path).map_err(|e| HapError::internal(e.to_string()))?;
        let other_pages = other.get_pages();
        for (&page_num, &page_id) in &other_pages {
            let _ = (page_num, page_id);
        }
        doc.reference_table.merge(other.reference_table);
    }
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    let pages = doc.get_pages().len() as i32;
    let size = std::fs::metadata(&p.output_path)?.len() as i64;
    Ok(json!({"pages": pages, "size": size}))
});


#[derive(Deserialize)] pub struct AddPageParams { pub doc_id: String, pub width_mm: Option<f64>, pub height_mm: Option<f64> }
hap_fn!(hap_pdf_add_page, AddPageParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let w_pt = p.width_mm.unwrap_or(210.0) * 72.0 / 25.4;
    let h_pt = p.height_mm.unwrap_or(297.0) * 72.0 / 25.4;

    let pages_ref = doc.catalog().ok()
        .and_then(|cat| cat.get(b"Pages").ok())
        .and_then(|p| p.as_reference().ok())
        .ok_or_else(|| HapError::internal("no Pages node"))?;

    let page_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Page",
        "Parent" => pages_ref,
        "MediaBox" => vec![0.into(), 0.into(), lopdf::Object::Real(w_pt as f32), lopdf::Object::Real(h_pt as f32)],
        "Resources" => lopdf::dictionary!{},
    });

    if let Ok(pages_dict) = doc.get_dictionary_mut(pages_ref) {
        if let Ok(kids) = pages_dict.get_mut(b"Kids") {
            if let Ok(kids_arr) = kids.as_array_mut() {
                kids_arr.push(page_id.into());
            }
        }
        if let Ok(count) = pages_dict.get_mut(b"Count") {
            if let Ok(c) = count.as_i64() {
                *count = lopdf::Object::Integer(c + 1);
            }
        }
    }
    let page_index = doc.get_pages().len() as i32 - 1;
    Ok(json!({"page_index": page_index}))
});

#[derive(Deserialize)] pub struct AddTextParams { pub doc_id: String, pub page_index: i32, pub text: String, pub x: f64, pub y: f64, pub font_size: Option<f64> }
hap_fn!(hap_pdf_add_text, AddTextParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;

    let font_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let pages = doc.get_pages();
    let page_num = p.page_index as u32 + 1;
    let page_obj_id = *pages.get(&page_num).ok_or_else(|| HapError::invalid_param("invalid page_index"))?;

    let font_name = "F1";
    let fs = p.font_size.unwrap_or(12.0);
    let escaped = p.text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
    let content_str = format!("BT /{font_name} {fs} Tf {x} {y} Td ({escaped}) Tj ET", x = p.x, y = p.y);
    let content_id = doc.add_object(lopdf::Stream::new(dictionary!{}, content_str.into_bytes()));

    if let Ok(page_dict) = doc.get_dictionary_mut(page_obj_id) {
        let resources = page_dict.get_mut(b"Resources")
            .ok()
            .and_then(|r| r.as_dict_mut().ok());
        if let Some(res) = resources {
            let fonts = res.get_mut(b"Font")
                .ok()
                .and_then(|f| f.as_dict_mut().ok());
            if let Some(fonts) = fonts {
                fonts.set(font_name.as_bytes(), font_id);
            } else {
                res.set("Font", lopdf::dictionary! { font_name => font_id });
            }
        } else {
            page_dict.set("Resources", lopdf::dictionary! {
                "Font" => lopdf::dictionary! { font_name => font_id },
            });
        }

        let existing = page_dict.get(b"Contents").ok().cloned();
        match existing {
            Some(lopdf::Object::Array(mut arr)) => {
                arr.push(content_id.into());
                page_dict.set("Contents", arr);
            },
            Some(lopdf::Object::Reference(r)) => {
                page_dict.set("Contents", vec![r.into(), content_id.into()]);
            },
            _ => {
                page_dict.set("Contents", content_id);
            }
        }
    }
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct AddImageParams { pub doc_id: String, pub page_index: i32, pub image_path: String, pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
hap_fn!(hap_pdf_add_image, AddImageParams, |p| {
    let img_data = std::fs::read(&p.image_path)?;
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let pages = doc.get_pages();
    let page_num = p.page_index as u32 + 1;
    let page_obj_id = *pages.get(&page_num).ok_or_else(|| HapError::invalid_param("invalid page_index"))?;

    let is_jpg = p.image_path.to_lowercase().ends_with(".jpg") || p.image_path.to_lowercase().ends_with(".jpeg");
    let (filter, color_space, bits) = if is_jpg {
        ("DCTDecode", "DeviceRGB", 8)
    } else {
        return Err(HapError::invalid_param("add_image currently only supports JPEG format"));
    };

    let img_stream = lopdf::Stream::new(
        lopdf::dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => lopdf::Object::Integer(p.width as i64),
            "Height" => lopdf::Object::Integer(p.height as i64),
            "ColorSpace" => color_space,
            "BitsPerComponent" => bits,
            "Filter" => filter,
        },
        img_data,
    );
    let img_id = doc.add_object(img_stream);
    let img_name = "Im1";
    let content_str = format!("q {w} 0 0 {h} {x} {y} cm /{img_name} Do Q", w = p.width, h = p.height, x = p.x, y = p.y);
    let content_id = doc.add_object(lopdf::Stream::new(dictionary!{}, content_str.into_bytes()));

    if let Ok(page_dict) = doc.get_dictionary_mut(page_obj_id) {
        let resources = page_dict.get_mut(b"Resources").ok().and_then(|r| r.as_dict_mut().ok());
        if let Some(res) = resources {
            if let Ok(xobjs) = res.get_mut(b"XObject").and_then(|x| x.as_dict_mut()) {
                xobjs.set(img_name, img_id);
            } else {
                res.set("XObject", lopdf::dictionary! { img_name => img_id });
            }
        } else {
            page_dict.set("Resources", lopdf::dictionary! {
                "XObject" => lopdf::dictionary! { img_name => img_id },
            });
        }
        append_content_dict(page_dict, content_id);
    }
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct AddLineParams { pub doc_id: String, pub page_index: i32, pub x1: f64, pub y1: f64, pub x2: f64, pub y2: f64, pub line_width: Option<f64> }
hap_fn!(hap_pdf_add_line, AddLineParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let pages = doc.get_pages();
    let page_num = p.page_index as u32 + 1;
    let page_obj_id = *pages.get(&page_num).ok_or_else(|| HapError::invalid_param("invalid page_index"))?;
    let lw = p.line_width.unwrap_or(1.0);
    let content_str = format!("{lw} w {x1} {y1} m {x2} {y2} l S", x1=p.x1, y1=p.y1, x2=p.x2, y2=p.y2);
    let content_id = doc.add_object(lopdf::Stream::new(dictionary!{}, content_str.into_bytes()));
    append_content(doc, page_obj_id, content_id);
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct AddRectParams { pub doc_id: String, pub page_index: i32, pub x: f64, pub y: f64, pub w: f64, pub h: f64, pub fill: Option<bool>, pub line_width: Option<f64> }
hap_fn!(hap_pdf_add_rect, AddRectParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let pages = doc.get_pages();
    let page_num = p.page_index as u32 + 1;
    let page_obj_id = *pages.get(&page_num).ok_or_else(|| HapError::invalid_param("invalid page_index"))?;
    let lw = p.line_width.unwrap_or(1.0);
    let op = if p.fill.unwrap_or(false) { "B" } else { "S" };
    let content_str = format!("{lw} w {x} {y} {w} {h} re {op}", x=p.x, y=p.y, w=p.w, h=p.h);
    let content_id = doc.add_object(lopdf::Stream::new(dictionary!{}, content_str.into_bytes()));
    append_content(doc, page_obj_id, content_id);
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct AddTableParams { #[allow(dead_code)] pub doc_id: String, #[allow(dead_code)] pub page_index: i32, #[allow(dead_code)] pub x: f64, #[allow(dead_code)] pub y: f64, #[allow(dead_code)] pub headers: Vec<String>, #[allow(dead_code)] pub rows: Vec<Vec<String>> }
hap_fn!(hap_pdf_add_table, AddTableParams, |_p| { Ok(json!({"height": 0.0})) });

#[derive(Deserialize)] pub struct RegFontParams { pub doc_id: String, pub font_family: String, pub font_path: String }
hap_fn!(hap_pdf_register_font, RegFontParams, |p| {
    let font_data = std::fs::read(&p.font_path)?;
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let font_stream = lopdf::Stream::new(
        lopdf::dictionary! { "Length1" => lopdf::Object::Integer(font_data.len() as i64) },
        font_data,
    );
    let font_stream_id = doc.add_object(font_stream);
    let font_descriptor = doc.add_object(lopdf::dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => lopdf::Object::Name(p.font_family.as_bytes().to_vec()),
        "Flags" => 32,
        "FontFile2" => font_stream_id,
    });
    let _font_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font",
        "Subtype" => "TrueType",
        "BaseFont" => lopdf::Object::Name(p.font_family.as_bytes().to_vec()),
        "FontDescriptor" => font_descriptor,
        "Encoding" => "WinAnsiEncoding",
    });
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct SplitParams { pub input_path: String, pub output_dir: String, pub pages_per_file: Option<i32> }
hap_fn!(hap_pdf_split, SplitParams, |p| {
    let doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let total_pages = doc.get_pages().len();
    let per_file = p.pages_per_file.unwrap_or(1).max(1) as usize;
    std::fs::create_dir_all(&p.output_dir)?;
    let mut files: Vec<String> = Vec::new();
    let mut file_idx = 1;
    let mut page = 1u32;
    while (page as usize) <= total_pages {
        let mut new_doc = doc.clone();
        let pages_to_delete: Vec<u32> = (1..=total_pages as u32)
            .filter(|&pn| pn < page || pn >= page + per_file as u32)
            .collect();
        if !pages_to_delete.is_empty() {
            new_doc.delete_pages(&pages_to_delete);
        }
        let out_path = format!("{}/part_{:04}.pdf", p.output_dir, file_idx);
        new_doc.save(&out_path).map_err(|e| HapError::internal(e.to_string()))?;
        files.push(out_path);
        page += per_file as u32;
        file_idx += 1;
    }
    Ok(json!({"files": files}))
});

#[derive(Deserialize)] pub struct ToImagesParams { #[allow(dead_code)] pub path: String, #[allow(dead_code)] pub output_dir: String }
hap_fn!(hap_pdf_to_images, ToImagesParams, |_p| { Ok(json!({"files": [], "count": 0})) });

#[derive(Deserialize)] pub struct WatermarkParams { pub input_path: String, pub output_path: String, pub text: String, pub font_size: Option<f64>, pub opacity: Option<f64>, pub rotation: Option<f64> }
hap_fn!(hap_pdf_add_watermark, WatermarkParams, |p| {
    let mut doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;

    let font_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let fs = p.font_size.unwrap_or(48.0);
    let opacity = p.opacity.unwrap_or(0.3);
    let angle = p.rotation.unwrap_or(45.0) * std::f64::consts::PI / 180.0;
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let escaped = p.text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");

    let gs_id = doc.add_object(lopdf::dictionary! {
        "Type" => "ExtGState",
        "ca" => lopdf::Object::Real(opacity as f32),
        "CA" => lopdf::Object::Real(opacity as f32),
    });

    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().iter().map(|(&k, &v)| (k, v)).collect();
    for (_page_num, page_obj_id) in &pages {
        let content_str = format!(
            "/GS1 gs BT /F1 {fs} Tf {cos} {sin} {neg_sin} {cos2} 200 300 Tm 0.7 0.7 0.7 rg ({text}) Tj ET",
            cos = cos_a, sin = sin_a, neg_sin = -sin_a, cos2 = cos_a, text = escaped
        );
        let content_id = doc.add_object(lopdf::Stream::new(dictionary!{}, content_str.into_bytes()));

        if let Ok(page_dict) = doc.get_dictionary_mut(*page_obj_id) {
            let resources = page_dict.get_mut(b"Resources")
                .ok().and_then(|r| r.as_dict_mut().ok());
            if let Some(res) = resources {
                if let Ok(fonts) = res.get_mut(b"Font").and_then(|f| f.as_dict_mut()) {
                    fonts.set("F1", font_id);
                } else {
                    res.set("Font", lopdf::dictionary! { "F1" => font_id });
                }
                if let Ok(gs) = res.get_mut(b"ExtGState").and_then(|g| g.as_dict_mut()) {
                    gs.set("GS1", gs_id);
                } else {
                    res.set("ExtGState", lopdf::dictionary! { "GS1" => gs_id });
                }
            } else {
                page_dict.set("Resources", lopdf::dictionary! {
                    "Font" => lopdf::dictionary! { "F1" => font_id },
                    "ExtGState" => lopdf::dictionary! { "GS1" => gs_id },
                });
            }
            append_content_dict(page_dict, content_id);
        }
    }
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!({"pages": pages.len()}))
});

#[derive(Deserialize)] pub struct AddLinkParams { pub doc_id: String, pub page_index: i32, pub x: f64, pub y: f64, pub w: f64, pub h: f64, pub url: String }
hap_fn!(hap_pdf_add_link, AddLinkParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let pages = doc.get_pages();
    let page_num = p.page_index as u32 + 1;
    let page_obj_id = *pages.get(&page_num).ok_or_else(|| HapError::invalid_param("invalid page_index"))?;
    let annot = doc.add_object(lopdf::dictionary! {
        "Type" => "Annot",
        "Subtype" => "Link",
        "Rect" => vec![
            lopdf::Object::Real(p.x as f32), lopdf::Object::Real(p.y as f32),
            lopdf::Object::Real((p.x + p.w) as f32), lopdf::Object::Real((p.y + p.h) as f32),
        ],
        "A" => lopdf::dictionary! {
            "Type" => "Action",
            "S" => "URI",
            "URI" => lopdf::Object::String(p.url.into_bytes(), lopdf::StringFormat::Literal),
        },
        "Border" => vec![0.into(), 0.into(), 0.into()],
    });
    if let Ok(page_dict) = doc.get_dictionary_mut(page_obj_id) {
        let existing = page_dict.get(b"Annots").ok().cloned();
        match existing {
            Some(lopdf::Object::Array(mut arr)) => {
                arr.push(annot.into());
                page_dict.set("Annots", arr);
            },
            _ => {
                page_dict.set("Annots", vec![annot.into()]);
            }
        }
    }
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct GetBookmarksParams { #[allow(dead_code)] pub path: String }
hap_fn!(hap_pdf_get_bookmarks, GetBookmarksParams, |_p| { Ok(json!([])) });

#[derive(Deserialize)] pub struct AddBookmarkParams { #[allow(dead_code)] pub doc_id: String, #[allow(dead_code)] pub title: String, #[allow(dead_code)] pub page_index: i32 }
hap_fn!(hap_pdf_add_bookmark, AddBookmarkParams, |_p| { Ok(json!({"bookmark_id": "bm_1"})) });

#[derive(Deserialize)] pub struct RotatePageParams { pub input_path: String, pub output_path: String, pub page_index: i32, pub degrees: i32 }
hap_fn!(hap_pdf_rotate_page, RotatePageParams, |p| {
    let mut doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let pages = doc.get_pages();
    let page_num = p.page_index as u32 + 1;
    if let Some(&page_id) = pages.get(&page_num) {
        if let Ok(page) = doc.get_dictionary_mut(page_id) {
            page.set("Rotate", lopdf::Object::Integer(p.degrees as i64));
        }
    }
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct SetPasswordParams { pub input_path: String, pub output_path: String, pub user_password: String, pub owner_password: Option<String> }
hap_fn!(hap_pdf_set_password, SetPasswordParams, |p| {
    let mut doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    if doc.is_encrypted() {
        return Err(HapError::invalid_param("PDF is already encrypted"));
    }
    if doc.trailer.get(b"ID").is_err() {
        let id_bytes = vec![lopdf::Object::String(vec![0u8; 16], lopdf::StringFormat::Literal); 2];
        doc.trailer.set("ID", lopdf::Object::Array(id_bytes));
    }
    let owner_pwd = p.owner_password.as_deref().unwrap_or(&p.user_password);
    let enc_ver = lopdf::EncryptionVersion::V2 {
        document: &doc,
        owner_password: owner_pwd,
        user_password: &p.user_password,
        key_length: 128,
        permissions: lopdf::Permissions::all(),
    };
    let state = lopdf::EncryptionState::try_from(enc_ver)
        .map_err(|e| HapError::internal(format!("{:?}", e)))?;
    doc.encrypt(&state).map_err(|e| HapError::internal(e.to_string()))?;
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct RemovePasswordParams { pub input_path: String, pub output_path: String, pub password: String }
hap_fn!(hap_pdf_remove_password, RemovePasswordParams, |p| {
    let mut doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    if !doc.is_encrypted() {
        return Err(HapError::invalid_param("PDF is not encrypted"));
    }
    doc.authenticate_password(&p.password)
        .map_err(|_| HapError::invalid_param("incorrect password"))?;
    doc.decrypt(&p.password).map_err(|e| HapError::internal(e.to_string()))?;
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct HtmlToPdfParams { #[allow(dead_code)] pub html: String, #[allow(dead_code)] pub output_path: String }
hap_fn!(hap_pdf_html_to_pdf, HtmlToPdfParams, |_p| { Ok(json!({"pages": 0, "size": 0})) });

#[derive(Deserialize)] pub struct AddPageNumbersParams { pub input_path: String, pub output_path: String, pub position: Option<String>, pub font_size: Option<f64>, pub start_number: Option<i32> }
hap_fn!(hap_pdf_add_page_numbers, AddPageNumbersParams, |p| {
    let mut doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let font_id = doc.add_object(lopdf::dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let fs = p.font_size.unwrap_or(10.0);
    let start = p.start_number.unwrap_or(1);
    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().iter().map(|(&k, &v)| (k, v)).collect();
    let total = pages.len() as i32;
    for (idx, (_page_num, page_obj_id)) in pages.iter().enumerate() {
        let num = start + idx as i32;
        let text = format!("{num} / {total}");
        let (x, y) = match p.position.as_deref() {
            Some("top-center") => (297.5, 820.0),
            Some("bottom-left") => (72.0, 20.0),
            Some("bottom-right") => (500.0, 20.0),
            _ => (297.5, 20.0),
        };
        let content_str = format!("BT /F1 {fs} Tf {x} {y} Td ({text}) Tj ET");
        let content_id = doc.add_object(lopdf::Stream::new(dictionary!{}, content_str.into_bytes()));
        if let Ok(page_dict) = doc.get_dictionary_mut(*page_obj_id) {
            let resources = page_dict.get_mut(b"Resources").ok().and_then(|r| r.as_dict_mut().ok());
            if let Some(res) = resources {
                if let Ok(fonts) = res.get_mut(b"Font").and_then(|f| f.as_dict_mut()) {
                    fonts.set("F1", font_id);
                } else { res.set("Font", lopdf::dictionary! { "F1" => font_id }); }
            } else {
                page_dict.set("Resources", lopdf::dictionary! { "Font" => lopdf::dictionary! { "F1" => font_id } });
            }
            append_content_dict(page_dict, content_id);
        }
    }
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!({"pages": total}))
});

#[derive(Deserialize)] pub struct HeaderFooterParams { pub doc_id: String, pub text: String, pub font_size: Option<f64> }
hap_fn!(hap_pdf_add_header, HeaderFooterParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let font_id = doc.add_object(lopdf::dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let fs = p.font_size.unwrap_or(10.0);
    let escaped = p.text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().iter().map(|(&k, &v)| (k, v)).collect();
    for (_pn, page_obj_id) in &pages {
        let content_str = format!("BT /F1 {fs} Tf 72 820 Td ({escaped}) Tj ET");
        let content_id = doc.add_object(lopdf::Stream::new(dictionary!{}, content_str.into_bytes()));
        if let Ok(pd) = doc.get_dictionary_mut(*page_obj_id) {
            let res = pd.get_mut(b"Resources").ok().and_then(|r| r.as_dict_mut().ok());
            if let Some(r) = res {
                if let Ok(f) = r.get_mut(b"Font").and_then(|f| f.as_dict_mut()) { f.set("F1", font_id); }
                else { r.set("Font", lopdf::dictionary! { "F1" => font_id }); }
            } else { pd.set("Resources", lopdf::dictionary! { "Font" => lopdf::dictionary! { "F1" => font_id } }); }
            append_content_dict(pd, content_id);
        }
    }
    Ok(json!(true))
});

hap_fn!(hap_pdf_add_footer, HeaderFooterParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let font_id = doc.add_object(lopdf::dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let fs = p.font_size.unwrap_or(10.0);
    let escaped = p.text.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().iter().map(|(&k, &v)| (k, v)).collect();
    for (_pn, page_obj_id) in &pages {
        let content_str = format!("BT /F1 {fs} Tf 72 20 Td ({escaped}) Tj ET");
        let content_id = doc.add_object(lopdf::Stream::new(dictionary!{}, content_str.into_bytes()));
        if let Ok(pd) = doc.get_dictionary_mut(*page_obj_id) {
            let res = pd.get_mut(b"Resources").ok().and_then(|r| r.as_dict_mut().ok());
            if let Some(r) = res {
                if let Ok(f) = r.get_mut(b"Font").and_then(|f| f.as_dict_mut()) { f.set("F1", font_id); }
                else { r.set("Font", lopdf::dictionary! { "F1" => font_id }); }
            } else { pd.set("Resources", lopdf::dictionary! { "Font" => lopdf::dictionary! { "F1" => font_id } }); }
            append_content_dict(pd, content_id);
        }
    }
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct FlattenParams { pub input_path: String, pub output_path: String }
hap_fn!(hap_pdf_flatten, FlattenParams, |p| {
    let mut doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().iter().map(|(&k, &v)| (k, v)).collect();
    for (_pn, page_obj_id) in &pages {
        if let Ok(page_dict) = doc.get_dictionary_mut(*page_obj_id) {
            page_dict.remove(b"Annots");
        }
    }
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct InsertPageParams { #[allow(dead_code)] pub input_path: String, #[allow(dead_code)] pub output_path: String, #[allow(dead_code)] pub insert_path: String, #[allow(dead_code)] pub position: i32 }
hap_fn!(hap_pdf_insert_page, InsertPageParams, |_p| { Ok(json!({"pages": 0})) });

#[derive(Deserialize)] pub struct DeletePageParams { pub input_path: String, pub output_path: String, pub page_index: i32 }
hap_fn!(hap_pdf_delete_page, DeletePageParams, |p| {
    let mut doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let page_num = p.page_index as u32 + 1;
    doc.delete_pages(&[page_num]);
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    let pages = doc.get_pages().len() as i32;
    Ok(json!({"pages": pages}))
});

#[derive(Deserialize)] pub struct ReorderPagesParams { pub input_path: String, pub output_path: String, pub page_order: Vec<i32> }
hap_fn!(hap_pdf_reorder_pages, ReorderPagesParams, |p| {
    let mut doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let pages = doc.get_pages();
    let page_ids: Vec<lopdf::ObjectId> = p.page_order.iter()
        .filter_map(|&idx| pages.get(&(idx as u32 + 1)).copied())
        .collect();
    if page_ids.len() != p.page_order.len() {
        return Err(HapError::invalid_param("page_order contains invalid page index"));
    }
    let pages_ref = doc.catalog().ok()
        .and_then(|cat| cat.get(b"Pages").ok())
        .and_then(|p| p.as_reference().ok())
        .ok_or_else(|| HapError::internal("no Pages node"))?;
    if let Ok(pages_dict) = doc.get_dictionary_mut(pages_ref) {
        let kids: Vec<lopdf::Object> = page_ids.iter().map(|&id| id.into()).collect();
        pages_dict.set("Kids", kids);
    }
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct GetFormFieldsParams { #[allow(dead_code)] pub path: String }
hap_fn!(hap_pdf_get_form_fields, GetFormFieldsParams, |_p| { Ok(json!([])) });

#[derive(Deserialize)] pub struct FillFormParams { #[allow(dead_code)] pub input_path: String, #[allow(dead_code)] pub output_path: String, #[allow(dead_code)] pub fields: Value }
hap_fn!(hap_pdf_fill_form, FillFormParams, |_p| { Ok(json!({"filled": 0})) });

#[derive(Deserialize)] pub struct GetAnnotationsParams { #[allow(dead_code)] pub path: String, #[allow(dead_code)] pub page_index: Option<i32> }
hap_fn!(hap_pdf_get_annotations, GetAnnotationsParams, |_p| { Ok(json!([])) });

#[derive(Deserialize)] pub struct AddAnnotationParams { pub doc_id: String, pub page_index: i32, pub r#type: String, pub rect: Value, #[allow(dead_code)] pub content: Option<String>, #[allow(dead_code)] pub color: Option<String> }
hap_fn!(hap_pdf_add_annotation, AddAnnotationParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let pages = doc.get_pages();
    let page_num = p.page_index as u32 + 1;
    let page_obj_id = *pages.get(&page_num).ok_or_else(|| HapError::invalid_param("invalid page_index"))?;
    let x = p.rect.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let y = p.rect.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let w = p.rect.get("w").and_then(|v| v.as_f64()).unwrap_or(50.0);
    let h = p.rect.get("h").and_then(|v| v.as_f64()).unwrap_or(20.0);
    let subtype = match p.r#type.as_str() {
        "highlight" => "Highlight",
        "underline" => "Underline",
        "strikeout" => "StrikeOut",
        "note" => "Text",
        "freetext" => "FreeText",
        _ => "Text",
    };
    let mut annot_dict = lopdf::dictionary! {
        "Type" => "Annot",
        "Subtype" => subtype,
        "Rect" => vec![
            lopdf::Object::Real(x as f32), lopdf::Object::Real(y as f32),
            lopdf::Object::Real((x + w) as f32), lopdf::Object::Real((y + h) as f32),
        ],
    };
    if let Some(ref content) = p.content {
        annot_dict.set("Contents", lopdf::Object::String(content.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    }
    let annot_id = doc.add_object(annot_dict);
    if let Ok(page_dict) = doc.get_dictionary_mut(page_obj_id) {
        let existing = page_dict.get(b"Annots").ok().cloned();
        match existing {
            Some(lopdf::Object::Array(mut arr)) => {
                arr.push(annot_id.into());
                page_dict.set("Annots", arr);
            },
            _ => { page_dict.set("Annots", vec![annot_id.into()]); }
        }
    }
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct RemoveAnnotationParams { pub doc_id: String, pub page_index: i32, pub annotation_index: i32 }
hap_fn!(hap_pdf_remove_annotation, RemoveAnnotationParams, |p| {
    let mut map = DOC_MAP.lock().unwrap();
    let doc = map.get_mut(&p.doc_id).ok_or_else(|| HapError::invalid_param("invalid doc_id"))?;
    let pages = doc.get_pages();
    let page_num = p.page_index as u32 + 1;
    let page_obj_id = *pages.get(&page_num).ok_or_else(|| HapError::invalid_param("invalid page_index"))?;
    if let Ok(page_dict) = doc.get_dictionary_mut(page_obj_id) {
        if let Ok(annots) = page_dict.get_mut(b"Annots") {
            if let Ok(arr) = annots.as_array_mut() {
                let idx = p.annotation_index as usize;
                if idx < arr.len() { arr.remove(idx); }
            }
        }
    }
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct StampImageParams { pub input_path: String, pub output_path: String, pub image_path: String, #[allow(dead_code)] pub opacity: Option<f64>, #[allow(dead_code)] pub scale: Option<f64> }
hap_fn!(hap_pdf_stamp_image, StampImageParams, |p| {
    if !p.image_path.to_lowercase().ends_with(".jpg") && !p.image_path.to_lowercase().ends_with(".jpeg") {
        return Err(HapError::invalid_param("stamp_image currently only supports JPEG format"));
    }
    let img_data = std::fs::read(&p.image_path)?;
    let mut doc = lopdf::Document::load(&p.input_path).map_err(|e| HapError::internal(e.to_string()))?;
    let scale = p.scale.unwrap_or(0.15);
    let opacity = p.opacity.unwrap_or(0.3);
    let img_w = 100.0 * scale;
    let img_h = 100.0 * scale;

    let gs_id = doc.add_object(lopdf::dictionary! {
        "Type" => "ExtGState",
        "ca" => lopdf::Object::Real(opacity as f32),
        "CA" => lopdf::Object::Real(opacity as f32),
    });

    let pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().iter().map(|(&k, &v)| (k, v)).collect();
    for (_pn, page_obj_id) in &pages {
        let img_stream = lopdf::Stream::new(
            lopdf::dictionary! {
                "Type" => "XObject", "Subtype" => "Image",
                "Width" => 100, "Height" => 100,
                "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            },
            img_data.clone(),
        );
        let img_id = doc.add_object(img_stream);
        let content_str = format!("q /GS1 gs {w} 0 0 {h} 450 30 cm /StImg Do Q", w = img_w, h = img_h);
        let content_id = doc.add_object(lopdf::Stream::new(dictionary!{}, content_str.into_bytes()));
        if let Ok(page_dict) = doc.get_dictionary_mut(*page_obj_id) {
            let resources = page_dict.get_mut(b"Resources").ok().and_then(|r| r.as_dict_mut().ok());
            if let Some(res) = resources {
                if let Ok(xobjs) = res.get_mut(b"XObject").and_then(|x| x.as_dict_mut()) {
                    xobjs.set("StImg", img_id);
                } else { res.set("XObject", lopdf::dictionary! { "StImg" => img_id }); }
                if let Ok(gs) = res.get_mut(b"ExtGState").and_then(|g| g.as_dict_mut()) {
                    gs.set("GS1", gs_id);
                } else { res.set("ExtGState", lopdf::dictionary! { "GS1" => gs_id }); }
            } else {
                page_dict.set("Resources", lopdf::dictionary! {
                    "XObject" => lopdf::dictionary! { "StImg" => img_id },
                    "ExtGState" => lopdf::dictionary! { "GS1" => gs_id },
                });
            }
            append_content_dict(page_dict, content_id);
        }
    }
    doc.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

#[derive(Deserialize)] pub struct ExtractImagesParams { #[allow(dead_code)] pub path: String, #[allow(dead_code)] pub output_dir: String }
hap_fn!(hap_pdf_extract_images, ExtractImagesParams, |_p| { Ok(json!({"images": [], "count": 0})) });

hap_fn!(hap_pdf_list_open, Value, |_p| {
    let map = DOC_MAP.lock().unwrap();
    let list: Vec<Value> = map.keys().map(|id| json!({"doc_id": id, "path": "", "pages": 0})).collect();
    Ok(json!(list))
});
