//! Files the called part declares in `read`, `write` and `keys`. Paths
//! are absolute: build them from [`env::home`](crate::env::home) and the
//! variables the part lists.

/// Complete lines of a file, from one read.
pub use crate::bindings::host::otty::host::files::Batch;
/// One file a listing found.
pub use crate::bindings::host::otty::host::files::FileInfo;
use crate::{HostError, JsonWithKeys};

/// How many bytes of lines one read of [`lines`] asks for.
pub const BATCH_BYTES: u32 = 1024 * 1024;

/// The batches of complete lines of one file, read one after another, as
/// [`lines`] gives them.
#[derive(Debug)]
pub struct Lines {
    path: String,
    markers: Vec<Vec<u8>>,
    offset: u64,
    done: bool,
}

impl Iterator for Lines {
    type Item = Result<Batch, HostError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let batch =
            read_lines(&self.path, self.offset, BATCH_BYTES, &self.markers);

        match &batch {
            Ok(batch) => {
                self.offset = batch.next_offset;
                self.done = batch.at_end;
            },
            Err(_) => self.done = true,
        }

        Some(batch)
    }
}

impl Batch {
    /// The lines of the batch, each without its newline.
    ///
    /// ```
    /// use otty_plugin_sdk::files::Batch;
    ///
    /// let batch = Batch {
    ///     lines: b"{\"a\":1}\n{\"b\":2}\n".to_vec(),
    ///     next_offset: 16,
    ///     skipped: 3,
    ///     at_end: true,
    /// };
    ///
    /// let lines: Vec<&[u8]> = batch.lines().collect();
    ///
    /// assert_eq!(lines, [&b"{\"a\":1}"[..], &b"{\"b\":2}"[..]]);
    /// ```
    pub fn lines(&self) -> impl Iterator<Item = &[u8]> {
        let text = self.lines.strip_suffix(b"\n").unwrap_or(&self.lines);

        text.split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
    }
}

/// The files whose paths match `glob`, such as
/// `/home/user/.example-agent/sessions/*.jsonl`.
pub fn list(glob: &str) -> Result<Vec<FileInfo>, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::files::list(glob)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::list_files(glob)
    }
}

/// The bytes of a small file, at most `max_bytes` (and 4 MiB). A key file
/// is refused: read it with [`json_with_keys`].
pub fn read(path: &str, max_bytes: u32) -> Result<Vec<u8>, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::files::read(path, max_bytes)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::read_file(path, max_bytes)
    }
}

/// The lines of `path` holding one of `markers` (every line when there is
/// none), batch by batch; the host leaves the other lines out and decodes
/// zstd files.
pub fn lines(path: &str, markers: &[&[u8]]) -> Lines {
    Lines {
        path: path.to_string(),
        markers: markers.iter().map(|marker| marker.to_vec()).collect(),
        offset: 0,
        done: false,
    }
}

/// One batch of complete lines of `path` from `offset`, about `max_bytes`
/// of them, only those holding one of `markers`.
pub fn read_lines(
    path: &str,
    offset: u64,
    max_bytes: u32,
    markers: &[Vec<u8>],
) -> Result<Batch, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::files::read_lines(
            path, offset, max_bytes, markers,
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::read_lines(path, offset, max_bytes, markers)
    }
}

/// A JSON key file the part declares in `keys`, its key fields replaced
/// by `{"$key": n}`.
pub fn json_with_keys(path: &str) -> Result<JsonWithKeys, HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::files::read_json_with_keys(path)
            .map(JsonWithKeys::from_wasm)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::read_json_with_keys(path)
    }
}

/// Deletes one file, or one empty folder, the part declares in `write`.
pub fn remove(path: &str) -> Result<(), HostError> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::host::otty::host::files::remove(path)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::remove_file(path)
    }
}
