use std::time::Duration;

use crate::errors::{VaultError, VaultResult};

/// Runtime settings that influence security-sensitive vault flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    auto_lock_timeout: Duration,
    clipboard_clear_timeout: Duration,
}

impl Settings {
    /// Build settings with explicit timeout values.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use otty_vault::Settings;
    ///
    /// let settings =
    ///     Settings::new(Duration::from_secs(60), Duration::from_secs(15))
    ///         .expect("settings should be valid");
    ///
    /// assert_eq!(settings.auto_lock_timeout(), Duration::from_secs(60));
    /// assert_eq!(settings.clipboard_clear_timeout(), Duration::from_secs(15));
    /// ```
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

    /// Return the inactivity timeout after which the vault should auto-lock.
    pub fn auto_lock_timeout(&self) -> Duration {
        self.auto_lock_timeout
    }

    /// Return the timeout after which copied secret data should be cleared.
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

impl Default for Settings {
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

    use super::Settings;
    use crate::VaultError;

    #[test]
    fn default_settings_match_contract_values() {
        let settings = Settings::default();

        assert_eq!(settings.auto_lock_timeout(), Duration::from_secs(30 * 60));
        assert_eq!(settings.clipboard_clear_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn settings_validation_rejects_zero_timeouts() {
        let auto_lock_error =
            Settings::new(Duration::ZERO, Duration::from_secs(30));
        let clipboard_error =
            Settings::new(Duration::from_secs(30), Duration::ZERO);

        assert_eq!(auto_lock_error, Err(VaultError::InvalidSettings));
        assert_eq!(clipboard_error, Err(VaultError::InvalidSettings));
    }
}
