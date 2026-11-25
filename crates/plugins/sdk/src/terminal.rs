//! Programs the called part declares in `terminal`, started in a terminal
//! nobody sees.

use crate::HostError;

/// A program in a hidden terminal, in OTTY's own working folder. OTTY
/// stops it when the call that started it ends; 60 s in all.
#[derive(Debug)]
pub struct HiddenTerminal {
    #[cfg(target_arch = "wasm32")]
    terminal: crate::bindings::host::otty::host::terminal::HiddenTerminal,
    #[cfg(not(target_arch = "wasm32"))]
    id: usize,
}

impl HiddenTerminal {
    /// Starts `program` with `args` and the variables `env` in a terminal
    /// of `cols` by `rows` cells.
    pub fn start(
        program: &str,
        args: &[String],
        env: &[(String, String)],
        cols: u16,
        rows: u16,
    ) -> Result<Self, HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            use crate::bindings::host::otty::host::terminal::HiddenTerminal as Wit;

            Wit::start(program, args, env, cols, rows)
                .map(|terminal| Self { terminal })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (cols, rows);
            crate::test_host::start_terminal(program, args, env)
                .map(|id| Self { id })
        }
    }

    /// Types `keys` into the terminal.
    pub fn send_keys(&self, keys: &str) -> Result<(), HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.terminal.send_keys(keys)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::test_host::send_keys(self.id, keys)
        }
    }

    /// The visible text of the screen.
    pub fn screen(&self) -> Result<String, HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.terminal.screen()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::test_host::screen(self.id)
        }
    }

    /// Whether the screen changed within `timeout_ms`.
    pub fn wait_for_change(&self, timeout_ms: u32) -> Result<bool, HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.terminal.wait_for_change(timeout_ms)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = timeout_ms;
            crate::test_host::wait_for_change(self.id)
        }
    }
}

/// The working folder OTTY starts every hidden terminal in: as OTTY names
/// it and, when links lead elsewhere, as the system resolves it. A program
/// that names its records after the folder it ran in, such as an agent's
/// session files, names them after this one. Only for a part that
/// declares a terminal.
pub fn working_folders() -> Result<Vec<String>, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::terminal::working_folders()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::terminal_folders()
    }
}
