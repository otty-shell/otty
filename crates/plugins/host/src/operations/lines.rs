//! `files.read-lines`: complete lines of a file from a byte offset, only
//! those holding one of the markers, so the plugin copies and parses only
//! the lines it wants (the spike: 2.1x faster than copying every line).
//!
//! A file that starts with the zstd magic bytes is decoded on the fly and
//! its offsets count decoded bytes. The decoder of the last zstd file is
//! kept open for the next batch of the same call batch, so reading a file
//! batch by batch decodes it once.

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use memchr::memmem::Finder;
use otty_plugin_manifest::{Environment, Permissions};
use ruzstd::decoding::{FrameDecoder, StreamingDecoder};

use crate::operations::real_path::reachable_file;
use crate::operations::serve::io_error;
use crate::operations::v0_1::otty::host::files::Batch;
use crate::operations::v0_1::otty::host::types::HostError;

/// The most bytes of lines one batch asks for (spec "Host operations":
/// about 1 MiB per batch).
pub(super) const BATCH_CAP: u32 = 1024 * 1024;

/// The longest line handed out. A longer one is left out and counted as
/// skipped, so a file with no newline never fills the host's memory.
const LONGEST_LINE: usize = 16 * 1024 * 1024;

/// The first bytes of a zstd frame.
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

/// The zstd file a part read lines of last, kept open where its last batch
/// stopped. One call batch keeps one. When the part moves to another zstd
/// file, the decoder of the last one is reset for it rather than made
/// anew, so reading the first lines of many files allocates its buffers
/// once.
#[derive(Default)]
pub(crate) struct ZstdCursor {
    open: Option<OpenZstd>,
}

/// A zstd file open at a decoded offset.
struct OpenZstd {
    path: PathBuf,
    /// The decoded offset where `ahead` starts.
    offset: u64,
    decoder: StreamingDecoder<BufReader<File>, FrameDecoder>,
    /// Bytes decoded past the last complete line handed out.
    ahead: Vec<u8>,
}

impl OpenZstd {
    /// The zstd file `file` at `path`, decoded up to `offset` with
    /// `decoder`, the decoder of the file read before or a new one.
    fn new(
        mut file: File,
        path: PathBuf,
        offset: u64,
        decoder: FrameDecoder,
    ) -> io::Result<Self> {
        file.seek(SeekFrom::Start(0))?;
        let mut decoder =
            StreamingDecoder::new_with_decoder(BufReader::new(file), decoder)
                .map_err(io::Error::other)?;

        io::copy(&mut (&mut decoder).take(offset), &mut io::sink())?;

        Ok(Self {
            path,
            offset,
            decoder,
            ahead: Vec::new(),
        })
    }
}

/// The markers a line must hold one of to be handed out, each with its
/// searcher built once per call.
struct Markers<'a> {
    finders: Vec<Finder<'a>>,
    /// No marker, or an empty one: every line is handed out.
    keeps_all: bool,
}

impl<'a> Markers<'a> {
    fn new(markers: &'a [Vec<u8>]) -> Self {
        let keeps_all = markers.is_empty()
            || markers.iter().any(|marker| marker.is_empty());
        let finders = markers.iter().map(Finder::new).collect();

        Self { finders, keeps_all }
    }

    /// Adds the lines of `bytes`, complete lines, that hold a marker to
    /// `kept`; gives how many were left out.
    fn keep_marked(&self, bytes: &[u8], kept: &mut Vec<u8>) -> u32 {
        if self.keeps_all {
            kept.extend_from_slice(bytes);
            return 0;
        }

        let mut skipped = 0;
        for line in bytes.split_inclusive(|byte| *byte == b'\n') {
            if self
                .finders
                .iter()
                .any(|finder| finder.find(line).is_some())
            {
                kept.extend_from_slice(line);
            } else {
                skipped += 1;
            }
        }

        skipped
    }
}

/// Complete lines of the file `path` from `offset`, about `max_bytes` of
/// them (at most [`BATCH_CAP`] asked for), only those holding one of
/// `markers`. The last line without a newline is handed out only at the
/// end of the file.
pub(super) fn read_lines(
    permissions: &Permissions,
    env: &Environment,
    cursor: &mut ZstdCursor,
    path: &str,
    offset: u64,
    max_bytes: u32,
    markers: &[Vec<u8>],
) -> Result<Batch, HostError> {
    let path = Path::new(path);
    let real = reachable_file(permissions, env, path, Permissions::may_read)?;
    let max_bytes = max_bytes.clamp(1, BATCH_CAP) as usize;
    let markers = Markers::new(markers);
    let failed = |error: io::Error| io_error(&error, path);

    let mut file = File::open(&real).map_err(failed)?;
    let mut magic = [0; 4];
    let magic_read = read_up_to(&mut file, &mut magic).map_err(failed)?;

    if magic_read < magic.len() || magic != ZSTD_MAGIC {
        file.seek(SeekFrom::Start(offset)).map_err(failed)?;
        let (batch, _) = next_batch(
            &mut file,
            Vec::new(),
            offset,
            max_bytes,
            &markers,
            LONGEST_LINE,
        )
        .map_err(failed)?;

        return Ok(batch);
    }

    let mut open = match cursor.open.take() {
        Some(open) if open.path == real && open.offset == offset => open,
        Some(mut other) => {
            let decoder = std::mem::take(&mut other.decoder.decoder);
            OpenZstd::new(file, real, offset, decoder).map_err(failed)?
        },
        None => OpenZstd::new(file, real, offset, FrameDecoder::new())
            .map_err(failed)?,
    };
    let ahead = std::mem::take(&mut open.ahead);
    let (batch, ahead) = next_batch(
        &mut open.decoder,
        ahead,
        offset,
        max_bytes,
        &markers,
        LONGEST_LINE,
    )
    .map_err(failed)?;

    open.offset = batch.next_offset;
    open.ahead = ahead;
    cursor.open = Some(open);

    Ok(batch)
}

/// The next batch from `reader`, which stands at `offset` after the
/// bytes `ahead`; gives the batch and the bytes read past its last line.
fn next_batch(
    reader: &mut impl Read,
    ahead: Vec<u8>,
    offset: u64,
    max_bytes: usize,
    markers: &Markers<'_>,
    longest_line: usize,
) -> io::Result<(Batch, Vec<u8>)> {
    let mut buffer = ahead;
    let mut dropped = 0;
    let mut skipped = 0;
    let mut wanted = max_bytes;
    // The start of the bytes of `buffer` not looked at for a newline yet,
    // so a long line is looked through once, not again at each read.
    let mut unsearched = 0;

    let (complete, at_end) = loop {
        if fill(reader, &mut buffer, wanted)? {
            break (buffer.len(), true);
        }
        if let Some(newline) = memchr::memrchr(b'\n', &buffer[unsearched..]) {
            break (unsearched + newline + 1, false);
        }
        unsearched = buffer.len();

        if buffer.len() >= longest_line {
            dropped += buffer.len() as u64;
            buffer.clear();
            dropped += drop_line(reader, &mut buffer)?;
            skipped += 1;
            wanted = max_bytes;
            unsearched = 0;
        } else {
            wanted = buffer.len() + max_bytes;
        }
    };

    let mut lines = Vec::new();
    skipped += markers.keep_marked(&buffer[..complete], &mut lines);
    let batch = Batch {
        lines,
        next_offset: offset + dropped + complete as u64,
        skipped,
        at_end,
    };

    Ok((batch, buffer.split_off(complete)))
}

/// Reads into `buffer` until it holds `wanted` bytes; gives whether the
/// end was reached first.
fn fill(
    reader: &mut impl Read,
    buffer: &mut Vec<u8>,
    wanted: usize,
) -> io::Result<bool> {
    while buffer.len() < wanted {
        let start = buffer.len();
        buffer.resize(wanted, 0);
        let read = read_up_to(reader, &mut buffer[start..])?;
        buffer.truncate(start + read);

        if read == 0 {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Reads past the end of the current line, keeping in `buffer` what comes
/// after its newline; gives how many bytes of the line were dropped.
fn drop_line(reader: &mut impl Read, buffer: &mut Vec<u8>) -> io::Result<u64> {
    let mut chunk = vec![0; 64 * 1024];
    let mut dropped = 0;

    loop {
        let read = read_up_to(reader, &mut chunk)?;
        if read == 0 {
            return Ok(dropped);
        }

        let read_bytes = &chunk[..read];
        if let Some(newline) = read_bytes.iter().position(|byte| *byte == b'\n')
        {
            buffer.extend_from_slice(&read_bytes[newline + 1..]);
            return Ok(dropped + newline as u64 + 1);
        }
        dropped += read as u64;
    }
}

/// Reads until `buffer` is full or the reader ends; gives how many bytes.
fn read_up_to(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {},
            Err(error) => return Err(error),
        }
    }

    Ok(filled)
}

#[cfg(test)]
mod tests {
    use ruzstd::encoding::{CompressionLevel, compress_to_vec};

    use super::*;
    use crate::operations::real_path::not_allowed;
    use crate::testing::{TempDir, debug_err, files_permissions, write_file};

    const USAGE: &[u8] = b"\"usage\"";

    struct Home {
        dir: TempDir,
        env: Environment,
    }

    impl Home {
        fn path(&self, relative: &str) -> String {
            self.dir.path().join(relative).display().to_string()
        }

        fn new() -> Self {
            let dir = TempDir::new();
            let env = Environment::new(dir.path().to_path_buf());

            Self { dir, env }
        }

        fn write(&self, relative: &str, bytes: &[u8]) {
            write_file(&self.dir.path().join(relative), bytes);
        }

        fn read(
            &self,
            cursor: &mut ZstdCursor,
            relative: &str,
            offset: u64,
            max_bytes: u32,
            markers: &[&[u8]],
        ) -> Result<Batch, HostError> {
            let permissions = files_permissions("alpha", &self.env);
            let markers: Vec<Vec<u8>> =
                markers.iter().map(|marker| marker.to_vec()).collect();

            read_lines(
                &permissions,
                &self.env,
                cursor,
                &self.path(relative),
                offset,
                max_bytes,
                &markers,
            )
        }

        /// Every batch of the file, read one after another.
        fn read_all(
            &self,
            relative: &str,
            max_bytes: u32,
            markers: &[&[u8]],
        ) -> Vec<Batch> {
            let mut cursor = ZstdCursor::default();
            let mut batches = Vec::new();
            let mut offset = 0;

            loop {
                let batch = self
                    .read(&mut cursor, relative, offset, max_bytes, markers)
                    .expect("a batch");
                offset = batch.next_offset;
                let at_end = batch.at_end;
                batches.push(batch);

                if at_end {
                    return batches;
                }
            }
        }
    }

    fn joined(batches: &[Batch]) -> String {
        let bytes: Vec<u8> = batches
            .iter()
            .flat_map(|batch| batch.lines.iter().copied())
            .collect();

        String::from_utf8(bytes).expect("text")
    }

    fn skipped(batches: &[Batch]) -> u32 {
        batches.iter().map(|batch| batch.skipped).sum()
    }

    fn texts(batches: &[Batch]) -> Vec<String> {
        batches
            .iter()
            .map(|batch| String::from_utf8_lossy(&batch.lines).into_owned())
            .collect()
    }

    /// Made-up session lines: every third one is an answer with usage.
    fn session_lines(count: usize) -> String {
        (0..count)
            .map(|index| {
                if index % 3 == 0 {
                    format!("{{\"type\":\"answer\",\"usage\":{index}}}\n")
                } else {
                    format!("{{\"type\":\"user\",\"text\":\"line {index}\"}}\n")
                }
            })
            .collect()
    }

    #[test]
    fn batches_end_on_complete_lines() {
        let home = Home::new();
        home.write(".alpha/a.jsonl", b"one\ntwo\nthree\nfour\n");

        let batches = home.read_all(".alpha/a.jsonl", 9, &[]);

        assert_eq!(texts(&batches), ["one\ntwo\n", "three\n", "four\n"]);
        let offsets: Vec<u64> =
            batches.iter().map(|batch| batch.next_offset).collect();
        assert_eq!(offsets, [8, 14, 19]);
        let ends: Vec<bool> =
            batches.iter().map(|batch| batch.at_end).collect();
        assert_eq!(ends, [false, false, true]);
    }

    #[test]
    fn the_last_line_without_a_newline_comes_only_at_the_end() {
        let home = Home::new();
        home.write(".alpha/a.jsonl", b"one\ntwo");

        let batches = home.read_all(".alpha/a.jsonl", 5, &[]);

        assert_eq!(texts(&batches), ["one\n", "two"]);
        assert!(!batches[0].at_end);
        assert_eq!(batches[1].next_offset, 7);
    }

    #[test]
    fn a_line_longer_than_the_batch_comes_whole() {
        let home = Home::new();
        home.write(".alpha/a.jsonl", b"aaaaaaaaaaaa\nb\n");

        let batches = home.read_all(".alpha/a.jsonl", 4, &[]);

        assert_eq!(texts(&batches), ["aaaaaaaaaaaa\nb\n"]);
    }

    #[test]
    fn markers_keep_their_lines_and_count_the_others() {
        let home = Home::new();
        home.write(".alpha/a.jsonl", session_lines(10).as_bytes());

        let batches = home.read_all(".alpha/a.jsonl", 64, &[USAGE]);

        let expected: String = session_lines(10)
            .lines()
            .filter(|line| line.contains("\"usage\""))
            .map(|line| format!("{line}\n"))
            .collect();
        assert_eq!(joined(&batches), expected);
        assert_eq!(skipped(&batches), 6);
    }

    #[test]
    fn any_of_several_markers_keeps_a_line() {
        let home = Home::new();
        home.write(".alpha/a.jsonl", b"{\"a\":1}\n{\"b\":2}\n{\"c\":3}\n");

        let batches =
            home.read_all(".alpha/a.jsonl", 1024, &[b"\"a\"", b"\"c\""]);

        assert_eq!(joined(&batches), "{\"a\":1}\n{\"c\":3}\n");
        assert_eq!(skipped(&batches), 1);
    }

    #[test]
    fn no_marker_or_an_empty_one_keeps_every_line() {
        let home = Home::new();
        home.write(".alpha/a.jsonl", b"a\nb\n");

        let none = home.read_all(".alpha/a.jsonl", 1024, &[]);
        let empty = home.read_all(".alpha/a.jsonl", 1024, &[b"x", b""]);

        assert_eq!(joined(&none), "a\nb\n");
        assert_eq!(joined(&empty), "a\nb\n");
        assert_eq!(skipped(&empty), 0);
    }

    #[test]
    fn a_marker_split_across_a_batch_edge_is_found() {
        let home = Home::new();
        let first = "{\"type\":\"user\"}\n";
        let second = "{\"type\":\"answer\",\"usage\":1}\n";
        home.write(".alpha/a.jsonl", format!("{first}{second}").as_bytes());
        let edge_in_marker =
            first.len() + second.find("usage").expect("in") + 2;

        let batches =
            home.read_all(".alpha/a.jsonl", edge_in_marker as u32, &[USAGE]);

        assert_eq!(joined(&batches), second);
        assert_eq!(skipped(&batches), 1);
    }

    #[test]
    fn a_zstd_file_gives_the_same_lines_as_its_plain_copy() {
        let home = Home::new();
        let content = session_lines(200);
        let compressed =
            compress_to_vec(content.as_bytes(), CompressionLevel::Fastest);
        home.write(".alpha/plain.jsonl", content.as_bytes());
        home.write(".alpha/packed.jsonl.zst", &compressed);

        let plain = home.read_all(".alpha/plain.jsonl", 300, &[USAGE]);
        let packed = home.read_all(".alpha/packed.jsonl.zst", 300, &[USAGE]);

        assert!(packed.len() > 2, "several batches: {}", packed.len());
        assert_eq!(texts(&packed), texts(&plain));
        assert_eq!(skipped(&packed), skipped(&plain));
        let last = packed.last().expect("a batch");
        assert_eq!(last.next_offset, content.len() as u64);
    }

    /// A session list reads the first line of many zstd files in one call
    /// batch: the decoder of one file is reset for the next, and each
    /// gives its own first line, also one longer than a read.
    #[test]
    fn the_first_lines_of_several_zstd_files_are_each_their_own() {
        let home = Home::new();
        let long_line = format!("{{\"meta\":\"{}\"}}\n", "x".repeat(5000));
        let files = [
            ("a", format!("{long_line}{}", session_lines(20))),
            ("b", format!("{{\"meta\":\"b\"}}\n{}", session_lines(5))),
            ("c", format!("{long_line}{}", session_lines(3))),
        ];
        for (name, content) in &files {
            let compressed =
                compress_to_vec(content.as_bytes(), CompressionLevel::Fastest);
            home.write(&format!(".alpha/{name}.jsonl.zst"), &compressed);
        }
        let mut cursor = ZstdCursor::default();

        let batches: Vec<Batch> = files
            .iter()
            .map(|(name, _)| {
                let path = format!(".alpha/{name}.jsonl.zst");
                home.read(&mut cursor, &path, 0, 100, &[]).expect("a batch")
            })
            .collect();

        for ((name, content), batch) in files.iter().zip(&batches) {
            let first_line = content.find('\n').expect("a first line") + 1;
            let read = batch.lines.len();
            assert!(read >= first_line, "{name}: the first line is whole");
            assert_eq!(batch.lines, content.as_bytes()[..read], "{name}");
            assert_eq!(batch.next_offset, read as u64, "{name}");
        }
    }

    #[test]
    fn a_zstd_file_read_from_an_offset_with_no_open_decoder_goes_on_alike() {
        let home = Home::new();
        let content = session_lines(60);
        let compressed =
            compress_to_vec(content.as_bytes(), CompressionLevel::Fastest);
        home.write(".alpha/packed.zst", &compressed);
        let mut going_on = ZstdCursor::default();
        let first = home
            .read(&mut going_on, ".alpha/packed.zst", 0, 200, &[])
            .expect("a batch");

        let next = home
            .read(
                &mut going_on,
                ".alpha/packed.zst",
                first.next_offset,
                200,
                &[],
            )
            .expect("a batch");
        let fresh = home
            .read(
                &mut ZstdCursor::default(),
                ".alpha/packed.zst",
                first.next_offset,
                200,
                &[],
            )
            .expect("a batch");

        assert_eq!(next.lines, fresh.lines);
        assert_eq!(next.next_offset, fresh.next_offset);
        let read_so_far = first.lines.len() + next.lines.len();
        assert_eq!(
            &content.as_bytes()[..read_so_far],
            [first.lines, next.lines].concat()
        );
    }

    #[test]
    fn a_line_longer_than_the_longest_is_left_out() {
        let content =
            [b"short\n".to_vec(), vec![b'x'; 50], b"\nafter\n".to_vec()]
                .concat();
        let mut reader = content.as_slice();

        let (batch, ahead) =
            next_batch(&mut reader, Vec::new(), 0, 4, &Markers::new(&[]), 8)
                .expect("a batch");
        let (rest, ahead) = next_batch(
            &mut reader,
            ahead,
            batch.next_offset,
            4,
            &Markers::new(&[]),
            8,
        )
        .expect("a batch");
        let (end, _) = next_batch(
            &mut reader,
            ahead,
            rest.next_offset,
            4,
            &Markers::new(&[]),
            8,
        )
        .expect("a batch");

        assert_eq!(batch.lines, b"short\n");
        assert_eq!(rest.lines, b"after\n");
        assert_eq!(rest.skipped, 1);
        assert_eq!(rest.next_offset, content.len() as u64);
        assert!(end.lines.is_empty());
        assert!(end.at_end);
    }

    #[test]
    fn an_offset_past_the_end_gives_an_empty_last_batch() {
        let home = Home::new();
        home.write(".alpha/a.jsonl", b"one\n");

        let batch = home
            .read(&mut ZstdCursor::default(), ".alpha/a.jsonl", 100, 64, &[])
            .expect("a batch");

        assert!(batch.lines.is_empty());
        assert!(batch.at_end);
        assert_eq!(batch.next_offset, 100);
    }

    #[test]
    fn files_outside_the_globs_and_key_files_are_refused() {
        let home = Home::new();
        home.write(".beta/a.jsonl", b"one\n");
        home.write(".alpha/auth.json", b"{}\n");
        let mut cursor = ZstdCursor::default();

        for relative in [".beta/a.jsonl", ".alpha/auth.json"] {
            let refused = home.read(&mut cursor, relative, 0, 64, &[]);

            assert_eq!(
                debug_err(refused).map(|batch| batch.lines),
                debug_err(Err(not_allowed(Path::new(&home.path(relative)))))
            );
        }
    }
}
