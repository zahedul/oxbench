//! Which tools are open, which one is active, favorites and recents. Saved between launches.

use serde::{Deserialize, Serialize};

use crate::registry::ToolId;

pub const MAX_FAVORITES: usize = 9;
const MAX_RECENT: usize = 6;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Workspace {
    pub open_tabs: Vec<ToolId>,
    /// `None` shows the home screen.
    pub active_tab: Option<ToolId>,
    pub favorites: Vec<ToolId>,
    /// Most recent first.
    pub recent: Vec<ToolId>,
}

impl Workspace {
    pub fn open_tool(&mut self, id: ToolId) {
        if !self.open_tabs.contains(&id) {
            self.open_tabs.push(id);
        }
        self.active_tab = Some(id);
        self.remember_recent(id);
    }

    /// Returns `false` when the tool was not open.
    pub fn close_tool(&mut self, id: ToolId) -> bool {
        let Some(index) = self.open_tabs.iter().position(|open| *open == id) else {
            return false;
        };
        self.open_tabs.remove(index);
        if self.active_tab == Some(id) {
            let neighbour = self.open_tabs.get(index).or_else(|| self.open_tabs.last());
            self.active_tab = neighbour.copied();
        }
        true
    }

    pub fn show_home(&mut self) {
        self.active_tab = None;
    }

    pub fn is_favorite(&self, id: ToolId) -> bool {
        self.favorites.contains(&id)
    }

    /// Returns `false` when the tool could not be added because the favorites are full.
    pub fn toggle_favorite(&mut self, id: ToolId) -> bool {
        if let Some(index) = self.favorites.iter().position(|fav| *fav == id) {
            self.favorites.remove(index);
            return true;
        }
        if self.favorites.len() >= MAX_FAVORITES {
            return false;
        }
        self.favorites.push(id);
        true
    }

    /// Drops ids that came from an older or corrupt save so the restored state is consistent.
    pub fn normalize(&mut self) {
        dedupe_in_place(&mut self.open_tabs);
        dedupe_in_place(&mut self.favorites);
        dedupe_in_place(&mut self.recent);
        self.favorites.truncate(MAX_FAVORITES);
        self.recent.truncate(MAX_RECENT);
        let is_active_open = self
            .active_tab
            .is_some_and(|id| self.open_tabs.contains(&id));
        if !is_active_open {
            self.active_tab = None;
        }
    }

    fn remember_recent(&mut self, id: ToolId) {
        self.recent.retain(|recent| *recent != id);
        self.recent.insert(0, id);
        self.recent.truncate(MAX_RECENT);
    }
}

fn dedupe_in_place(ids: &mut Vec<ToolId>) {
    let mut seen = Vec::with_capacity(ids.len());
    ids.retain(|id| {
        let is_new = !seen.contains(id);
        seen.push(*id);
        is_new
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ToolId::{Bcrypt, EpochConverter, HashGenerator, JsonViewer, JwtDecoder};

    fn workspace_with_tabs(tabs: &[ToolId]) -> Workspace {
        let mut workspace = Workspace::default();
        for tab in tabs {
            workspace.open_tool(*tab);
        }
        workspace
    }

    #[test]
    fn opening_a_tool_adds_one_tab_and_activates_it() {
        let mut workspace = workspace_with_tabs(&[JsonViewer, JwtDecoder]);
        workspace.open_tool(JsonViewer);
        assert_eq!(workspace.open_tabs, [JsonViewer, JwtDecoder]);
        assert_eq!(workspace.active_tab, Some(JsonViewer));
    }

    #[test]
    fn closing_the_active_tab_activates_the_next_one() {
        let mut workspace = workspace_with_tabs(&[JsonViewer, JwtDecoder, Bcrypt]);
        workspace.open_tool(JwtDecoder);
        assert!(workspace.close_tool(JwtDecoder));
        assert_eq!(workspace.active_tab, Some(Bcrypt));
    }

    #[test]
    fn closing_the_last_active_tab_falls_back_to_the_previous_one() {
        let mut workspace = workspace_with_tabs(&[JsonViewer, JwtDecoder]);
        workspace.close_tool(JwtDecoder);
        assert_eq!(workspace.active_tab, Some(JsonViewer));
    }

    #[test]
    fn closing_the_only_tab_shows_home() {
        let mut workspace = workspace_with_tabs(&[JsonViewer]);
        workspace.close_tool(JsonViewer);
        assert_eq!(workspace.active_tab, None);
        assert_eq!(workspace.open_tabs, []);
    }

    #[test]
    fn closing_an_inactive_tab_keeps_the_active_one() {
        let mut workspace = workspace_with_tabs(&[JsonViewer, JwtDecoder]);
        workspace.close_tool(JsonViewer);
        assert_eq!(workspace.active_tab, Some(JwtDecoder));
    }

    #[test]
    fn closing_a_tool_that_is_not_open_does_nothing() {
        let mut workspace = workspace_with_tabs(&[JsonViewer]);
        assert!(!workspace.close_tool(Bcrypt));
        assert_eq!(workspace.open_tabs, [JsonViewer]);
    }

    #[test]
    fn recent_tools_are_most_recent_first_without_duplicates() {
        let workspace = workspace_with_tabs(&[JsonViewer, JwtDecoder, JsonViewer]);
        assert_eq!(workspace.recent, [JsonViewer, JwtDecoder]);
    }

    #[test]
    fn recent_tools_are_capped() {
        let workspace = workspace_with_tabs(&ToolId::ALL);
        assert_eq!(workspace.recent.len(), MAX_RECENT);
        assert_eq!(workspace.recent[0], ToolId::KeyPair);
    }

    #[test]
    fn toggling_a_favorite_adds_then_removes_it() {
        let mut workspace = Workspace::default();
        assert!(workspace.toggle_favorite(HashGenerator));
        assert!(workspace.is_favorite(HashGenerator));
        assert!(workspace.toggle_favorite(HashGenerator));
        assert!(!workspace.is_favorite(HashGenerator));
    }

    #[test]
    fn favorites_stop_at_the_limit() {
        let mut workspace = Workspace::default();
        for id in ToolId::ALL.into_iter().take(MAX_FAVORITES) {
            assert!(workspace.toggle_favorite(id));
        }
        assert!(!workspace.toggle_favorite(ToolId::KeyPair));
        assert_eq!(workspace.favorites.len(), MAX_FAVORITES);
    }

    #[test]
    fn normalize_repairs_an_inconsistent_save() {
        let mut workspace = Workspace {
            open_tabs: vec![JsonViewer, JsonViewer],
            active_tab: Some(EpochConverter),
            favorites: vec![Bcrypt, Bcrypt],
            recent: vec![],
        };
        workspace.normalize();
        assert_eq!(workspace.open_tabs, [JsonViewer]);
        assert_eq!(workspace.favorites, [Bcrypt]);
        assert_eq!(workspace.active_tab, None);
    }
}
