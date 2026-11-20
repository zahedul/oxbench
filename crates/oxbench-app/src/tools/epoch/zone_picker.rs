use eframe::egui;
use oxbench_core::epoch::{DateTime, Utc, Zone, format_offset, list_named_zones};

const PICKER_WIDTH: f32 = 300.0;
const POPUP_HEIGHT: f32 = 360.0;

/// The zone's name for headings and rows: the IANA name for the system zone when it is known.
pub fn describe_zone(zone: Zone, local_zone_name: Option<&str>) -> String {
    match (zone, local_zone_name) {
        (Zone::Local, Some(name)) => format!("Local · {name}"),
        (Zone::Local, None) => "Local time".to_owned(),
        (zone, _) => zone.name().to_owned(),
    }
}

/// A dropdown of every IANA zone with a filter box, plus Local and UTC at the top.
pub fn show_zone_picker(
    ui: &mut egui::Ui,
    zone: &mut Zone,
    filter: &mut String,
    local_zone_name: Option<&str>,
    now: DateTime<Utc>,
) {
    let label = ui.label("Time zone");
    let combo = egui::ComboBox::from_id_salt("epoch_zone")
        .selected_text(describe_zone(*zone, local_zone_name))
        .width(PICKER_WIDTH)
        .height(POPUP_HEIGHT)
        // Clicking the filter box must not close the list; a pick closes it explicitly below.
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(filter)
                    .hint_text("Filter zones")
                    .desired_width(f32::INFINITY),
            );
            let mut is_picked = false;
            let needle = filter.trim().to_lowercase();
            let fixed_choices = [
                (Zone::Local, describe_zone(Zone::Local, local_zone_name)),
                (Zone::Utc, "UTC".to_owned()),
            ];
            for (choice, name) in fixed_choices {
                if name.to_lowercase().contains(&needle) {
                    is_picked |= show_zone_option(ui, zone, choice, &name, now);
                }
            }
            ui.separator();
            let named_zones = list_named_zones()
                .iter()
                .filter(|tz| tz.name().to_lowercase().contains(&needle));
            for tz in named_zones {
                is_picked |= show_zone_option(ui, zone, Zone::Named(*tz), tz.name(), now);
            }
            if is_picked {
                filter.clear();
                ui.close();
            }
        });
    // Names the dropdown for screen readers, since its visible label sits beside it.
    let _ = combo.response.labelled_by(label.id);
}

fn show_zone_option(
    ui: &mut egui::Ui,
    zone: &mut Zone,
    choice: Zone,
    name: &str,
    now: DateTime<Utc>,
) -> bool {
    let offset = format_offset(choice.to_zoned(now).offset().local_minus_utc());
    let text = if offset == name {
        name.to_owned()
    } else {
        format!("{name}  ({offset})")
    };
    ui.selectable_value(zone, choice, text).clicked()
}
