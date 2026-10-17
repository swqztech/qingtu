use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    KeepOriginal,
    Jpeg,
    Png,
    WebP,
}

#[derive(Clone, Debug)]
pub struct JobOptions {
    pub quality: u8,
    /// 0 = 不缩放
    pub max_side: u32,
    pub format: OutputFormat,
    pub output_dir: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct JobResult {
    pub input: PathBuf,
    pub output: PathBuf,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub message: String,
}

fn detect_format(path: &Path) -> Option<ImageFormat> {
    let data = fs::read(path).unwrap_or_default();
    image::guess_format(&data).ok()
}

fn target_format(path: &Path, choice: OutputFormat) -> ImageFormat {
    match choice {
        OutputFormat::KeepOriginal => detect_format(path).unwrap_or(ImageFormat::Png),
        OutputFormat::Jpeg => ImageFormat::Jpeg,
        OutputFormat::Png => ImageFormat::Png,
        OutputFormat::WebP => ImageFormat::WebP,
    }
}

fn ext_for(fmt: ImageFormat) -> &'static str {
    match fmt {
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Png => "png",
        ImageFormat::WebP => "webp",
        ImageFormat::Gif => "gif",
        ImageFormat::Bmp => "bmp",
        _ => "img",
    }
}

fn resize_if_needed(img: DynamicImage, max_side: u32) -> DynamicImage {
    if max_side == 0 {
        return img;
    }
    let (w, h) = (img.width(), img.height());
    let longest = w.max(h);
    if longest <= max_side {
        return img;
    }
    let scale = max_side as f32 / longest as f32;
    let nw = (w as f32 * scale).round().max(1.0) as u32;
    let nh = (h as f32 * scale).round().max(1.0) as u32;
    img.resize(nw, nh, FilterType::Lanczos3)
}

fn output_path(input: &Path, fmt: ImageFormat, output_dir: &Path) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    output_dir.join(format!("{}.{}", stem, ext_for(fmt)))
}

pub fn default_output_dir(input: &Path) -> PathBuf {
    input
        .parent()
        .map(|p| p.join("compressed"))
        .unwrap_or_else(|| PathBuf::from("compressed"))
}

pub fn compress_one(input: &Path, opts: &JobOptions) -> Result<JobResult, String> {
    if !input.is_file() {
        return Err("不是有效文件".into());
    }
    let bytes_before = fs::metadata(input).map(|m| m.len()).unwrap_or(0);
    let fmt = target_format(input, opts.format);
    let out_dir = opts
        .output_dir
        .clone()
        .or_else(|| input.parent().map(|p| p.join("compressed")))
        .unwrap_or_else(|| PathBuf::from("compressed"));
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let img = image::open(input).map_err(|e| e.to_string())?;
    let img = resize_if_needed(img, opts.max_side);
    let out_path = output_path(input, fmt, &out_dir);
    let quality = opts.quality.clamp(1, 100);

    match fmt {
        ImageFormat::Jpeg => {
            let rgb = img.to_rgb8();
            let mut buf = Vec::new();
            let enc = JpegEncoder::new_with_quality(&mut buf, quality);
            enc.write_image(rgb.as_raw(), rgb.width(), rgb.height(), ExtendedColorType::Rgb8)
                .map_err(|e| e.to_string())?;
            fs::write(&out_path, &buf).map_err(|e| e.to_string())?;
        }
        ImageFormat::Png => {
            let rgba = img.to_rgba8();
            let mut buf = Vec::new();
            let (comp, filt) = png_level(quality);
            let enc = PngEncoder::new_with_quality(&mut buf, comp, filt);
            enc.write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                ExtendedColorType::Rgba8,
            )
            .map_err(|e| e.to_string())?;
            fs::write(&out_path, &buf).map_err(|e| e.to_string())?;
        }
        ImageFormat::WebP => {
            let rgba = img.to_rgba8();
            let enc = webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height());
            let mem = enc.encode(quality as f32);
            fs::write(&out_path, mem.as_ref()).map_err(|e| e.to_string())?;
        }
        _ => {
            img.save_with_format(&out_path, fmt).map_err(|e| e.to_string())?;
        }
    }

    let bytes_after = fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    Ok(JobResult {
        input: input.to_path_buf(),
        output: out_path,
        bytes_before,
        bytes_after,
        message: "完成".into(),
    })
}

fn png_level(q: u8) -> (CompressionType, PngFilter) {
    if q >= 85 {
        (CompressionType::Best, PngFilter::Paeth)
    } else if q >= 50 {
        (CompressionType::Default, PngFilter::Sub)
    } else {
        (CompressionType::Fast, PngFilter::NoFilter)
    }
}

pub fn run_jobs(paths: &[PathBuf], opts: &JobOptions) -> Vec<JobResult> {
    paths.iter().map(|p| match compress_one(p, opts) {
        Ok(r) => r,
        Err(e) => JobResult {
            input: p.clone(),
            output: PathBuf::new(),
            bytes_before: 0,
            bytes_after: 0,
            message: e,
        },
    }).collect()
}
