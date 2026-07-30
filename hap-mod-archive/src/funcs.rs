use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use std::io::{Read, Write};
use std::path::Path;
use base64::Engine;

// ---------- zip ----------
#[derive(Deserialize)]
pub struct ZipParams { pub source_paths: Vec<String>, pub dest_path: String, #[allow(dead_code)] pub password: Option<String>, #[allow(dead_code)] pub encryption: Option<String>, pub compression_level: Option<i32>, #[allow(dead_code)] pub method: Option<String>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_archive_zip, ZipParams, |p| {
    let file = std::fs::File::create(&p.dest_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(p.compression_level.unwrap_or(6) as i64));
    let mut count = 0i32;
    for path_str in &p.source_paths {
        let path = Path::new(path_str);
        if path.is_file() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            zip.start_file(name, opts).map_err(|e| HapError::internal(e.to_string()))?;
            let data = std::fs::read(path)?;
            zip.write_all(&data)?;
            count += 1;
        } else if path.is_dir() {
            add_dir_to_zip(&mut zip, path, path, opts, &mut count)?;
        }
    }
    zip.finish().map_err(|e| HapError::internal(e.to_string()))?;
    let size = std::fs::metadata(&p.dest_path)?.len() as i64;
    Ok(json!({"success": true, "size": size, "file_count": count}))
});

fn add_dir_to_zip<W: Write + std::io::Seek>(zip: &mut zip::ZipWriter<W>, base: &Path, dir: &Path, opts: zip::write::SimpleFileOptions, count: &mut i32) -> Result<(), HapError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap_or(&path);
        if path.is_dir() {
            zip.add_directory(rel.to_string_lossy(), opts).map_err(|e| HapError::internal(e.to_string()))?;
            add_dir_to_zip(zip, base, &path, opts, count)?;
        } else {
            zip.start_file(rel.to_string_lossy(), opts).map_err(|e| HapError::internal(e.to_string()))?;
            zip.write_all(&std::fs::read(&path)?)?;
            *count += 1;
        }
    }
    Ok(())
}

// ---------- unzip ----------
#[derive(Deserialize)]
pub struct UnzipParams { pub archive_path: String, pub dest_dir: String, #[allow(dead_code)] pub password: Option<String>, pub overwrite: Option<bool>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_archive_unzip, UnzipParams, |p| {
    let file = std::fs::File::open(&p.archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| HapError::internal(e.to_string()))?;
    let overwrite = p.overwrite.unwrap_or(true);
    let mut files = vec![];
    let mut total_size = 0i64;
    std::fs::create_dir_all(&p.dest_dir)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| HapError::internal(e.to_string()))?;
        let outpath = Path::new(&p.dest_dir).join(entry.mangled_name());
        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if !overwrite && outpath.exists() { continue; }
            if let Some(parent) = outpath.parent() { std::fs::create_dir_all(parent)?; }
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            std::fs::write(&outpath, &buf)?;
            total_size += buf.len() as i64;
            files.push(outpath.to_string_lossy().to_string());
        }
    }
    Ok(json!({"files": files, "total_size": total_size}))
});

// ---------- list_entries ----------
#[derive(Deserialize)]
pub struct ListEntriesParams { pub archive_path: String, #[allow(dead_code)] pub password: Option<String> }
hap_fn!(hap_archive_list_entries, ListEntriesParams, |p| {
    let file = std::fs::File::open(&p.archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| HapError::internal(e.to_string()))?;
    let mut entries = vec![];
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| HapError::internal(e.to_string()))?;
        entries.push(json!({
            "name": entry.name(),
            "size": entry.size() as i64,
            "compressed_size": entry.compressed_size() as i64,
            "is_dir": entry.is_dir(),
            "encrypted": entry.encrypted(),
        }));
    }
    Ok(json!(entries))
});

// ---------- extract_entry ----------
#[derive(Deserialize)]
pub struct ExtractEntryParams { pub archive_path: String, pub entry_name: String, pub dest_path: String, #[allow(dead_code)] pub password: Option<String>, pub overwrite: Option<bool> }
hap_fn!(hap_archive_extract_entry, ExtractEntryParams, |p| {
    if !p.overwrite.unwrap_or(true) && Path::new(&p.dest_path).exists() {
        return Err(HapError::invalid_param("target already exists"));
    }
    let file = std::fs::File::open(&p.archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| HapError::internal(e.to_string()))?;
    let mut entry = archive.by_name(&p.entry_name).map_err(|e| HapError::internal(e.to_string()))?;
    if let Some(parent) = Path::new(&p.dest_path).parent() { std::fs::create_dir_all(parent)?; }
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    std::fs::write(&p.dest_path, &buf)?;
    Ok(json!(true))
});

// ---------- tar ----------
#[derive(Deserialize)]
pub struct TarParams { pub source_paths: Vec<String>, pub dest_path: String, pub compress: Option<String>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_archive_tar, TarParams, |p| {
    let file = std::fs::File::create(&p.dest_path)?;
    let compress = p.compress.as_deref().unwrap_or("none");
    let mut count = 0i32;
    match compress {
        "gzip" => {
            let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut ar = tar::Builder::new(enc);
            for path_str in &p.source_paths {
                let path = Path::new(path_str);
                if path.is_file() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                    ar.append_path_with_name(path, name).map_err(|e| HapError::internal(e.to_string()))?;
                    count += 1;
                } else if path.is_dir() {
                    ar.append_dir_all(path.file_name().and_then(|n| n.to_str()).unwrap_or("dir"), path)
                        .map_err(|e| HapError::internal(e.to_string()))?;
                }
            }
            ar.finish().map_err(|e| HapError::internal(e.to_string()))?;
        }
        "zstd" => {
            let enc = zstd::Encoder::new(file, 3).map_err(|e| HapError::internal(e.to_string()))?;
            let mut ar = tar::Builder::new(enc);
            for path_str in &p.source_paths {
                let path = Path::new(path_str);
                if path.is_file() {
                    ar.append_path_with_name(path, path.file_name().and_then(|n| n.to_str()).unwrap_or("file"))
                        .map_err(|e| HapError::internal(e.to_string()))?;
                    count += 1;
                }
            }
            let enc = ar.into_inner().map_err(|e| HapError::internal(e.to_string()))?;
            enc.finish().map_err(|e| HapError::internal(e.to_string()))?;
        }
        _ => {
            let mut ar = tar::Builder::new(file);
            for path_str in &p.source_paths {
                let path = Path::new(path_str);
                if path.is_file() {
                    ar.append_path_with_name(path, path.file_name().and_then(|n| n.to_str()).unwrap_or("file"))
                        .map_err(|e| HapError::internal(e.to_string()))?;
                    count += 1;
                }
            }
            ar.finish().map_err(|e| HapError::internal(e.to_string()))?;
        }
    }
    let size = std::fs::metadata(&p.dest_path)?.len() as i64;
    Ok(json!({"success": true, "size": size, "file_count": count}))
});

// ---------- untar ----------
#[derive(Deserialize)]
pub struct UntarParams { pub archive_path: String, pub dest_dir: String, #[allow(dead_code)] pub overwrite: Option<bool>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_archive_untar, UntarParams, |p| {
    std::fs::create_dir_all(&p.dest_dir)?;
    let file = std::fs::File::open(&p.archive_path)?;
    let path = Path::new(&p.archive_path);
    let ext = path.to_string_lossy();
    let mut files = vec![];
    let mut total_size = 0i64;
    if ext.ends_with(".tar.gz") || ext.ends_with(".tgz") {
        let dec = flate2::read::GzDecoder::new(file);
        let mut ar = tar::Archive::new(dec);
        for entry in ar.entries().map_err(|e| HapError::internal(e.to_string()))? {
            let mut entry = entry.map_err(|e| HapError::internal(e.to_string()))?;
            entry.unpack_in(&p.dest_dir).map_err(|e| HapError::internal(e.to_string()))?;
            total_size += entry.size() as i64;
            if let Ok(path) = entry.path() { files.push(path.to_string_lossy().to_string()); }
        }
    } else if ext.ends_with(".tar.zst") || ext.ends_with(".tar.zstd") {
        let dec = zstd::Decoder::new(file).map_err(|e| HapError::internal(e.to_string()))?;
        let mut ar = tar::Archive::new(dec);
        for entry in ar.entries().map_err(|e| HapError::internal(e.to_string()))? {
            let mut entry = entry.map_err(|e| HapError::internal(e.to_string()))?;
            entry.unpack_in(&p.dest_dir).map_err(|e| HapError::internal(e.to_string()))?;
            total_size += entry.size() as i64;
            if let Ok(path) = entry.path() { files.push(path.to_string_lossy().to_string()); }
        }
    } else {
        let mut ar = tar::Archive::new(file);
        for entry in ar.entries().map_err(|e| HapError::internal(e.to_string()))? {
            let mut entry = entry.map_err(|e| HapError::internal(e.to_string()))?;
            entry.unpack_in(&p.dest_dir).map_err(|e| HapError::internal(e.to_string()))?;
            total_size += entry.size() as i64;
            if let Ok(path) = entry.path() { files.push(path.to_string_lossy().to_string()); }
        }
    }
    Ok(json!({"files": files, "total_size": total_size}))
});

// ---------- compress_bytes ----------
#[derive(Deserialize)]
pub struct CompressBytesParams { pub data: String, pub algorithm: String, pub level: Option<i32> }
hap_fn!(hap_archive_compress_bytes, CompressBytesParams, |p| {
    let input = p.data.as_bytes();
    let compressed = match p.algorithm.as_str() {
        "gzip" => {
            let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(p.level.unwrap_or(6) as u32));
            enc.write_all(input)?;
            enc.finish()?
        }
        "zstd" => {
            zstd::encode_all(std::io::Cursor::new(input), p.level.unwrap_or(3))
                .map_err(|e| HapError::internal(e.to_string()))?
        }
        _ => return Err(HapError::invalid_param(format!("unsupported: {}", p.algorithm))),
    };
    Ok(json!(base64::engine::general_purpose::STANDARD.encode(&compressed)))
});

// ---------- decompress_bytes ----------
#[derive(Deserialize)]
pub struct DecompressBytesParams { pub data: String, pub algorithm: String }
hap_fn!(hap_archive_decompress_bytes, DecompressBytesParams, |p| {
    let input = base64::engine::general_purpose::STANDARD.decode(&p.data)
        .map_err(|e| HapError::invalid_param(format!("base64: {e}")))?;
    let decompressed = match p.algorithm.as_str() {
        "gzip" => {
            let mut dec = flate2::read::GzDecoder::new(&input[..]);
            let mut buf = Vec::new();
            dec.read_to_end(&mut buf)?;
            buf
        }
        "zstd" => {
            zstd::decode_all(std::io::Cursor::new(&input))
                .map_err(|e| HapError::internal(e.to_string()))?
        }
        _ => return Err(HapError::invalid_param(format!("unsupported: {}", p.algorithm))),
    };
    Ok(json!(String::from_utf8_lossy(&decompressed).into_owned()))
});

// ---------- is_archive ----------
#[derive(Deserialize)]
pub struct IsArchiveParams { pub path: String }
hap_fn!(hap_archive_is_archive, IsArchiveParams, |p| {
    let mut buf = [0u8; 512];
    let n = {
        let mut f = std::fs::File::open(&p.path)?;
        f.read(&mut buf)?
    };
    let buf = &buf[..n];
    if buf.len() >= 4 && buf[0..4] == [0x50, 0x4B, 0x03, 0x04] {
        return Ok(json!({"is_archive": true, "format": "zip"}));
    }
    if buf.len() >= 262 && buf[257..262] == [0x75, 0x73, 0x74, 0x61, 0x72] {
        return Ok(json!({"is_archive": true, "format": "tar"}));
    }
    if buf.len() >= 3 && buf[0] == 0x1F && buf[1] == 0x8B {
        return Ok(json!({"is_archive": true, "format": "tar.gz"}));
    }
    if buf.len() >= 4 && buf[0..4] == [0x28, 0xB5, 0x2F, 0xFD] {
        return Ok(json!({"is_archive": true, "format": "tar.zst"}));
    }
    Ok(json!({"is_archive": false, "format": "unknown"}))
});

// ---------- test_archive ----------
#[derive(Deserialize)]
pub struct TestArchiveParams { pub archive_path: String, #[allow(dead_code)] pub password: Option<String> }
hap_fn!(hap_archive_test_archive, TestArchiveParams, |p| {
    let file = std::fs::File::open(&p.archive_path)?;
    match zip::ZipArchive::new(file) {
        Ok(mut ar) => {
            let mut errors = vec![];
            for i in 0..ar.len() {
                match ar.by_index(i) {
                    Ok(mut entry) => {
                        let mut buf = Vec::new();
                        if entry.read_to_end(&mut buf).is_err() {
                            errors.push(format!("读取 {} 失败", entry.name()));
                        }
                    }
                    Err(e) => errors.push(format!("条目 {i}: {e}")),
                }
            }
            Ok(json!({"valid": errors.is_empty(), "errors": errors}))
        }
        Err(e) => Ok(json!({"valid": false, "errors": [e.to_string()]})),
    }
});

// ---------- add_to_zip ----------
#[derive(Deserialize)]
pub struct AddToZipParams { pub archive_path: String, pub file_paths: Vec<String>, #[allow(dead_code)] pub password: Option<String>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_archive_add_to_zip, AddToZipParams, |p| {
    let file = std::fs::File::open(&p.archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| HapError::internal(e.to_string()))?;
    let tmp_path = format!("{}.tmp", p.archive_path);
    let tmp_file = std::fs::File::create(&tmp_path)?;
    let mut writer = zip::ZipWriter::new(tmp_file);
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).map_err(|e| HapError::internal(e.to_string()))?;
        writer.raw_copy_file(entry).map_err(|e| HapError::internal(e.to_string()))?;
    }
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut added = 0i32;
    for fp in &p.file_paths {
        let path = Path::new(fp);
        if path.is_file() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            writer.start_file(name, opts).map_err(|e| HapError::internal(e.to_string()))?;
            writer.write_all(&std::fs::read(path)?)?;
            added += 1;
        }
    }
    writer.finish().map_err(|e| HapError::internal(e.to_string()))?;
    std::fs::rename(&tmp_path, &p.archive_path)?;
    let new_size = std::fs::metadata(&p.archive_path)?.len() as i64;
    Ok(json!({"added": added, "new_size": new_size}))
});

// ---------- remove_from_zip ----------
#[derive(Deserialize)]
pub struct RemoveFromZipParams { pub archive_path: String, pub entry_names: Vec<String> }
hap_fn!(hap_archive_remove_from_zip, RemoveFromZipParams, |p| {
    let file = std::fs::File::open(&p.archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| HapError::internal(e.to_string()))?;
    let tmp_path = format!("{}.tmp", p.archive_path);
    let tmp_file = std::fs::File::create(&tmp_path)?;
    let mut writer = zip::ZipWriter::new(tmp_file);
    let opts = zip::write::SimpleFileOptions::default();
    let mut removed = 0i32;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| HapError::internal(e.to_string()))?;
        if p.entry_names.contains(&entry.name().to_string()) {
            removed += 1;
            continue;
        }
        if entry.is_dir() {
            writer.add_directory(entry.name(), opts).map_err(|e| HapError::internal(e.to_string()))?;
        } else {
            writer.start_file(entry.name(), opts).map_err(|e| HapError::internal(e.to_string()))?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            writer.write_all(&buf)?;
        }
    }
    writer.finish().map_err(|e| HapError::internal(e.to_string()))?;
    std::fs::rename(&tmp_path, &p.archive_path)?;
    let new_size = std::fs::metadata(&p.archive_path)?.len() as i64;
    Ok(json!({"removed": removed, "new_size": new_size}))
});

// ---------- extract_auto ----------
#[derive(Deserialize)]
pub struct ExtractAutoParams { pub archive_path: String, pub dest_dir: String, #[allow(dead_code)] pub password: Option<String>, pub overwrite: Option<bool>, #[allow(dead_code)] pub callback_id: Option<String> }
hap_fn!(hap_archive_extract_auto, ExtractAutoParams, |p| {
    let ext = p.archive_path.to_lowercase();
    if ext.ends_with(".zip") {
        let params = serde_json::json!({"archive_path": p.archive_path, "dest_dir": p.dest_dir, "overwrite": p.overwrite.unwrap_or(true)});
        let ps = serde_json::to_string(&params).unwrap();
        let cs = std::ffi::CString::new(ps).unwrap();
        let result = hap_archive_unzip(cs.as_ptr());
        let s = unsafe { std::ffi::CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { hap_common::ffi::free_c_string(result as *mut _) };
        let mut v: serde_json::Value = serde_json::from_str(&s).unwrap();
        v["format"] = json!("zip");
        return Ok(v);
    }
    if ext.ends_with(".tar") || ext.ends_with(".tar.gz") || ext.ends_with(".tgz") || ext.ends_with(".tar.zst") {
        let params = serde_json::json!({"archive_path": p.archive_path, "dest_dir": p.dest_dir});
        let ps = serde_json::to_string(&params).unwrap();
        let cs = std::ffi::CString::new(ps).unwrap();
        let result = hap_archive_untar(cs.as_ptr());
        let s = unsafe { std::ffi::CStr::from_ptr(result) }.to_str().unwrap().to_string();
        unsafe { hap_common::ffi::free_c_string(result as *mut _) };
        let mut v: serde_json::Value = serde_json::from_str(&s).unwrap();
        v["format"] = json!("tar");
        return Ok(v);
    }
    Err(HapError::invalid_param("unsupported format"))
});

// ---------- read_entry_bytes ----------
#[derive(Deserialize)]
pub struct ReadEntryBytesParams { pub archive_path: String, pub entry_name: String, #[allow(dead_code)] pub password: Option<String> }
hap_fn!(hap_archive_read_entry_bytes, ReadEntryBytesParams, |p| {
    let file = std::fs::File::open(&p.archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| HapError::internal(e.to_string()))?;
    let mut entry = archive.by_name(&p.entry_name).map_err(|e| HapError::internal(e.to_string()))?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    Ok(json!(base64::engine::general_purpose::STANDARD.encode(&buf)))
});
