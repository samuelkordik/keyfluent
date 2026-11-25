use kf_app::probe::ProbeApp;
use kf_app::{KeyfluentApp, APP_NAME, MIN_WINDOW_SIZE};

fn main() -> eframe::Result {
    let key_probe = std::env::args().any(|a| a == "--key-probe");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_inner_size(MIN_WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| {
            if key_probe {
                Ok(Box::new(ProbeApp::new(&cc.egui_ctx)))
            } else {
                Ok(Box::<KeyfluentApp>::default())
            }
        }),
    )
}
