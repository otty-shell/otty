use std::time::Duration;

/// Configuration knobs that influence how the terminal runtime behaves.
#[derive(Clone, Debug)]
pub struct TerminalOptions {
    /// Timeout used when polling for PTY events.
    pub poll_timeout: Duration,
    /// Size of the temporary buffer used to drain PTY output.
    pub read_buffer_capacity: usize,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            poll_timeout: Duration::from_millis(16),
            read_buffer_capacity: 4096,
        }
    }
}
