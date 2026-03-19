use crate::errors::{VaultError, VaultResult};

/// Current logical schema format version for vault storage.
pub(crate) const CURRENT_FORMAT_VERSION: u32 = 1;

/// Logical tables fixed by the foundation contract.
pub(crate) const LOGICAL_TABLES: [&str; 3] =
    ["vault_metadata", "secret_groups", "secrets"];

/// Minimal interface for future migration runners.
pub(crate) trait MigrationRunner {
    fn run(&self, current_version: Option<u32>) -> VaultResult<u32>;
}

/// Placeholder migration runner used until SQLCipher-backed migrations land.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ScaffoldMigrationRunner;

impl MigrationRunner for ScaffoldMigrationRunner {
    fn run(&self, current_version: Option<u32>) -> VaultResult<u32> {
        match current_version {
            Some(version) if version > CURRENT_FORMAT_VERSION => {
                Err(VaultError::MigrationFailed)
            },
            Some(version) => Ok(version),
            None => Ok(CURRENT_FORMAT_VERSION),
        }
    }
}

pub(crate) fn run_migrations(current_version: Option<u32>) -> VaultResult<u32> {
    let _logical_tables = LOGICAL_TABLES;
    ScaffoldMigrationRunner.run(current_version)
}

#[cfg(test)]
mod tests {
    use super::{CURRENT_FORMAT_VERSION, run_migrations};
    use crate::VaultError;

    #[test]
    fn missing_version_bootstraps_current_format() {
        assert_eq!(run_migrations(None), Ok(CURRENT_FORMAT_VERSION));
    }

    #[test]
    fn existing_current_version_is_accepted() {
        assert_eq!(
            run_migrations(Some(CURRENT_FORMAT_VERSION)),
            Ok(CURRENT_FORMAT_VERSION)
        );
    }

    #[test]
    fn future_version_is_rejected() {
        assert_eq!(
            run_migrations(Some(CURRENT_FORMAT_VERSION + 1)),
            Err(VaultError::MigrationFailed)
        );
    }
}
