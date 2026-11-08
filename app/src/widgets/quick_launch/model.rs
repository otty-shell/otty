use std::collections::HashMap;

use iced::Point;
use iced::widget::Id;

use super::state::{InlineEditState, QuickLaunchState};
use super::types::{
    CommandLaunchOptions, ContextMenuTarget, DropTarget, InlineEditKind,
    LaunchInfo, NodePath, QuickLaunch, QuickLaunchFile, QuickLaunchFolder,
    QuickLaunchNode, QuickLaunchType, SshLaunchOptions, SshSecretKind,
};
use crate::domain::exit_behavior::ExitBehavior;
use crate::i18n::{self, Key};

/// Read-only view model for the quick launch tree.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchTreeViewModel<'a> {
    pub(super) data: &'a QuickLaunchFile,
    pub(super) selected_path: Option<&'a NodePath>,
    pub(super) hovered_path: Option<&'a NodePath>,
    pub(super) inline_edit: Option<InlineEditViewModel<'a>>,
    pub(super) launching: &'a HashMap<NodePath, LaunchInfo>,
    pub(super) drop_target: Option<&'a DropTarget>,
}

impl<'a> QuickLaunchTreeViewModel<'a> {
    /// Build the view model from the quick launch state.
    pub(super) fn from_state(state: &'a QuickLaunchState) -> Self {
        Self {
            data: state.data(),
            selected_path: state.selected_path(),
            hovered_path: state.hovered_path(),
            inline_edit: state
                .inline_edit()
                .map(InlineEditViewModel::from_state),
            launching: state.launching(),
            drop_target: state.drop_target(),
        }
    }

    /// Every launch in the tree, folders left out, in tree order (a
    /// folder's launches come where the folder is).
    pub(crate) fn launches(&self) -> Vec<LaunchViewModel<'a>> {
        let mut launches = Vec::new();

        collect_launches(self.data.root(), &[], &mut launches);

        launches
    }

    /// The launches directly at the top of the tree, folders and what is
    /// inside them left out, in tree order.
    pub(crate) fn top_level_launches(&self) -> Vec<LaunchViewModel<'a>> {
        self.data
            .root()
            .children()
            .iter()
            .filter_map(|node| match node {
                QuickLaunchNode::Command(command) => {
                    Some(launch(command, vec![command.title().to_string()]))
                },
                QuickLaunchNode::Folder(_) => None,
            })
            .collect()
    }
}

#[cfg(test)]
impl<'a> QuickLaunchTreeViewModel<'a> {
    /// Tree of `data` that no one is using: nothing selected, hovered,
    /// renamed or dragged over. `launching` holds the launches that are
    /// starting.
    fn idle(
        data: &'a QuickLaunchFile,
        launching: &'a HashMap<NodePath, LaunchInfo>,
    ) -> Self {
        Self {
            data,
            selected_path: None,
            hovered_path: None,
            inline_edit: None,
            launching,
            drop_target: None,
        }
    }
}

/// One launch of the tree, for lists outside the tree such as the command
/// palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchViewModel<'a> {
    /// Path in the tree: the folder titles, then the launch title.
    pub(crate) path: NodePath,
    pub(crate) title: &'a str,
    /// The launch connects over SSH.
    pub(crate) is_ssh: bool,
}

/// The name input in the tree: a new folder's name or a new name for a
/// launch or folder.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InlineEditViewModel<'a> {
    /// Whether the input names a new folder or renames a node, and where.
    pub(super) kind: &'a InlineEditKind,
    /// The name typed so far.
    pub(super) value: &'a str,
    /// Why the typed name can not be used, shown under the input.
    pub(super) error: Option<&'a str>,
    /// Widget id of the input, used to focus it.
    pub(super) id: &'a Id,
}

impl<'a> InlineEditViewModel<'a> {
    /// Build the name input from the inline edit in progress.
    pub(super) fn from_state(edit: &'a InlineEditState) -> Self {
        Self {
            kind: &edit.kind,
            value: &edit.value,
            error: edit.error.as_deref(),
            id: &edit.id,
        }
    }
}

/// The context menu of the tree, opened by a right click.
#[derive(Debug, Clone)]
pub(crate) struct ContextMenuViewModel {
    /// What was right-clicked: a launch, a folder or the empty area.
    pub(super) target: ContextMenuTarget,
    /// Where the right click was, in window coordinates.
    pub(super) cursor: Point,
    /// The right-clicked launch is running, so the menu offers Kill.
    pub(super) is_running: bool,
}

impl ContextMenuViewModel {
    /// Build the open context menu; `None` while it is closed.
    pub(super) fn from_state(state: &QuickLaunchState) -> Option<Self> {
        let menu = state.context_menu()?;
        let target = menu.target();

        let is_running = match &target {
            ContextMenuTarget::Command(path) => state.is_launching(path),
            _ => false,
        };

        Some(Self {
            target,
            cursor: menu.cursor(),
            is_running,
        })
    }
}

/// The tab that shows why a launch failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ErrorTabViewModel<'a> {
    pub(super) title: &'a str,
    /// The error detail, shown in the terminal font.
    pub(super) message: &'a str,
}

impl<'a> ErrorTabViewModel<'a> {
    /// Build the error tab `tab_id`; `None` when the tab has no saved error.
    pub(super) fn from_state(
        state: &'a QuickLaunchState,
        tab_id: u64,
    ) -> Option<Self> {
        let error = state.error_tab(tab_id)?;

        Some(Self {
            title: error.title(),
            message: error.message(),
        })
    }
}

/// The dialog that asks for an SSH key passphrase or password.
///
/// It has no `Debug`: the typed secret must never reach logs.
#[derive(Clone)]
pub(crate) struct SecretPromptViewModel<'a> {
    /// Names the key file or the account the secret unlocks.
    pub(super) title: String,
    /// Says which launch asks and which account it logs in as.
    pub(super) hint: String,
    /// The label of the secret field.
    pub(super) label: &'static str,
    /// Shown under the field after a wrong secret.
    pub(super) error: Option<&'static str>,
    /// The secret typed so far.
    pub(super) value: &'a str,
    /// Widget id of the secret field, used to focus it.
    pub(super) input_id: &'a Id,
}

impl<'a> SecretPromptViewModel<'a> {
    /// Build the open dialog; `None` while no launch asks for a secret.
    pub(super) fn from_state(state: &'a QuickLaunchState) -> Option<Self> {
        let prompt = state.secret_prompt()?;
        let request = prompt.request();
        let account = prompt.account();

        let (title, hint, label, retry_error) = match request.kind() {
            SshSecretKind::KeyPassphrase => (
                i18n::secret_prompt_passphrase_title(request.target()),
                i18n::secret_prompt_passphrase_hint(
                    prompt.launch_title(),
                    &account,
                ),
                i18n::t(Key::SecretPromptPassphraseLabel),
                i18n::t(Key::SecretPromptWrongPassphrase),
            ),
            SshSecretKind::Password => (
                i18n::secret_prompt_password_title(request.target()),
                i18n::secret_prompt_password_hint(
                    prompt.launch_title(),
                    &account,
                ),
                i18n::t(Key::SecretPromptPasswordLabel),
                i18n::t(Key::SecretPromptPasswordRejected),
            ),
        };

        Some(Self {
            title,
            hint,
            label,
            error: request.is_retry().then_some(retry_error),
            value: prompt.value(),
            input_id: prompt.input_id(),
        })
    }
}

/// The form that creates a launch or edits a saved one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WizardFormViewModel<'a> {
    pub(super) title: &'a str,
    /// The form creates a new launch; otherwise it edits a saved one.
    pub(super) is_create_mode: bool,
    pub(super) command_type: QuickLaunchType,
    /// The fields of a local command; `None` for an SSH launch.
    pub(super) custom: Option<&'a CommandLaunchOptions>,
    /// The fields of an SSH connection; `None` for a local command.
    pub(super) ssh: Option<&'a SshLaunchOptions>,
    pub(super) exit_behavior: ExitBehavior,
    /// Why the last save failed.
    pub(super) error: Option<&'a str>,
}

impl<'a> WizardFormViewModel<'a> {
    /// Build the form of the launch editor in tab `tab_id`; `None` when the
    /// tab has no editor.
    pub(super) fn from_state(
        state: &'a QuickLaunchState,
        tab_id: u64,
    ) -> Option<Self> {
        let editor = state.wizard().editor(tab_id)?;

        Some(Self {
            title: editor.title(),
            is_create_mode: editor.is_create_mode(),
            command_type: editor.command_type(),
            custom: editor.custom(),
            ssh: editor.ssh(),
            exit_behavior: editor.exit_behavior(),
            error: editor.error(),
        })
    }
}

/// Add the launches of `folder` (at tree path `folder_path`) and of every
/// folder below it to `launches`, in tree order.
fn collect_launches<'a>(
    folder: &'a QuickLaunchFolder,
    folder_path: &[String],
    launches: &mut Vec<LaunchViewModel<'a>>,
) {
    for node in folder.children() {
        let mut path = folder_path.to_vec();
        path.push(node.title().to_string());

        match node {
            QuickLaunchNode::Folder(child) => {
                collect_launches(child, &path, launches);
            },
            QuickLaunchNode::Command(command) => {
                launches.push(launch(command, path));
            },
        }
    }
}

/// The launch of `command`, which sits at `path` in the tree.
fn launch(command: &QuickLaunch, path: NodePath) -> LaunchViewModel<'_> {
    LaunchViewModel {
        path,
        title: command.title(),
        is_ssh: command.is_ssh(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::widgets::quick_launch::state::{
        ContextMenuState, QuickLaunchErrorState, SecretPromptState,
    };
    use crate::widgets::quick_launch::types::{
        CommandSpec, CustomCommand, SshCommand, SshSecretRequest,
    };

    const TAB_ID: u64 = 7;

    fn command(title: &str) -> QuickLaunchNode {
        QuickLaunchNode::Command(QuickLaunch {
            title: String::from(title),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::from("htop"),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: None,
                },
            },
            exit_behavior: ExitBehavior::Close,
        })
    }

    fn ssh(title: &str) -> QuickLaunchNode {
        QuickLaunchNode::Command(QuickLaunch {
            title: String::from(title),
            spec: CommandSpec::Ssh {
                ssh: SshCommand {
                    host: String::from("example.com"),
                    port: 22,
                    user: None,
                    identity_file: None,
                    extra_args: Vec::new(),
                },
            },
            exit_behavior: ExitBehavior::Close,
        })
    }

    fn folder(title: &str, children: Vec<QuickLaunchNode>) -> QuickLaunchNode {
        QuickLaunchNode::Folder(QuickLaunchFolder {
            title: String::from(title),
            expanded: false,
            children,
        })
    }

    fn path(segments: &[&str]) -> NodePath {
        segments
            .iter()
            .map(|segment| String::from(*segment))
            .collect()
    }

    #[test]
    fn launches_walk_the_tree_in_order_and_skip_folders() {
        let mut data = QuickLaunchFile::default();
        data.root.children = vec![
            command("htop"),
            folder(
                "servers",
                vec![ssh("laptop"), folder("prod", vec![ssh("storage")])],
            ),
            folder("empty", Vec::new()),
            command("logs"),
        ];
        let launching = HashMap::new();
        let vm = QuickLaunchTreeViewModel {
            data: &data,
            selected_path: None,
            hovered_path: None,
            inline_edit: None,
            launching: &launching,
            drop_target: None,
        };

        let launches = vm.launches();

        assert_eq!(
            launches,
            vec![
                LaunchViewModel {
                    path: path(&["htop"]),
                    title: "htop",
                    is_ssh: false,
                },
                LaunchViewModel {
                    path: path(&["servers", "laptop"]),
                    title: "laptop",
                    is_ssh: true,
                },
                LaunchViewModel {
                    path: path(&["servers", "prod", "storage"]),
                    title: "storage",
                    is_ssh: true,
                },
                LaunchViewModel {
                    path: path(&["logs"]),
                    title: "logs",
                    is_ssh: false,
                },
            ]
        );
    }

    #[test]
    fn top_level_launches_leave_out_folders_and_what_is_inside_them() {
        let mut data = QuickLaunchFile::default();
        data.root.children = vec![
            command("htop"),
            folder("servers", vec![ssh("laptop")]),
            ssh("router"),
        ];
        let launching = HashMap::new();
        let vm = QuickLaunchTreeViewModel::idle(&data, &launching);

        let launches = vm.top_level_launches();

        assert_eq!(
            launches,
            vec![
                LaunchViewModel {
                    path: path(&["htop"]),
                    title: "htop",
                    is_ssh: false,
                },
                LaunchViewModel {
                    path: path(&["router"]),
                    title: "router",
                    is_ssh: true,
                },
            ]
        );
    }

    fn ssh_launch(title: &str, user: Option<&str>) -> QuickLaunch {
        QuickLaunch {
            title: String::from(title),
            spec: CommandSpec::Ssh {
                ssh: SshCommand {
                    host: String::from("192.168.1.20"),
                    port: 22,
                    user: user.map(String::from),
                    identity_file: None,
                    extra_args: Vec::new(),
                },
            },
            exit_behavior: ExitBehavior::KeepOpen,
        }
    }

    fn state_with_menu(target: ContextMenuTarget) -> QuickLaunchState {
        let mut state = QuickLaunchState::default();
        state.set_context_menu(Some(
            ContextMenuState::default()
                .with_target(target)
                .with_cursor(Point::new(40.0, 30.0)),
        ));

        state
    }

    /// The state of a launch that asks for a secret of `kind` for `target`.
    fn state_with_prompt(
        kind: SshSecretKind,
        target: &str,
        is_retry: bool,
    ) -> QuickLaunchState {
        let mut state = QuickLaunchState::default();
        state.open_secret_prompt(SecretPromptState::new(
            path(&["laptop"]),
            Box::new(ssh_launch("laptop", Some("ubuntu"))),
            SshSecretRequest::new(kind, target.to_string(), is_retry),
        ));

        state
    }

    #[test]
    fn given_rename_in_progress_when_tree_built_then_it_shows_the_typed_name_and_error()
     {
        let mut state = QuickLaunchState::default();
        let id = Id::unique();
        state.set_inline_edit(Some(InlineEditState {
            kind: InlineEditKind::Rename {
                path: path(&["htop"]),
            },
            value: String::from("top"),
            error: Some(String::from("taken")),
            id: id.clone(),
        }));

        let vm = QuickLaunchTreeViewModel::from_state(&state);

        let edit = vm.inline_edit.expect("the rename input should show");
        assert!(matches!(
            edit.kind,
            InlineEditKind::Rename { path: renamed } if renamed == &path(&["htop"])
        ));
        assert_eq!(edit.value, "top");
        assert_eq!(edit.error, Some("taken"));
        assert_eq!(edit.id, &id);
    }

    #[test]
    fn given_no_inline_edit_when_tree_built_then_no_input_shows() {
        let state = QuickLaunchState::default();

        let vm = QuickLaunchTreeViewModel::from_state(&state);

        assert!(vm.inline_edit.is_none());
    }

    #[test]
    fn given_closed_menu_when_context_menu_built_then_there_is_none() {
        let state = QuickLaunchState::default();

        assert!(ContextMenuViewModel::from_state(&state).is_none());
    }

    #[test]
    fn given_menu_of_running_launch_when_built_then_it_offers_kill() {
        let mut state =
            state_with_menu(ContextMenuTarget::Command(path(&["build"])));
        state.begin_launch(path(&["build"]), Arc::default());

        let vm = ContextMenuViewModel::from_state(&state)
            .expect("the menu should be open");

        assert!(vm.is_running);
        assert_eq!(vm.cursor, Point::new(40.0, 30.0));
        assert!(matches!(
            vm.target,
            ContextMenuTarget::Command(ref target) if target == &path(&["build"])
        ));
    }

    #[test]
    fn given_menu_of_idle_launch_when_built_then_it_is_not_running() {
        let mut state =
            state_with_menu(ContextMenuTarget::Command(path(&["build"])));
        state.begin_launch(path(&["deploy"]), Arc::default());

        let vm = ContextMenuViewModel::from_state(&state)
            .expect("the menu should be open");

        assert!(!vm.is_running);
    }

    #[test]
    fn given_menu_of_folder_when_built_then_it_is_not_running() {
        let mut state =
            state_with_menu(ContextMenuTarget::Folder(path(&["servers"])));
        state.begin_launch(path(&["servers"]), Arc::default());

        let vm = ContextMenuViewModel::from_state(&state)
            .expect("the menu should be open");

        assert!(!vm.is_running);
        assert!(matches!(vm.target, ContextMenuTarget::Folder(_)));
    }

    #[test]
    fn given_error_tab_when_built_then_it_shows_its_title_and_message() {
        let mut state = QuickLaunchState::default();
        state.set_error_tab(
            TAB_ID,
            QuickLaunchErrorState::new(
                String::from("Launch failed"),
                String::from("htop: not found"),
            ),
        );

        let vm = ErrorTabViewModel::from_state(&state, TAB_ID);

        assert_eq!(
            vm,
            Some(ErrorTabViewModel {
                title: "Launch failed",
                message: "htop: not found",
            })
        );
    }

    #[test]
    fn given_tab_without_error_when_error_tab_built_then_there_is_none() {
        let state = QuickLaunchState::default();

        assert_eq!(ErrorTabViewModel::from_state(&state, TAB_ID), None);
    }

    #[test]
    fn given_no_prompt_when_secret_prompt_built_then_there_is_none() {
        let state = QuickLaunchState::default();

        assert!(SecretPromptViewModel::from_state(&state).is_none());
    }

    #[test]
    fn given_key_passphrase_prompt_when_built_then_texts_name_the_key_file() {
        let state = state_with_prompt(
            SshSecretKind::KeyPassphrase,
            "~/.ssh/id_ed25519",
            false,
        );

        let vm = SecretPromptViewModel::from_state(&state)
            .expect("the prompt should be open");

        assert_eq!(
            vm.title,
            i18n::secret_prompt_passphrase_title("~/.ssh/id_ed25519")
        );
        assert_eq!(
            vm.hint,
            i18n::secret_prompt_passphrase_hint(
                "laptop",
                "ubuntu@192.168.1.20"
            )
        );
        assert_eq!(vm.label, i18n::t(Key::SecretPromptPassphraseLabel));
        assert_eq!(vm.error, None);
    }

    #[test]
    fn given_password_prompt_when_built_then_texts_name_the_account() {
        let state = state_with_prompt(
            SshSecretKind::Password,
            "ubuntu@192.168.1.20:22",
            false,
        );

        let vm = SecretPromptViewModel::from_state(&state)
            .expect("the prompt should be open");

        assert_eq!(
            vm.title,
            i18n::secret_prompt_password_title("ubuntu@192.168.1.20:22")
        );
        assert_eq!(
            vm.hint,
            i18n::secret_prompt_password_hint("laptop", "ubuntu@192.168.1.20")
        );
        assert_eq!(vm.label, i18n::t(Key::SecretPromptPasswordLabel));
        assert_eq!(vm.error, None);
    }

    #[test]
    fn given_retry_when_prompt_built_then_it_says_the_last_secret_was_wrong() {
        let passphrase =
            state_with_prompt(SshSecretKind::KeyPassphrase, "key", true);
        let password =
            state_with_prompt(SshSecretKind::Password, "u@h:22", true);

        let passphrase_vm = SecretPromptViewModel::from_state(&passphrase)
            .expect("the prompt should be open");
        let password_vm = SecretPromptViewModel::from_state(&password)
            .expect("the prompt should be open");

        assert_eq!(
            passphrase_vm.error,
            Some(i18n::t(Key::SecretPromptWrongPassphrase))
        );
        assert_eq!(
            password_vm.error,
            Some(i18n::t(Key::SecretPromptPasswordRejected))
        );
    }

    #[test]
    fn given_typed_secret_when_prompt_built_then_the_field_shows_it() {
        let mut state =
            state_with_prompt(SshSecretKind::Password, "u@h:22", false);
        state
            .secret_prompt_mut()
            .expect("the prompt should be open")
            .set_value(String::from("hunter2"));

        let vm = SecretPromptViewModel::from_state(&state)
            .expect("the prompt should be open");

        let prompt = state.secret_prompt().expect("the prompt should be open");
        assert_eq!(vm.value, "hunter2");
        assert_eq!(vm.input_id, prompt.input_id());
    }

    #[test]
    fn given_tab_without_editor_when_form_built_then_there_is_none() {
        let state = QuickLaunchState::default();

        assert!(WizardFormViewModel::from_state(&state, TAB_ID).is_none());
    }

    #[test]
    fn given_new_launch_editor_when_form_built_then_it_is_an_empty_custom_launch()
     {
        let mut state = QuickLaunchState::default();
        state
            .wizard_mut()
            .initialize_create(TAB_ID, path(&["servers"]));

        let vm = WizardFormViewModel::from_state(&state, TAB_ID)
            .expect("the tab should have an editor");

        assert!(vm.is_create_mode);
        assert_eq!(vm.title, "");
        assert_eq!(vm.command_type, QuickLaunchType::Custom);
        assert_eq!(vm.custom.map(|custom| custom.program()), Some(""));
        assert!(vm.ssh.is_none());
        assert_eq!(vm.exit_behavior, ExitBehavior::default());
        assert_eq!(vm.error, None);
    }

    #[test]
    fn given_saved_ssh_launch_when_form_built_then_it_edits_that_launch() {
        let mut state = QuickLaunchState::default();
        let launch = ssh_launch("laptop", Some("ubuntu"));
        state
            .wizard_mut()
            .initialize_edit(TAB_ID, path(&["laptop"]), &launch);

        let vm = WizardFormViewModel::from_state(&state, TAB_ID)
            .expect("the tab should have an editor");

        assert!(!vm.is_create_mode);
        assert_eq!(vm.title, "laptop");
        assert_eq!(vm.command_type, QuickLaunchType::Ssh);
        assert_eq!(vm.ssh.map(|ssh| ssh.host()), Some("192.168.1.20"));
        assert_eq!(vm.ssh.map(|ssh| ssh.user()), Some("ubuntu"));
        assert!(vm.custom.is_none());
        assert_eq!(vm.exit_behavior, ExitBehavior::KeepOpen);
    }

    #[test]
    fn given_failed_save_when_form_built_then_it_shows_the_error() {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(TAB_ID, Vec::new());
        state
            .wizard_mut()
            .editor_mut(TAB_ID)
            .expect("the tab should have an editor")
            .set_error(String::from("Title is required"));

        let vm = WizardFormViewModel::from_state(&state, TAB_ID)
            .expect("the tab should have an editor");

        assert_eq!(vm.error, Some("Title is required"));
    }
}
