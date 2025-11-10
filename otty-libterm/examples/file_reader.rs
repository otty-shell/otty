//! Minimal example that drives [`Terminal`] without the [`Runtime`] loop.
//!
//! Instead of wiring a PTY session into Mio, this demo streams the contents
//! of an arbitrary file through the escape parser and prints the resulting
//! surface snapshot. Run it with:
//!
//! ```bash
//! cargo run -p otty-libterm --example file_reader -- <path> [columns rows]
//! ```
//!
//! Columns/rows default to `80x24` if not specified.

use std::{
    collections::VecDeque,
    env,
    fs::File,
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
    process::ExitStatus,
};

use anyhow::{Context, Result};
use otty_libterm::{
    Terminal, TerminalOptions, TerminalSnapshot, escape,
    pty::{PtySize, Session, SessionError},
    surface::{GridRow, Surface, SurfaceConfig},
};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let path = PathBuf::from(
        args.next()
            .context("usage: file_reader <path> [columns rows]")?,
    );
    let columns = parse_dimension(args.next(), 80, "columns")?;
    let rows = parse_dimension(args.next(), 24, "rows")?;

    let session = FileSession::open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let surface = Surface::new(SurfaceConfig {
        columns,
        rows,
        ..SurfaceConfig::default()
    });
    let parser: escape::Parser<escape::vte::Parser> = Default::default();
    let mut terminal =
        Terminal::new(session, surface, parser, TerminalOptions::default())?;

    while terminal.poll_session()? {}

    let snapshot = terminal.snapshot();
    println!(
        "\nRendered snapshot for {} ({}x{}, cursor: {}:{})\n",
        path.display(),
        snapshot.surface.columns,
        snapshot.surface.rows,
        snapshot.surface.cursor_row,
        snapshot.surface.cursor_col,
    );
    dump_surface(&snapshot);
    Ok(())
}

fn parse_dimension(
    value: Option<String>,
    fallback: usize,
    name: &str,
) -> Result<usize> {
    match value {
        Some(raw) => Ok(raw
            .parse::<usize>()
            .with_context(|| format!("invalid {name} value: {raw}"))?),
        None => Ok(fallback),
    }
}

fn dump_surface(snapshot: &TerminalSnapshot) {
    let rows: Vec<_> = snapshot.surface.grid.display_iter().collect();
    let start = rows
        .iter()
        .position(|row| !is_row_blank(row))
        .unwrap_or(rows.len());

    for row in rows.into_iter().skip(start) {
        let line: String = row.cells.iter().map(|cell| cell.ch).collect();
        println!("{}", line.trim_end());
    }
}

fn is_row_blank(row: &GridRow) -> bool {
    row.cells.iter().all(|cell| cell.is_blank())
}

/// Extremely small [`Session`] implementation that feeds data from a file.
struct FileSession {
    reader: BufReader<File>,
    pending: VecDeque<u8>,
    last_byte: Option<u8>,
}

impl FileSession {
    fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
            pending: VecDeque::new(),
            last_byte: None,
        })
    }

    fn maybe_refill(&mut self) -> io::Result<()> {
        if !self.pending.is_empty() {
            return Ok(());
        }

        let mut buf = [0u8; 4096];
        let count = self.reader.read(&mut buf)?;
        for &byte in &buf[..count] {
            if byte == b'\n' && self.last_byte != Some(b'\r') {
                self.pending.push_back(b'\r');
            }
            self.pending.push_back(byte);
            self.last_byte = Some(byte);
        }

        Ok(())
    }
}

impl Session for FileSession {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError> {
        self.maybe_refill()?;
        if self.pending.is_empty() {
            return Ok(0);
        }

        let mut written = 0usize;
        while written < buf.len() {
            match self.pending.pop_front() {
                Some(byte) => {
                    buf[written] = byte;
                    written += 1;
                },
                None => break,
            }
        }

        Ok(written)
    }

    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError> {
        // The file-backed session is read-only; pretend the write succeeded so
        // callers scanning escape logs can reuse the same interface.
        Ok(input.len())
    }

    fn resize(&mut self, _size: PtySize) -> Result<(), SessionError> {
        Ok(())
    }

    fn close(&mut self) -> Result<i32, SessionError> {
        Ok(0)
    }

    fn try_get_child_exit_status(
        &mut self,
    ) -> Result<Option<ExitStatus>, SessionError> {
        Ok(None)
    }
}
