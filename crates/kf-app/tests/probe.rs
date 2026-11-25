//! Key-probe tests: event descriptions and the headless window.

use eframe::egui;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use kf_app::probe::{describe, ProbeApp};

#[test]
fn describes_ctrl_b_and_percent() {
    let ctrl_b = egui::Event::Key {
        key: egui::Key::B,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::CTRL,
    };
    assert_eq!(
        describe(&ctrl_b).as_deref(),
        Some("Key B pressed=true mods=C")
    );
    assert_eq!(
        describe(&egui::Event::Text("%".into())).as_deref(),
        Some("Text \"%\"")
    );
}

#[test]
fn probe_logs_typed_text() {
    let mut h = Harness::builder().build_eframe(|_| ProbeApp::default());
    h.key_press(egui::Key::A);
    h.event(egui::Event::Text("%".into()));
    h.run_steps(2);
    h.get_by_label_contains("Text \"%\"");
}
