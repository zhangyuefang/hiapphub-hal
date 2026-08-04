use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

// ---------- generate_qr ----------
#[derive(Deserialize)]
pub struct GenerateQrParams {
    pub data: String, pub size: Option<i32>,
    pub error_level: Option<String>, pub format: Option<String>,
    #[allow(dead_code)] pub fg_color: Option<String>, #[allow(dead_code)] pub bg_color: Option<String>,
}
hap_fn!(hap_barcode_generate_qr, GenerateQrParams, |p| {
    let ec = match p.error_level.as_deref() {
        Some("L") => qrcode::EcLevel::L,
        Some("Q") => qrcode::EcLevel::Q,
        Some("H") => qrcode::EcLevel::H,
        _ => qrcode::EcLevel::M,
    };
    let code = qrcode::QrCode::with_error_correction_level(&p.data, ec)
        .map_err(|e| HapError::internal(e.to_string()))?;
    let size = p.size.unwrap_or(256) as u32;
    if p.format.as_deref() == Some("svg") {
        let svg = code.render::<qrcode::render::svg::Color>().min_dimensions(size, size).build();
        return Ok(json!({"data": svg, "width": size, "height": size}));
    }
    let img = code.render::<image::Luma<u8>>().min_dimensions(size, size).build();
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageLuma8(img.clone()).write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| HapError::internal(e.to_string()))?;
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, buf.into_inner());
    Ok(json!({"data": b64, "width": img.width(), "height": img.height()}))
});

// ---------- save_qr ----------
#[derive(Deserialize)]
pub struct SaveQrParams {
    pub data: String, pub output_path: String, pub size: Option<i32>,
    pub error_level: Option<String>, pub format: Option<String>,
    #[allow(dead_code)] pub fg_color: Option<String>, #[allow(dead_code)] pub bg_color: Option<String>,
}
hap_fn!(hap_barcode_save_qr, SaveQrParams, |p| {
    let ec = match p.error_level.as_deref() {
        Some("L") => qrcode::EcLevel::L,
        Some("Q") => qrcode::EcLevel::Q,
        Some("H") => qrcode::EcLevel::H,
        _ => qrcode::EcLevel::M,
    };
    let code = qrcode::QrCode::with_error_correction_level(&p.data, ec)
        .map_err(|e| HapError::internal(e.to_string()))?;
    let size = p.size.unwrap_or(256) as u32;
    if let Some(parent) = Path::new(&p.output_path).parent() { std::fs::create_dir_all(parent)?; }
    if p.format.as_deref() == Some("svg") {
        let svg = code.render::<qrcode::render::svg::Color>().min_dimensions(size, size).build();
        std::fs::write(&p.output_path, svg)?;
    } else {
        let img = code.render::<image::Luma<u8>>().min_dimensions(size, size).build();
        image::DynamicImage::ImageLuma8(img).save(&p.output_path)
            .map_err(|e| HapError::internal(e.to_string()))?;
    }
    Ok(json!(true))
});

fn encode_barcode(data: &str, format: &str) -> Result<Vec<u8>, HapError> {
    use barcoders::sym;
    let encoded: Vec<u8> = match format {
        "code128" => sym::code128::Code128::new(data).map_err(|e| HapError::internal(e.to_string()))?.encode(),
        "code39" => sym::code39::Code39::new(data).map_err(|e| HapError::internal(e.to_string()))?.encode(),
        "ean13" => sym::ean13::EAN13::new(data).map_err(|e| HapError::internal(e.to_string()))?.encode(),
        "ean8" => sym::ean8::EAN8::new(data).map_err(|e| HapError::internal(e.to_string()))?.encode(),
        "codabar" => sym::codabar::Codabar::new(data).map_err(|e| HapError::internal(e.to_string()))?.encode(),
        "code93" => sym::code93::Code93::new(data).map_err(|e| HapError::internal(e.to_string()))?.encode(),
        _ => return Err(HapError::invalid_param(format!("unsupported barcode format: {}", format))),
    };
    Ok(encoded)
}

fn barcode_to_image(encoded: &[u8], height: u32) -> image::DynamicImage {
    use barcoders::generators::image::Image;
    let gen = Image::image_buffer(height);
    let img_buf = gen.generate_buffer(encoded).unwrap_or_default();
    image::DynamicImage::ImageRgba8(img_buf)
}

#[derive(Deserialize)]
pub struct GenerateBarcodeParams {
    pub data: String, pub format: String,
    #[allow(dead_code)] pub width: Option<i32>, pub height: Option<i32>,
    #[allow(dead_code)] pub show_text: Option<bool>,
}
hap_fn!(hap_barcode_generate_barcode, GenerateBarcodeParams, |p| {
    let encoded = encode_barcode(&p.data, &p.format)?;
    let h = p.height.unwrap_or(80) as u32;
    let img = barcode_to_image(&encoded, h);
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).map_err(|e| HapError::internal(e.to_string()))?;
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, buf.into_inner());
    Ok(json!({"data": b64, "width": img.width(), "height": img.height()}))
});

#[derive(Deserialize)]
pub struct SaveBarcodeParams {
    pub data: String, pub format: String, pub output_path: String,
    #[allow(dead_code)] pub width: Option<i32>, pub height: Option<i32>,
}
hap_fn!(hap_barcode_save_barcode, SaveBarcodeParams, |p| {
    let encoded = encode_barcode(&p.data, &p.format)?;
    let h = p.height.unwrap_or(80) as u32;
    let img = barcode_to_image(&encoded, h);
    if let Some(parent) = Path::new(&p.output_path).parent() { std::fs::create_dir_all(parent)?; }
    img.save(&p.output_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});

fn decode_from_dynamic_image(img: &image::DynamicImage) -> serde_json::Value {
    let luma = img.to_luma8();
    let w = luma.width();
    let h = luma.height();
    let raw = luma.into_raw();
    let mut results = Vec::new();
    if let Ok(detected) = rxing::helpers::detect_multiple_in_luma(raw, w, h) {
        for r in &detected {
            results.push(json!({
                "format": format!("{:?}", r.getBarcodeFormat()),
                "data": r.getText(),
            }));
        }
    }
    json!(results)
}

#[derive(Deserialize)]
pub struct DecodeImageParams { pub image_path: String }
hap_fn!(hap_barcode_decode_image, DecodeImageParams, |p| {
    let img = image::open(&p.image_path).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(decode_from_dynamic_image(&img))
});

#[derive(Deserialize)]
pub struct DecodeBase64Params { pub image_data: String }
hap_fn!(hap_barcode_decode_base64, DecodeBase64Params, |p| {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &p.image_data)
        .map_err(|e| HapError::internal(e.to_string()))?;
    let img = image::load_from_memory(&bytes).map_err(|e| HapError::internal(e.to_string()))?;
    Ok(decode_from_dynamic_image(&img))
});

// ---------- generate_qr_with_logo (stub) ----------
#[derive(Deserialize)]
pub struct QrWithLogoParams {
    pub data: String, #[allow(dead_code)] pub logo_path: String, pub output_path: String,
    pub size: Option<i32>, pub error_level: Option<String>,
    #[allow(dead_code)] pub logo_size_ratio: Option<f64>,
}
hap_fn!(hap_barcode_generate_qr_with_logo, QrWithLogoParams, |p| {
    let ec = qrcode::EcLevel::H;
    let code = qrcode::QrCode::with_error_correction_level(&p.data, ec)
        .map_err(|e| HapError::internal(e.to_string()))?;
    let size = p.size.unwrap_or(256) as u32;
    let qr_img = code.render::<image::Luma<u8>>().min_dimensions(size, size).build();
    let mut base = image::DynamicImage::ImageLuma8(qr_img).to_rgba8();
    let logo = image::open(&p.logo_path).map_err(|e| HapError::internal(e.to_string()))?;
    let ratio = p.logo_size_ratio.unwrap_or(0.2);
    let logo_size = (size as f64 * ratio) as u32;
    let logo_resized = logo.resize_exact(logo_size, logo_size, image::imageops::FilterType::Lanczos3);
    let offset_x = (base.width() - logo_size) / 2;
    let offset_y = (base.height() - logo_size) / 2;
    image::imageops::overlay(&mut base, &logo_resized.to_rgba8(), offset_x as i64, offset_y as i64);
    if let Some(parent) = Path::new(&p.output_path).parent() { std::fs::create_dir_all(parent)?; }
    image::DynamicImage::ImageRgba8(base).save(&p.output_path)
        .map_err(|e| HapError::internal(e.to_string()))?;
    Ok(json!(true))
});
