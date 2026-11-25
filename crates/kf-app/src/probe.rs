//! Key probe: lists every raw key and text event egui receives, unfiltered.

use eframe::egui;

const MAX_LINES: usize = 40;

/// Describes a Key, Text, Copy, Cut, Paste or Ime event; `None` for every other event.
pub fn describe(event: &egui::Event) -> Option<String> {
    match event {
        egui::Event::Key {
            key,
            physical_key,
            pressed,
            repeat,
            modifiers,
        } => Some(format!(
            "Key {key:?} pressed={pressed} repeat={repeat} physical={physical_key:?} mods={}",
            describe_mods(*modifiers)
        )),
        egui::Event::Text(text) => Some(format!("Text {text:?}")),
        egui::Event::Copy => Some("Copy".to_owned()),
        egui::Event::Cut => Some("Cut".to_owned()),
        egui::Event::Paste(text) => Some(format!("Paste {text:?}")),
        egui::Event::Ime(ime) => Some(format!("Ime {ime:?}")),
        _ => None,
    }
}

/// Raw modifier flags. `command` is skipped: on Linux/Windows it mirrors `ctrl`.
fn describe_mods(m: egui::Modifiers) -> String {
    let flags = [
        (m.ctrl, "C"),
        (m.alt, "M"),
        (m.shift, "S"),
        (m.mac_cmd, "Cmd"),
    ];
    let names: Vec<&str> = flags
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, name)| *name)
        .collect();
    if names.is_empty() {
        "-".to_owned()
    } else {
        names.join("+")
    }
}

/// App that shows the last events it received.
#[derive(Default)]
pub struct ProbeApp {
    log: Vec<String>,
}

impl eframe::App for ProbeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Stop egui from consuming keys for its own shortcuts (Cmd/Ctrl +/- zoom).
        // No widget here is focusable, so Tab has no focus to move.
        ui.ctx().options_mut(|o| o.zoom_with_keyboard = false);

        let events = ui.ctx().input(|i| i.raw.events.clone());
        self.log.extend(events.iter().filter_map(describe));
        if self.log.len() > MAX_LINES {
            let excess = self.log.len() - MAX_LINES;
            self.log.drain(..excess);
        }

        ui.label("Press keys. Every event egui receives is listed below.");
        ui.separator();
        for line in self.log.iter().rev() {
            ui.label(line);
        }
    }
}
