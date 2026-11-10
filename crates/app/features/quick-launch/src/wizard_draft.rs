//! The launch editor's draft turned into a launch: the typed fields are
//! trimmed and checked, and empty optional fields become `None`.

use otty_app_domain::quick_launch::{
    CommandSpec, CustomCommand, EnvVar, QuickLaunch, SshCommand,
};

use crate::constants::SSH_DEFAULT_PORT;
use crate::errors::QuickLaunchWizardError;
use crate::state::WizardEditorState;
use crate::types::QuickLaunchType;

/// Build a domain quick launch command from editor draft state.
pub(crate) fn build_command(
    editor: &WizardEditorState,
) -> Result<QuickLaunch, QuickLaunchWizardError> {
    let title = editor.title().trim();
    if title.is_empty() {
        return Err(QuickLaunchWizardError::TitleRequired);
    }

    let spec = match editor.command_type() {
        QuickLaunchType::Custom => {
            let Some(custom) = editor.custom() else {
                return Err(QuickLaunchWizardError::MissingCustomDraft);
            };
            let program = custom.program().trim();
            if program.is_empty() {
                return Err(QuickLaunchWizardError::ProgramRequired);
            }

            let env = custom
                .env()
                .iter()
                .filter_map(|(key, value)| {
                    let key = key.trim();
                    if key.is_empty() {
                        return None;
                    }
                    Some(EnvVar::new(key.to_string(), value.clone()))
                })
                .collect::<Vec<_>>();

            let working_directory = custom.working_directory().trim();

            CommandSpec::Custom {
                custom: CustomCommand::new(
                    program.to_string(),
                    custom.args().to_vec(),
                    env,
                    if working_directory.is_empty() {
                        None
                    } else {
                        Some(working_directory.to_string())
                    },
                ),
            }
        },
        QuickLaunchType::Ssh => {
            let Some(ssh) = editor.ssh() else {
                return Err(QuickLaunchWizardError::MissingSshDraft);
            };
            let host = ssh.host().trim();
            if host.is_empty() {
                return Err(QuickLaunchWizardError::HostRequired);
            }

            let port = ssh.port().trim();
            let port = if port.is_empty() {
                SSH_DEFAULT_PORT
            } else {
                port.parse::<u16>()
                    .map_err(|_| QuickLaunchWizardError::InvalidPort)?
            };

            CommandSpec::Ssh {
                ssh: SshCommand::new(
                    host.to_string(),
                    port,
                    optional_string(ssh.user()),
                    optional_string(ssh.identity_file()),
                    ssh.extra_args().to_vec(),
                ),
            }
        },
    };

    Ok(QuickLaunch::new(
        title.to_string(),
        spec,
        editor.exit_behavior(),
    ))
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::exit_behavior::ExitBehavior;

    use super::*;

    #[test]
    fn given_empty_title_when_building_command_then_returns_title_required() {
        let editor = WizardEditorState::new(vec![]);
        let result = build_command(&editor);
        assert!(matches!(result, Err(QuickLaunchWizardError::TitleRequired)));
    }

    #[test]
    fn given_custom_editor_when_building_command_then_returns_custom_launch() {
        let mut editor = WizardEditorState::new(vec![]);
        editor.set_title(String::from("Build"));
        editor.set_program(String::from("cargo"));
        editor.set_arg_input(String::from("check"));
        editor.submit_arg();
        editor.add_env();
        editor.update_env_key(0, String::from("RUST_LOG"));
        editor.update_env_value(0, String::from("debug"));
        editor.set_working_directory(String::from("/tmp/project"));

        let quick_launch =
            build_command(&editor).expect("build should succeed");
        assert_eq!(quick_launch.title(), "Build");
        let CommandSpec::Custom { custom } = quick_launch.spec() else {
            panic!("expected custom command");
        };
        assert_eq!(custom.program(), "cargo");
        assert_eq!(custom.args(), &[String::from("check")]);
        assert_eq!(custom.env().len(), 1);
        assert_eq!(custom.env()[0].key(), "RUST_LOG");
        assert_eq!(custom.env()[0].value(), "debug");
        assert_eq!(custom.working_directory(), Some("/tmp/project"));
    }

    #[test]
    fn given_chosen_exit_behavior_when_building_command_then_launch_keeps_it() {
        let mut editor = WizardEditorState::new(vec![]);
        editor.set_title(String::from("List"));
        editor.set_program(String::from("ls"));
        editor.set_exit_behavior(ExitBehavior::KeepOpen);

        let quick_launch =
            build_command(&editor).expect("build should succeed");

        assert_eq!(quick_launch.exit_behavior(), ExitBehavior::KeepOpen);
    }

    #[test]
    fn given_invalid_ssh_port_when_building_command_then_returns_error() {
        let mut editor = WizardEditorState::new(vec![]);
        editor.set_title(String::from("SSH"));
        editor.set_command_type(QuickLaunchType::Ssh);
        editor.set_host(String::from("example.com"));
        editor.set_port(String::from("not-a-port"));

        let result = build_command(&editor);
        assert!(matches!(result, Err(QuickLaunchWizardError::InvalidPort)));
    }
}
