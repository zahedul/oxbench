use eframe::egui::ThemePreference;

pub const fn next_theme(current: ThemePreference) -> ThemePreference {
    match current {
        ThemePreference::System => ThemePreference::Light,
        ThemePreference::Light => ThemePreference::Dark,
        ThemePreference::Dark => ThemePreference::System,
    }
}

pub const fn theme_label(theme: ThemePreference) -> &'static str {
    match theme {
        ThemePreference::System => "Theme: System",
        ThemePreference::Light => "Theme: Light",
        ThemePreference::Dark => "Theme: Dark",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_through_all_three_themes() {
        let mut theme = ThemePreference::System;
        for _ in 0..3 {
            theme = next_theme(theme);
        }
        assert_eq!(theme, ThemePreference::System);
    }
}
