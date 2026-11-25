//! The plugin's log, shown in Plugins mode. It never holds keys,
//! tokens, prompts or session text.

/// How important a line of the log is.
pub use crate::bindings::host::otty::host::log::Level;

/// Adds one line at `level`; the log keeps the newest 64 KiB.
pub fn write(level: Level, text: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::log::write(level, text);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::write_log(level, text);
    }
}

/// Adds one line for whoever debugs the plugin.
pub fn debug(text: &str) {
    write(Level::Debug, text);
}

/// Adds one line on what the plugin did.
pub fn info(text: &str) {
    write(Level::Info, text);
}

/// Adds one line on something that went wrong but did not stop the call.
pub fn warn(text: &str) {
    write(Level::Warn, text);
}

/// Adds one line on what failed.
pub fn error(text: &str) {
    write(Level::Error, text);
}
