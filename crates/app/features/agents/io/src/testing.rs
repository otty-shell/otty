//! Helpers that the tests of several files share: running a task to its
//! end, writing files, made-up programs to run, and the agents and the
//! providers with the built Anthropic and OpenAI plugins or the built
//! OpenCode plugin of `assets/plugins/`.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;

use iced::Task;
use iced::futures::StreamExt;
use iced::futures::executor::block_on;
use iced_test::runtime::Action;
use iced_test::runtime::task::into_stream;
#[cfg(unix)]
use otty_agents::AgentCommand;
use otty_app_agents::{AgentList, AgentsIntent};
#[cfg(unix)]
use otty_app_testing::temp_dir::TempDir;
use otty_plugin_host::PartRef;

/// The folder of the built Anthropic plugin.
const ANTHROPIC_COPY: &str = "../../../../../assets/plugins/anthropic";
/// The folder of the built OpenAI plugin.
const OPENAI_COPY: &str = "../../../../../assets/plugins/openai";
/// The folder of the built OpenCode plugin.
const OPENCODE_COPY: &str = "../../../../../assets/plugins/opencode";
/// The folder next to the test binary its compiled plugins are kept in,
/// so the plugin is compiled once for all the tests, not in each.
const COMPILED_FOLDER: &str = "otty-agents-io-compiled-plugins";

/// The agents of Agents mode with the built Anthropic and OpenAI plugins
/// installed, loaded as the app loads them: their parts Claude Code and
/// Codex. The plugins see `home` as the home folder and no variable;
/// nothing they start runs, since the shell is not there.
pub(crate) fn agents_with_plugins(home: &std::path::Path) -> AgentList {
    agents_with_plugins_and_shell(home, "/no/such/shell")
}

/// [`agents_with_plugins`] whose plugins start their programs and hidden
/// terminals through the shell `shell`.
pub(crate) fn agents_with_plugins_and_shell(
    home: &std::path::Path,
    shell: &str,
) -> AgentList {
    let parts = [(ANTHROPIC_COPY, "claude-code"), (OPENAI_COPY, "codex")];

    agents_with_built_in(home, &parts, shell)
}

/// The agents of Agents mode with the built OpenCode plugin installed,
/// loaded as the app loads it: its part OpenCode. The plugin sees `home`
/// as the home folder and no variable.
pub(crate) fn agents_with_opencode(home: &std::path::Path) -> AgentList {
    agents_with_built_in(home, &[(OPENCODE_COPY, "opencode")], "/no/such/shell")
}

/// The provider parts of the built Anthropic and OpenAI plugins, loaded
/// as the app loads them. The plugins see `home` as the home
/// folder and no variable.
pub(crate) fn providers_with_plugins(
    home: &std::path::Path,
) -> crate::plugin_parts::ProviderList {
    let parts = [
        (ANTHROPIC_COPY, PartRef::provider("anthropic")),
        (OPENAI_COPY, PartRef::provider("openai")),
    ];

    readers_of(home, &parts, "/no/such/shell").providers
}

/// The agent parts Claude Code and Codex and the provider parts Anthropic
/// and OpenAI of the built plugins, loaded as the app loads them, whose
/// programs and hidden terminals start through the shell `shell`. The
/// plugins see `home` as the home folder and no variable.
#[cfg(unix)]
pub(crate) fn agents_and_providers_with_shell(
    home: &std::path::Path,
    shell: &str,
) -> (AgentList, crate::plugin_parts::ProviderList) {
    let parts = [
        (ANTHROPIC_COPY, PartRef::agent("claude-code")),
        (ANTHROPIC_COPY, PartRef::provider("anthropic")),
        (OPENAI_COPY, PartRef::agent("codex")),
        (OPENAI_COPY, PartRef::provider("openai")),
    ];

    let loaded = readers_of(home, &parts, shell);
    (loaded.readers.agents, loaded.providers)
}

/// The agents with the agent parts of the built plugins: each
/// `(copy, part)` names the folder of a copy and the part turned on in
/// it. The plugins start their programs through `shell`.
fn agents_with_built_in(
    home: &std::path::Path,
    parts: &[(&str, &str)],
    shell: &str,
) -> AgentList {
    let parts: Vec<(&str, PartRef)> = parts
        .iter()
        .map(|(copy, part)| (*copy, PartRef::agent(part)))
        .collect();

    readers_of(home, &parts, shell).readers.agents
}

/// The readers of the parts of the built plugins: each `(copy,
/// part)` names the folder of a copy and the part turned on in it. The
/// plugins start their programs through `shell`.
fn readers_of(
    home: &std::path::Path,
    parts: &[(&str, PartRef)],
    shell: &str,
) -> crate::plugin_readers::LoadedReaders {
    let compiled = std::env::current_exe()
        .ok()
        .and_then(|binary| Some(binary.parent()?.join(COMPILED_FOLDER)))
        .expect("the folder of the test binary");
    let engine = crate::plugin_readers::plugin_engine(&compiled, shell)
        .expect("the engine");
    let mut plugins: Vec<crate::plugin_parts::InstalledParts> = Vec::new();
    for (copy, part) in parts {
        let folder =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(copy);
        match plugins.iter_mut().find(|plugin| plugin.folder == folder) {
            Some(plugin) => plugin.on.push(part.clone()),
            None => plugins.push(crate::plugin_parts::InstalledParts {
                folder,
                on: vec![part.clone()],
            }),
        }
    }

    crate::plugin_readers::readers_now(
        &engine,
        &plugins,
        home.to_path_buf(),
        |_| None,
        &[],
    )
}

/// Run `task` to its end and return the intents it answers with.
pub(crate) fn run(task: Task<AgentsIntent>) -> Vec<AgentsIntent> {
    run_all(task)
}

/// Run `task`, which answers once, to its end and return its answer.
pub(crate) fn run_task<T: Send + 'static>(task: Task<T>) -> T {
    run_all(task).pop().expect("an answer")
}

/// Run `task` to its end and return what it answers with.
fn run_all<T: Send + 'static>(task: Task<T>) -> Vec<T> {
    let stream = into_stream(task).expect("a task");
    let actions: Vec<Action<T>> = block_on(stream.collect());

    actions
        .into_iter()
        .filter_map(|action| match action {
            Action::Output(intent) => Some(intent),
            _ => None,
        })
        .collect()
}

/// Write `text` to the file at `path`, making its folders.
#[cfg(unix)]
pub(crate) fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent"))
        .expect("the folder is made");
    std::fs::write(path, text).expect("the file is written");
}

/// Write an executable `/bin/sh` script with `body` at `name` in `folder`.
///
/// The file is written by a `/bin/sh` child, not by the test process. A
/// file the test process writes is open for writing for a moment, and
/// another test thread that starts a program then may copy that open file
/// into its child; running the script fails with "Text file busy" until
/// that child has started its program. Here no thread of the test process
/// ever holds the file open for writing, so it can always be run.
#[cfg(unix)]
pub(crate) fn executable_script(
    folder: &Path,
    name: &str,
    body: &str,
) -> PathBuf {
    let path = folder.join(name);
    let text = format!("#!/bin/sh\n{body}\n");

    let status = Command::new("/bin/sh")
        .arg("-c")
        .arg("printf '%s' \"$2\" > \"$1\"")
        .arg("sh")
        .arg(&path)
        .arg(&text)
        .status()
        .expect("the script should be written");
    assert!(status.success(), "the script should be written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the script should be executable");

    path
}

/// A made-up shell in `folder`: it checks that it was started as a login
/// and interactive shell and runs the script with `/bin/sh`, so the user's
/// own start files are never read. Each start adds a line to `starts` in
/// the folder, which [`starts`] counts.
#[cfg(unix)]
pub(crate) fn fake_shell(folder: &TempDir) -> String {
    let starts = folder.path().join("starts");
    let body = format!(
        "echo started >> '{}'\n\
         [ \"$1\" = -l ] && [ \"$2\" = -i ] && [ \"$3\" = -c ] || exit 2\n\
         exec /bin/sh -c \"$4\"",
        starts.display()
    );

    executable_script(folder.path(), "fake-shell", &body)
        .display()
        .to_string()
}

/// How many times the [`fake_shell`] of `folder` was started.
#[cfg(unix)]
pub(crate) fn starts(folder: &TempDir) -> usize {
    std::fs::read_to_string(folder.path().join("starts"))
        .map_or(0, |text| text.lines().count())
}

/// A made-up agent program in `folder` that runs `body`.
#[cfg(unix)]
pub(crate) fn agent(folder: &TempDir, body: &str) -> AgentCommand {
    let path = executable_script(folder.path(), "agent", body);

    AgentCommand::new(path.display().to_string())
}

/// Whether a process with `pid` still exists.
#[cfg(unix)]
pub(crate) fn is_running(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
