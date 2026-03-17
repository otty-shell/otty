use crate::model::VaultStatus;

/// Process-local lock state for an opened vault handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VaultSession {
    status: VaultStatus,
}

impl VaultSession {
    pub(crate) fn new_unlocked() -> Self {
        Self {
            status: VaultStatus::Unlocked,
        }
    }

    pub(crate) fn status(&self) -> VaultStatus {
        self.status
    }

    pub(crate) fn is_unlocked(&self) -> bool {
        self.status == VaultStatus::Unlocked
    }

    pub(crate) fn lock(&mut self) {
        self.status = VaultStatus::Locked;
    }

    pub(crate) fn unlock(&mut self) {
        self.status = VaultStatus::Unlocked;
    }
}
