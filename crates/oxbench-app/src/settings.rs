//! Preferences that change how Oxbench fits into the desktop, saved between launches.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub should_check_for_updates: bool,
    pub is_global_shortcut_enabled: bool,
    pub should_show_menu_bar_icon: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            should_check_for_updates: true,
            is_global_shortcut_enabled: true,
            should_show_menu_bar_icon: true,
        }
    }
}
