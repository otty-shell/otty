use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Point;

use super::model::{
    CommandSpec, EnvVar, LaunchInfo, NodePath, QuickLaunch, QuickLaunchFile,
    QuickLaunchType, SSH_DEFAULT_PORT,
};

// ---------------------------------------------------------------------------
// Quick Launch Tree State
// ---------------------------------------------------------------------------

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
    /// Create state from pre-loaded data.
    pub(crate) fn with_data(data: QuickLaunchFile) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

    // --- Data access ---

    pub(crate) fn data(&self) -> &QuickLaunchFile {
        &self.data
    }

    pub(crate) fn data_mut(&mut self) -> &mut QuickLaunchFile {
        &mut self.data
    }

    // --- Error tabs ---

    pub(crate) fn error_tab(
        &self,
        tab_id: u64,
    ) -> Option<&QuickLaunchErrorState> {
        self.error_tabs.get(&tab_id)
    }

    pub(crate) fn set_error_tab(
        &mut self,
        tab_id: u64,
        state: QuickLaunchErrorState,
    ) {
        self.error_tabs.insert(tab_id, state);
    }

    pub(crate) fn remove_error_tab(&mut self, tab_id: u64) {
        self.error_tabs.remove(&tab_id);
    }

    // --- Dirty / persist ---

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn is_persist_in_flight(&self) -> bool {
        self.persist_in_flight
    }

    pub(crate) fn begin_persist(&mut self) {
        self.persist_in_flight = true;
    }

    pub(crate) fn complete_persist(&mut self) {
        self.persist_in_flight = false;
        self.dirty = false;
    }

    pub(crate) fn fail_persist(&mut self) {
        self.persist_in_flight = false;
    }

    // --- Selection / hover ---

    pub(crate) fn selected_path(&self) -> Option<&NodePath> {
        self.selected_path.as_ref()
    }

    pub(crate) fn selected_path_cloned(&self) -> Option<NodePath> {
        self.selected_path.clone()
    }

    pub(crate) fn set_selected_path(&mut self, path: Option<NodePath>) {
        self.selected_path = path;
    }

    pub(crate) fn clear_selected_path(&mut self) {
        self.selected_path = None;
    }

    pub(crate) fn hovered_path(&self) -> Option<&NodePath> {
        self.hovered_path.as_ref()
    }

    pub(crate) fn set_hovered_path(&mut self, path: Option<NodePath>) {
        self.hovered_path = path;
    }

    // --- Pressed ---

    pub(crate) fn pressed_path(&self) -> Option<&NodePath> {
        self.pressed_path.as_ref()
    }

    pub(crate) fn set_pressed_path(&mut self, path: Option<NodePath>) {
        self.pressed_path = path;
    }

    pub(crate) fn clear_pressed_path(&mut self) {
        self.pressed_path = None;
    }

    // --- Cursor ---

    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }

    pub(crate) fn set_cursor(&mut self, position: Point) {
        self.cursor = position;
    }

    // --- Context menu ---

    pub(crate) fn context_menu(&self) -> Option<&ContextMenuState> {
        self.context_menu.as_ref()
    }

    pub(crate) fn context_menu_cloned(&self) -> Option<ContextMenuState> {
        self.context_menu.clone()
    }

    pub(crate) fn set_context_menu(&mut self, menu: Option<ContextMenuState>) {
        self.context_menu = menu;
    }

    pub(crate) fn clear_context_menu(&mut self) {
        self.context_menu = None;
    }

    // --- Inline edit ---

    pub(crate) fn inline_edit(&self) -> Option<&InlineEditState> {
        self.inline_edit.as_ref()
    }

    pub(crate) fn inline_edit_mut(&mut self) -> Option<&mut InlineEditState> {
        self.inline_edit.as_mut()
    }

    pub(crate) fn set_inline_edit(&mut self, edit: Option<InlineEditState>) {
        self.inline_edit = edit;
    }

    pub(crate) fn clear_inline_edit(&mut self) {
        self.inline_edit = None;
    }

    pub(crate) fn take_inline_edit(&mut self) -> Option<InlineEditState> {
        self.inline_edit.take()
    }

    // --- Drag & drop ---

    pub(crate) fn drag(&self) -> Option<&DragState> {
        self.drag.as_ref()
    }

    pub(crate) fn drag_mut(&mut self) -> Option<&mut DragState> {
        self.drag.as_mut()
    }

    pub(crate) fn set_drag(&mut self, drag: Option<DragState>) {
        self.drag = drag;
    }

    pub(crate) fn take_drag(&mut self) -> Option<DragState> {
        self.drag.take()
    }

    pub(crate) fn clear_drag(&mut self) {
        self.drag = None;
    }

    pub(crate) fn drop_target(&self) -> Option<&DropTarget> {
        self.drop_target.as_ref()
    }

    pub(crate) fn set_drop_target(&mut self, target: Option<DropTarget>) {
        self.drop_target = target;
    }

    pub(crate) fn take_drop_target(&mut self) -> Option<DropTarget> {
        self.drop_target.take()
    }

    pub(crate) fn clear_drop_target(&mut self) {
        self.drop_target = None;
    }

    // --- Launching ---

    pub(crate) fn launching(&self) -> &HashMap<NodePath, LaunchInfo> {
        &self.launching
    }

    pub(crate) fn launching_mut(
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

    pub(crate) fn begin_launch(
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

    pub(crate) fn remove_launch(
        &mut self,
        path: &[String],
    ) -> Option<LaunchInfo> {
        self.launching.remove(path)
    }

    pub(crate) fn cancel_launch(&mut self, path: &[String]) {
        if let Some(info) = self.launching.get(path) {
            info.cancel.store(true, Ordering::Relaxed);
            self.canceled_launches.push(info.id);
        }
    }

    pub(crate) fn take_canceled_launch(&mut self, launch_id: u64) -> bool {
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

    // --- Blink ---

    pub(crate) fn blink_nonce(&self) -> u64 {
        self.blink_nonce
    }

    pub(crate) fn advance_blink_nonce(&mut self) {
        self.blink_nonce = self.blink_nonce.wrapping_add(1);
    }

    #[cfg(test)]
    pub(crate) fn set_blink_nonce_for_tests(&mut self, value: u64) {
        self.blink_nonce = value;
    }

    // --- Wizard ---

    pub(crate) fn wizard(&self) -> &WizardState {
        &self.wizard
    }

    pub(crate) fn wizard_mut(&mut self) -> &mut WizardState {
        &mut self.wizard
    }
}

// ---------------------------------------------------------------------------
// Error tab state
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Context menu state
// ---------------------------------------------------------------------------

/// Active context menu state.
#[derive(Debug, Clone)]
pub(crate) struct ContextMenuState {
    pub(crate) target: super::model::ContextMenuTarget,
    pub(crate) cursor: Point,
}

// ---------------------------------------------------------------------------
// Inline edit state
// ---------------------------------------------------------------------------

/// Kind of inline edit operation.
#[derive(Debug, Clone)]
pub(crate) enum InlineEditKind {
    CreateFolder { parent_path: NodePath },
    Rename { path: NodePath },
}

/// Active inline edit state.
#[derive(Debug, Clone)]
pub(crate) struct InlineEditState {
    pub(crate) kind: InlineEditKind,
    pub(crate) value: String,
    pub(crate) error: Option<String>,
    pub(crate) id: iced::widget::Id,
}

// ---------------------------------------------------------------------------
// Drag & drop state
// ---------------------------------------------------------------------------

/// Active drag state.
#[derive(Debug, Clone)]
pub(crate) struct DragState {
    pub(crate) source: NodePath,
    pub(crate) origin: Point,
    pub(crate) active: bool,
}

/// Drop target for a drag operation.
#[derive(Debug, Clone)]
pub(crate) enum DropTarget {
    Root,
    Folder(NodePath),
}

// ---------------------------------------------------------------------------
// Wizard state (merged from quick_launch_wizard)
// ---------------------------------------------------------------------------

/// Wizard editor states keyed by tab id.
#[derive(Debug, Default, Clone)]
pub(crate) struct WizardState {
    editors: HashMap<u64, WizardEditorState>,
}

impl WizardState {
    pub(crate) fn editor(&self, tab_id: u64) -> Option<&WizardEditorState> {
        self.editors.get(&tab_id)
    }

    pub(crate) fn editor_mut(
        &mut self,
        tab_id: u64,
    ) -> Option<&mut WizardEditorState> {
        self.editors.get_mut(&tab_id)
    }

    pub(crate) fn initialize_create(&mut self, tab_id: u64, parent: NodePath) {
        self.editors
            .insert(tab_id, WizardEditorState::new_create(parent));
    }

    pub(crate) fn initialize_edit(
        &mut self,
        tab_id: u64,
        path: NodePath,
        command: &QuickLaunch,
    ) {
        self.editors
            .insert(tab_id, WizardEditorState::from_command(path, command));
    }

    pub(crate) fn remove_tab(&mut self, tab_id: u64) {
        self.editors.remove(&tab_id);
    }

    #[cfg(test)]
    pub(crate) fn set_editor(
        &mut self,
        tab_id: u64,
        editor: WizardEditorState,
    ) {
        self.editors.insert(tab_id, editor);
    }
}

/// Editor mode.
#[derive(Debug, Clone)]
pub(crate) enum WizardMode {
    Create { parent_path: NodePath },
    Edit { path: NodePath },
}

/// Local command editor options.
#[derive(Debug, Clone, Default)]
pub(crate) struct CommandLaunchOptions {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    working_directory: String,
}

impl CommandLaunchOptions {
    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }

    pub(crate) fn env(&self) -> &[(String, String)] {
        &self.env
    }

    pub(crate) fn working_directory(&self) -> &str {
        &self.working_directory
    }

    pub(crate) fn set_program(&mut self, value: String) {
        self.program = value;
    }

    pub(crate) fn set_working_directory(&mut self, value: String) {
        self.working_directory = value;
    }

    pub(crate) fn add_arg(&mut self) {
        self.args.push(String::new());
    }

    pub(crate) fn remove_arg(&mut self, index: usize) {
        if index < self.args.len() {
            self.args.remove(index);
        }
    }

    pub(crate) fn update_arg(&mut self, index: usize, value: String) {
        if let Some(arg) = self.args.get_mut(index) {
            *arg = value;
        }
    }

    pub(crate) fn add_env(&mut self) {
        self.env.push((String::new(), String::new()));
    }

    pub(crate) fn remove_env(&mut self, index: usize) {
        if index < self.env.len() {
            self.env.remove(index);
        }
    }

    pub(crate) fn update_env_key(&mut self, index: usize, value: String) {
        if let Some(pair) = self.env.get_mut(index) {
            pair.0 = value;
        }
    }

    pub(crate) fn update_env_value(&mut self, index: usize, value: String) {
        if let Some(pair) = self.env.get_mut(index) {
            pair.1 = value;
        }
    }
}

/// SSH command editor options.
#[derive(Debug, Clone)]
pub(crate) struct SshLaunchOptions {
    host: String,
    port: String,
    user: String,
    identity_file: String,
    extra_args: Vec<String>,
}

impl Default for SshLaunchOptions {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: SSH_DEFAULT_PORT.to_string(),
            user: String::new(),
            identity_file: String::new(),
            extra_args: Vec::new(),
        }
    }
}

impl SshLaunchOptions {
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) fn port(&self) -> &str {
        &self.port
    }

    pub(crate) fn user(&self) -> &str {
        &self.user
    }

    pub(crate) fn identity_file(&self) -> &str {
        &self.identity_file
    }

    pub(crate) fn extra_args(&self) -> &[String] {
        &self.extra_args
    }

    pub(crate) fn set_host(&mut self, value: String) {
        self.host = value;
    }

    pub(crate) fn set_port(&mut self, value: String) {
        self.port = value;
    }

    pub(crate) fn set_user(&mut self, value: String) {
        self.user = value;
    }

    pub(crate) fn set_identity_file(&mut self, value: String) {
        self.identity_file = value;
    }

    pub(crate) fn add_extra_arg(&mut self) {
        self.extra_args.push(String::new());
    }

    pub(crate) fn remove_extra_arg(&mut self, index: usize) {
        if index < self.extra_args.len() {
            self.extra_args.remove(index);
        }
    }

    pub(crate) fn update_extra_arg(&mut self, index: usize, value: String) {
        if let Some(arg) = self.extra_args.get_mut(index) {
            *arg = value;
        }
    }
}

/// Wizard editor options variant.
#[derive(Debug, Clone)]
enum WizardOptions {
    Custom(CommandLaunchOptions),
    Ssh(SshLaunchOptions),
}

impl WizardOptions {
    fn command_type(&self) -> QuickLaunchType {
        match self {
            Self::Custom(_) => QuickLaunchType::Custom,
            Self::Ssh(_) => QuickLaunchType::Ssh,
        }
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
    pub(crate) fn new_create(parent_path: NodePath) -> Self {
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
            CommandSpec::Custom { custom } => Self {
                mode: WizardMode::Edit { path },
                title: command.title.clone(),
                options: WizardOptions::Custom(CommandLaunchOptions {
                    program: custom.program.clone(),
                    args: custom.args.clone(),
                    env: custom
                        .env
                        .iter()
                        .map(|EnvVar { key, value }| {
                            (key.clone(), value.clone())
                        })
                        .collect(),
                    working_directory: custom
                        .working_directory
                        .clone()
                        .unwrap_or_default(),
                }),
                error: None,
            },
            CommandSpec::Ssh { ssh } => Self {
                mode: WizardMode::Edit { path },
                title: command.title.clone(),
                options: WizardOptions::Ssh(SshLaunchOptions {
                    host: ssh.host.clone(),
                    port: ssh.port.to_string(),
                    user: ssh.user.clone().unwrap_or_default(),
                    identity_file: ssh
                        .identity_file
                        .clone()
                        .unwrap_or_default(),
                    extra_args: ssh.extra_args.clone(),
                }),
                error: None,
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

    pub(crate) fn set_title(&mut self, value: String) {
        self.title = value;
    }

    pub(crate) fn set_error(&mut self, value: String) {
        self.error = Some(value);
    }

    pub(crate) fn clear_error(&mut self) {
        self.error = None;
    }

    pub(crate) fn set_command_type(&mut self, command_type: QuickLaunchType) {
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

    // --- Delegate field mutations ---

    pub(crate) fn set_program(&mut self, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.set_program(value);
        }
    }

    pub(crate) fn set_working_directory(&mut self, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.set_working_directory(value);
        }
    }

    pub(crate) fn add_arg(&mut self) {
        if let Some(custom) = self.custom_mut() {
            custom.add_arg();
        }
    }

    pub(crate) fn remove_arg(&mut self, index: usize) {
        if let Some(custom) = self.custom_mut() {
            custom.remove_arg(index);
        }
    }

    pub(crate) fn update_arg(&mut self, index: usize, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.update_arg(index, value);
        }
    }

    pub(crate) fn add_env(&mut self) {
        if let Some(custom) = self.custom_mut() {
            custom.add_env();
        }
    }

    pub(crate) fn remove_env(&mut self, index: usize) {
        if let Some(custom) = self.custom_mut() {
            custom.remove_env(index);
        }
    }

    pub(crate) fn update_env_key(&mut self, index: usize, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.update_env_key(index, value);
        }
    }

    pub(crate) fn update_env_value(&mut self, index: usize, value: String) {
        if let Some(custom) = self.custom_mut() {
            custom.update_env_value(index, value);
        }
    }

    pub(crate) fn set_host(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_host(value);
        }
    }

    pub(crate) fn set_user(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_user(value);
        }
    }

    pub(crate) fn set_port(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_port(value);
        }
    }

    pub(crate) fn set_identity_file(&mut self, value: String) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.set_identity_file(value);
        }
    }

    pub(crate) fn add_extra_arg(&mut self) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.add_extra_arg();
        }
    }

    pub(crate) fn remove_extra_arg(&mut self, index: usize) {
        if let Some(ssh) = self.ssh_mut() {
            ssh.remove_extra_arg(index);
        }
    }

    pub(crate) fn update_extra_arg(&mut self, index: usize, value: String) {
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

    #[test]
    fn given_default_state_when_created_then_not_dirty() {
        let state = QuickLaunchState::default();
        assert!(!state.is_dirty());
        assert!(!state.is_persist_in_flight());
    }

    #[test]
    fn given_create_editor_when_switching_command_type_then_options_reset() {
        let mut editor = WizardEditorState::new_create(vec![]);
        editor.set_program(String::from("bash"));
        editor.set_command_type(QuickLaunchType::Ssh);
        assert_eq!(editor.command_type(), QuickLaunchType::Ssh);
        assert!(editor.custom().is_none());
        let ssh = editor.ssh().expect("ssh options should exist");
        assert_eq!(ssh.port(), SSH_DEFAULT_PORT.to_string());
    }
}
