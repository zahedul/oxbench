use eframe::egui::{self, Key, Modifiers};

use super::ShellAction;
use crate::registry::{ToolId, ToolMeta, search_tools};
use crate::style::fonts::icon_font;
use crate::style::palette::Palette;
use crate::widgets::search_field::show_search_field;

const PALETTE_WIDTH: f32 = 560.0;
const RESULTS_HEIGHT: f32 = 360.0;
const ROW_HEIGHT: f32 = 36.0;

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
        let palette = Palette::of_context(ctx);
        let frame = egui::Frame::default()
            .fill(palette.panel)
            .stroke(egui::Stroke::new(1.0, palette.border_strong))
            .corner_radius(12)
            .inner_margin(10)
            .shadow(ctx.global_style().visuals.window_shadow);
        let modal = egui::Modal::new(egui::Id::unique("command_palette"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(PALETTE_WIDTH);
                let input = show_search_field(ui, &mut self.query, "Search tools…", 16.0);
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
                            let row = show_result_row(ui, *tool, is_selected);
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

fn show_result_row(ui: &mut egui::Ui, tool: ToolMeta, is_selected: bool) -> egui::Response {
    let palette = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, true, is_selected, tool.name)
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    let fill = if is_selected {
        palette.accent_soft
    } else if response.hovered() {
        palette.surface_hover
    } else {
        egui::Color32::TRANSPARENT
    };
    painter.rect_filled(rect, 6, fill);
    let icon_color = if is_selected {
        palette.accent
    } else {
        palette.text_weak
    };
    painter.text(
        rect.left_center() + egui::vec2(12.0, 0.0),
        egui::Align2::LEFT_CENTER,
        tool.icon,
        icon_font(17.0),
        icon_color,
    );
    painter.text(
        rect.left_center() + egui::vec2(40.0, 0.0),
        egui::Align2::LEFT_CENTER,
        tool.name,
        egui::FontId::proportional(14.5),
        palette.text_strong,
    );
    painter.text(
        rect.right_center() - egui::vec2(12.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        tool.category.label(),
        egui::FontId::proportional(12.5),
        palette.text_weak,
    );
    response
}
