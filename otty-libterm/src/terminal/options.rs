/// Configuration knobs that influence how the terminal runtime behaves.
#[derive(Clone, Debug)]
pub struct TerminalOptions {
    /// Size of the temporary buffer used to drain PTY output.
    pub read_buffer_capacity: usize,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            read_buffer_capacity: 4096,
        }
    }
}
