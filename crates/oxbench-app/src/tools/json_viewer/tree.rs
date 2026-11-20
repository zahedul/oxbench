use eframe::egui;
use oxbench_core::json::{
    PathSegment, SearchResult, Value, format_json_path, preview_value, search_json,
};

use crate::style::palette::Palette;
use crate::widgets::search_field::show_search_field;

const AUTO_OPEN_DEPTH: usize = 2;
/// Very large arrays and objects show their first children only; Query reaches the rest.
const MAX_CHILDREN_SHOWN: usize = 500;
const SEARCH_LIMIT: usize = 500;

#[derive(Default)]
pub struct TreeView {
    search: String,
    cached_search: Option<CachedSearch>,
}

struct CachedSearch {
    revision: u64,
    query: String,
    result: SearchResult,
}

impl TreeView {
    pub fn show(&mut self, ui: &mut egui::Ui, document: &Value, revision: u64) {
        show_search_field(ui, &mut self.search, "Search keys and values", 14.0);
        ui.add_space(4.0);
        if self.search.trim().is_empty() {
            show_tree(ui, document);
            return;
        }
        let result = self.search_result(document, revision);
        show_search_hits(ui, result);
    }

    fn search_result(&mut self, document: &Value, revision: u64) -> &SearchResult {
        let cached = match self.cached_search.take() {
            Some(cached) if cached.revision == revision && cached.query == self.search => cached,
            _ => CachedSearch {
                revision,
                query: self.search.clone(),
                result: search_json(document, &self.search, SEARCH_LIMIT),
            },
        };
        &self.cached_search.insert(cached).result
    }
}

fn show_tree(ui: &mut egui::Ui, document: &Value) {
    egui::ScrollArea::both()
        .id_salt("json_tree")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().interact_size.y = 22.0;
            ui.spacing_mut().item_spacing.y = 2.0;
            let mut path = Vec::new();
            show_node(ui, "$", document, &mut path);
        });
}

fn show_node(ui: &mut egui::Ui, label: &str, value: &Value, path: &mut Vec<PathSegment>) {
    match value {
        Value::Object(_) | Value::Array(_) => {
            let header = format!("{label}  {}", preview_value(value));
            let response = egui::CollapsingHeader::new(header)
                .id_salt(format_json_path(path))
                .default_open(path.len() < AUTO_OPEN_DEPTH)
                .show(ui, |ui| show_children(ui, value, path));
            add_node_menu(&response.header_response, path, value);
        }
        scalar => {
            let row = ui
                .horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{label}:"))
                            .monospace()
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(scalar.to_string())
                            .monospace()
                            .color(scalar_color(ui, scalar)),
                    );
                })
                .response
                .interact(egui::Sense::click());
            add_node_menu(&row, path, value);
        }
    }
}

fn show_children(ui: &mut egui::Ui, value: &Value, path: &mut Vec<PathSegment>) {
    let total = match value {
        Value::Object(map) => {
            for (key, child) in map.iter().take(MAX_CHILDREN_SHOWN) {
                path.push(PathSegment::Key(key.clone()));
                show_node(ui, key, child, path);
                path.pop();
            }
            map.len()
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate().take(MAX_CHILDREN_SHOWN) {
                path.push(PathSegment::Index(index));
                show_node(ui, &format!("[{index}]"), child, path);
                path.pop();
            }
            items.len()
        }
        _ => 0,
    };
    if total > MAX_CHILDREN_SHOWN {
        ui.weak(format!(
            "{} more not shown. Use the Query tab to reach them.",
            total - MAX_CHILDREN_SHOWN
        ));
    }
}

fn add_node_menu(response: &egui::Response, path: &[PathSegment], value: &Value) {
    response.context_menu(|ui| {
        if ui.button("Copy path").clicked() {
            ui.ctx().copy_text(format_json_path(path));
        }
        if ui.button("Copy value").clicked() {
            ui.ctx().copy_text(value.to_string());
        }
    });
}

fn show_search_hits(ui: &mut egui::Ui, result: &SearchResult) {
    let summary = match (result.hits.len(), result.is_truncated) {
        (0, _) => "No matches.".to_owned(),
        (count, true) => format!("First {count} matches. Click a path to copy it."),
        (1, false) => "1 match. Click the path to copy it.".to_owned(),
        (count, false) => format!("{count} matches. Click a path to copy it."),
    };
    ui.weak(summary);
    egui::ScrollArea::both()
        .id_salt("json_tree_search")
        .auto_shrink(false)
        .show(ui, |ui| {
            for hit in &result.hits {
                ui.horizontal(|ui| {
                    let path_link = ui.link(egui::RichText::new(&hit.path).monospace());
                    if path_link.on_hover_text("Copy path").clicked() {
                        ui.ctx().copy_text(hit.path.clone());
                    }
                    ui.weak(&hit.preview);
                });
            }
        });
}

fn scalar_color(ui: &egui::Ui, value: &Value) -> egui::Color32 {
    let palette = Palette::of(ui);
    match value {
        Value::String(_) => palette.syntax_string,
        Value::Number(_) => palette.syntax_number,
        Value::Bool(_) => palette.syntax_bool,
        _ => palette.syntax_null,
    }
}
