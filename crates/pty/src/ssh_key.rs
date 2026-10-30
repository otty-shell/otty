//! Inspection of SSH private key files before they are handed to libssh2.

use std::io;
use std::path::Path;

const OPENSSH_HEADER: &str = "-----BEGIN OPENSSH PRIVATE KEY-----";
// Base64 of the `openssh-key-v1` magic followed by the cipher name `none`.
// Every OpenSSH key without a passphrase starts its body with it.
const OPENSSH_UNENCRYPTED_BODY: &str = "b3BlbnNzaC1rZXktdjEAAAAABG5vbmU";
const PEM_ENCRYPTED_MARKER: &str = "Proc-Type: 4,ENCRYPTED";
const PKCS8_ENCRYPTED_HEADER: &str = "-----BEGIN ENCRYPTED PRIVATE KEY-----";

/// Report whether the private key file at `path` is protected by a passphrase.
pub(crate) fn is_key_file_encrypted(path: &Path) -> io::Result<bool> {
    let contents = std::fs::read(path)?;
    Ok(is_key_encrypted(&String::from_utf8_lossy(&contents)))
}

/// Report whether the key text is protected by a passphrase. Unknown formats
/// count as unencrypted, so libssh2 reports its own error for them.
fn is_key_encrypted(contents: &str) -> bool {
    let contents = contents.trim_start();

    if let Some(body) = contents.strip_prefix(OPENSSH_HEADER) {
        return !body.trim_start().starts_with(OPENSSH_UNENCRYPTED_BODY);
    }

    contents.starts_with(PKCS8_ENCRYPTED_HEADER)
        || contents.contains(PEM_ENCRYPTED_MARKER)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{is_key_encrypted, is_key_file_encrypted};

    const OPENSSH_PLAIN: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\n\
        b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAA\n\
        -----END OPENSSH PRIVATE KEY-----\n";
    const OPENSSH_ENCRYPTED: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\n\
        b3BlbnNzaC1rZXktdjEAAAAACmFlczI1Ni1jdHIAAAAGYmNyeXB0AAAAGAAA\n\
        -----END OPENSSH PRIVATE KEY-----\n";
    const PEM_PLAIN: &str = "-----BEGIN RSA PRIVATE KEY-----\n\
        MIIEowIBAAKCAQEA\n\
        -----END RSA PRIVATE KEY-----\n";
    const PEM_ENCRYPTED: &str = "-----BEGIN RSA PRIVATE KEY-----\n\
        Proc-Type: 4,ENCRYPTED\n\
        DEK-Info: AES-128-CBC,73560A84A0AA42FEC3D4638DC5627118\n\
        \n\
        MIIEowIBAAKCAQEA\n\
        -----END RSA PRIVATE KEY-----\n";
    const PKCS8_PLAIN: &str = "-----BEGIN PRIVATE KEY-----\n\
        MIIEvQIBADANBgkqhkiG9w0BAQEFAASC\n\
        -----END PRIVATE KEY-----\n";
    const PKCS8_ENCRYPTED: &str = "-----BEGIN ENCRYPTED PRIVATE KEY-----\n\
        MIIFHDBOBgkqhkiG9w0BBQ0wQTApBgkq\n\
        -----END ENCRYPTED PRIVATE KEY-----\n";

    struct TempKeyFile {
        path: PathBuf,
    }

    impl TempKeyFile {
        fn new(name: &str, contents: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("otty-pty-{name}-{}", std::process::id()));
            std::fs::write(&path, contents).expect("write temp key file");
            Self { path }
        }
    }

    impl Drop for TempKeyFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn given_unencrypted_openssh_key_when_checked_then_not_encrypted() {
        assert!(!is_key_encrypted(OPENSSH_PLAIN));
    }

    #[test]
    fn given_encrypted_openssh_key_when_checked_then_encrypted() {
        assert!(is_key_encrypted(OPENSSH_ENCRYPTED));
    }

    #[test]
    fn given_openssh_key_with_crlf_lines_when_checked_then_cipher_is_read() {
        let plain = OPENSSH_PLAIN.replace('\n', "\r\n");
        let encrypted = OPENSSH_ENCRYPTED.replace('\n', "\r\n");

        assert!(!is_key_encrypted(&plain));
        assert!(is_key_encrypted(&encrypted));
    }

    #[test]
    fn given_unencrypted_pem_key_when_checked_then_not_encrypted() {
        assert!(!is_key_encrypted(PEM_PLAIN));
    }

    #[test]
    fn given_encrypted_pem_key_when_checked_then_encrypted() {
        assert!(is_key_encrypted(PEM_ENCRYPTED));
    }

    #[test]
    fn given_unencrypted_pkcs8_key_when_checked_then_not_encrypted() {
        assert!(!is_key_encrypted(PKCS8_PLAIN));
    }

    #[test]
    fn given_encrypted_pkcs8_key_when_checked_then_encrypted() {
        assert!(is_key_encrypted(PKCS8_ENCRYPTED));
    }

    #[test]
    fn given_unknown_contents_when_checked_then_not_encrypted() {
        assert!(!is_key_encrypted("not a key"));
    }

    #[test]
    fn given_encrypted_key_file_when_checked_then_encrypted() {
        let file = TempKeyFile::new("encrypted", OPENSSH_ENCRYPTED);

        let result = is_key_file_encrypted(&file.path);

        assert!(result.expect("read key file"));
    }

    #[test]
    fn given_missing_key_file_when_checked_then_io_error_returned() {
        let path = std::env::temp_dir().join("otty-pty-missing-key-file");

        let result = is_key_file_encrypted(&path);

        assert_eq!(
            result.expect_err("missing file").kind(),
            std::io::ErrorKind::NotFound
        );
    }
}
