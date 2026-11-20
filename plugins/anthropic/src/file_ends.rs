//! The start and the end of a session file. A session file can be large,
//! and what the session list and the version need is written at its
//! start (the first prompt and folder) and at its end (the newest title,
//! time and version), so only those are read.

use otty_plugin_sdk::HostError;
use otty_plugin_sdk::files::{self, BATCH_BYTES, FileInfo};

/// How much is read from the start and from the end of a session file.
pub(crate) const END_BYTES: u64 = 128 * 1024;

/// The lines of the start and of the end of `file`, in the order of the
/// file, each about `end_bytes` long; the whole file when it is at most
/// twice that. A line cut by the start of the end is left out.
pub(crate) fn read_ends(
    file: &FileInfo,
    end_bytes: u64,
) -> Result<Vec<Vec<u8>>, HostError> {
    let mut lines = Vec::new();

    if file.size <= 2 * end_bytes {
        read_to_end(&file.path, 0, &mut lines)?;
        return Ok(lines);
    }

    let start_bytes = u32::try_from(end_bytes).unwrap_or(BATCH_BYTES);
    let start = files::read_lines(&file.path, 0, start_bytes, &[])?;
    lines.extend(start.lines().map(<[u8]>::to_vec));

    let mut end = Vec::new();
    read_to_end(&file.path, file.size - end_bytes, &mut end)?;
    lines.extend(end.into_iter().skip(1));

    Ok(lines)
}

/// Adds the lines of the file at `path` from `offset` to its end to
/// `lines`.
fn read_to_end(
    path: &str,
    mut offset: u64,
    lines: &mut Vec<Vec<u8>>,
) -> Result<(), HostError> {
    loop {
        let batch = files::read_lines(path, offset, BATCH_BYTES, &[])?;
        lines.extend(batch.lines().map(<[u8]>::to_vec));

        if batch.at_end || batch.next_offset <= offset {
            return Ok(());
        }
        offset = batch.next_offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{as_part, host, path};

    const SESSION: &str = ".claude/projects/-home-dev-shop/1f0c2b7e-8d4a-4e6f-9b0c-1a2d3e4f5a6b.jsonl";

    fn ends_of(end_bytes: u64) -> (Vec<Vec<u8>>, usize) {
        let host = host();
        let session = path(&host, SESSION);

        as_part(&host, || {
            let found = files::list(&session).expect("the listing");
            let all = read_ends(&found[0], u64::MAX / 4).expect("the file");
            let ends = read_ends(&found[0], end_bytes).expect("the ends");

            (ends, all.len())
        })
    }

    #[test]
    fn a_small_file_is_read_whole() {
        let (lines, all) = ends_of(END_BYTES);

        assert_eq!(lines.len(), all);
        assert_eq!(all, 8);
    }

    #[test]
    fn a_large_file_gives_its_first_and_last_whole_lines() {
        let (lines, _) = ends_of(200);

        let first = String::from_utf8_lossy(&lines[0]);
        let last = String::from_utf8_lossy(lines.last().expect("a line"));
        assert!(first.contains("\"type\":\"user\""), "{first}");
        assert!(last.contains("last-prompt"), "{last}");
        assert!(lines.len() < 8);
        for line in &lines {
            assert!(serde_json::from_slice::<serde_json::Value>(line).is_ok());
        }
    }
}
