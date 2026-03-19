use crate::model::VaultState;

/// Process-local lock state for an opened vault handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VaultSession {
    state: VaultState,
}

impl VaultSession {
    pub(crate) fn new_unlocked() -> Self {
        Self {
            state: VaultState::Unlocked,
        }
    }

    pub(crate) fn state(&self) -> VaultState {
        self.state
    }

    pub(crate) fn is_unlocked(&self) -> bool {
        self.state == VaultState::Unlocked
    }

    pub(crate) fn lock(&mut self) {
        self.state = VaultState::Locked;
    }

    pub(crate) fn unlock(&mut self) {
        self.state = VaultState::Unlocked;
    }
}

#[cfg(test)]
mod tests {
    use super::VaultSession;
    use crate::VaultState;

    #[test]
    fn session_transitions_between_locked_and_unlocked_states() {
        let mut session = VaultSession::new_unlocked();
        assert!(session.is_unlocked());
        assert_eq!(session.state(), VaultState::Unlocked);

        session.lock();
        assert!(!session.is_unlocked());
        assert_eq!(session.state(), VaultState::Locked);

        session.unlock();
        assert!(session.is_unlocked());
        assert_eq!(session.state(), VaultState::Unlocked);
    }
}
