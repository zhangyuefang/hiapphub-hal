use hap_common::{hap_fn, HapError};
use image::{GenericImageView, DynamicImage};
use serde::Deserialize;
use serde_json::{json, Value};
use super::basic::{load_img, save_img, parse_color};

// ---------- blur ----------
#[derive(Deserialize)]
pub struct BlurParams { pub path: String, pub radius: f64, pub output: String }
hap_fn!(hap_image_blur, BlurParams, |p| {
    let img = load_img(&p.path)?;
    let blurred = img.blur(p.radius as f32);
    save_img(&blurred, &p.output)?;
    Ok(json!(true))
});

// ---------- sharpen ----------
#[derive(Deserialize)]
pub struct SharpenParams { pub path: String, pub amount: f64, pub output: String }
hap_fn!(hap_image_sharpen, SharpenParams, |p| {
    let img = load_img(&p.path)?;
    let sharpened = img.unsharpen(p.amount as f32, 1);
    save_img(&sharpened, &p.output)?;
    Ok(json!(true))
});

// ---------- grayscale ----------
#[derive(Deserialize)]
pub struct GrayscaleParams { pub path: String, pub output: String }
hap_fn!(hap_image_grayscale, GrayscaleParams, |p| {
    let img = load_img(&p.path)?;
    let gray = img.grayscale();
    save_img(&gray, &p.output)?;
    Ok(json!(true))
});

// ---------- adjust (brightness/contrast) ----------
#[derive(Deserialize)]
pub struct AdjustParams {
    pub path: String, pub output: String,
    pub brightness: Option<f64>, pub contrast: Option<f64>,
    #[allow(dead_code)] pub saturation: Option<f64>, #[allow(dead_code)] pub hue: Option<f64>,
}
hap_fn!(hap_image_adjust, AdjustParams, |p| {
    let mut img = load_img(&p.path)?;
    if let Some(b) = p.brightness {
        img = img.brighten((b * 100.0) as i32);
    }
    if let Some(c) = p.contrast {
        img = img.adjust_contrast(c as f32 * 100.0);
    }
    save_img(&img, &p.output)?;
    Ok(json!(true))
});

// ---------- invert ----------
#[derive(Deserialize)]
pub struct InvertParams { pub path: String, pub output: String }
hap_fn!(hap_image_invert, InvertParams, |p| {
    let mut img = load_img(&p.path)?;
    img.invert();
    save_img(&img, &p.output)?;
    Ok(json!(true))
});

// ---------- sepia ----------
#[derive(Deserialize)]
pub struct SepiaParams { pub path: String, pub output: String, pub intensity: Option<f64> }
hap_fn!(hap_image_sepia, SepiaParams, |p| {
    let img = load_img(&p.path)?;
    let rgba = img.to_rgba8();
    let intensity = p.intensity.unwrap_or(0.8) as f32;
    let sepia_img = image::RgbaImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let px = rgba.get_pixel(x, y);
        let r = px[0] as f32; let g = px[1] as f32; let b = px[2] as f32;
        let sr = (r * 0.393 + g * 0.769 + b * 0.189).min(255.0);
        let sg = (r * 0.349 + g * 0.686 + b * 0.168).min(255.0);
        let sb = (r * 0.272 + g * 0.534 + b * 0.131).min(255.0);
        let nr = (r * (1.0 - intensity) + sr * intensity) as u8;
        let ng = (g * (1.0 - intensity) + sg * intensity) as u8;
        let nb = (b * (1.0 - intensity) + sb * intensity) as u8;
        image::Rgba([nr, ng, nb, px[3]])
    });
    save_img(&DynamicImage::ImageRgba8(sepia_img), &p.output)?;
    Ok(json!(true))
});

// ---------- opacity ----------
#[derive(Deserialize)]
pub struct OpacityParams { pub path: String, pub output: String, pub opacity: f64 }
hap_fn!(hap_image_opacity, OpacityParams, |p| {
    let img = load_img(&p.path)?;
    let mut rgba = img.to_rgba8();
    let factor = p.opacity.clamp(0.0, 1.0) as f32;
    for px in rgba.pixels_mut() {
        px[3] = (px[3] as f32 * factor) as u8;
    }
    save_img(&DynamicImage::ImageRgba8(rgba), &p.output)?;
    Ok(json!(true))
});

// ---------- pixelate ----------
#[derive(Deserialize)]
pub struct PixelateParams { pub path: String, pub output: String, pub block_size: u32 }
hap_fn!(hap_image_pixelate, PixelateParams, |p| {
    let img = load_img(&p.path)?;
    let (w, h) = img.dimensions();
    let bs = p.block_size.max(1);
    let small = img.resize_exact(w / bs, h / bs, image::imageops::FilterType::Nearest);
    let pixelated = small.resize_exact(w, h, image::imageops::FilterType::Nearest);
    save_img(&pixelated, &p.output)?;
    Ok(json!(true))
});

// ---------- histogram ----------
#[derive(Deserialize)]
pub struct HistogramParams { pub path: String }
hap_fn!(hap_image_histogram, HistogramParams, |p| {
    let img = load_img(&p.path)?;
    let rgba = img.to_rgba8();
    let mut r_hist = vec![0i32; 256];
    let mut g_hist = vec![0i32; 256];
    let mut b_hist = vec![0i32; 256];
    let mut l_hist = vec![0i32; 256];
    for px in rgba.pixels() {
        r_hist[px[0] as usize] += 1;
        g_hist[px[1] as usize] += 1;
        b_hist[px[2] as usize] += 1;
        let lum = (0.299 * px[0] as f64 + 0.587 * px[1] as f64 + 0.114 * px[2] as f64) as usize;
        l_hist[lum.min(255)] += 1;
    }
    Ok(json!({"r": r_hist, "g": g_hist, "b": b_hist, "luminance": l_hist}))
});

// ---------- overlay ----------
#[derive(Deserialize)]
pub struct OverlayParams {
    pub base_path: String, pub overlay_path: String, pub output: String,
    pub x: i32, pub y: i32, #[allow(dead_code)] pub opacity: Option<f64>,
}
hap_fn!(hap_image_overlay, OverlayParams, |p| {
    let mut base = load_img(&p.base_path)?;
    let overlay = load_img(&p.overlay_path)?;
    image::imageops::overlay(&mut base, &overlay, p.x as i64, p.y as i64);
    save_img(&base, &p.output)?;
    Ok(json!(true))
});

// ---------- pad ----------
#[derive(Deserialize)]
pub struct PadParams {
    pub path: String, pub output: String,
    pub top: u32, pub right: u32, pub bottom: u32, pub left: u32,
    pub color: Option<String>,
}
hap_fn!(hap_image_pad, PadParams, |p| {
    let img = load_img(&p.path)?;
    let (w, h) = img.dimensions();
    let nw = w + p.left + p.right;
    let nh = h + p.top + p.bottom;
    let bg = parse_color(p.color.as_deref().unwrap_or("#FFFFFF"));
    let mut new_img = image::RgbaImage::from_pixel(nw, nh, bg);
    image::imageops::overlay(&mut new_img, &img.to_rgba8(), p.left as i64, p.top as i64);
    save_img(&DynamicImage::ImageRgba8(new_img), &p.output)?;
    Ok(json!(true))
});

// ---------- concat ----------
#[derive(Deserialize)]
pub struct ConcatParams {
    pub paths: Vec<String>, pub output: String, pub direction: String,
    pub gap: Option<u32>, pub bg_color: Option<String>, #[allow(dead_code)] pub align: Option<String>,
}
hap_fn!(hap_image_concat, ConcatParams, |p| {
    let imgs: Vec<DynamicImage> = p.paths.iter().map(|path| load_img(path)).collect::<Result<_, _>>()?;
    if imgs.is_empty() { return Err(HapError::invalid_param("at least one image required")); }
    let gap = p.gap.unwrap_or(0);
    let bg = parse_color(p.bg_color.as_deref().unwrap_or("#FFFFFF"));
    if p.direction == "horizontal" {
        let total_w: u32 = imgs.iter().map(|i| i.width()).sum::<u32>() + gap * (imgs.len() as u32 - 1);
        let max_h: u32 = imgs.iter().map(|i| i.height()).max().unwrap_or(0);
        let mut canvas = image::RgbaImage::from_pixel(total_w, max_h, bg);
        let mut x = 0i64;
        for img in &imgs {
            image::imageops::overlay(&mut canvas, &img.to_rgba8(), x, 0);
            x += img.width() as i64 + gap as i64;
        }
        save_img(&DynamicImage::ImageRgba8(canvas), &p.output)?;
    } else {
        let max_w: u32 = imgs.iter().map(|i| i.width()).max().unwrap_or(0);
        let total_h: u32 = imgs.iter().map(|i| i.height()).sum::<u32>() + gap * (imgs.len() as u32 - 1);
        let mut canvas = image::RgbaImage::from_pixel(max_w, total_h, bg);
        let mut y = 0i64;
        for img in &imgs {
            image::imageops::overlay(&mut canvas, &img.to_rgba8(), 0, y);
            y += img.height() as i64 + gap as i64;
        }
        save_img(&DynamicImage::ImageRgba8(canvas), &p.output)?;
    }
    Ok(json!(true))
});

// ---------- mask ----------
#[derive(Deserialize)]
pub struct MaskParams { pub path: String, pub mask_path: String, pub output: String }
hap_fn!(hap_image_mask, MaskParams, |p| {
    let img = load_img(&p.path)?;
    let mask = load_img(&p.mask_path)?.to_luma8();
    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    for y in 0..h {
        for x in 0..w {
            let mask_val = if x < mask.width() && y < mask.height() { mask.get_pixel(x, y)[0] } else { 0 };
            let px = rgba.get_pixel_mut(x, y);
            px[3] = ((px[3] as f32) * (mask_val as f32 / 255.0)) as u8;
        }
    }
    save_img(&DynamicImage::ImageRgba8(rgba), &p.output)?;
    Ok(json!(true))
});

// ---------- trim ----------
#[derive(Deserialize)]
pub struct TrimParams { pub path: String, pub output: String, pub tolerance: Option<i32> }
hap_fn!(hap_image_trim, TrimParams, |p| {
    let img = load_img(&p.path)?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let tol = p.tolerance.unwrap_or(10);
    let bg = rgba.get_pixel(0, 0);
    let is_bg = |px: &image::Rgba<u8>| -> bool {
        (px[0] as i32 - bg[0] as i32).abs() <= tol &&
        (px[1] as i32 - bg[1] as i32).abs() <= tol &&
        (px[2] as i32 - bg[2] as i32).abs() <= tol
    };
    let mut top = 0u32; let mut bottom = h; let mut left = 0u32; let mut right = w;
    'outer_top: for y in 0..h { for x in 0..w { if !is_bg(rgba.get_pixel(x, y)) { top = y; break 'outer_top; } } }
    'outer_bot: for y in (0..h).rev() { for x in 0..w { if !is_bg(rgba.get_pixel(x, y)) { bottom = y + 1; break 'outer_bot; } } }
    'outer_left: for x in 0..w { for y in top..bottom { if !is_bg(rgba.get_pixel(x, y)) { left = x; break 'outer_left; } } }
    'outer_right: for x in (0..w).rev() { for y in top..bottom { if !is_bg(rgba.get_pixel(x, y)) { right = x + 1; break 'outer_right; } } }
    let nw = right - left; let nh = bottom - top;
    let trimmed = top > 0 || left > 0 || right < w || bottom < h;
    if trimmed {
        let sub = image::imageops::crop_imm(&rgba, left, top, nw, nh).to_image();
        save_img(&DynamicImage::ImageRgba8(sub), &p.output)?;
    } else {
        save_img(&img, &p.output)?;
    }
    Ok(json!({"trimmed": trimmed, "new_width": nw, "new_height": nh}))
});

// ---------- round_corners ----------
#[derive(Deserialize)]
pub struct RoundCornersParams { pub path: String, pub output: String, pub radius: u32 }
hap_fn!(hap_image_round_corners, RoundCornersParams, |p| {
    let img = load_img(&p.path)?;
    let mut rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let r = p.radius.min(w / 2).min(h / 2);
    for y in 0..h {
        for x in 0..w {
            let (cx, cy) = if x < r && y < r { (r, r) }
                else if x >= w - r && y < r { (w - r - 1, r) }
                else if x < r && y >= h - r { (r, h - r - 1) }
                else if x >= w - r && y >= h - r { (w - r - 1, h - r - 1) }
                else { continue };
            let dx = x as f64 - cx as f64; let dy = y as f64 - cy as f64;
            if dx * dx + dy * dy > (r as f64) * (r as f64) {
                rgba.get_pixel_mut(x, y)[3] = 0;
            }
        }
    }
    save_img(&DynamicImage::ImageRgba8(rgba), &p.output)?;
    Ok(json!(true))
});

// ---------- extract_colors ----------
#[derive(Deserialize)]
pub struct ExtractColorsParams { pub path: String, pub count: Option<i32> }
hap_fn!(hap_image_extract_colors, ExtractColorsParams, |p| {
    let img = load_img(&p.path)?;
    let rgba = img.resize(64, 64, image::imageops::FilterType::Nearest).to_rgba8();
    let mut buckets: std::collections::HashMap<(u8, u8, u8), usize> = std::collections::HashMap::new();
    let total = rgba.pixels().count();
    for px in rgba.pixels() {
        let key = (px[0] / 16 * 16, px[1] / 16 * 16, px[2] / 16 * 16);
        *buckets.entry(key).or_default() += 1;
    }
    let mut sorted: Vec<_> = buckets.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let count = p.count.unwrap_or(5) as usize;
    let colors: Vec<Value> = sorted.into_iter().take(count).map(|((r, g, b), cnt)| {
        json!({"color": format!("#{:02x}{:02x}{:02x}", r, g, b), "percent": cnt as f64 / total as f64})
    }).collect();
    Ok(json!(colors))
});

// ---------- compare ----------
#[derive(Deserialize)]
pub struct CompareParams { pub path_a: String, pub path_b: String }
hap_fn!(hap_image_compare, CompareParams, |p| {
    let a = load_img(&p.path_a)?;
    let b = load_img(&p.path_b)?;
    let (aw, ah) = a.dimensions();
    let (bw, bh) = b.dimensions();
    if aw != bw || ah != bh {
        return Ok(json!({"identical": false, "similarity": 0.0, "diff_pixels": (aw * ah) as i32}));
    }
    let ra = a.to_rgba8(); let rb = b.to_rgba8();
    let mut diff = 0i32;
    let total = (aw * ah) as f64;
    for (pa, pb) in ra.pixels().zip(rb.pixels()) {
        if pa != pb { diff += 1; }
    }
    let sim = 1.0 - (diff as f64 / total);
    Ok(json!({"identical": diff == 0, "similarity": sim, "diff_pixels": diff}))
});
