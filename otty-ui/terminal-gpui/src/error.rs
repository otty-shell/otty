use std::error::Error;
use std::sync::Arc;

use thiserror::Error;

/// Errors produced while validating terminal presentation settings.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum ConfigError {
    /// The font size must be finite and positive.
    #[error("font size must be finite and positive")]
    InvalidFontSize,
    /// The relative line height must be finite and positive.
    #[error("line height must be finite and positive")]
    InvalidLineHeight,
    /// Padding must be finite and non-negative.
    #[error("padding must be finite and non-negative")]
    InvalidPadding,
    /// Border widths must be finite and non-negative.
    #[error("border width must be finite and non-negative")]
    InvalidBorderWidth,
    /// Corner radius must be finite and non-negative.
    #[error("corner radius must be finite and non-negative")]
    InvalidCornerRadius,
    /// Scroll multiplier must be finite and positive.
    #[error("scroll multiplier must be finite and positive")]
    InvalidScrollMultiplier,
    /// A color was not in `#rrggbb` form.
    #[error("invalid terminal color: {0}")]
    InvalidColor(String),
    /// Cell dimensions must be finite and positive.
    #[error("cell dimensions must be finite and positive")]
    InvalidCellMetrics,
}

/// Errors returned while starting or running a terminal backend.
#[derive(Debug, Error)]
pub enum BackendError {
    /// The shared terminal core failed.
    #[error(transparent)]
    Core(#[from] otty_libterm::Error),
    /// A custom backend failed at an infrastructure boundary.
    #[error("custom terminal backend failed: {0}")]
    External(Arc<dyn Error + Send + Sync>),
    /// The runtime thread could not be created.
    #[error("failed to spawn terminal runtime: {0}")]
    ThreadSpawn(std::io::Error),
}

impl BackendError {
    /// Wrap an error returned by a custom backend without discarding its source.
    pub fn external(error: impl Error + Send + Sync + 'static) -> Self {
        Self::External(Arc::new(error))
    }
}

/// Errors returned by terminal operations requested by a host.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OperationError {
    /// There is no running backend to receive the operation.
    #[error("terminal backend is not running")]
    BackendUnavailable,
    /// The bounded request queue is temporarily full.
    #[error("terminal request queue is full")]
    Backpressure,
    /// The backend request channel has closed.
    #[error("terminal backend is disconnected")]
    BackendDisconnected,
    /// The requested selection or block has no copyable content.
    #[error("terminal content is not available")]
    ContentUnavailable,
}
