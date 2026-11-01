use crate::i18n::Key;

/// A way of working OTTY can switch to.
///
/// The list is closed (spec Decision 1): a new mode is a new variant and a
/// new widget, never a plugin. The app owns each mode's name and icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Terminal tabs and sessions. The default mode, always the first tab
    /// of the switcher and never pinned or unpinned.
    Terminal,
    /// Containers, images, volumes and networks of the local Docker or
    /// Podman engines.
    Docker,
}

impl Mode {
    /// Every mode, in the order the All modes menu lists them.
    pub(crate) const ALL: [Self; 2] = [Self::Terminal, Self::Docker];

    /// Translation key of the mode's display name.
    pub(crate) fn name_key(self) -> Key {
        match self {
            Self::Terminal => Key::ModeTerminal,
            Self::Docker => Key::ModeDocker,
        }
    }

    /// Name of the mode in `modes.json`, e.g. `"docker"`.
    pub(crate) fn storage_name(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Docker => "docker",
        }
    }

    /// Mode saved under `name` in `modes.json`; `None` for a name this
    /// version does not know.
    pub(crate) fn from_storage_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.storage_name() == name)
    }
}

/// Most modes that can be pinned besides Terminal (spec Decision 2).
pub(crate) const MAX_PINNED: usize = 4;

/// Modes pinned on first start and whenever `modes.json` cannot be read.
pub(crate) const DEFAULT_PINNED: [Mode; 1] = [Mode::Docker];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_loads_back_from_its_storage_name() {
        for mode in Mode::ALL {
            assert_eq!(
                Mode::from_storage_name(mode.storage_name()),
                Some(mode)
            );
        }
        assert_eq!(Mode::from_storage_name("kubernetes"), None);
    }

    #[test]
    fn every_mode_has_its_own_name() {
        assert_eq!(Mode::Terminal.name_key(), Key::ModeTerminal);
        assert_eq!(Mode::Docker.name_key(), Key::ModeDocker);
    }
}
