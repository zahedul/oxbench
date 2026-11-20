use eframe::egui;

use crate::registry::ToolId;
use crate::tool::Tool;

/// Stands in for a tool until its real UI is built.
pub struct ComingSoon {
    id: ToolId,
}

impl ComingSoon {
    pub fn new(id: ToolId) -> Self {
        Self { id }
    }
}

impl Tool for ComingSoon {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let meta = self.id.meta();
        ui.heading(meta.name);
        ui.label(meta.summary);
        ui.add_space(12.0);
        ui.weak("This tool is planned and not built yet.");
    }
}
