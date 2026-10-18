//! Node Empire — windowed game entry point.

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("Node Empire"),
        ..Default::default()
    };
    eframe::run_native(
        "Node Empire",
        options,
        Box::new(|cc| {
            // Dark RTS-flavoured theme.
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(22, 24, 30);
            visuals.window_fill = egui::Color32::from_rgb(28, 30, 38);
            visuals.extreme_bg_color = egui::Color32::from_rgb(16, 17, 22);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(node_empire::engine::App::new()))
        }),
    )
}
