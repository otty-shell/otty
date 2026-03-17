use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use otty_vault::{
    SecretBytes, SecretKind, SecretMetadata, SqlCipherVault, Vault,
    VaultMetadataNode,
};
use secrecy::SecretString;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = temp_vault_path("simple");
    let passphrase = SecretString::from("correct horse battery staple");

    // 1. Open or bootstrap the vault at the chosen path.
    let mut vault = SqlCipherVault::open(&path, passphrase.clone())?;

    // 2. Create a folder and a secret in that folder.
    let folder_id = vault.create_folder(None, "demo")?;
    let metadata = SecretMetadata::new(
        folder_id,
        "api-token",
        Some(String::from("service-user")),
        SecretKind::Password,
        vec![String::from("example")],
    )?;
    let secret_id = vault
        .create_secret(metadata, SecretBytes::new(b"top-secret".to_vec()))?;

    // 3. Read metadata tree and print one-level summary.
    let tree = vault.metadata_tree()?;
    println!("Root folder: {}", tree.root().name());
    for child in tree.root().children() {
        match child {
            VaultMetadataNode::Folder(folder) => {
                println!("  Folder: {}", folder.name());
            },
            VaultMetadataNode::Secret(secret) => {
                println!("  Secret: {}", secret.metadata().name());
            },
        }
    }

    // 4. Search by name and print resolved path.
    let results = vault.find_secrets_by_name("token")?;
    for item in results {
        println!("Found secret at: {}", item.path());
    }

    // 5. Read secret value explicitly and show its size.
    let value = vault.get_secret_value(&secret_id)?;
    println!("Secret value size: {} bytes", value.len());

    // 6. Lock and unlock the handle.
    vault.lock()?;
    vault.unlock(passphrase)?;

    // 7. Cleanup the temporary vault file.
    let _ = fs::remove_file(path);

    Ok(())
}

fn temp_vault_path(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("otty-vault-{prefix}-{stamp}.db"))
}
