//! Programs the called part declares in `run`.

use crate::HostError;
/// What a program wrote and how it ended.
pub use crate::bindings::host::otty::host::programs::Output;

/// A program spoken to line by line, such as `codex app-server`. OTTY
/// stops it when the call that started it ends; 60 s in all.
#[derive(Debug)]
pub struct Talk {
    #[cfg(target_arch = "wasm32")]
    talk: crate::bindings::host::otty::host::programs::Talk,
    #[cfg(not(target_arch = "wasm32"))]
    id: usize,
}

impl Talk {
    /// Starts `program` with `args` and the variables `env`.
    pub fn start(
        program: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> Result<Self, HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            use crate::bindings::host::otty::host::programs::Talk as Wit;

            Wit::start(program, args, env).map(|talk| Self { talk })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::test_host::start_talk(program, args, env)
                .map(|id| Self { id })
        }
    }

    /// Sends one line; the newline is added.
    pub fn send_line(&self, line: &str) -> Result<(), HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.talk.send_line(line)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::test_host::send_line(self.id, line)
        }
    }

    /// The next line the program wrote, without its newline; `None` when
    /// it ended.
    pub fn next_line(
        &self,
        timeout_ms: u32,
    ) -> Result<Option<String>, HostError> {
        #[cfg(target_arch = "wasm32")]
        {
            self.talk.next_line(timeout_ms)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = timeout_ms;
            crate::test_host::next_line(self.id)
        }
    }
}

/// Runs `program` once with `args` and the variables `env`, and gives
/// what it wrote: at most 10 s and 1 MiB.
pub fn run(
    program: &str,
    args: &[String],
    env: &[(String, String)],
    timeout_ms: u32,
) -> Result<Output, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::programs::run(
            program, args, env, timeout_ms,
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = timeout_ms;
        crate::test_host::run_program(program, args, env)
    }
}
