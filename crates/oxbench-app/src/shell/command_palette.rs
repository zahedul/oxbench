use eframe::egui::{self, Key, Modifiers};

use super::ShellAction;
use crate::registry::{ToolId, search_tools};

const PALETTE_WIDTH: f32 = 480.0;
const RESULTS_HEIGHT: f32 = 320.0;

#[derive(Default)]
pub struct CommandPalette {
    is_open: bool,
    query: String,
    selected_index: usize,
    should_focus_input: bool,
}

struct PaletteKeys {
    is_down: bool,
    is_up: bool,
    is_enter: bool,
}

impl CommandPalette {
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
        self.should_focus_input = true;
    }

    fn close(&mut self) {
        self.is_open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context, actions: &mut Vec<ShellAction>) {
        if !self.is_open {
            return;
        }
        // Consumed before the text field runs, so arrows and Enter drive the list, not the cursor.
        let keys = ctx.input_mut(|input| PaletteKeys {
            is_down: input.consume_key(Modifiers::NONE, Key::ArrowDown),
            is_up: input.consume_key(Modifiers::NONE, Key::ArrowUp),
            is_enter: input.consume_key(Modifiers::NONE, Key::Enter),
        });
        let results = search_tools(&self.query);
        self.move_selection(&keys, results.len());

        let mut chosen: Option<ToolId> = None;
        let modal = egui::Modal::new(egui::Id::unique("command_palette")).show(ctx, |ui| {
            ui.set_width(PALETTE_WIDTH);
            let input = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Search tools…")
                    .desired_width(f32::INFINITY),
            );
            if self.should_focus_input {
                input.request_focus();
                self.should_focus_input = false;
            }
            if input.changed() {
                self.selected_index = 0;
            }
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(RESULTS_HEIGHT)
                .show(ui, |ui| {
                    if results.is_empty() {
                        ui.weak("No tools match.");
                    }
                    for (index, tool) in results.iter().enumerate() {
                        let is_selected = index == self.selected_index;
                        let label = format!("{}    {}", tool.name, tool.category.label());
                        let row = ui.add_sized(
                            [ui.available_width(), 24.0],
                            egui::Button::selectable(is_selected, label),
                        );
                        if is_selected && (keys.is_down || keys.is_up) {
                            row.scroll_to_me(None);
                        }
                        if row.clicked() {
                            chosen = Some(tool.id);
                        }
                    }
                });
        });

        if keys.is_enter {
            chosen = chosen.or_else(|| results.get(self.selected_index).map(|tool| tool.id));
        }
        if let Some(id) = chosen {
            actions.push(ShellAction::OpenTool(id));
            self.close();
            return;
        }
        if modal.should_close() {
            self.close();
        }
    }

    fn move_selection(&mut self, keys: &PaletteKeys, result_count: usize) {
        if result_count == 0 {
            self.selected_index = 0;
            return;
        }
        let last = result_count - 1;
        if keys.is_down {
            self.selected_index = (self.selected_index + 1).min(last);
        }
        if keys.is_up {
            self.selected_index = self.selected_index.saturating_sub(1);
        }
        self.selected_index = self.selected_index.min(last);
    }
}
