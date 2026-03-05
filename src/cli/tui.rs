//! FJ-107: Interactive TUI mode.
//!
//! Terminal UI for browsing plan, approving resources selectively,
//! and viewing live apply status. Uses ANSI escape codes for
//! terminal manipulation without external dependencies.

use std::io::Write;

/// ANSI escape sequences.
pub mod ansi {
    /// Clear the entire screen.
    pub const CLEAR_SCREEN: &str = "\x1b[2J";
    /// Move cursor to top-left.
    pub const CURSOR_HOME: &str = "\x1b[H";
    /// Bold text.
    pub const BOLD: &str = "\x1b[1m";
    /// Reset all attributes.
    pub const RESET: &str = "\x1b[0m";
    /// Green foreground.
    pub const GREEN: &str = "\x1b[32m";
    /// Red foreground.
    pub const RED: &str = "\x1b[31m";
    /// Yellow foreground.
    pub const YELLOW: &str = "\x1b[33m";
    /// Cyan foreground.
    pub const CYAN: &str = "\x1b[36m";
    /// Dim text.
    pub const DIM: &str = "\x1b[2m";
}

/// A TUI item that can be selected/approved.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TuiItem {
    /// Resource identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Planned action (create, update, destroy).
    pub action: String,
    /// Whether the item is selected for approval.
    pub selected: bool,
}

/// TUI view state.
#[derive(Debug)]
pub struct TuiState {
    /// Selectable items.
    pub items: Vec<TuiItem>,
    /// Current cursor position.
    pub cursor: usize,
    /// View title.
    pub title: String,
    /// Current interaction mode.
    pub mode: TuiMode,
}

/// TUI interaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiMode {
    /// Browsing items.
    Browse,
    /// Selecting items for approval.
    Select,
    /// Confirming selection.
    Confirm,
}

impl TuiState {
    /// Create a new TUI state with the given title and items.
    pub fn new(title: &str, items: Vec<TuiItem>) -> Self {
        TuiState {
            cursor: 0,
            title: title.to_string(),
            mode: TuiMode::Browse,
            items,
        }
    }

    /// Move cursor up.
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor down.
    pub fn cursor_down(&mut self) {
        if self.cursor + 1 < self.items.len() {
            self.cursor += 1;
        }
    }

    /// Toggle selection on current item.
    pub fn toggle_select(&mut self) {
        if let Some(item) = self.items.get_mut(self.cursor) {
            item.selected = !item.selected;
        }
    }

    /// Select all items.
    pub fn select_all(&mut self) {
        for item in &mut self.items {
            item.selected = true;
        }
    }

    /// Deselect all items.
    pub fn deselect_all(&mut self) {
        for item in &mut self.items {
            item.selected = false;
        }
    }

    /// Get selected items.
    pub fn selected(&self) -> Vec<&TuiItem> {
        self.items.iter().filter(|i| i.selected).collect()
    }

    /// Render the TUI to a string (for testing).
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "{}{} {}{}\n\n",
            ansi::BOLD,
            ansi::CYAN,
            self.title,
            ansi::RESET
        ));

        for (i, item) in self.items.iter().enumerate() {
            let cursor = if i == self.cursor { ">" } else { " " };
            let check = if item.selected { "[x]" } else { "[ ]" };
            let color = action_color(&item.action);
            out.push_str(&format!(
                " {cursor} {check} {color}{}{} — {}{}\n",
                item.id,
                ansi::RESET,
                item.description,
                ansi::RESET
            ));
        }

        out.push_str(&format!(
            "\n{}[j/k] move  [space] toggle  [a] all  [enter] confirm  [q] quit{}\n",
            ansi::DIM,
            ansi::RESET
        ));
        out
    }

    /// Render to stderr (actual TUI output).
    pub fn display(&self) {
        let rendered = self.render();
        eprint!("{}{}{}", ansi::CLEAR_SCREEN, ansi::CURSOR_HOME, rendered);
        let _ = std::io::stderr().flush();
    }
}

fn action_color(action: &str) -> &str {
    match action {
        "create" => ansi::GREEN,
        "update" => ansi::YELLOW,
        "destroy" => ansi::RED,
        _ => ansi::RESET,
    }
}

/// Build TUI items from plan changes.
pub fn plan_to_tui_items(changes: &[(String, String, String)]) -> Vec<TuiItem> {
    changes
        .iter()
        .map(|(id, action, desc)| TuiItem {
            id: id.clone(),
            description: desc.clone(),
            action: action.clone(),
            selected: action != "destroy",
        })
        .collect()
}

/// TUI result after user interaction.
#[derive(Debug, serde::Serialize)]
pub struct TuiResult {
    /// IDs of approved items.
    pub approved: Vec<String>,
    /// IDs of rejected items.
    pub rejected: Vec<String>,
    /// Whether the user confirmed.
    pub confirmed: bool,
}

/// Build result from TUI state.
pub fn build_result(state: &TuiState) -> TuiResult {
    let approved: Vec<String> = state
        .items
        .iter()
        .filter(|i| i.selected)
        .map(|i| i.id.clone())
        .collect();
    let rejected: Vec<String> = state
        .items
        .iter()
        .filter(|i| !i.selected)
        .map(|i| i.id.clone())
        .collect();
    TuiResult {
        confirmed: state.mode == TuiMode::Confirm,
        approved,
        rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_items() -> Vec<TuiItem> {
        vec![
            TuiItem {
                id: "pkg-nginx".into(),
                description: "install nginx".into(),
                action: "create".into(),
                selected: true,
            },
            TuiItem {
                id: "file-conf".into(),
                description: "write config".into(),
                action: "update".into(),
                selected: true,
            },
            TuiItem {
                id: "svc-old".into(),
                description: "remove service".into(),
                action: "destroy".into(),
                selected: false,
            },
        ]
    }

    #[test]
    fn test_tui_state_new() {
        let state = TuiState::new("Plan Review", sample_items());
        assert_eq!(state.cursor, 0);
        assert_eq!(state.items.len(), 3);
    }

    #[test]
    fn test_cursor_navigation() {
        let mut state = TuiState::new("test", sample_items());
        state.cursor_down();
        assert_eq!(state.cursor, 1);
        state.cursor_down();
        assert_eq!(state.cursor, 2);
        state.cursor_down(); // at end, stays
        assert_eq!(state.cursor, 2);
        state.cursor_up();
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn test_toggle_select() {
        let mut state = TuiState::new("test", sample_items());
        assert!(state.items[0].selected);
        state.toggle_select();
        assert!(!state.items[0].selected);
        state.toggle_select();
        assert!(state.items[0].selected);
    }

    #[test]
    fn test_select_all() {
        let mut state = TuiState::new("test", sample_items());
        state.select_all();
        assert!(state.items.iter().all(|i| i.selected));
    }

    #[test]
    fn test_deselect_all() {
        let mut state = TuiState::new("test", sample_items());
        state.deselect_all();
        assert!(state.items.iter().all(|i| !i.selected));
    }

    #[test]
    fn test_selected() {
        let state = TuiState::new("test", sample_items());
        let sel = state.selected();
        assert_eq!(sel.len(), 2); // pkg-nginx and file-conf
    }

    #[test]
    fn test_render_contains_items() {
        let state = TuiState::new("Plan Review", sample_items());
        let rendered = state.render();
        assert!(rendered.contains("pkg-nginx"));
        assert!(rendered.contains("file-conf"));
        assert!(rendered.contains("svc-old"));
    }

    #[test]
    fn test_plan_to_tui_items() {
        let changes = vec![
            ("A".into(), "create".into(), "install A".into()),
            ("B".into(), "destroy".into(), "remove B".into()),
        ];
        let items = plan_to_tui_items(&changes);
        assert_eq!(items.len(), 2);
        assert!(items[0].selected); // create → selected
        assert!(!items[1].selected); // destroy → not selected
    }

    #[test]
    fn test_build_result() {
        let state = TuiState::new("test", sample_items());
        let result = build_result(&state);
        assert_eq!(result.approved.len(), 2);
        assert_eq!(result.rejected.len(), 1);
        assert!(!result.confirmed); // Browse mode, not confirmed
    }

    #[test]
    fn test_result_serde() {
        let result = TuiResult {
            approved: vec!["A".into()],
            rejected: vec!["B".into()],
            confirmed: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"confirmed\":true"));
    }
}
