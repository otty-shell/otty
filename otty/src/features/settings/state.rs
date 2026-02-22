use super::model::{
    SettingsData, default_palette, is_hex_color_prefix, is_valid_hex_color,
};

/// Top-level settings pages shown in the settings tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsSection {
    Terminal,
    Theme,
}

impl SettingsSection {
    pub(crate) fn title(&self) -> &'static str {
        match self {
            SettingsSection::Terminal => "Terminal",
            SettingsSection::Theme => "Theme",
        }
    }
}

/// Node kinds for the settings navigation tree.
#[derive(Debug, Clone)]
pub(crate) enum SettingsNodeKind {
    Folder,
    Section(SettingsSection),
}

/// Navigation tree node for the settings sidebar.
#[derive(Debug, Clone)]
pub(crate) struct SettingsNode {
    title: String,
    expanded: bool,
    kind: SettingsNodeKind,
    children: Vec<SettingsNode>,
}

impl SettingsNode {
    pub(crate) fn folder(
        title: impl Into<String>,
        children: Vec<SettingsNode>,
    ) -> Self {
        Self {
            title: title.into(),
            expanded: true,
            kind: SettingsNodeKind::Folder,
            children,
        }
    }

    pub(crate) fn section(section: SettingsSection) -> Self {
        Self {
            title: section.title().to_string(),
            expanded: false,
            kind: SettingsNodeKind::Section(section),
            children: Vec::new(),
        }
    }

    pub(crate) fn is_folder(&self) -> bool {
        matches!(self.kind, SettingsNodeKind::Folder)
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn children(&self) -> &[SettingsNode] {
        &self.children
    }

    pub(crate) fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub(crate) fn section_kind(&self) -> Option<SettingsSection> {
        match self.kind {
            SettingsNodeKind::Section(section) => Some(section),
            _ => None,
        }
    }
}

/// Palette presets available in the theme editor.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SettingsPreset {
    OttyDark,
}

impl SettingsPreset {
    pub(crate) fn palette(&self) -> Vec<String> {
        match self {
            SettingsPreset::OttyDark => default_palette(),
        }
    }
}

/// Stored and draft settings state for the settings tab.
#[derive(Debug)]
pub(crate) struct SettingsState {
    baseline: SettingsData,
    draft: SettingsData,
    palette_inputs: Vec<String>,
    tree: Vec<SettingsNode>,
    selected_section: SettingsSection,
    selected_path: Vec<String>,
    hovered_path: Option<Vec<String>>,
    dirty: bool,
}

impl SettingsState {
    /// Return persisted settings currently used as dirty baseline.
    #[cfg(test)]
    pub(crate) fn baseline(&self) -> &SettingsData {
        &self.baseline
    }

    /// Return editable settings draft.
    pub(crate) fn draft(&self) -> &SettingsData {
        &self.draft
    }

    /// Return raw palette text input values.
    pub(crate) fn palette_inputs(&self) -> &[String] {
        &self.palette_inputs
    }

    /// Return sidebar tree nodes.
    pub(crate) fn tree(&self) -> &[SettingsNode] {
        &self.tree
    }

    /// Return currently selected settings section.
    pub(crate) fn selected_section(&self) -> SettingsSection {
        self.selected_section
    }

    /// Return selected tree row path.
    pub(crate) fn selected_path(&self) -> &Vec<String> {
        &self.selected_path
    }

    /// Return currently hovered tree row path.
    pub(crate) fn hovered_path(&self) -> Option<&Vec<String>> {
        self.hovered_path.as_ref()
    }

    /// Return whether the draft differs from the persisted baseline.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Create state from a persisted settings payload.
    pub(crate) fn from_settings(settings: SettingsData) -> Self {
        let tree = vec![SettingsNode::folder(
            "General",
            vec![
                SettingsNode::section(SettingsSection::Terminal),
                SettingsNode::section(SettingsSection::Theme),
            ],
        )];
        let selected_section = SettingsSection::Terminal;
        let selected_path = vec![
            String::from("General"),
            selected_section.title().to_string(),
        ];
        let palette_inputs = settings.theme_palette().to_vec();

        Self {
            baseline: settings.clone(),
            draft: settings,
            palette_inputs,
            tree,
            selected_section,
            selected_path,
            hovered_path: None,
            dirty: false,
        }
    }

    /// Replace persisted and draft values using freshly loaded settings.
    pub(crate) fn replace_with_settings(&mut self, settings: SettingsData) {
        self.baseline = settings.clone();
        self.draft = settings;
        self.palette_inputs = self.draft.theme_palette().to_vec();
        self.hovered_path = None;
        self.dirty = false;
    }

    /// Return normalized draft settings ready for persistence.
    pub(crate) fn normalized_draft(&self) -> SettingsData {
        self.draft.normalized()
    }

    pub(crate) fn mark_saved(&mut self, settings: SettingsData) {
        self.replace_with_settings(settings);
    }

    pub(crate) fn reset(&mut self) {
        let baseline = self.baseline.clone();
        self.replace_with_settings(baseline);
    }

    pub(crate) fn set_shell(&mut self, value: String) {
        self.draft.set_terminal_shell(value);
        self.update_dirty();
    }

    pub(crate) fn set_editor(&mut self, value: String) {
        self.draft.set_terminal_editor(value);
        self.update_dirty();
    }

    pub(crate) fn set_palette_input(&mut self, index: usize, value: String) {
        if index >= self.palette_inputs.len() {
            return;
        }

        if !is_hex_color_prefix(&value) {
            return;
        }

        self.palette_inputs[index] = value.clone();
        if is_valid_hex_color(&value)
            && self.draft.set_theme_palette_entry(index, value)
        {
            self.update_dirty();
        }
    }

    pub(crate) fn apply_preset(&mut self, preset: SettingsPreset) {
        let palette = preset.palette();
        self.draft.set_theme_palette(palette.clone());
        self.palette_inputs = palette;
        self.update_dirty();
    }

    pub(crate) fn select_path(&mut self, path: &[String]) {
        if let Some(node) = find_node_mut(&mut self.tree, path) {
            if node.is_folder() {
                node.expanded = !node.expanded;
                return;
            }

            if let Some(section) = node.section_kind() {
                self.selected_section = section;
                self.selected_path = path.to_vec();
            }
        }
    }

    pub(crate) fn set_hovered_path(&mut self, path: Option<Vec<String>>) {
        self.hovered_path = path;
    }

    fn update_dirty(&mut self) {
        self.dirty = self.draft != self.baseline;
    }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self::from_settings(SettingsData::default())
    }
}

fn find_node_mut<'a>(
    nodes: &'a mut [SettingsNode],
    path: &[String],
) -> Option<&'a mut SettingsNode> {
    if path.is_empty() {
        return None;
    }

    for node in nodes {
        if node.title == path[0] {
            if path.len() == 1 {
                return Some(node);
            }

            return find_node_mut(&mut node.children, &path[1..]);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{SettingsPreset, SettingsSection, SettingsState};
    use crate::features::settings::SettingsData;

    #[test]
    fn given_default_state_when_set_shell_then_marks_dirty() {
        let mut state = SettingsState::default();

        state.set_shell(String::from("/bin/zsh"));

        assert_eq!(state.draft.terminal_shell(), "/bin/zsh");
        assert!(state.dirty);
    }

    #[test]
    fn given_dirty_state_when_reset_then_restores_baseline() {
        let mut state = SettingsState::default();
        let baseline = state.baseline.clone();
        state.set_editor(String::from("vim"));

        state.reset();

        assert_eq!(state.draft, baseline);
        assert!(!state.dirty);
    }

    #[test]
    fn given_invalid_palette_prefix_when_set_palette_input_then_ignores_change()
    {
        let mut state = SettingsState::default();
        let before_input = state.palette_inputs[0].clone();
        let before_palette = state.draft.theme_palette()[0].clone();

        state.set_palette_input(0, String::from("x12345"));

        assert_eq!(state.palette_inputs[0], before_input);
        assert_eq!(state.draft.theme_palette()[0], before_palette);
    }

    #[test]
    fn given_valid_palette_value_when_set_palette_input_then_updates_palette() {
        let mut state = SettingsState::default();

        state.set_palette_input(0, String::from("#112233"));

        assert_eq!(state.palette_inputs[0], "#112233");
        assert_eq!(state.draft.theme_palette()[0], "#112233");
        assert!(state.dirty);
    }

    #[test]
    fn given_preset_when_apply_preset_then_updates_inputs_and_marks_dirty() {
        let mut baseline = SettingsData::default();
        baseline.set_theme_palette_entry(0, String::from("#123456"));
        let mut state = SettingsState::from_settings(baseline);

        state.apply_preset(SettingsPreset::OttyDark);

        assert_eq!(state.palette_inputs, state.draft.theme_palette());
        assert!(state.dirty);
    }

    #[test]
    fn given_folder_path_when_select_path_then_toggles_expansion() {
        let mut state = SettingsState::default();
        let path = vec![String::from("General")];
        assert!(state.tree[0].expanded);

        state.select_path(&path);

        assert!(!state.tree[0].expanded);
    }

    #[test]
    fn given_section_path_when_select_path_then_updates_selected_section() {
        let mut state = SettingsState::default();
        let path = vec![String::from("General"), String::from("Theme")];

        state.select_path(&path);

        assert_eq!(state.selected_section, SettingsSection::Theme);
        assert_eq!(state.selected_path, path);
    }
}
