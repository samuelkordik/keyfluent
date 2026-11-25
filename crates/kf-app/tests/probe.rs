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
        Some("Key B pressed=true repeat=false physical=None mods=C")
    );
    assert_eq!(
        describe(&egui::Event::Text("%".into())).as_deref(),
        Some("Text \"%\"")
    );
    let plain = egui::Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: false,
        repeat: true,
        modifiers: egui::Modifiers::NONE,
    };
    assert_eq!(
        describe(&plain).as_deref(),
        Some("Key A pressed=false repeat=true physical=None mods=-")
    );
    assert_eq!(
        describe(&egui::Event::Paste("x".into())).as_deref(),
        Some("Paste \"x\"")
    );
    assert_eq!(describe(&egui::Event::Copy).as_deref(), Some("Copy"));
    assert_eq!(describe(&egui::Event::Cut).as_deref(), Some("Cut"));
    assert_eq!(
        describe(&egui::Event::Ime(egui::ImeEvent::Commit("e".into()))).as_deref(),
        Some("Ime Commit(\"e\")")
    );
    assert_eq!(describe(&egui::Event::PointerGone), None);
}

#[test]
fn probe_logs_typed_text() {
    let mut h = Harness::builder().build_eframe(|_| ProbeApp::default());
    h.key_press(egui::Key::A);
    h.event(egui::Event::Text("%".into()));
    h.run_steps(2);
    h.get_by_label_contains("Key A pressed=true");
    h.get_by_label_contains("Text \"%\"");
}
