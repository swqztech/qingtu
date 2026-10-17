use crate::compress::{JobOptions, JobResult, OutputFormat, run_jobs};
use crate::stitch;
use eframe::egui;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Compress,
    Stitch,
}

pub struct QingtuApp {
    tab: Tab,
    files: Vec<PathBuf>,
    quality: u32,
    max_side: u32,
    format: OutputFormat,
    output_dir: String,
    use_custom_out: bool,
    log: String,
    busy: bool,
}

impl QingtuApp {
    pub fn new() -> Self {
        Self {
            tab: Tab::Compress,
            files: Vec::new(),
            quality: 80,
            max_side: 0,
            format: OutputFormat::KeepOriginal,
            output_dir: String::new(),
            use_custom_out: false,
            log: String::new(),
            busy: false,
        }
    }

    fn add_paths(&mut self, paths: Vec<PathBuf>) {
        for p in paths {
            if p.is_file() && is_image(&p) && !self.files.contains(&p) {
                self.files.push(p);
            }
        }
    }

    fn job_options(&self) -> JobOptions {
        JobOptions {
            quality: self.quality as u8,
            max_side: self.max_side,
            format: self.format,
            output_dir: if self.use_custom_out && !self.output_dir.is_empty() {
                Some(PathBuf::from(&self.output_dir))
            } else {
                None
            },
        }
    }

    fn start_compress(&mut self) {
        if self.files.is_empty() {
            self.log = "请先添加图片".into();
            return;
        }
        self.busy = true;
        let opts = self.job_options();
        let results = run_jobs(&self.files, &opts);
        self.busy = false;
        self.log = format_results(&results);
    }

    fn start_stitch(&mut self) {
        if self.files.len() < 2 {
            self.log = "拼接需要至少 2 张图".into();
            return;
        }
        let out = if self.use_custom_out && !self.output_dir.is_empty() {
            PathBuf::from(&self.output_dir).join("stitched.png")
        } else if let Some(first) = self.files.first() {
            crate::compress::default_output_dir(first).join("stitched.png")
        } else {
            PathBuf::from("stitched.png")
        };
        match stitch::stitch_horizontal(&self.files, &out) {
            Ok(()) => self.log = format!("拼接完成 → {}", out.display()),
            Err(e) => self.log = e,
        }
    }
}

impl eframe::App for QingtuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply(ctx);

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths = i
                    .raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect();
                self.add_paths(paths);
            }
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG).inner_margin(20.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("轻图").size(34.0).color(theme::TEAL));
                    ui.label(
                        egui::RichText::new("把体积压下去，细节留住")
                            .size(13.0)
                            .color(theme::TEXT),
                    );
                    ui.label(
                        egui::RichText::new("本地处理 · 不上传 · PNG / JPG / WebP")
                            .size(11.0)
                            .color(theme::MUTED),
                    );
                });
                ui.add_space(12.0);
                tab_row(ui, &mut self.tab);
                ui.add_space(14.0);

                drop_zone(ui, ctx, self);
                ui.add_space(12.0);

                if self.tab == Tab::Compress {
                    compress_opts(ui, self);
                } else {
                    ui.label(
                        egui::RichText::new("按顺序横向拼接，高度对齐最短边")
                            .size(12.0)
                            .color(theme::MUTED),
                    );
                }

                ui.add_space(10.0);
                out_dir_row(ui, self);
                ui.add_space(14.0);

                let n = self.files.len();
                let label = if self.tab == Tab::Compress {
                    format!("开始压缩 ({})", n)
                } else {
                    format!("开始拼接 ({})", n)
                };
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE))
                                .fill(theme::ORANGE)
                                .min_size(egui::vec2(140.0, 36.0)),
                        )
                        .clicked()
                        && !self.busy
                    {
                        if self.tab == Tab::Compress {
                            self.start_compress();
                        } else {
                            self.start_stitch();
                        }
                    }
                    if ui.button("清空列表").clicked() {
                        self.files.clear();
                        self.log.clear();
                    }
                });

                if !self.log.is_empty() {
                    ui.add_space(10.0);
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(&self.log)
                                    .monospace()
                                    .size(11.0)
                                    .color(theme::TEXT),
                            );
                        });
                }
            });
    }
}

fn is_image(p: &PathBuf) -> bool {
    matches!(
        p.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()),
        Some(ext) if ["png", "jpg", "jpeg", "webp", "gif", "bmp"].contains(&ext.as_str())
    )
}

fn format_results(results: &[JobResult]) -> String {
    results
        .iter()
        .map(|r| {
            if r.output.as_os_str().is_empty() {
                format!(
                    "{} → {}",
                    r.input.file_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
                    r.message
                )
            } else {
                format!(
                    "{} → {} ({} → {})",
                    r.input.file_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
                    r.output.file_name().map(|s| s.to_string_lossy()).unwrap_or_default(),
                    human_bytes(r.bytes_before),
                    human_bytes(r.bytes_after),
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn human_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{} B", n)
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.1} MB", n as f64 / 1024.0 / 1024.0)
    }
}

fn tab_row(ui: &mut egui::Ui, tab: &mut Tab) {
    ui.horizontal(|ui| {
        if tab_btn(ui, "压缩", *tab == Tab::Compress) {
            *tab = Tab::Compress;
        }
        if tab_btn(ui, "拼接", *tab == Tab::Stitch) {
            *tab = Tab::Stitch;
        }
    });
}

fn tab_btn(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    let stroke = if on {
        egui::Stroke::new(1.5, theme::TEAL)
    } else {
        egui::Stroke::new(1.0, theme::BORDER)
    };
    ui.add(
        egui::Button::new(
            egui::RichText::new(label).color(if on { theme::TEAL } else { theme::MUTED }),
        )
        .fill(if on { theme::PANEL } else { theme::BG2 })
        .stroke(stroke)
        .min_size(egui::vec2(72.0, 30.0)),
    )
    .clicked()
}

fn drop_zone(ui: &mut egui::Ui, _ctx: &egui::Context, app: &mut QingtuApp) {
    egui::Frame::none()
        .stroke(egui::Stroke::new(1.5, theme::TEAL_DIM))
        .rounding(8.0)
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("拖拽图片到这里").color(theme::MUTED));
                ui.add_space(8.0);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("选择图片").color(egui::Color32::WHITE),
                        )
                        .fill(theme::ORANGE),
                    )
                    .clicked()
                {
                    if let Some(files) = rfd::FileDialog::new()
                        .add_filter("图片", &["png", "jpg", "jpeg", "webp", "gif", "bmp"])
                        .pick_files()
                    {
                        app.add_paths(files);
                    }
                }
                if !app.files.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("已选 {} 个文件", app.files.len()))
                            .size(11.0)
                            .color(theme::TEAL),
                    );
                }
            });
        });
}

fn compress_opts(ui: &mut egui::Ui, app: &mut QingtuApp) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("质量").color(theme::MUTED));
        ui.add(egui::Slider::new(&mut app.quality, 1..=100));
        ui.label(format!("{}", app.quality));
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("最长边").color(theme::MUTED));
        ui.add(egui::Slider::new(&mut app.max_side, 0..=8192).logarithmic(true));
        let t = if app.max_side == 0 {
            "不限制".to_string()
        } else {
            format!("{} px", app.max_side)
        };
        ui.label(t);
    });
    ui.label(
        egui::RichText::new("(0 = 不缩放)")
            .size(11.0)
            .color(theme::MUTED),
    );
    ui.add_space(6.0);
    ui.label(egui::RichText::new("输出格式").color(theme::MUTED));
    ui.horizontal_wrapped(|ui| {
        fmt_pill(ui, "保持原格式", app.format == OutputFormat::KeepOriginal, || {
            app.format = OutputFormat::KeepOriginal;
        });
        fmt_pill(ui, "JPEG", app.format == OutputFormat::Jpeg, || {
            app.format = OutputFormat::Jpeg;
        });
        fmt_pill(ui, "PNG", app.format == OutputFormat::Png, || {
            app.format = OutputFormat::Png;
        });
        fmt_pill(ui, "WebP", app.format == OutputFormat::WebP, || {
            app.format = OutputFormat::WebP;
        });
    });
}

fn fmt_pill(ui: &mut egui::Ui, label: &str, on: bool, mut set: impl FnMut()) {
    if ui
        .add(
            egui::Button::new(egui::RichText::new(label).size(12.0).color(if on {
                theme::TEAL
            } else {
                theme::MUTED
            }))
            .fill(if on { theme::PANEL } else { theme::BG2 })
            .stroke(egui::Stroke::new(
                1.0,
                if on { theme::TEAL_DIM } else { theme::BORDER },
            )),
        )
        .clicked()
    {
        set();
    }
}

fn out_dir_row(ui: &mut egui::Ui, app: &mut QingtuApp) {
    ui.checkbox(&mut app.use_custom_out, "自定义输出目录");
    let hint = if app.use_custom_out {
        "选择输出文件夹"
    } else {
        "默认：源文件旁 /compressed"
    };
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.output_dir)
                .desired_width(ui.available_width() - 90.0)
                .hint_text(hint),
        );
        if ui.button("输出目录").clicked() {
            if let Some(d) = rfd::FileDialog::new().pick_folder() {
                app.output_dir = d.display().to_string();
                app.use_custom_out = true;
            }
        }
    });
}

mod theme {
    use eframe::egui::{self, Color32};

    pub const BG: Color32 = Color32::from_rgb(26, 38, 41);
    pub const BG2: Color32 = Color32::from_rgb(18, 28, 32);
    pub const PANEL: Color32 = Color32::from_rgb(32, 48, 52);
    pub const TEAL: Color32 = Color32::from_rgb(72, 196, 182);
    pub const TEAL_DIM: Color32 = Color32::from_rgb(48, 120, 112);
    pub const ORANGE: Color32 = Color32::from_rgb(230, 140, 70);
    pub const TEXT: Color32 = Color32::from_rgb(210, 218, 222);
    pub const MUTED: Color32 = Color32::from_rgb(120, 135, 142);
    pub const BORDER: Color32 = Color32::from_rgb(55, 72, 78);

    pub fn apply(ctx: &egui::Context) {
        let mut v = ctx.style().visuals.clone();
        v.dark_mode = true;
        v.panel_fill = BG;
        v.window_fill = BG;
        ctx.set_visuals(v);
    }
}
