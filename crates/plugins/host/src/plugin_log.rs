//! The log of one plugin: what it writes to stderr (and, with issue 04,
//! through `log.write`), at most 64 KiB, the oldest bytes dropped first.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use wasmtime_wasi::cli::OutputFile;

/// How many bytes a plugin's log keeps (spec "Engine limits").
pub const LOG_CAP: usize = 64 * 1024;

/// The capped log of one plugin, shared by all its calls. Cloning gives
/// another handle to the same log.
#[derive(Debug, Clone, Default)]
pub struct PluginLog {
    bytes: Arc<Mutex<VecDeque<u8>>>,
}

impl PluginLog {
    /// The log as text, invalid UTF-8 replaced, for the Plugins page.
    pub fn text(&self) -> String {
        let bytes = self.bytes.lock().unwrap_or_else(PoisonError::into_inner);
        let (front, back) = bytes.as_slices();

        String::from_utf8_lossy(&[front, back].concat()).into_owned()
    }

    /// An empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `written` at the end, dropping the oldest bytes beyond
    /// [`LOG_CAP`].
    pub(crate) fn push(&self, written: &[u8]) {
        let mut bytes =
            self.bytes.lock().unwrap_or_else(PoisonError::into_inner);

        let kept = &written[written.len().saturating_sub(LOG_CAP)..];
        bytes.extend(kept);
        let extra = bytes.len().saturating_sub(LOG_CAP);
        bytes.drain(..extra);
    }

    /// A stderr for the WASI context of the plugin's calls: the write end
    /// of a pipe whose bytes a thread moves into this log. The thread ends
    /// when the last copy of the stream is dropped. A pipe, not a buffer,
    /// so a plugin that writes much never fills up and never fails.
    pub(crate) fn stderr(&self) -> io::Result<OutputFile> {
        let (mut reader, writer) = io::pipe()?;
        let log = self.clone();

        thread::Builder::new()
            .name("otty-plugin-log".to_string())
            .spawn(move || {
                let mut chunk = [0_u8; 4096];
                loop {
                    match reader.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => log.push(&chunk[..read]),
                    }
                }
            })?;

        Ok(OutputFile::new(File::from(OwnedFd::from(writer))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_keeps_what_was_written() {
        let log = PluginLog::new();

        log.push(b"first line\n");
        log.push(b"second line\n");

        assert_eq!(log.text(), "first line\nsecond line\n");
    }

    #[test]
    fn the_oldest_bytes_are_dropped_past_the_cap() {
        let log = PluginLog::new();

        log.push(&vec![b'a'; LOG_CAP - 2]);
        log.push(b"bbbb");

        let text = log.text();
        assert_eq!(text.len(), LOG_CAP);
        assert!(text.starts_with("aa"));
        assert!(text.ends_with("bbbb"));
    }

    #[test]
    fn one_write_larger_than_the_cap_keeps_its_end() {
        let log = PluginLog::new();

        log.push(&[vec![b'a'; LOG_CAP], b"end".to_vec()].concat());

        let text = log.text();
        assert_eq!(text.len(), LOG_CAP);
        assert!(text.ends_with("aend"));
    }
}
