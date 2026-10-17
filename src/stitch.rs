use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};
use std::fs;
use std::path::{Path, PathBuf};

pub fn stitch_horizontal(inputs: &[PathBuf], output: &Path) -> Result<(), String> {
    if inputs.len() < 2 {
        return Err("拼接至少需要 2 张图片".into());
    }
    let mut images: Vec<DynamicImage> = Vec::new();
    for p in inputs {
        images.push(image::open(p).map_err(|e| e.to_string())?);
    }
    let target_h = images.iter().map(|i| i.height()).min().unwrap_or(0);
    if target_h == 0 {
        return Err("图片高度无效".into());
    }
    let scaled: Vec<DynamicImage> = images
        .into_iter()
        .map(|img| {
            if img.height() == target_h {
                img
            } else {
                let w = (img.width() as f32 * target_h as f32 / img.height() as f32)
                    .round()
                    .max(1.0) as u32;
                img.resize(w, target_h, FilterType::Lanczos3)
            }
        })
        .collect();
    let total_w: u32 = scaled.iter().map(|i| i.width()).sum();
    let mut canvas = RgbaImage::new(total_w, target_h);
    let mut x = 0u32;
    for img in scaled {
        let rgba = img.to_rgba8();
        image::imageops::overlay(&mut canvas, &rgba, x as i64, 0);
        x += rgba.width();
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    canvas.save(output).map_err(|e| e.to_string())?;
    Ok(())
}
