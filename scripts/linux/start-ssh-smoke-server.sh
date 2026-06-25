#!/usr/bin/env bash

set -euo pipefail

test_root="${RUNNER_TEMP:-/tmp}/otty-ssh-smoke"
test_user="otty-ssh-test"
test_port="2222"

rm -rf "${test_root}"
mkdir -p "${test_root}" /run/sshd

if ! id "${test_user}" >/dev/null 2>&1; then
    useradd --create-home --shell /bin/bash "${test_user}"
fi
passwd --delete "${test_user}"

ssh-keygen -q -t ed25519 -N "" -f "${test_root}/client_key"
ssh-keygen -q -t ed25519 -N "" -f "${test_root}/host_key"

install -d -m 700 -o "${test_user}" -g "${test_user}" "/home/${test_user}/.ssh"
install -m 600 -o "${test_user}" -g "${test_user}" \
    "${test_root}/client_key.pub" "/home/${test_user}/.ssh/authorized_keys"

cat >"${test_root}/sshd_config" <<EOF
Port ${test_port}
ListenAddress 127.0.0.1
HostKey ${test_root}/host_key
PidFile ${test_root}/sshd.pid
AuthorizedKeysFile .ssh/authorized_keys
PasswordAuthentication no
ChallengeResponseAuthentication no
UsePAM no
PermitRootLogin no
AllowUsers ${test_user}
StrictModes yes
PrintMotd no
Subsystem sftp internal-sftp
EOF

/usr/sbin/sshd -t -f "${test_root}/sshd_config"
/usr/sbin/sshd -f "${test_root}/sshd_config" -E "${test_root}/sshd.log"

for _ in $(seq 1 50); do
    if ssh-keyscan -p "${test_port}" 127.0.0.1 >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

if ! ssh-keyscan -p "${test_port}" 127.0.0.1 >/dev/null 2>&1; then
    cat "${test_root}/sshd.log"
    exit 1
fi

if [[ -z "${GITHUB_ENV:-}" ]]; then
    echo "GITHUB_ENV must be set so the SSH test configuration reaches the next step" >&2
    exit 1
fi

{
    echo "OTTY_SSH_TEST_TARGET=127.0.0.1:${test_port}"
    echo "OTTY_SSH_TEST_USER=${test_user}"
    echo "OTTY_SSH_TEST_PRIVATE_KEY=${test_root}/client_key"
} >>"${GITHUB_ENV}"
