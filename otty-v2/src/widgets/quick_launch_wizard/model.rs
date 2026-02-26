use super::errors::QuickLaunchWizardError;
use super::state::QuickLaunchWizardEditorState;
use crate::widgets::quick_launch::{
    CommandSpec, CustomCommand, EnvVar, QuickLaunch, QuickLaunchType,
    SSH_DEFAULT_PORT, SshCommand,
};

/// Build a domain quick launch command from editor draft state.
pub(crate) fn build_command(
    editor: &QuickLaunchWizardEditorState,
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

                    Some(EnvVar {
                        key: key.to_string(),
                        value: value.clone(),
                    })
                })
                .collect::<Vec<_>>();

            let working_directory = custom.working_directory().trim();

            CommandSpec::Custom {
                custom: CustomCommand {
                    program: program.to_string(),
                    args: custom.args().to_vec(),
                    env,
                    working_directory: if working_directory.is_empty() {
                        None
                    } else {
                        Some(working_directory.to_string())
                    },
                },
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
                ssh: SshCommand {
                    host: host.to_string(),
                    port,
                    user: optional_string(ssh.user()),
                    identity_file: optional_string(ssh.identity_file()),
                    extra_args: ssh.extra_args().to_vec(),
                },
            }
        },
    };

    Ok(QuickLaunch {
        title: title.to_string(),
        spec,
    })
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
    use super::super::state::QuickLaunchWizardEditorState;
    use super::*;

    #[test]
    fn given_empty_title_when_building_command_then_returns_title_required() {
        let editor = QuickLaunchWizardEditorState::new_create(vec![]);

        let result = build_command(&editor);

        assert!(matches!(result, Err(QuickLaunchWizardError::TitleRequired)));
    }

    #[test]
    fn given_custom_editor_when_building_command_then_returns_custom_launch() {
        let mut editor = QuickLaunchWizardEditorState::new_create(vec![]);
        editor.set_title(String::from("Build"));
        editor.set_program(String::from("cargo"));
        editor.add_arg();
        editor.update_arg(0, String::from("check"));
        editor.add_env();
        editor.update_env_key(0, String::from("RUST_LOG"));
        editor.update_env_value(0, String::from("debug"));
        editor.set_working_directory(String::from("/tmp/project"));

        let quick_launch =
            build_command(&editor).expect("build should succeed");

        assert_eq!(quick_launch.title, "Build");
        let CommandSpec::Custom { custom } = quick_launch.spec else {
            panic!("expected custom command");
        };
        assert_eq!(custom.program, "cargo");
        assert_eq!(custom.args, vec![String::from("check")]);
        assert_eq!(custom.env.len(), 1);
        assert_eq!(custom.env[0].key, "RUST_LOG");
        assert_eq!(custom.env[0].value, "debug");
        assert_eq!(
            custom.working_directory,
            Some(String::from("/tmp/project")),
        );
    }

    #[test]
    fn given_invalid_ssh_port_when_building_command_then_returns_error() {
        let command = QuickLaunch {
            title: String::from("SSH"),
            spec: CommandSpec::Ssh {
                ssh: SshCommand {
                    host: String::from("example.com"),
                    port: SSH_DEFAULT_PORT,
                    user: None,
                    identity_file: None,
                    extra_args: Vec::new(),
                },
            },
        };
        let mut editor = QuickLaunchWizardEditorState::from_command(
            vec![String::from("SSH")],
            &command,
        );
        editor.set_port(String::from("not-a-port"));

        let result = build_command(&editor);

        assert!(matches!(result, Err(QuickLaunchWizardError::InvalidPort)));
    }
}
