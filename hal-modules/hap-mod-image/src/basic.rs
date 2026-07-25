use hap_common::{hap_fn, HapError};
use image::{GenericImageView, DynamicImage, ImageFormat};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

pub fn load_img(path: &str) -> Result<DynamicImage, HapError> {
    image::open(path).map_err(|e| HapError::internal(format!("failed to load image: {e}")))
}

pub fn save_img(img: &DynamicImage, path: &str) -> Result<(), HapError> {
    if let Some(parent) = Path::new(path).parent() { std::fs::create_dir_all(parent)?; }
    img.save(path).map_err(|e| HapError::internal(format!("failed to save image: {e}")))
}

pub fn parse_color(hex: &str) -> image::Rgba<u8> {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
    let a = if hex.len() >= 8 { u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) } else { 255 };
    image::Rgba([r, g, b, a])
}

pub fn guess_format(path: &str) -> ImageFormat {
    ImageFormat::from_path(path).unwrap_or(ImageFormat::Png)
}

// ---------- info ----------
#[derive(Deserialize)]
pub struct InfoParams { pub path: String }
hap_fn!(hap_image_info, InfoParams, |p| {
    let img = load_img(&p.path)?;
    let (w, h) = img.dimensions();
    let meta = std::fs::metadata(&p.path)?;
    let format = guess_format(&p.path);
    Ok(json!({
        "width": w, "height": h,
        "format": format!("{:?}", format).to_lowercase(),
        "color_space": "rgba",
        "has_alpha": img.color().has_alpha(),
        "file_size": meta.len() as i64,
    }))
});

// ---------- resize ----------
#[derive(Deserialize)]
pub struct ResizeParams { pub path: String, pub width: u32, pub height: u32, pub output: String, pub fit: Option<String> }
hap_fn!(hap_image_resize, ResizeParams, |p| {
    let img = load_img(&p.path)?;
    let resized = match p.fit.as_deref() {
        Some("contain") => img.resize(p.width, p.height, image::imageops::FilterType::Lanczos3),
        Some("fill") => img.resize_exact(p.width, p.height, image::imageops::FilterType::Lanczos3),
        _ => img.resize_to_fill(p.width, p.height, image::imageops::FilterType::Lanczos3),
    };
    save_img(&resized, &p.output)?;
    Ok(json!(true))
});

// ---------- crop ----------
#[derive(Deserialize)]
pub struct CropParams { pub path: String, pub x: u32, pub y: u32, pub w: u32, pub h: u32, pub output: String }
hap_fn!(hap_image_crop, CropParams, |p| {
    let mut img = load_img(&p.path)?;
    let cropped = img.crop(p.x, p.y, p.w, p.h);
    save_img(&cropped, &p.output)?;
    Ok(json!(true))
});

// ---------- rotate ----------
#[derive(Deserialize)]
pub struct RotateParams { pub path: String, pub degrees: i32, pub output: String, #[allow(dead_code)] pub bg_color: Option<String> }
hap_fn!(hap_image_rotate, RotateParams, |p| {
    let img = load_img(&p.path)?;
    let rotated = match p.degrees % 360 {
        90 | -270 => img.rotate90(),
        180 | -180 => img.rotate180(),
        270 | -90 => img.rotate270(),
        _ => img.rotate90(),
    };
    save_img(&rotated, &p.output)?;
    Ok(json!(true))
});

// ---------- flip ----------
#[derive(Deserialize)]
pub struct FlipParams { pub path: String, pub direction: String, pub output: String }
hap_fn!(hap_image_flip, FlipParams, |p| {
    let img = load_img(&p.path)?;
    let flipped = if p.direction == "horizontal" { img.fliph() } else { img.flipv() };
    save_img(&flipped, &p.output)?;
    Ok(json!(true))
});

// ---------- convert ----------
#[derive(Deserialize)]
pub struct ConvertParams { pub path: String, pub format: String, pub output: String, #[allow(dead_code)] pub quality: Option<i32> }
hap_fn!(hap_image_convert, ConvertParams, |p| {
    let img = load_img(&p.path)?;
    save_img(&img, &p.output)?;
    Ok(json!(true))
});

// ---------- compress ----------
#[derive(Deserialize)]
pub struct CompressParams { pub path: String, pub output: String, pub quality: i32, pub max_width: Option<u32>, pub max_height: Option<u32> }
hap_fn!(hap_image_compress, CompressParams, |p| {
    let original_size = std::fs::metadata(&p.path)?.len() as i64;
    let mut img = load_img(&p.path)?;
    if let (Some(mw), Some(mh)) = (p.max_width, p.max_height) {
        img = img.resize(mw, mh, image::imageops::FilterType::Lanczos3);
    }
    let rgb = img.to_rgb8();
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, p.quality as u8);
    rgb.write_with_encoder(encoder).map_err(|e| HapError::internal(e.to_string()))?;
    let data = buf.into_inner();
    let compressed_size = data.len() as i64;
    if let Some(parent) = Path::new(&p.output).parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(&p.output, &data)?;
    Ok(json!({"original_size": original_size, "compressed_size": compressed_size}))
});

// ---------- thumbnail ----------
#[derive(Deserialize)]
pub struct ThumbnailParams { pub path: String, pub size: u32, pub output: String, #[allow(dead_code)] pub format: Option<String> }
hap_fn!(hap_image_thumbnail, ThumbnailParams, |p| {
    let img = load_img(&p.path)?;
    let thumb = img.resize_to_fill(p.size, p.size, image::imageops::FilterType::Lanczos3);
    save_img(&thumb, &p.output)?;
    Ok(json!(true))
});

// ---------- to_base64 ----------
#[derive(Deserialize)]
pub struct ToBase64Params { pub path: String, pub format: Option<String> }
hap_fn!(hap_image_to_base64, ToBase64Params, |p| {
    let data = std::fs::read(&p.path)?;
    Ok(json!(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data)))
});

// ---------- from_base64 ----------
#[derive(Deserialize)]
pub struct FromBase64Params { pub data: String, pub output: String, #[allow(dead_code)] pub format: Option<String> }
hap_fn!(hap_image_from_base64, FromBase64Params, |p| {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &p.data)
        .map_err(|e| HapError::invalid_param(e.to_string()))?;
    if let Some(parent) = Path::new(&p.output).parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(&p.output, &bytes)?;
    Ok(json!(true))
});

// ---------- create_blank ----------
#[derive(Deserialize)]
pub struct CreateBlankParams { pub width: u32, pub height: u32, pub color: Option<String>, pub output: String, #[allow(dead_code)] pub format: Option<String> }
hap_fn!(hap_image_create_blank, CreateBlankParams, |p| {
    let color = parse_color(p.color.as_deref().unwrap_or("#FFFFFF"));
    let img = image::RgbaImage::from_pixel(p.width, p.height, color);
    let dyn_img = DynamicImage::ImageRgba8(img);
    save_img(&dyn_img, &p.output)?;
    Ok(json!(true))
});

// ---------- get_pixel ----------
#[derive(Deserialize)]
pub struct GetPixelParams { pub path: String, pub x: i32, pub y: i32 }
hap_fn!(hap_image_get_pixel, GetPixelParams, |p| {
    let img = load_img(&p.path)?;
    let pixel = img.get_pixel(p.x as u32, p.y as u32);
    Ok(json!({
        "r": pixel[0] as i32, "g": pixel[1] as i32, "b": pixel[2] as i32, "a": pixel[3] as i32,
        "hex": format!("#{:02x}{:02x}{:02x}", pixel[0], pixel[1], pixel[2]),
    }))
});

// ---------- to_icon (multi-size ICO) ----------
#[derive(Deserialize)]
pub struct ToIconParams {
    pub path: String,
    pub output: String,
    pub sizes: Option<Vec<u32>>,
}
hap_fn!(hap_image_to_icon, ToIconParams, |p| {
    let img = load_img(&p.path)?;
    let sizes = p.sizes.unwrap_or_else(|| vec![16, 32, 48, 64, 128, 256]);
    if let Some(parent) = Path::new(&p.output).parent() { std::fs::create_dir_all(parent)?; }
    let mut png_frames: Vec<Vec<u8>> = Vec::new();
    for &s in &sizes {
        let resized = img.resize_to_fill(s, s, image::imageops::FilterType::Lanczos3).to_rgba8();
        let mut png_buf = std::io::Cursor::new(Vec::new());
        resized.write_with_encoder(image::codecs::png::PngEncoder::new(&mut png_buf))
            .map_err(|e| HapError::internal(e.to_string()))?;
        png_frames.push(png_buf.into_inner());
    }
    let count = sizes.len() as u16;
    let mut ico_buf: Vec<u8> = Vec::new();
    ico_buf.extend_from_slice(&[0, 0, 1, 0]);
    ico_buf.extend_from_slice(&count.to_le_bytes());
    let mut data_offset = 6u32 + count as u32 * 16;
    for (i, &s) in sizes.iter().enumerate() {
        let w: u8 = if s >= 256 { 0 } else { s as u8 };
        ico_buf.push(w);
        ico_buf.push(w);
        ico_buf.push(0);
        ico_buf.push(0);
        ico_buf.extend_from_slice(&1u16.to_le_bytes());
        ico_buf.extend_from_slice(&32u16.to_le_bytes());
        let size = png_frames[i].len() as u32;
        ico_buf.extend_from_slice(&size.to_le_bytes());
        ico_buf.extend_from_slice(&data_offset.to_le_bytes());
        data_offset += size;
    }
    for frame in &png_frames {
        ico_buf.extend_from_slice(frame);
    }
    std::fs::write(&p.output, &ico_buf)?;
    Ok(json!({"sizes": sizes, "file_size": ico_buf.len() as i64}))
});
