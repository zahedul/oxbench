use eframe::egui;

/// One tool's UI. Each open tab owns its tool, so inputs survive tab switches.
pub trait Tool {
    fn ui(&mut self, ui: &mut egui::Ui);
}
