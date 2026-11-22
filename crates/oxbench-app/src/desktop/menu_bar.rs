//! The menu bar icon (the tray on Windows), with a menu that opens any tool.
//!
//! Linux is left out: its tray needs a GTK event loop beside winit's, and desktops differ in
//! whether they show tray icons at all.

#[cfg(not(target_os = "linux"))]
pub use supported::{IS_MENU_BAR_SUPPORTED, MenuBarIcon};
#[cfg(target_os = "linux")]
pub use unsupported::{IS_MENU_BAR_SUPPORTED, MenuBarIcon};

#[cfg(not(target_os = "linux"))]
mod supported {
    use std::sync::mpsc::Sender;

    use eframe::egui;
    use tray_icon::menu::{
        IsMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
    };
    use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

    use crate::APP_NAME;
    use crate::desktop::{DesktopEvent, deliver};
    use crate::registry::ToolId;

    pub const IS_MENU_BAR_SUPPORTED: bool = true;

    /// macOS draws a template icon in the menu bar's own colour; Windows shows the app icon.
    #[cfg(target_os = "macos")]
    const ICON_PNG: &[u8] = include_bytes!("../../../../assets/icons/tray-template.png");
    #[cfg(not(target_os = "macos"))]
    const ICON_PNG: &[u8] = include_bytes!("../../../../assets/icons/32x32.png");

    const SHOW_ID: &str = "show";
    const QUIT_ID: &str = "quit";
    const TOOL_ID_PREFIX: &str = "tool:";

    pub struct MenuBarIcon {
        icon: Option<TrayIcon>,
        problem: Option<String>,
    }

    impl MenuBarIcon {
        pub fn new(ctx: &egui::Context, sender: Sender<DesktopEvent>) -> Self {
            let ctx = ctx.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                if let Some(desktop_event) = find_menu_event(&event.id) {
                    deliver(&ctx, &sender, desktop_event);
                }
            }));
            Self {
                icon: None,
                problem: None,
            }
        }

        pub fn set_visible(&mut self, should_show: bool) {
            if should_show == self.icon.is_some() {
                return;
            }
            if !should_show {
                // Dropping the icon removes it from the menu bar.
                self.icon = None;
                self.problem = None;
                return;
            }
            match build_tray_icon() {
                Ok(icon) => {
                    self.icon = Some(icon);
                    self.problem = None;
                }
                Err(message) => self.problem = Some(message),
            }
        }

        pub fn problem(&self) -> Option<&str> {
            self.problem.as_deref()
        }
    }

    fn build_tray_icon() -> Result<TrayIcon, String> {
        let menu = build_menu().map_err(|error| format!("The menu couldn't be built: {error}"))?;
        let icon = load_icon()?;
        #[cfg(target_os = "macos")]
        let builder = TrayIconBuilder::new().with_icon_templated(icon);
        #[cfg(not(target_os = "macos"))]
        let builder = TrayIconBuilder::new().with_icon(icon);
        builder
            .with_tooltip(APP_NAME)
            .with_menu(Box::new(menu))
            .build()
            .map_err(|error| format!("The icon couldn't be added: {error}"))
    }

    fn load_icon() -> Result<Icon, String> {
        let image = eframe::icon_data::from_png_bytes(ICON_PNG)
            .map_err(|error| format!("The icon image is damaged: {error}"))?;
        Icon::from_rgba(image.rgba, image.width, image.height)
            .map_err(|error| format!("The icon image is damaged: {error}"))
    }

    fn build_menu() -> tray_icon::menu::Result<Menu> {
        let tool_items: Vec<MenuItem> = ToolId::ALL
            .iter()
            .enumerate()
            .map(|(index, tool)| {
                MenuItem::with_id(
                    format!("{TOOL_ID_PREFIX}{index}"),
                    tool.meta().name,
                    true,
                    None,
                )
            })
            .collect();
        let tool_refs: Vec<&dyn IsMenuItem> = tool_items
            .iter()
            .map(|item| item as &dyn IsMenuItem)
            .collect();
        let tools = Submenu::with_items("Open tool", true, &tool_refs)?;
        let show = MenuItem::with_id(SHOW_ID, format!("Show {APP_NAME}"), true, None);
        let quit = MenuItem::with_id(QUIT_ID, format!("Quit {APP_NAME}"), true, None);
        Menu::with_items(&[&show, &tools, &PredefinedMenuItem::separator(), &quit])
    }

    fn find_menu_event(id: &MenuId) -> Option<DesktopEvent> {
        let id = id.as_ref();
        if id == SHOW_ID {
            return Some(DesktopEvent::Show);
        }
        if id == QUIT_ID {
            return Some(DesktopEvent::Quit);
        }
        let index: usize = id.strip_prefix(TOOL_ID_PREFIX)?.parse().ok()?;
        ToolId::ALL.get(index).copied().map(DesktopEvent::OpenTool)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn menu_ids_map_back_to_events() {
            assert_eq!(
                find_menu_event(&MenuId::new(SHOW_ID)),
                Some(DesktopEvent::Show)
            );
            assert_eq!(
                find_menu_event(&MenuId::new(QUIT_ID)),
                Some(DesktopEvent::Quit)
            );
            assert_eq!(
                find_menu_event(&MenuId::new("tool:0")),
                Some(DesktopEvent::OpenTool(ToolId::ALL[0]))
            );
            assert_eq!(find_menu_event(&MenuId::new("tool:99")), None);
            assert_eq!(find_menu_event(&MenuId::new("other")), None);
        }

        #[test]
        fn the_icon_image_loads() {
            assert!(load_icon().is_ok());
        }
    }
}

#[cfg(target_os = "linux")]
mod unsupported {
    use std::sync::mpsc::Sender;

    use eframe::egui;

    use crate::desktop::DesktopEvent;

    pub const IS_MENU_BAR_SUPPORTED: bool = false;

    pub struct MenuBarIcon;

    impl MenuBarIcon {
        pub fn new(_ctx: &egui::Context, _sender: Sender<DesktopEvent>) -> Self {
            Self
        }

        pub fn set_visible(&mut self, _should_show: bool) {}

        pub fn problem(&self) -> Option<&str> {
            None
        }
    }
}
