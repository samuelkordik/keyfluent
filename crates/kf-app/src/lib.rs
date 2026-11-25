//! Keyfluent egui application.

// Linked now so the dependency graph matches the plan; used from P1 onward.
use kf_tmux as _;

/// Window title.
pub const APP_NAME: &str = "Keyfluent";

/// Minimum window size in points (spec 6.6).
pub const MIN_WINDOW_SIZE: [f32; 2] = [1000.0, 700.0];

/// Root application state.
#[derive(Default)]
pub struct KeyfluentApp;

impl eframe::App for KeyfluentApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading(APP_NAME);
        ui.label(format!("Version {}", kf_core::VERSION));
    }
}
