#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod ui;

fn main() -> eframe::Result {
    let icon = image::load_from_memory(include_bytes!("../assets/telysta.png"))
        .expect("embedded icon")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Telysta · 图片成书")
            .with_inner_size([1260.0, 900.0])
            .with_min_inner_size([1060.0, 780.0])
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
