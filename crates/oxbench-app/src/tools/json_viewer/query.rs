use eframe::egui;
use oxbench_core::CoreResult;
use oxbench_core::json::{FormatOptions, QueryMatch, Value, format_pretty, query_json_path};

use crate::widgets::code_editor::show_code_view;
use crate::widgets::copy_button::show_copy_button;
use crate::widgets::status::{StatusTone, show_status};

#[derive(Default)]
pub struct QueryPanel {
    expression: String,
    should_show_paths: bool,
    cached: Option<CachedQuery>,
}

struct CachedQuery {
    revision: u64,
    expression: String,
    should_show_paths: bool,
    format_options: FormatOptions,
    outcome: CoreResult<QueryOutput>,
}

struct QueryOutput {
    match_count: usize,
    text: String,
}

impl QueryPanel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        document: &Value,
        revision: u64,
        format_options: FormatOptions,
    ) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.expression)
                    .hint_text("$.orders[?@.total > 50].id")
                    .code_editor()
                    .desired_width(ui.available_width() - 110.0),
            );
            ui.checkbox(&mut self.should_show_paths, "Show paths");
        });
        if self.expression.trim().is_empty() {
            ui.weak("Type a JSONPath expression (RFC 9535). Tip: right-click a node in Tree to copy its path.");
            return;
        }
        self.refresh(document, revision, format_options);
        let Some(cached) = &self.cached else {
            return;
        };
        match &cached.outcome {
            Ok(output) => {
                ui.horizontal(|ui| {
                    show_copy_button(ui, "Copy result", &output.text);
                    ui.weak(match output.match_count {
                        1 => "1 match".to_owned(),
                        count => format!("{count} matches"),
                    });
                });
                show_code_view(ui, "json_query_output", &output.text);
            }
            Err(error) => show_status(ui, StatusTone::Error, &error.to_string()),
        }
    }

    fn refresh(&mut self, document: &Value, revision: u64, format_options: FormatOptions) {
        let is_fresh = self.cached.as_ref().is_some_and(|cached| {
            cached.revision == revision
                && cached.expression == self.expression
                && cached.should_show_paths == self.should_show_paths
                && cached.format_options == format_options
        });
        if is_fresh {
            return;
        }
        let outcome = query_json_path(document, &self.expression)
            .and_then(|matches| render_matches(&matches, self.should_show_paths, format_options));
        self.cached = Some(CachedQuery {
            revision,
            expression: self.expression.clone(),
            should_show_paths: self.should_show_paths,
            format_options,
            outcome,
        });
    }
}

fn render_matches(
    matches: &[QueryMatch],
    should_show_paths: bool,
    format_options: FormatOptions,
) -> CoreResult<QueryOutput> {
    let text = if should_show_paths {
        matches
            .iter()
            .map(|found| format!("{} = {}", found.path, found.value))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let values = Value::Array(matches.iter().map(|found| found.value.clone()).collect());
        format_pretty(&values, format_options)?
    };
    Ok(QueryOutput {
        match_count: matches.len(),
        text,
    })
}
