#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([760.0, 650.0])
            .with_min_inner_size([620.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "ani-cli-rs",
        options,
        Box::new(|cc| Ok(Box::new(ani_lib::gui::AniGuiApp::new(cc)))),
    )
}
