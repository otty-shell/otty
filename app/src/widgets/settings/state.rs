use super::types::{
    LanguageSetting, SettingsData, SettingsNode, SettingsSection,
};
use crate::theme::BuiltinTheme;

/// Stored and draft settings state for the settings widget.
#[derive(Debug)]
pub(crate) struct SettingsState {
    baseline: SettingsData,
    draft: SettingsData,
    tree: Vec<SettingsNode>,
    selected_section: SettingsSection,
    selected_path: Vec<String>,
    hovered_path: Option<Vec<String>>,
    dirty: bool,
    saving: bool,
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

    /// Return whether a save is currently being written to disk.
    pub(crate) fn is_saving(&self) -> bool {
        self.saving
    }

    /// Create state from a persisted settings payload.
    pub(crate) fn from_settings(settings: SettingsData) -> Self {
        let selected_section = SettingsSection::General;

        Self {
            baseline: settings.clone(),
            draft: settings,
            tree: build_tree(),
            selected_section,
            selected_path: vec![selected_section.title().to_string()],
            hovered_path: None,
            dirty: false,
            saving: false,
        }
    }

    /// Replace persisted and draft values using freshly loaded settings.
    ///
    /// Rebuilds the navigation tree because section titles are localized and
    /// double as tree path segments: a language change invalidates every
    /// previously stored path.
    pub(super) fn replace_with_settings(&mut self, settings: SettingsData) {
        self.baseline = settings.clone();
        self.draft = settings;

        self.tree = build_tree();
        self.selected_path = vec![self.selected_section.title().to_string()];
        self.hovered_path = None;
        self.dirty = false;
    }

    /// Return normalized draft settings ready for persistence.
    pub(crate) fn normalized_draft(&self) -> SettingsData {
        self.draft.normalized()
    }

    /// Capture the current draft for one save, locking out further edits.
    pub(super) fn begin_save(&mut self) -> Option<SettingsData> {
        if self.saving || !self.dirty {
            return None;
        }

        self.saving = true;
        Some(self.normalized_draft())
    }

    /// Mark the draft as saved by replacing baseline with the given data.
    pub(super) fn mark_saved(&mut self, settings: SettingsData) {
        self.saving = false;
        self.replace_with_settings(settings);
    }

    /// Unlock editing so a failed save can be retried.
    pub(super) fn mark_save_failed(&mut self) {
        self.saving = false;
    }

    /// Reset draft to baseline.
    pub(super) fn reset(&mut self) {
        let baseline = self.baseline.clone();
        self.replace_with_settings(baseline);
    }

    /// Update the interface language in the draft.
    pub(super) fn set_language(&mut self, value: LanguageSetting) {
        self.draft.set_language(value);
        self.update_dirty();
    }

    /// Update the shell field in the draft.
    pub(super) fn set_shell(&mut self, value: String) {
        self.draft.set_terminal_shell(value);
        self.update_dirty();
    }

    /// Update the editor field in the draft.
    pub(super) fn set_editor(&mut self, value: String) {
        self.draft.set_terminal_editor(value);
        self.update_dirty();
    }

    /// Update whether sibling panes are evened out on split and close.
    pub(super) fn set_equalize_panes(&mut self, value: bool) {
        self.draft.set_equalize_panes(value);
        self.update_dirty();
    }

    /// Select the theme in the draft.
    pub(super) fn set_theme(&mut self, value: BuiltinTheme) {
        self.draft.set_theme(value);
        self.update_dirty();
    }

    /// Select a tree path (toggles folders, selects sections).
    pub(super) fn select_path(&mut self, path: &[String]) {
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
    pub(super) fn set_hovered_path(&mut self, path: Option<Vec<String>>) {
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

/// Build the navigation tree using the active locale's section titles.
fn build_tree() -> Vec<SettingsNode> {
    SettingsSection::ALL
        .into_iter()
        .map(SettingsNode::section)
        .collect()
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
    use crate::i18n::Locale;
    use crate::theme::BuiltinTheme;
    use crate::widgets::settings::types::{
        LanguageSetting, SettingsData, SettingsSection,
    };

    #[test]
    fn given_default_state_when_set_shell_then_marks_dirty() {
        let mut state = SettingsState::default();
        let shell = format!("{}-changed", state.draft.terminal_shell());

        state.set_shell(shell.clone());

        assert_eq!(state.draft.terminal_shell(), shell);
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
    fn given_save_in_flight_when_begin_save_called_again_then_second_is_ignored()
     {
        let mut state = SettingsState::default();
        state.set_editor(String::from("vim"));

        assert!(state.begin_save().is_some());

        assert!(state.begin_save().is_none());
    }

    #[test]
    fn given_default_state_when_set_theme_then_draft_theme_updates_and_marks_dirty()
     {
        let mut state = SettingsState::default();

        state.set_theme(BuiltinTheme::OttyLight);

        assert_eq!(state.draft.theme(), BuiltinTheme::OttyLight);
        assert!(state.dirty);
    }

    #[test]
    fn given_changed_theme_when_saved_theme_selected_again_then_state_is_clean()
    {
        let mut settings = SettingsData::default();
        settings.set_theme(BuiltinTheme::OttyLight);
        let mut state = SettingsState::from_settings(settings);

        state.set_theme(BuiltinTheme::OttyDark);
        state.set_theme(BuiltinTheme::OttyLight);

        assert_eq!(state.draft.theme(), BuiltinTheme::OttyLight);
        assert!(!state.dirty);
    }

    #[test]
    fn given_dirty_theme_when_reset_then_saved_theme_is_restored() {
        let mut state = SettingsState::default();
        state.set_theme(BuiltinTheme::OttyLight);

        state.reset();

        assert_eq!(state.draft.theme(), BuiltinTheme::OttyDark);
        assert!(!state.dirty);
    }

    #[test]
    fn given_section_path_when_select_path_then_updates_selected_section() {
        let mut state = SettingsState::default();
        let path = section_path(&state, SettingsSection::Appearance);

        state.select_path(&path);

        assert_eq!(state.selected_section, SettingsSection::Appearance);
        assert_eq!(state.selected_path, path);
    }

    #[test]
    fn given_language_change_when_applied_then_draft_updates_and_marks_dirty() {
        let mut state = SettingsState::default();

        state.set_language(LanguageSetting::Fixed(Locale::ZhCn));

        assert_eq!(
            state.draft.language(),
            LanguageSetting::Fixed(Locale::ZhCn)
        );
        assert!(state.dirty);
    }

    #[test]
    fn given_default_state_when_tree_built_then_general_section_comes_first() {
        let state = SettingsState::default();

        let sections: Vec<Option<SettingsSection>> = state
            .tree()
            .iter()
            .map(|node| node.section_kind())
            .collect();

        assert_eq!(
            sections,
            vec![
                Some(SettingsSection::General),
                Some(SettingsSection::Appearance),
                Some(SettingsSection::Terminal),
            ]
        );
    }

    /// Read a section's tree path from the state so the assertion does not
    /// depend on which locale rendered the titles.
    fn section_path(
        state: &SettingsState,
        section: SettingsSection,
    ) -> Vec<String> {
        state
            .tree()
            .iter()
            .find(|node| node.section_kind() == Some(section))
            .map(|node| vec![node.title().to_string()])
            .expect("section should be present in the settings tree")
    }
}
