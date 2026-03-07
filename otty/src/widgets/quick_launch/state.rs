use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Point;

use super::types::{
    CommandLaunchOptions, CommandSpec, ContextMenuTarget, DropTarget, EnvVar,
    InlineEditKind, LaunchInfo, NodePath, QuickLaunch, QuickLaunchFile,
    QuickLaunchType, SshLaunchOptions, WizardMode, WizardOptions,
};

/// Core state for the quick launch tree and interactions.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchState {
    data: QuickLaunchFile,
    error_tabs: HashMap<u64, QuickLaunchErrorState>,
    dirty: bool,
    persist_in_flight: bool,
    selected_path: Option<NodePath>,
    hovered_path: Option<NodePath>,
    pressed_path: Option<NodePath>,
    launching: HashMap<NodePath, LaunchInfo>,
    canceled_launches: Vec<u64>,
    next_launch_id: u64,
    blink_nonce: u64,
    context_menu: Option<ContextMenuState>,
    inline_edit: Option<InlineEditState>,
    drag: Option<DragState>,
    drop_target: Option<DropTarget>,
    cursor: Point,
    wizard: WizardState,
}

impl Default for QuickLaunchState {
    fn default() -> Self {
        Self {
            data: QuickLaunchFile::default(),
            error_tabs: HashMap::new(),
            dirty: false,
            persist_in_flight: false,
            selected_path: None,
            hovered_path: None,
            pressed_path: None,
            launching: HashMap::new(),
            canceled_launches: Vec::new(),
            next_launch_id: 1,
            blink_nonce: 0,
            context_menu: None,
            inline_edit: None,
            drag: None,
            drop_target: None,
            cursor: Point::ORIGIN,
            wizard: WizardState::default(),
        }
    }
}

impl QuickLaunchState {
    pub(crate) fn with_data(data: QuickLaunchFile) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

    pub(crate) fn data(&self) -> &QuickLaunchFile {
        &self.data
    }

    pub(super) fn data_mut(&mut self) -> &mut QuickLaunchFile {
        &mut self.data
    }

    pub(crate) fn error_tab(
        &self,
        tab_id: u64,
    ) -> Option<&QuickLaunchErrorState> {
        self.error_tabs.get(&tab_id)
    }

    pub(super) fn set_error_tab(
        &mut self,
        tab_id: u64,
        state: QuickLaunchErrorState,
    ) {
        self.error_tabs.insert(tab_id, state);
    }

    pub(super) fn remove_error_tab(&mut self, tab_id: u64) {
        self.error_tabs.remove(&tab_id);
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(super) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn is_persist_in_flight(&self) -> bool {
        self.persist_in_flight
    }

    pub(super) fn begin_persist(&mut self) {
        self.persist_in_flight = true;
    }

    pub(super) fn complete_persist(&mut self) {
        self.persist_in_flight = false;
        self.dirty = false;
    }

    pub(super) fn fail_persist(&mut self) {
        self.persist_in_flight = false;
    }

    pub(crate) fn selected_path(&self) -> Option<&NodePath> {
        self.selected_path.as_ref()
    }

    pub(crate) fn selected_path_cloned(&self) -> Option<NodePath> {
        self.selected_path.clone()
    }

    pub(super) fn set_selected_path(&mut self, path: Option<NodePath>) {
        self.selected_path = path;
    }

    pub(super) fn clear_selected_path(&mut self) {
        self.selected_path = None;
    }

    pub(crate) fn hovered_path(&self) -> Option<&NodePath> {
        self.hovered_path.as_ref()
    }

    pub(super) fn set_hovered_path(&mut self, path: Option<NodePath>) {
        self.hovered_path = path;
    }

    pub(crate) fn pressed_path(&self) -> Option<&NodePath> {
        self.pressed_path.as_ref()
    }

    pub(super) fn set_pressed_path(&mut self, path: Option<NodePath>) {
        self.pressed_path = path;
    }

    pub(super) fn clear_pressed_path(&mut self) {
        self.pressed_path = None;
    }

    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }

    pub(super) fn set_cursor(&mut self, position: Point) {
        self.cursor = position;
    }

    pub(crate) fn context_menu(&self) -> Option<&ContextMenuState> {
        self.context_menu.as_ref()
    }

    pub(crate) fn context_menu_cloned(&self) -> Option<ContextMenuState> {
        self.context_menu.clone()
    }

    pub(super) fn set_context_menu(&mut self, menu: Option<ContextMenuState>) {
        self.context_menu = menu;
    }

    pub(super) fn clear_context_menu(&mut self) {
        self.context_menu = None;
    }

    pub(crate) fn inline_edit(&self) -> Option<&InlineEditState> {
        self.inline_edit.as_ref()
    }

    pub(super) fn inline_edit_mut(&mut self) -> Option<&mut InlineEditState> {
        self.inline_edit.as_mut()
    }

    pub(super) fn set_inline_edit(&mut self, edit: Option<InlineEditState>) {
        self.inline_edit = edit;
    }

    pub(super) fn clear_inline_edit(&mut self) {
        self.inline_edit = None;
    }

    pub(super) fn take_inline_edit(&mut self) -> Option<InlineEditState> {
        self.inline_edit.take()
    }

    pub(crate) fn drag(&self) -> Option<&DragState> {
        self.drag.as_ref()
    }

    pub(super) fn drag_mut(&mut self) -> Option<&mut DragState> {
        self.drag.as_mut()
    }

    pub(super) fn set_drag(&mut self, drag: Option<DragState>) {
        self.drag = drag;
    }

    pub(super) fn take_drag(&mut self) -> Option<DragState> {
        self.drag.take()
    }

    pub(super) fn clear_drag(&mut self) {
        self.drag = None;
    }

    pub(crate) fn drop_target(&self) -> Option<&DropTarget> {
        self.drop_target.as_ref()
    }

    pub(super) fn set_drop_target(&mut self, target: Option<DropTarget>) {
        self.drop_target = target;
    }

    pub(super) fn take_drop_target(&mut self) -> Option<DropTarget> {
        self.drop_target.take()
    }

    pub(super) fn clear_drop_target(&mut self) {
        self.drop_target = None;
    }

    pub(crate) fn launching(&self) -> &HashMap<NodePath, LaunchInfo> {
        &self.launching
    }

    pub(super) fn launching_mut(
        &mut self,
    ) -> &mut HashMap<NodePath, LaunchInfo> {
        &mut self.launching
    }

    pub(crate) fn has_active_launches(&self) -> bool {
        !self.launching.is_empty()
    }

    pub(crate) fn is_launching(&self, path: &NodePath) -> bool {
        self.launching.contains_key(path)
    }

    pub(crate) fn launch_info(&self, path: &[String]) -> Option<&LaunchInfo> {
        self.launching.get(path)
    }

    pub(super) fn begin_launch(
        &mut self,
        path: NodePath,
        cancel: Arc<AtomicBool>,
    ) -> u64 {
        let id = self.next_launch_id;
        self.next_launch_id += 1;
        self.launching.insert(
            path,
            LaunchInfo {
                id,
                launch_ticks: 0,
                is_indicator_highlighted: false,
                cancel,
            },
        );
        id
    }

    pub(super) fn remove_launch(
        &mut self,
        path: &[String],
    ) -> Option<LaunchInfo> {
        self.launching.remove(path)
    }

    pub(super) fn cancel_launch(&mut self, path: &[String]) {
        if let Some(info) = self.launching.get(path) {
            info.cancel.store(true, Ordering::Relaxed);
            self.canceled_launches.push(info.id);
        }
    }

    pub(super) fn take_canceled_launch(&mut self, launch_id: u64) -> bool {
        if let Some(pos) = self
            .canceled_launches
            .iter()
            .position(|&id| id == launch_id)
        {
            self.canceled_launches.remove(pos);
            true
        } else {
            false
        }
    }

    pub(crate) fn blink_nonce(&self) -> u64 {
        self.blink_nonce
    }

    pub(super) fn advance_blink_nonce(&mut self) {
        self.blink_nonce = self.blink_nonce.wrapping_add(1);
    }

    #[cfg(test)]
    pub(super) fn set_blink_nonce_for_tests(&mut self, value: u64) {
        self.blink_nonce = value;
    }

    pub(crate) fn wizard(&self) -> &WizardState {
        &self.wizard
    }

    pub(super) fn wizard_mut(&mut self) -> &mut WizardState {
        &mut self.wizard
    }
}

/// Payload for an error tab.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchErrorState {
    title: String,
    message: String,
}

impl QuickLaunchErrorState {
    pub(crate) fn new(title: String, message: String) -> Self {
        Self { title, message }
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

/// Active context menu state.
#[derive(Debug, Default, Clone)]
pub(crate) struct ContextMenuState {
    target: ContextMenuTarget,
    cursor: Point,
}

impl ContextMenuState {
    pub(super) fn with_target(mut self, target: ContextMenuTarget) -> Self {
        self.target = target;
        self
    }

    pub(super) fn with_cursor(mut self, cursor: Point) -> Self {
        self.cursor = cursor;
        self
    }

    pub(crate) fn target(&self) -> ContextMenuTarget {
        self.target.clone()
    }

    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }
}

/// Active inline edit state.
#[derive(Debug, Clone)]
pub(crate) struct InlineEditState {
    pub(super) kind: InlineEditKind,
    pub(super) value: String,
    pub(super) error: Option<String>,
    pub(super) id: iced::widget::Id,
}

/// Active drag state.
#[derive(Debug, Clone)]
pub(crate) struct DragState {
    pub(super) source: NodePath,
    pub(super) origin: Point,
    pub(super) active: bool,
}

/// Wizard editor states keyed by tab id.
#[derive(Debug, Default, Clone)]
pub(crate) struct WizardState {
    editors: HashMap<u64, WizardEditorState>,
}

impl WizardState {
    pub(crate) fn editor(&self, tab_id: u64) -> Option<&WizardEditorState> {
        self.editors.get(&tab_id)
    }

    pub(super) fn editor_mut(
        &mut self,
        tab_id: u64,
    ) -> Option<&mut WizardEditorState> {
        self.editors.get_mut(&tab_id)
    }

    pub(super) fn initialize_create(&mut self, tab_id: u64, parent: NodePath) {
        self.editors.insert(tab_id, WizardEditorState::new(parent));
    }

    pub(super) fn initialize_edit(
        &mut self,
        tab_id: u64,
        path: NodePath,
        command: &QuickLaunch,
    ) {
        self.editors
            .insert(tab_id, WizardEditorState::from_command(path, command));
    }

    pub(super) fn remove_tab(&mut self, tab_id: u64) {
        self.editors.remove(&tab_id);
    }

    #[cfg(test)]
    pub(super) fn set_editor(
        &mut self,
        tab_id: u64,
        editor: WizardEditorState,
    ) {
        self.editors.insert(tab_id, editor);
    }
}

/// Runtime state for a quick launch editor tab.
#[derive(Debug, Clone)]
pub(crate) struct WizardEditorState {
    mode: WizardMode,
    title: String,
    options: WizardOptions,
    error: Option<String>,
}

impl WizardEditorState {
    /// Build state for creating a command in the target folder.
    pub(crate) fn new(parent_path: NodePath) -> Self {
        Self {
            mode: WizardMode::Create { parent_path },
            title: String::new(),
            options: WizardOptions::Custom(CommandLaunchOptions::default()),
            error: None,
        }
    }

    /// Build state from an existing command.
    pub(crate) fn from_command(path: NodePath, command: &QuickLaunch) -> Self {
        match &command.spec {
            CommandSpec::Custom { custom } => {
                let mut options = CommandLaunchOptions::default();
                options.set_program(custom.program.clone());
                options.set_working_directory(
                    custom.working_directory().unwrap_or_default().to_string(),
                );
                options.set_args(custom.args.clone());
                options.set_envs(
                    custom
                        .env
                        .iter()
                        .map(|EnvVar { key, value }| {
                            (key.clone(), value.clone())
                        })
                        .collect(),
                );

                Self {
                    mode: WizardMode::Edit { path },
                    title: command.title.clone(),
                    options: WizardOptions::Custom(options),
                    error: None,
                }
            },
            CommandSpec::Ssh { ssh } => {
                let mut options = SshLaunchOptions::default();
                options.set_host(ssh.host.clone());
                options.set_port(ssh.port.to_string());
                options.set_user(ssh.user.clone().unwrap_or_default());
                options.set_identity_file(
                    ssh.identity_file.clone().unwrap_or_default(),
                );
                options.set_extra_args(ssh.extra_args.clone());

                Self {
                    mode: WizardMode::Edit { path },
                    title: command.title.clone(),
                    options: WizardOptions::Ssh(options),
                    error: None,
                }
            },
        }
    }

    pub(crate) fn mode(&self) -> &WizardMode {
        &self.mode
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(crate) fn command_type(&self) -> QuickLaunchType {
        self.options.command_type()
    }

    pub(crate) fn is_create_mode(&self) -> bool {
        matches!(self.mode, WizardMode::Create { .. })
    }

    pub(super) fn set_title(&mut self, value: String) {
        self.title = value;
    }

    pub(super) fn set_error(&mut self, value: String) {
        self.error = Some(value);
    }

    pub(super) fn clear_error(&mut self) {
        self.error = None;
    }

    pub(super) fn set_command_type(&mut self, command_type: QuickLaunchType) {
        if self.command_type() == command_type {
            return;
        }
        self.options = match command_type {
            QuickLaunchType::Custom => {
                WizardOptions::Custom(CommandLaunchOptions::default())
            },
            QuickLaunchType::Ssh => {
                WizardOptions::Ssh(SshLaunchOptions::default())
            },
        };
    }

    pub(crate) fn custom(&self) -> Option<&CommandLaunchOptions> {
        match &self.options {
            WizardOptions::Custom(custom) => Some(custom),
            _ => None,
        }
    }

    pub(crate) fn ssh(&self) -> Option<&SshLaunchOptions> {
        match &self.options {
            WizardOptions::Ssh(ssh) => Some(ssh),
            _ => None,
        }
    }

    pub(super) fn set_program(&mut self, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.set_program(value);
        }
    }

    pub(super) fn set_working_directory(&mut self, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.set_working_directory(value);
        }
    }

    pub(super) fn add_arg(&mut self) {
        if let Some(custom) = self.custom_mut() {
            custom.add_arg();
        }
    }

    pub(super) fn remove_arg(&mut self, index: usize) {
        if let Some(custom) = self.custom_mut() {
            custom.remove_arg(index);
        }
    }

    pub(super) fn update_arg(&mut self, index: usize, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.update_arg(index, value);
        }
    }

    pub(super) fn add_env(&mut self) {
        if let Some(custom) = self.custom_mut() {
            custom.add_env();
        }
    }

    pub(super) fn remove_env(&mut self, index: usize) {
        if let Some(custom) = self.custom_mut() {
            custom.remove_env(index);
        }
    }

    pub(super) fn update_env_key(&mut self, index: usize, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.update_env_key(index, value);
        }
    }

    pub(super) fn update_env_value(&mut self, index: usize, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.update_env_value(index, value);
        }
    }

    pub(super) fn set_host(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_host(value);
        }
    }

    pub(super) fn set_user(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_user(value);
        }
    }

    pub(super) fn set_port(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_port(value);
        }
    }

    pub(super) fn set_identity_file(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_identity_file(value);
        }
    }

    pub(super) fn add_extra_arg(&mut self) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.add_extra_arg();
        }
    }

    pub(super) fn remove_extra_arg(&mut self, index: usize) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.remove_extra_arg(index);
        }
    }

    pub(super) fn update_extra_arg(&mut self, index: usize, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.update_extra_arg(index, value);
        }
    }

    fn custom_mut(&mut self) -> Option<&mut CommandLaunchOptions> {
        match &mut self.options {
            WizardOptions::Custom(custom) => Some(custom),
            _ => None,
        }
    }

    fn ssh_mut(&mut self) -> Option<&mut SshLaunchOptions> {
        match &mut self.options {
            WizardOptions::Ssh(ssh) => Some(ssh),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::quick_launch::constants::SSH_DEFAULT_PORT;

    #[test]
    fn given_default_state_when_created_then_not_dirty() {
        let state = QuickLaunchState::default();
        assert!(!state.is_dirty());
        assert!(!state.is_persist_in_flight());
    }

    #[test]
    fn given_create_editor_when_switching_command_type_then_options_reset() {
        let mut editor = WizardEditorState::new(vec![]);
        editor.set_program(String::from("bash"));
        editor.set_command_type(QuickLaunchType::Ssh);
        assert_eq!(editor.command_type(), QuickLaunchType::Ssh);
        assert!(editor.custom().is_none());
        let ssh = editor.ssh().expect("ssh options should exist");
        assert_eq!(ssh.port(), SSH_DEFAULT_PORT.to_string());
    }
}
