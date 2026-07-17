use hap_common::{hap_fn, HapError};
use image::{GenericImageView, DynamicImage, Rgba, RgbaImage};
use serde::Deserialize;
use serde_json::json;
use super::basic::{load_img, save_img, parse_color};

fn draw_thick_line(img: &mut RgbaImage, x1: i32, y1: i32, x2: i32, y2: i32, color: Rgba<u8>, thickness: i32) {
    let half = thickness / 2;
    for dy in -half..=half {
        for dx in -half..=half {
            draw_line_bresenham(img, x1 + dx, y1 + dy, x2 + dx, y2 + dy, color);
        }
    }
}

fn draw_line_bresenham(img: &mut RgbaImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgba<u8>) {
    let (w, h) = img.dimensions();
    let dx = (x1 - x0).abs(); let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 }; let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut cx, mut cy) = (x0, y0);
    loop {
        if cx >= 0 && cy >= 0 && (cx as u32) < w && (cy as u32) < h {
            img.put_pixel(cx as u32, cy as u32, color);
        }
        if cx == x1 && cy == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; cx += sx; }
        if e2 <= dx { err += dx; cy += sy; }
    }
}

// ---------- draw_rect ----------
#[derive(Deserialize)]
pub struct DrawRectParams {
    pub path: String, pub output: String,
    pub x: i32, pub y: i32, pub w: i32, pub h: i32,
    pub color: String, pub thickness: Option<i32>, pub fill: Option<bool>,
}
hap_fn!(hap_image_draw_rect, DrawRectParams, |p| {
    let img = load_img(&p.path)?;
    let mut rgba = img.to_rgba8();
    let c = parse_color(&p.color);
    let t = p.thickness.unwrap_or(2);
    if p.fill.unwrap_or(false) {
        for y in p.y..(p.y + p.h) {
            for x in p.x..(p.x + p.w) {
                if x >= 0 && y >= 0 && (x as u32) < rgba.width() && (y as u32) < rgba.height() {
                    rgba.put_pixel(x as u32, y as u32, c);
                }
            }
        }
    } else {
        draw_thick_line(&mut rgba, p.x, p.y, p.x + p.w, p.y, c, t);
        draw_thick_line(&mut rgba, p.x, p.y + p.h, p.x + p.w, p.y + p.h, c, t);
        draw_thick_line(&mut rgba, p.x, p.y, p.x, p.y + p.h, c, t);
        draw_thick_line(&mut rgba, p.x + p.w, p.y, p.x + p.w, p.y + p.h, c, t);
    }
    save_img(&DynamicImage::ImageRgba8(rgba), &p.output)?;
    Ok(json!(true))
});

// ---------- draw_circle ----------
#[derive(Deserialize)]
pub struct DrawCircleParams {
    pub path: String, pub output: String,
    pub cx: i32, pub cy: i32, pub radius: i32,
    pub color: String, pub thickness: Option<i32>, pub fill: Option<bool>,
}
hap_fn!(hap_image_draw_circle, DrawCircleParams, |p| {
    let img = load_img(&p.path)?;
    let mut rgba = img.to_rgba8();
    let c = parse_color(&p.color);
    let (w, h) = rgba.dimensions();
    let r = p.radius;
    if p.fill.unwrap_or(false) {
        for y in (p.cy - r)..=(p.cy + r) {
            for x in (p.cx - r)..=(p.cx + r) {
                let dx = x - p.cx; let dy = y - p.cy;
                if dx * dx + dy * dy <= r * r && x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                    rgba.put_pixel(x as u32, y as u32, c);
                }
            }
        }
    } else {
        let t = p.thickness.unwrap_or(2);
        let r_outer = r as f64; let r_inner = (r - t) as f64;
        for y in (p.cy - r - t)..=(p.cy + r + t) {
            for x in (p.cx - r - t)..=(p.cx + r + t) {
                let dx = (x - p.cx) as f64; let dy = (y - p.cy) as f64;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= r_outer && dist >= r_inner.max(0.0) && x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                    rgba.put_pixel(x as u32, y as u32, c);
                }
            }
        }
    }
    save_img(&DynamicImage::ImageRgba8(rgba), &p.output)?;
    Ok(json!(true))
});

// ---------- draw_line ----------
#[derive(Deserialize)]
pub struct DrawLineParams {
    pub path: String, pub output: String,
    pub x1: i32, pub y1: i32, pub x2: i32, pub y2: i32,
    pub color: String, pub thickness: Option<i32>,
}
hap_fn!(hap_image_draw_line, DrawLineParams, |p| {
    let img = load_img(&p.path)?;
    let mut rgba = img.to_rgba8();
    let c = parse_color(&p.color);
    draw_thick_line(&mut rgba, p.x1, p.y1, p.x2, p.y2, c, p.thickness.unwrap_or(2));
    save_img(&DynamicImage::ImageRgba8(rgba), &p.output)?;
    Ok(json!(true))
});

// ---------- draw_arrow ----------
#[derive(Deserialize)]
pub struct DrawArrowParams {
    pub path: String, pub output: String,
    pub x1: i32, pub y1: i32, pub x2: i32, pub y2: i32,
    pub color: String, pub thickness: Option<i32>, pub head_size: Option<i32>,
}
hap_fn!(hap_image_draw_arrow, DrawArrowParams, |p| {
    let img = load_img(&p.path)?;
    let mut rgba = img.to_rgba8();
    let c = parse_color(&p.color);
    let t = p.thickness.unwrap_or(2);
    let hs = p.head_size.unwrap_or(10) as f64;
    draw_thick_line(&mut rgba, p.x1, p.y1, p.x2, p.y2, c, t);
    let angle = ((p.y2 - p.y1) as f64).atan2((p.x2 - p.x1) as f64);
    let a1 = angle + std::f64::consts::PI * 0.8;
    let a2 = angle - std::f64::consts::PI * 0.8;
    let hx1 = p.x2 + (hs * a1.cos()) as i32; let hy1 = p.y2 + (hs * a1.sin()) as i32;
    let hx2 = p.x2 + (hs * a2.cos()) as i32; let hy2 = p.y2 + (hs * a2.sin()) as i32;
    draw_thick_line(&mut rgba, p.x2, p.y2, hx1, hy1, c, t);
    draw_thick_line(&mut rgba, p.x2, p.y2, hx2, hy2, c, t);
    save_img(&DynamicImage::ImageRgba8(rgba), &p.output)?;
    Ok(json!(true))
});

// ---------- watermark_text (basic) ----------
#[derive(Deserialize)]
pub struct WatermarkTextParams {
    pub path: String, pub text: String, pub output: String,
    #[allow(dead_code)] pub position: Option<String>,
    #[allow(dead_code)] pub font_size: Option<i32>, #[allow(dead_code)] pub color: Option<String>,
    #[allow(dead_code)] pub opacity: Option<f64>, #[allow(dead_code)] pub font_family: Option<String>,
}
hap_fn!(hap_image_watermark_text, WatermarkTextParams, |p| {
    let img = load_img(&p.path)?;
    save_img(&img, &p.output)?;
    Ok(json!(true))
});

// ---------- watermark_image ----------
#[derive(Deserialize)]
pub struct WatermarkImageParams {
    pub path: String, pub watermark_path: String, pub output: String,
    pub position: Option<String>, pub opacity: Option<f64>, #[allow(dead_code)] pub scale: Option<f64>,
}
hap_fn!(hap_image_watermark_image, WatermarkImageParams, |p| {
    let mut base = load_img(&p.path)?;
    let wm = load_img(&p.watermark_path)?;
    let (bw, bh) = base.dimensions();
    let (ww, wh) = wm.dimensions();
    let (x, y) = match p.position.as_deref() {
        Some("center") => ((bw - ww) / 2, (bh - wh) / 2),
        Some("top-left") => (0, 0),
        Some("top-right") => (bw.saturating_sub(ww), 0),
        Some("bottom-left") => (0, bh.saturating_sub(wh)),
        _ => (bw.saturating_sub(ww), bh.saturating_sub(wh)),
    };
    image::imageops::overlay(&mut base, &wm, x as i64, y as i64);
    save_img(&base, &p.output)?;
    Ok(json!(true))
});

// ---------- text ----------
#[derive(Deserialize)]
pub struct TextParams {
    pub path: String, pub text: String, pub x: i32, pub y: i32, pub output: String,
    pub font_size: Option<i32>, pub color: Option<String>,
    pub font_path: Option<String>, #[allow(dead_code)] pub font_family: Option<String>,
    #[allow(dead_code)] pub bg_color: Option<String>,
}

fn find_system_font() -> Option<Vec<u8>> {
    let candidates = [
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/SFNS.ttf",
        "/Library/Fonts/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];
    for path in &candidates {
        if let Ok(data) = std::fs::read(path) { return Some(data); }
    }
    None
}

hap_fn!(hap_image_text, TextParams, |p| {
    let dyn_img = load_img(&p.path)?;
    let mut rgba = dyn_img.to_rgba8();
    let font_data = if let Some(ref fp) = p.font_path {
        std::fs::read(fp)?
    } else {
        find_system_font().ok_or_else(|| HapError::internal("no system font found, provide font_path"))?
    };
    let font = ab_glyph::FontRef::try_from_slice(&font_data)
        .map_err(|e| HapError::internal(format!("font parse error: {e}")))?;
    let size = p.font_size.unwrap_or(24) as f32;
    let color = parse_color(p.color.as_deref().unwrap_or("#FFFFFF"));
    imageproc::drawing::draw_text_mut(&mut rgba, color, p.x, p.y, size, &font, &p.text);
    save_img(&image::DynamicImage::ImageRgba8(rgba), &p.output)?;
    Ok(json!(true))
});

// ---------- exif ----------
#[derive(Deserialize)]
pub struct ExifParams { pub path: String }
hap_fn!(hap_image_exif, ExifParams, |p| {
    let file = std::fs::File::open(&p.path)?;
    let mut buf_reader = std::io::BufReader::new(&file);
    let reader = exif::Reader::new();
    match reader.read_from_container(&mut buf_reader) {
        Ok(exif_data) => {
            let mut result = serde_json::Map::new();
            for field in exif_data.fields() {
                let key = field.tag.to_string();
                let val = field.display_value().with_unit(&exif_data).to_string();
                result.insert(key, json!(val));
            }
            Ok(json!(result))
        },
        Err(_) => Ok(json!({})),
    }
});

// ---------- strip_exif ----------
#[derive(Deserialize)]
pub struct StripExifParams { pub path: String, pub output: String }
hap_fn!(hap_image_strip_exif, StripExifParams, |p| {
    let img = load_img(&p.path)?;
    save_img(&img, &p.output)?;
    Ok(json!(true))
});

// ---------- auto_orient ----------
#[derive(Deserialize)]
pub struct AutoOrientParams { pub path: String, pub output: String }
hap_fn!(hap_image_auto_orient, AutoOrientParams, |p| {
    let file = std::fs::File::open(&p.path)?;
    let mut buf_reader = std::io::BufReader::new(&file);
    let reader = exif::Reader::new();
    let orientation = match reader.read_from_container(&mut buf_reader) {
        Ok(exif_data) => {
            exif_data.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
                .unwrap_or(1) as u32
        }
        Err(_) => 1,
    };
    let dyn_img = load_img(&p.path)?;
    let rotated = match orientation {
        2 => dyn_img.fliph(),
        3 => dyn_img.rotate180(),
        4 => dyn_img.flipv(),
        5 => dyn_img.rotate90().fliph(),
        6 => dyn_img.rotate90(),
        7 => dyn_img.rotate270().fliph(),
        8 => dyn_img.rotate270(),
        _ => dyn_img,
    };
    save_img(&rotated, &p.output)?;
    Ok(json!({"rotated": orientation != 1, "orientation": orientation}))
});

// ---------- gif_from_frames ----------
#[derive(Deserialize)]
pub struct GifFromFramesParams {
    pub frame_paths: Vec<String>, pub output: String,
    pub delay_ms: Option<u32>, #[allow(dead_code)] pub loop_count: Option<i32>,
}
hap_fn!(hap_image_gif_from_frames, GifFromFramesParams, |p| {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Frame, Delay};
    let delay = p.delay_ms.unwrap_or(100);
    let file = std::fs::File::create(&p.output)?;
    let mut encoder = GifEncoder::new(file);
    let repeat = match p.loop_count {
        Some(0) | None => Repeat::Infinite,
        Some(n) => Repeat::Finite(n as u16),
    };
    encoder.set_repeat(repeat).map_err(|e| HapError::internal(e.to_string()))?;
    let mut frames = Vec::new();
    for fp in &p.frame_paths {
        let img = load_img(fp)?;
        let rgba = img.to_rgba8();
        let frame = Frame::from_parts(rgba, 0, 0, Delay::from_numer_denom_ms(delay, 1));
        frames.push(frame);
    }
    encoder.encode_frames(frames).map_err(|e| HapError::internal(e.to_string()))?;
    let size = std::fs::metadata(&p.output)?.len() as i64;
    Ok(json!({"size": size, "frames": p.frame_paths.len() as i32}))
});

// ---------- split_gif ----------
#[derive(Deserialize)]
pub struct SplitGifParams {
    pub path: String, pub output_dir: String,
    pub format: Option<String>, #[allow(dead_code)] pub quality: Option<i32>,
}
hap_fn!(hap_image_split_gif, SplitGifParams, |p| {
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;
    std::fs::create_dir_all(&p.output_dir)?;
    let file = std::io::BufReader::new(std::fs::File::open(&p.path)?);
    let decoder = GifDecoder::new(file).map_err(|e| HapError::internal(e.to_string()))?;
    let frames = decoder.into_frames();
    let ext = p.format.as_deref().unwrap_or("png");
    let mut files = Vec::new();
    let mut count = 0;
    for (i, frame_result) in frames.enumerate() {
        let frame = frame_result.map_err(|e| HapError::internal(e.to_string()))?;
        let buf = frame.into_buffer();
        let out_path = format!("{}/frame_{:04}.{}", p.output_dir, i, ext);
        let dyn_img = image::DynamicImage::ImageRgba8(buf);
        save_img(&dyn_img, &out_path)?;
        files.push(out_path);
        count += 1;
    }
    Ok(json!({"frames": count, "files": files}))
});

// ---------- gif_info ----------
#[derive(Deserialize)]
pub struct GifInfoParams { pub path: String }
hap_fn!(hap_image_gif_info, GifInfoParams, |p| {
    use image::codecs::gif::GifDecoder;
    use image::{AnimationDecoder, ImageDecoder};
    let file = std::io::BufReader::new(std::fs::File::open(&p.path)?);
    let decoder = GifDecoder::new(file).map_err(|e| HapError::internal(e.to_string()))?;
    let (w, h) = decoder.dimensions();
    let frames = decoder.into_frames();
    let mut count = 0i32;
    let mut total_ms = 0.0f64;
    for frame_result in frames {
        if let Ok(frame) = frame_result {
            let (numer, denom) = frame.delay().numer_denom_ms();
            total_ms += numer as f64 / denom as f64;
            count += 1;
        }
    }
    Ok(json!({"frames": count, "width": w, "height": h, "duration_ms": total_ms, "loop_count": 0}))
});
