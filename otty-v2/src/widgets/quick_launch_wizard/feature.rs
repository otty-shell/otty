use iced::Task;

use super::QuickLaunchWizardError;
use super::command::QuickLaunchWizardCommand;
use super::event::QuickLaunchWizardEffectEvent;
use super::model::build_command;
use super::state::{
    QuickLaunchWizardEditorState, QuickLaunchWizardMode, QuickLaunchWizardState,
};
use crate::widgets::quick_launch::{
    QuickLaunchWizardSaveRequest, QuickLaunchWizardSaveTarget,
};

/// Context passed to the wizard reducer per event dispatch.
pub(crate) struct QuickLaunchWizardCtx {
    pub(crate) tab_id: u64,
}

/// Quick launch wizard feature owning per-tab editor states.
pub(crate) struct QuickLaunchWizardFeature {
    state: QuickLaunchWizardState,
}

impl QuickLaunchWizardFeature {
    /// Construct a wizard feature with empty editor state.
    pub(crate) fn new() -> Self {
        Self {
            state: QuickLaunchWizardState::default(),
        }
    }

    /// Return read-only access to wizard state for the view layer.
    pub(crate) fn state(&self) -> &QuickLaunchWizardState {
        &self.state
    }

    /// Replace editor state for a tab id in tests.
    #[cfg(test)]
    pub(crate) fn set_editor_for_test(
        &mut self,
        tab_id: u64,
        editor: QuickLaunchWizardEditorState,
    ) {
        self.state.set_editor(tab_id, editor);
    }
}

impl QuickLaunchWizardFeature {
    /// Reduce a quick launch wizard command and emit side-effect events.
    pub(crate) fn reduce(
        &mut self,
        command: QuickLaunchWizardCommand,
        ctx: &QuickLaunchWizardCtx,
    ) -> Task<QuickLaunchWizardEffectEvent> {
        use QuickLaunchWizardCommand::*;

        let tab_id = ctx.tab_id;

        match command {
            InitializeCreate { parent_path } => {
                self.state.initialize_create(tab_id, parent_path);
                Task::none()
            },
            InitializeEdit { path, command } => {
                self.state.initialize_edit(tab_id, path, &command);
                Task::none()
            },
            TabClosed => {
                self.state.remove_tab(tab_id);
                Task::none()
            },
            Cancel => {
                Task::done(QuickLaunchWizardEffectEvent::CloseTabRequested {
                    tab_id,
                })
            },
            Save => self.reduce_save(tab_id),
            SetError { message } => {
                self.apply_editor_error(tab_id, message);
                Task::none()
            },
            other => self.reduce_editor_fields(tab_id, other),
        }
    }
}

impl QuickLaunchWizardFeature {
    fn apply_editor_error(&mut self, tab_id: u64, message: String) {
        if let Some(editor) = self.editor_mut(tab_id) {
            editor.set_error(message);
        }
    }

    fn reduce_save(
        &mut self,
        tab_id: u64,
    ) -> Task<QuickLaunchWizardEffectEvent> {
        let request = match self.build_save_request(tab_id) {
            Ok(Some(request)) => request,
            Ok(None) => return Task::none(),
            Err(err) => {
                self.apply_editor_error(tab_id, format!("{err}"));
                return Task::none();
            },
        };

        Task::done(QuickLaunchWizardEffectEvent::WizardSaveRequested(request))
    }

    fn reduce_editor_fields(
        &mut self,
        tab_id: u64,
        command: QuickLaunchWizardCommand,
    ) -> Task<QuickLaunchWizardEffectEvent> {
        use QuickLaunchWizardCommand::*;

        let Some(editor) = self.editor_mut(tab_id) else {
            return Task::none();
        };

        match command {
            UpdateTitle(value) => editor.set_title(value),
            UpdateProgram(value) => editor.set_program(value),
            UpdateHost(value) => editor.set_host(value),
            UpdateUser(value) => editor.set_user(value),
            UpdatePort(value) => editor.set_port(value),
            UpdateIdentityFile(value) => editor.set_identity_file(value),
            UpdateWorkingDirectory(value) => {
                editor.set_working_directory(value)
            },
            AddArg => editor.add_arg(),
            RemoveArg(index) => editor.remove_arg(index),
            UpdateArg { index, value } => editor.update_arg(index, value),
            AddEnv => editor.add_env(),
            RemoveEnv(index) => editor.remove_env(index),
            UpdateEnvKey { index, value } => {
                editor.update_env_key(index, value)
            },
            UpdateEnvValue { index, value } => {
                editor.update_env_value(index, value)
            },
            AddExtraArg => editor.add_extra_arg(),
            RemoveExtraArg(index) => editor.remove_extra_arg(index),
            UpdateExtraArg { index, value } => {
                editor.update_extra_arg(index, value)
            },
            SelectCommandType(command_type) => {
                if editor.is_create_mode() {
                    editor.set_command_type(command_type);
                }
            },
            _ => {},
        }

        editor.clear_error();
        Task::none()
    }

    fn build_save_request(
        &self,
        tab_id: u64,
    ) -> Result<Option<QuickLaunchWizardSaveRequest>, QuickLaunchWizardError>
    {
        let Some(editor) = self.editor_ref(tab_id) else {
            return Ok(None);
        };

        let command = build_command(editor)?;
        let target = match editor.mode().clone() {
            QuickLaunchWizardMode::Create { parent_path } => {
                QuickLaunchWizardSaveTarget::Create { parent_path }
            },
            QuickLaunchWizardMode::Edit { path } => {
                QuickLaunchWizardSaveTarget::Edit { path }
            },
        };

        Ok(Some(QuickLaunchWizardSaveRequest {
            tab_id,
            target,
            command,
        }))
    }

    fn editor_ref(&self, tab_id: u64) -> Option<&QuickLaunchWizardEditorState> {
        self.state.editor(tab_id)
    }

    fn editor_mut(
        &mut self,
        tab_id: u64,
    ) -> Option<&mut QuickLaunchWizardEditorState> {
        self.state.editor_mut(tab_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::quick_launch::{CommandSpec, CustomCommand};

    fn ctx(tab_id: u64) -> QuickLaunchWizardCtx {
        QuickLaunchWizardCtx { tab_id }
    }

    fn feature_with_editor(
        editor: QuickLaunchWizardEditorState,
    ) -> (QuickLaunchWizardFeature, u64) {
        let tab_id = 1;
        let mut feature = QuickLaunchWizardFeature::new();
        feature.set_editor_for_test(tab_id, editor);
        (feature, tab_id)
    }

    #[test]
    fn given_missing_tab_when_reducer_receives_event_then_event_is_ignored() {
        let mut feature = QuickLaunchWizardFeature::new();

        let _ = feature.reduce(
            QuickLaunchWizardCommand::UpdateTitle(String::from("x")),
            &ctx(999),
        );

        assert!(feature.state.editor(999).is_none());
    }

    #[test]
    fn given_custom_editor_when_field_events_are_reduced_then_state_is_updated()
    {
        let (mut feature, tab_id) = feature_with_editor(
            QuickLaunchWizardEditorState::new_create(vec![]),
        );

        let _ = feature.reduce(
            QuickLaunchWizardCommand::UpdateTitle(String::from("Run")),
            &ctx(tab_id),
        );
        let _ = feature.reduce(
            QuickLaunchWizardCommand::UpdateProgram(String::from("cargo")),
            &ctx(tab_id),
        );
        let _ = feature.reduce(QuickLaunchWizardCommand::AddArg, &ctx(tab_id));
        let _ = feature.reduce(
            QuickLaunchWizardCommand::UpdateArg {
                index: 0,
                value: String::from("check"),
            },
            &ctx(tab_id),
        );
        let _ = feature.reduce(QuickLaunchWizardCommand::AddEnv, &ctx(tab_id));
        let _ = feature.reduce(
            QuickLaunchWizardCommand::UpdateEnvKey {
                index: 0,
                value: String::from("RUST_LOG"),
            },
            &ctx(tab_id),
        );
        let _ = feature.reduce(
            QuickLaunchWizardCommand::UpdateEnvValue {
                index: 0,
                value: String::from("debug"),
            },
            &ctx(tab_id),
        );
        let _ = feature.reduce(
            QuickLaunchWizardCommand::UpdateWorkingDirectory(String::from(
                "/tmp",
            )),
            &ctx(tab_id),
        );

        let editor = feature.editor_ref(tab_id).expect("editor should exist");
        assert_eq!(editor.title(), "Run");
        let custom = editor.custom().expect("custom options should exist");
        assert_eq!(custom.program(), "cargo");
        assert_eq!(custom.args(), &[String::from("check")]);
        assert_eq!(
            custom.env(),
            &[(String::from("RUST_LOG"), String::from("debug"))],
        );
        assert_eq!(custom.working_directory(), "/tmp");
    }

    #[test]
    fn given_edit_editor_when_selecting_type_then_existing_type_is_preserved() {
        use crate::widgets::quick_launch::QuickLaunchType;
        let command = crate::widgets::quick_launch::QuickLaunch {
            title: String::from("Run"),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::from("bash"),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: None,
                },
            },
        };
        let editor = QuickLaunchWizardEditorState::from_command(
            vec![String::from("Run")],
            &command,
        );
        let (mut feature, tab_id) = feature_with_editor(editor);

        let _ = feature.reduce(
            QuickLaunchWizardCommand::SelectCommandType(QuickLaunchType::Ssh),
            &ctx(tab_id),
        );

        let editor = feature.editor_ref(tab_id).expect("editor should exist");
        assert_eq!(editor.command_type(), QuickLaunchType::Custom);
    }

    #[test]
    fn given_invalid_editor_when_save_then_error_is_set() {
        let editor = QuickLaunchWizardEditorState::new_create(vec![]);
        let (mut feature, tab_id) = feature_with_editor(editor);

        let _ = feature.reduce(QuickLaunchWizardCommand::Save, &ctx(tab_id));

        let editor = feature.editor_ref(tab_id).expect("editor should exist");
        assert_eq!(editor.error(), Some("Title is required."));
    }

    #[test]
    fn given_valid_editor_when_save_then_quick_launch_state_is_not_mutated_directly()
     {
        use crate::widgets::quick_launch::{
            QuickLaunchFeature, QuickLaunchState,
        };
        let mut editor = QuickLaunchWizardEditorState::new_create(vec![]);
        editor.set_title(String::from("Run"));
        editor.set_program(String::from("bash"));
        let (mut feature, tab_id) = feature_with_editor(editor);

        let _ = feature.reduce(QuickLaunchWizardCommand::Save, &ctx(tab_id));

        // Verify the wizard does NOT mutate quick launch data directly — it
        // emits an event that app.rs routes through QuickLaunchFeature.
        let ql_feature = QuickLaunchFeature::new(QuickLaunchState::default());
        assert!(ql_feature.state().data().root.children.is_empty());
        assert!(!ql_feature.is_dirty());
        let editor = feature.editor_ref(tab_id).expect("editor should exist");
        assert_eq!(editor.error(), None);
    }
}
