use std::time::{Duration, Instant};
use std::{env, thread};

use otty_pty::{SSHAuth, Session, ssh};

const COMMAND: &[u8] = b"printf 'OTTY_SSH_SMOKE_OK\\n'; exit\r";
const EXPECTED_OUTPUT: &str = "OTTY_SSH_SMOKE_OK";
const IO_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_DELAY: Duration = Duration::from_millis(10);

#[test]
#[ignore = "requires a local SSH server configured through OTTY_SSH_TEST_* variables"]
fn ssh_backend_authenticates_opens_pty_and_exchanges_data() {
    let target = required_env("OTTY_SSH_TEST_TARGET");
    let user = required_env("OTTY_SSH_TEST_USER");
    let private_key = required_env("OTTY_SSH_TEST_PRIVATE_KEY");
    let mut session = ssh()
        .with_host(&target)
        .with_user(&user)
        .with_auth(SSHAuth::KeyFile {
            private_key_path: private_key,
            passphrase: None,
        })
        .with_timeout(IO_TIMEOUT)
        .spawn()
        .expect(
            "SSH handshake, key authentication, and PTY setup should succeed",
        );

    write_with_timeout(&mut session, COMMAND);
    let output = read_until_exit(&mut session);

    assert!(
        output.contains(EXPECTED_OUTPUT),
        "remote PTY output should contain the smoke marker; output: {output:?}"
    );
    assert_eq!(
        session.close().expect("SSH session should close cleanly"),
        0
    );
}

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}

fn write_with_timeout(session: &mut impl Session, mut input: &[u8]) {
    let deadline = Instant::now() + IO_TIMEOUT;

    while !input.is_empty() {
        assert!(
            Instant::now() < deadline,
            "timed out while writing the SSH smoke command"
        );

        let written = session
            .write(input)
            .expect("writing to the remote PTY should succeed");
        input = &input[written..];

        if written == 0 {
            thread::sleep(RETRY_DELAY);
        }
    }
}

fn read_until_exit(session: &mut impl Session) -> String {
    let deadline = Instant::now() + IO_TIMEOUT;
    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];

    loop {
        assert!(
            Instant::now() < deadline,
            "timed out while reading SSH smoke output"
        );

        let read = session
            .read(&mut buffer)
            .expect("reading from the remote PTY should succeed");
        output.extend_from_slice(&buffer[..read]);

        if session
            .try_get_child_exit_status()
            .expect("querying the remote exit status should succeed")
            .is_some()
        {
            break;
        }

        if read == 0 {
            thread::sleep(RETRY_DELAY);
        }
    }

    String::from_utf8_lossy(&output).into_owned()
}
