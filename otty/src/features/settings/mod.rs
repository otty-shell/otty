pub(crate) mod errors;
mod model;
mod storage;

use model::is_hex_color_prefix;
use storage::{SettingsLoadStatus, load_settings, save_settings};

pub(crate) use errors::SettingsError;
pub(crate) use model::is_valid_hex_color;
pub(crate) use model::{SettingsData, default_palette, palette_label};

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
    pub(crate) title: String,
    pub(crate) expanded: bool,
    pub(crate) kind: SettingsNodeKind,
    pub(crate) children: Vec<SettingsNode>,
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

/// UI events emitted by the settings tab.
#[derive(Debug, Clone)]
pub(crate) enum SettingsEvent {
    Save,
    Reset,
    NodePressed { path: Vec<String> },
    NodeHovered { path: Option<Vec<String>> },
    ShellChanged(String),
    EditorChanged(String),
    PaletteChanged { index: usize, value: String },
    ApplyPreset(SettingsPreset),
}

/// Stored and draft settings state for the settings tab.
#[derive(Debug)]
pub(crate) struct SettingsState {
    pub(crate) baseline: SettingsData,
    pub(crate) draft: SettingsData,
    pub(crate) palette_inputs: Vec<String>,
    pub(crate) tree: Vec<SettingsNode>,
    pub(crate) selected_section: SettingsSection,
    pub(crate) selected_path: Vec<String>,
    pub(crate) hovered_path: Option<Vec<String>>,
    pub(crate) dirty: bool,
}

impl SettingsState {
    pub(crate) fn load() -> Self {
        match load_settings() {
            Ok(load) => {
                if let SettingsLoadStatus::Invalid(message) = &load.status {
                    log::warn!("settings file invalid: {message}");
                }
                Self::from_settings(load.settings)
            },
            Err(err) => {
                log::warn!("settings load failed: {err}");
                Self::from_settings(SettingsData::default())
            },
        }
    }

    pub(crate) fn reload(&mut self) {
        match load_settings() {
            Ok(load) => {
                if let SettingsLoadStatus::Invalid(message) = &load.status {
                    log::warn!("settings file invalid: {message}");
                }
                self.reset_to_settings(load.settings);
            },
            Err(err) => {
                log::warn!("settings reload failed: {err}");
            },
        }
    }

    pub(crate) fn persist(&mut self) -> Result<SettingsData, SettingsError> {
        let normalized = self.draft.normalized();
        save_settings(&normalized)?;
        self.mark_saved(normalized.clone());
        Ok(normalized)
    }

    pub(crate) fn mark_saved(&mut self, settings: SettingsData) {
        self.reset_to_settings(settings);
    }

    pub(crate) fn reset(&mut self) {
        let baseline = self.baseline.clone();
        self.reset_to_settings(baseline);
    }

    pub(crate) fn set_shell(&mut self, value: String) {
        self.draft.terminal.shell = value;
        self.update_dirty();
    }

    pub(crate) fn set_editor(&mut self, value: String) {
        self.draft.terminal.editor = value;
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
        if model::is_valid_hex_color(&value)
            && index < self.draft.theme.palette.len()
        {
            self.draft.theme.palette[index] = value;
            self.update_dirty();
        }
    }

    pub(crate) fn apply_preset(&mut self, preset: SettingsPreset) {
        let palette = preset.palette();
        self.draft.theme.palette = palette.clone();
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

    pub(crate) fn update_dirty(&mut self) {
        self.dirty = self.draft != self.baseline;
    }

    fn from_settings(settings: SettingsData) -> Self {
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
        let palette_inputs = settings.theme.palette.clone();

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

    fn reset_to_settings(&mut self, settings: SettingsData) {
        self.baseline = settings.clone();
        self.draft = settings;
        self.palette_inputs = self.draft.theme.palette.clone();
        self.hovered_path = None;
        self.dirty = false;
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
