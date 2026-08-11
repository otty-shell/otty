#![cfg(unix)]

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

use otty_libterm::escape::{
    Action, EscapeActor, EscapeParser, Parser, ProtocolEvent, ProtocolEventKind,
};
use otty_libterm::pty::{self, Session, SessionError};

struct ShellOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
}

#[derive(Default)]
struct ProtocolActor {
    events: Vec<ProtocolEvent>,
}

impl EscapeActor for ProtocolActor {
    fn handle(&mut self, action: Action) {
        if let Action::ProtocolEvent(event) = action {
            self.events.push(event);
        }
    }
}

fn integration_asset(file_name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/shell-integrations")
        .join(file_name)
}

fn parse_events(bytes: &[u8]) -> Vec<ProtocolEvent> {
    let mut parser = Parser::<otty_libterm::escape::vte::Parser>::new();
    let mut actor = ProtocolActor::default();
    parser.advance(bytes, &mut actor);
    actor.events
}

fn assert_empty_enter_completion(events: &[ProtocolEvent]) {
    let completed_empty_block =
        events
            .iter()
            .enumerate()
            .find_map(|(completion_index, event)| match event.kind() {
                ProtocolEventKind::CommandEnd {
                    block_id,
                    exit_code: Some(0),
                    ..
                } => {
                    let was_prepared =
                        events[..completion_index].iter().any(|candidate| {
                            matches!(
                                candidate.kind(),
                                ProtocolEventKind::PromptPrepare {
                                    block_id: prepared_id,
                                    ..
                                } if prepared_id == block_id
                            )
                        });
                    let was_started = events.iter().any(|candidate| {
                        matches!(
                            candidate.kind(),
                            ProtocolEventKind::CommandStart {
                                block_id: started_id,
                                ..
                            } if started_id == block_id
                        )
                    });
                    let next_prompt_exists = events[completion_index + 1..]
                        .iter()
                        .any(|candidate| {
                            matches!(
                                candidate.kind(),
                                ProtocolEventKind::PromptPrepare {
                                    block_id: next_id,
                                    ..
                                } if next_id != block_id
                            )
                        });

                    (was_prepared && !was_started && next_prompt_exists)
                        .then(|| block_id.clone())
                },
                _ => None,
            });

    assert!(
        completed_empty_block.is_some(),
        "an empty Enter should complete its own prepared block: {events:#?}",
    );
}

fn run_shell(shell: &str, args: &[&str], script: &str) -> ShellOutput {
    let integration = integration_asset(&format!("otty.{shell}"));
    let integration = integration
        .to_str()
        .expect("integration path must be valid UTF-8");
    let mut builder = pty::local(shell);
    for arg in args {
        builder = builder.with_arg(arg);
    }
    let mut session = builder
        .with_arg(script)
        .with_env("OTTY_TERMINAL_SESSION_ID", "test-session")
        .with_env("OTTY_TEST_INTEGRATION", integration)
        .set_controling_tty_enable()
        .spawn()
        .expect("shell should start through PTY");
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut stdout = Vec::new();
    let mut buffer = [0u8; 4096];
    let mut status = None;
    let mut pty_closed = false;

    while status.is_none() || !pty_closed {
        match session.read(&mut buffer) {
            Ok(0) => {},
            Ok(count) => stdout.extend_from_slice(&buffer[..count]),
            Err(SessionError::IO(error))
                if error.kind() == ErrorKind::WouldBlock => {},
            Err(SessionError::IO(error))
                if error.kind() == ErrorKind::Interrupted => {},
            Err(SessionError::IO(error)) if error.raw_os_error() == Some(5) => {
                pty_closed = true;
            },
            Err(error) => panic!("failed to read shell PTY: {error}"),
        }

        if status.is_none() {
            status = session
                .try_get_child_exit_status()
                .expect("shell exit status should be readable");
        }
        if Instant::now() >= deadline {
            panic!("shell PTY timed out after 15 seconds");
        }
        if status.is_none() || !pty_closed {
            thread::sleep(Duration::from_millis(1));
        }
    }

    ShellOutput {
        status: status.expect("shell should report an exit status"),
        stdout,
    }
}

#[test]
fn shell_harness_uses_a_real_pty() {
    let output = run_shell(
        "bash",
        &["--noprofile", "--norc", "-i", "-c"],
        r#"
if [[ -t 0 && -t 1 && -t 2 ]]; then
    printf OTTY_PTY_CONFIRMED
    exit 0
fi
exit 91
"#,
    );

    assert!(
        output.status.success()
            && output
                .stdout
                .windows(b"OTTY_PTY_CONFIRMED".len())
                .any(|window| window == b"OTTY_PTY_CONFIRMED"),
        "shell integration harness must execute through a PTY",
    );
}

#[test]
fn legacy_fixture_reproduces_nested_loss_and_collision_ids() {
    let output = run_shell(
        "bash",
        &["--noprofile", "--norc", "-i", "-c"],
        r#"
otty_block_seq=0
legacy_id() {
    otty_block_seq=$((otty_block_seq + 1))
    printf 'LEGACY_ID=cmd-%s\n' "$otty_block_seq"
}
legacy_id
bash --noprofile --norc -i -c '
otty_block_seq=0
otty_block_seq=$((otty_block_seq + 1))
printf "LEGACY_ID=cmd-%s\n" "$otty_block_seq"
if ! declare -F _otty_preexec >/dev/null; then
    printf "LEGACY_NESTED_INTEGRATION_LOST\n"
fi
'
exit 0
"#,
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let ids = stdout
        .lines()
        .filter_map(|line| {
            line.trim_end_matches('\r').strip_prefix("LEGACY_ID=")
        })
        .collect::<Vec<_>>();

    assert!(output.status.success(), "legacy PTY output: {stdout}");
    assert_eq!(ids, ["cmd-1", "cmd-1"]);
    assert!(stdout.contains("LEGACY_NESTED_INTEGRATION_LOST"));
}

#[test]
fn bash_v2_reports_exact_command_completion_and_is_idempotent() {
    let output = run_shell(
        "bash",
        &["--noprofile", "--norc", "-i", "-c"],
        r#"
existing_prompt_hook() { :; }
PROMPT_COMMAND=existing_prompt_hook
trap 'printf EXISTING_EXIT_HOOK' EXIT
source "$OTTY_TEST_INTEGRATION"
source "$OTTY_TEST_INTEGRATION"
[[ $PROMPT_COMMAND == *existing_prompt_hook* ]] && printf EXISTING_PROMPT_HOOK
_otty_precmd
_otty_preexec "printf ok"
(exit 7)
_otty_precmd
_otty_preexec "false | true"
BP_PIPESTATUS=(1 0)
(exit 0)
_otty_precmd
exit 0
"#,
    );
    assert!(
        output.status.success(),
        "bash PTY output: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output
            .stdout
            .windows(b"EXISTING_PROMPT_HOOK".len())
            .any(|window| { window == b"EXISTING_PROMPT_HOOK" }),
        "existing PROMPT_COMMAND should remain chained",
    );
    assert!(
        output
            .stdout
            .windows(b"EXISTING_EXIT_HOOK".len())
            .any(|window| { window == b"EXISTING_EXIT_HOOK" }),
        "existing EXIT trap should remain chained",
    );

    let events = parse_events(&output.stdout);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event.kind(),
                ProtocolEventKind::ShellHello { .. }
            ))
            .count(),
        1,
    );
    let start = events
        .iter()
        .find_map(|event| match event.kind() {
            ProtocolEventKind::CommandStart {
                block_id, command, ..
            } if command.as_deref() == Some("printf ok") => {
                Some((block_id.clone(), event.shell_instance_id().to_string()))
            },
            _ => None,
        })
        .expect("command_start event");
    assert!(events.iter().any(|event| matches!(
        event.kind(),
        ProtocolEventKind::CommandEnd {
            block_id,
            exit_code: Some(7),
            ..
        } if block_id == &start.0
    )));
    assert!(events.iter().any(|event| matches!(
        event.kind(),
        ProtocolEventKind::CommandEnd {
            pipe_status,
            ..
        } if pipe_status == &[1, 0]
    )));
    assert!(events.windows(2).all(|pair| {
        pair[0].shell_instance_id() != pair[1].shell_instance_id()
            || pair[0].sequence() < pair[1].sequence()
    }));
}

#[test]
fn bash_v2_reports_empty_enter_as_own_completion() {
    let output = run_shell(
        "bash",
        &["--noprofile", "--norc", "-i", "-c"],
        r#"
source "$OTTY_TEST_INTEGRATION"
trap - DEBUG
PROMPT_COMMAND=
_otty_active_block_id=
_otty_prepared_block_id=
_otty_precmd
_otty_precmd
exit 0
"#,
    );
    assert!(
        output.status.success(),
        "bash PTY output: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let events = parse_events(&output.stdout);

    assert_empty_enter_completion(&events);
}

#[test]
fn bash_v2_prompt_markers_preserve_fedora_prompt_expansion() {
    let output = run_shell(
        "bash",
        &["--noprofile", "--norc", "-i", "-c"],
        r#"
PROMPT_START=
PROMPT_END=
PS1='${PROMPT_START@P}user$ ${PROMPT_END@P}'
source "$OTTY_TEST_INTEGRATION"
printf '\nOTTY_EXPANDED_PROMPT=<%s>\n' "${PS1@P}"
exit 0
"#,
    );
    assert!(
        output.status.success(),
        "bash PTY output: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let expanded_prompt = stdout
        .lines()
        .find(|line| line.contains("OTTY_EXPANDED_PROMPT="))
        .expect("expanded prompt marker should be printed");
    assert!(
        expanded_prompt.contains("user$ "),
        "expanded prompt should retain visible content: {expanded_prompt:?}",
    );
    assert!(
        expanded_prompt.contains("\u{1b}]133;A\u{1b}\\")
            && expanded_prompt.contains("\u{1b}]133;B\u{1b}\\"),
        "expanded prompt should retain OSC 133 boundaries: {expanded_prompt:?}",
    );
    assert!(
        !expanded_prompt.contains("PROMPT_START@P")
            && !expanded_prompt.contains("PROMPT_END@P"),
        "prompt parameter syntax must not become visible: {expanded_prompt:?}",
    );
}

#[test]
fn nested_bash_uses_unique_shell_and_block_ids() {
    let output = run_shell(
        "bash",
        &["--noprofile", "--norc", "-i", "-c"],
        r#"
source "$OTTY_TEST_INTEGRATION"
_otty_precmd
_otty_preexec "bash"
bash --noprofile --norc -i -c 'source "$OTTY_TEST_INTEGRATION"; _otty_precmd; _otty_preexec "true"; true; _otty_precmd; exit 0'
_otty_precmd
exit 0
"#,
    );
    assert!(
        output.status.success(),
        "bash PTY output: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let events = parse_events(&output.stdout);
    let shell_ids = events
        .iter()
        .filter(|event| {
            matches!(event.kind(), ProtocolEventKind::ShellHello { .. })
        })
        .map(|event| event.shell_instance_id().to_string())
        .collect::<std::collections::HashSet<_>>();
    let block_ids = events
        .iter()
        .filter_map(|event| match event.kind() {
            ProtocolEventKind::PromptPrepare { block_id, .. } => {
                Some(block_id.clone())
            },
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(shell_ids.len(), 2);
    assert_eq!(
        block_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        block_ids.len(),
    );
}

#[test]
fn zsh_v2_reports_completion_when_zsh_is_available() {
    if Command::new("zsh").arg("--version").output().is_err() {
        return;
    }

    let output = run_shell(
        "zsh",
        &["-d", "-f", "-i", "-c"],
        r#"
source "$OTTY_TEST_INTEGRATION"
source "$OTTY_TEST_INTEGRATION"
_otty_precmd
_otty_preexec "false"
false
_otty_precmd
exit 0
"#,
    );
    assert!(
        output.status.success(),
        "zsh PTY output: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let events = parse_events(&output.stdout);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event.kind(),
                ProtocolEventKind::ShellHello { .. }
            ))
            .count(),
        1,
    );
    assert!(
        events.iter().any(|event| matches!(
            event.kind(),
            ProtocolEventKind::CommandEnd {
                exit_code: Some(1),
                ..
            }
        )),
        "events: {events:#?}; PTY output: {}",
        String::from_utf8_lossy(&output.stdout),
    );
}

#[test]
fn zsh_v2_reports_empty_enter_as_own_completion_when_zsh_is_available() {
    if Command::new("zsh").arg("--version").output().is_err() {
        return;
    }

    let output = run_shell(
        "zsh",
        &["-d", "-f", "-i", "-c"],
        r#"
source "$OTTY_TEST_INTEGRATION"
add-zsh-hook -d preexec _otty_preexec
add-zsh-hook -d precmd _otty_precmd
_otty_active_block_id=
_otty_prepared_block_id=
_otty_precmd
_otty_precmd
exit 0
"#,
    );
    assert!(
        output.status.success(),
        "zsh PTY output: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let events = parse_events(&output.stdout);

    assert_empty_enter_completion(&events);
}
