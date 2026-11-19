//! The version of an agent's program as it prints it for `--version`, run
//! through the user's login shell the way the agent tab starts it. The
//! feature asks for it only while Agents mode is on screen and once per
//! agent in a run, for profiles whose session files name no version.

use iced::Task;
use otty_agents::{AgentId, agent_of, version_in_output};
use otty_app_agents::{AgentList, AgentsIntent, LoadedVersion};
use otty_app_shell::quote_word;

use crate::background::on_own_thread;
use crate::login_shell::{SHELL_TIMEOUT, ask_login_shell};

/// Ask the program of `agent` for its version in the background, through
/// the user's shell at `shell`. The answer comes back as
/// [`AgentsIntent::VersionLoaded`], with no version when the program
/// fails, prints none, takes too long or the agent is not known.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
///
/// use otty_agents::AgentId;
/// use otty_app_agents::AgentList;
///
/// let agents: AgentList = Arc::from(otty_agents::known_agents());
/// let read = otty_app_agents_io::load_version(
///     &agents,
///     AgentId::new("codex"),
///     "/bin/zsh",
/// );
///
/// assert_eq!(read.units(), 1);
/// ```
pub fn load_version(
    agents: &AgentList,
    agent: AgentId,
    shell: &str,
) -> Task<AgentsIntent> {
    let program = agent_of(agents, agent)
        .map(|known| known.descriptor().program().to_string());
    let shell = shell.to_string();

    on_own_thread(
        format!("asking {} for its version", agent.as_str()),
        move || program.and_then(|program| program_version(&shell, &program)),
        || None,
        move |version| {
            AgentsIntent::VersionLoaded(LoadedVersion { agent, version })
        },
    )
}

/// The version `program` prints for `--version`, run through the user's
/// login shell at `shell`. It only prints and ends: it reads no login and
/// starts no session.
fn program_version(shell: &str, program: &str) -> Option<String> {
    let script = format!("{} --version", quote_word(program));

    let output = ask_login_shell(shell, &script, SHELL_TIMEOUT)?;

    version_in_output(&String::from_utf8_lossy(&output))
}

#[cfg(all(test, unix))]
mod tests {
    use otty_app_testing::temp_dir::TempDir;

    use super::*;
    use crate::testing::{executable_script, fake_shell, run, starts};

    #[test]
    fn the_version_is_what_the_program_prints_for_version() {
        let folder = TempDir::new("version-prints");
        let shell = fake_shell(&folder);
        let program = executable_script(
            folder.path(),
            "made-up-agent",
            "[ \"$1\" = --version ] && echo 'made-up-agent 1.2.3'",
        );

        let version = program_version(&shell, &program.display().to_string());

        assert_eq!(version, Some(String::from("1.2.3")));
        assert_eq!(starts(&folder), 1);
    }

    #[test]
    fn a_program_that_fails_or_prints_no_version_has_none() {
        let folder = TempDir::new("version-none");
        let shell = fake_shell(&folder);
        let failing = executable_script(folder.path(), "failing", "exit 1");
        let silent = executable_script(folder.path(), "silent", "echo hello");

        assert_eq!(
            program_version(&shell, &failing.display().to_string()),
            None
        );
        assert_eq!(
            program_version(&shell, &silent.display().to_string()),
            None
        );
    }

    #[test]
    fn the_answer_names_the_agent_asked() {
        let folder = TempDir::new("version-answer");
        let shell = executable_script(
            folder.path(),
            "prints-a-version",
            "echo 'codex-cli 0.169.0'",
        );
        let agents: AgentList =
            std::sync::Arc::from(otty_agents::known_agents());

        let intents = run(load_version(
            &agents,
            AgentId::new("codex"),
            &shell.display().to_string(),
        ));

        assert_eq!(
            intents,
            vec![AgentsIntent::VersionLoaded(LoadedVersion {
                agent: AgentId::new("codex"),
                version: Some(String::from("0.169.0")),
            })]
        );
    }
}
