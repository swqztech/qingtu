#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod compress;
mod fonts;
mod stitch;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 640.0])
            .with_min_inner_size([480.0, 560.0])
            .with_title("轻图"),
        ..Default::default()
    };
    eframe::run_native(
        "轻图",
        options,
        Box::new(|cc| {
            fonts::setup_cjk_fonts(&cc.egui_ctx);
            Ok(Box::new(app::QingtuApp::new()))
        }),
    )
}
