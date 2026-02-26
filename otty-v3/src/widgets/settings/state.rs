use super::model::{
    SettingsData, SettingsNode, SettingsPreset, SettingsSection,
    is_hex_color_prefix, is_valid_hex_color,
};

/// Stored and draft settings state for the settings widget.
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

    /// Mark the draft as saved by replacing baseline with the given data.
    pub(crate) fn mark_saved(&mut self, settings: SettingsData) {
        self.replace_with_settings(settings);
    }

    /// Reset draft to baseline.
    pub(crate) fn reset(&mut self) {
        let baseline = self.baseline.clone();
        self.replace_with_settings(baseline);
    }

    /// Update the shell field in the draft.
    pub(crate) fn set_shell(&mut self, value: String) {
        self.draft.set_terminal_shell(value);
        self.update_dirty();
    }

    /// Update the editor field in the draft.
    pub(crate) fn set_editor(&mut self, value: String) {
        self.draft.set_terminal_editor(value);
        self.update_dirty();
    }

    /// Update a palette input, propagating valid values to the draft.
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

    /// Apply a theme preset palette to the draft.
    pub(crate) fn apply_preset(&mut self, preset: SettingsPreset) {
        let palette = preset.palette();
        self.draft.set_theme_palette(palette.clone());
        self.palette_inputs = palette;
        self.update_dirty();
    }

    /// Select a tree path (toggles folders, selects sections).
    pub(crate) fn select_path(&mut self, path: &[String]) {
        if let Some(node) = find_node_mut(&mut self.tree, path) {
            if node.is_folder() {
                node.toggle_expanded();
                return;
            }

            if let Some(section) = node.section_kind() {
                self.selected_section = section;
                self.selected_path = path.to_vec();
            }
        }
    }

    /// Set the currently hovered tree path.
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

    for node in nodes.iter_mut() {
        if node.title() == path[0] {
            if path.len() == 1 {
                return Some(node);
            }

            return find_node_mut(node.children_mut(), &path[1..]);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::SettingsState;
    use crate::widgets::settings::model::{
        SettingsData, SettingsPreset, SettingsSection,
    };

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
        assert!(state.tree[0].is_expanded());

        state.select_path(&path);

        assert!(!state.tree[0].is_expanded());
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
