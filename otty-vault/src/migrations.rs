use crate::errors::{VaultError, VaultResult};

/// Current schema format version for vault storage.
pub(crate) const CURRENT_FORMAT_VERSION: u32 = 1;

/// Minimal interface for future migration runners.
pub(crate) trait MigrationRunner {
    fn run(&self, current_version: Option<u32>) -> VaultResult<u32>;
}

/// Placeholder migration runner for the foundation phase.
pub(crate) struct NoopMigrationRunner;

impl MigrationRunner for NoopMigrationRunner {
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
    NoopMigrationRunner.run(current_version)
}
