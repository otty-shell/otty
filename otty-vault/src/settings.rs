use std::time::Duration;

use crate::errors::{VaultError, VaultResult};

/// Vault runtime settings used by security-sensitive flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaultSettings {
    auto_lock_timeout: Duration,
    clipboard_clear_timeout: Duration,
}

impl VaultSettings {
    /// Create settings with explicit timeout values.
    pub fn new(
        auto_lock_timeout: Duration,
        clipboard_clear_timeout: Duration,
    ) -> VaultResult<Self> {
        let candidate = Self {
            auto_lock_timeout,
            clipboard_clear_timeout,
        };
        candidate.validate()?;
        Ok(candidate)
    }

    /// Duration after which an inactive unlocked vault should lock automatically.
    pub fn auto_lock_timeout(&self) -> Duration {
        self.auto_lock_timeout
    }

    /// Duration after which copied secret data should be cleared from clipboard.
    pub fn clipboard_clear_timeout(&self) -> Duration {
        self.clipboard_clear_timeout
    }

    pub(crate) fn validate(&self) -> VaultResult<()> {
        if self.auto_lock_timeout.is_zero()
            || self.clipboard_clear_timeout.is_zero()
        {
            return Err(VaultError::InvalidSettings);
        }
        Ok(())
    }
}

impl Default for VaultSettings {
    fn default() -> Self {
        Self {
            auto_lock_timeout: Duration::from_secs(30 * 60),
            clipboard_clear_timeout: Duration::from_secs(30),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::VaultSettings;
    use crate::errors::VaultError;

    #[test]
    fn default_settings_match_contract_values() {
        let settings = VaultSettings::default();
        assert_eq!(settings.auto_lock_timeout(), Duration::from_secs(30 * 60));
        assert_eq!(settings.clipboard_clear_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn settings_validation_rejects_zero_timeouts() {
        let result = VaultSettings::new(Duration::ZERO, Duration::from_secs(1));
        assert_eq!(result, Err(VaultError::InvalidSettings));
    }
}
