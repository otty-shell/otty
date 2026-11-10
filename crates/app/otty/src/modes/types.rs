use otty_app_domain::mode::Mode;

/// Most modes that can be pinned besides Terminal (spec Decision 2).
pub(crate) const MAX_PINNED: usize = 4;

/// Modes pinned on first start and whenever `modes.json` cannot be read.
pub(crate) const DEFAULT_PINNED: [Mode; 1] = [Mode::Containers];
