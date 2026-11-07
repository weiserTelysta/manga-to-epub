#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod ui;

fn main() -> eframe::Result {
    let icon = image::load_from_memory(include_bytes!("../assets/telysta.png"))
        .expect("embedded icon")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    let window_size = std::env::var("TELYSTA_WINDOW_SIZE")
        .ok()
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some([w.parse::<f32>().ok()?, h.parse::<f32>().ok()?])
        })
        .unwrap_or([1280.0, 800.0]);
    let options = eframe::NativeOptions {
        persist_window: std::env::var_os("TELYSTA_SCREENSHOT").is_none(),
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Telysta · 图片成书")
            .with_inner_size(window_size)
            .with_min_inner_size([800.0, 600.0])
            .with_icon(eframe::egui::IconData {
                rgba: icon.into_raw(),
                width,
                height,
            }),
        ..Default::default()
    };
    eframe::run_native(
        "Telysta",
        options,
        Box::new(|cc| Ok(Box::new(ui::TelystaApp::new(cc)))),
    )
}
