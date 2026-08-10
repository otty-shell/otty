use std::sync::Arc;

use crate::index::{Column, Point};
use crate::{Dimensions, Flags, Surface};

/// Provenance of canonical command text retained by a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandSource {
    /// Text captured by the OTTY input path before Enter was sent.
    InputBuffer,
    /// Text supplied by the shell command-start hook.
    ShellIntegration,
    /// Text reconstructed from rendered command echo cells.
    CommandEcho,
}

/// Canonical command text with explicit provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRecord {
    text: String,
    source: CommandSource,
}

impl CommandRecord {
    /// Return canonical command text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Return the source used to obtain command text.
    pub fn source(&self) -> CommandSource {
        self.source
    }

    /// Record command text supplied by a shell lifecycle event.
    pub fn from_shell(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            source: CommandSource::ShellIntegration,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct BlockContent {
    prompt_start: Option<Point>,
    prompt_text: Option<Arc<str>>,
    header_text: Option<Arc<str>>,
    output_text: Option<Arc<str>>,
    command: Option<CommandRecord>,
    output_starts_on_new_line: bool,
}

impl BlockContent {
    pub(super) fn prompt_text(&self) -> Option<Arc<str>> {
        self.prompt_text.clone()
    }

    pub(super) fn output_text(&self) -> Option<Arc<str>> {
        self.output_text.clone()
    }

    pub(super) fn estimated_text_bytes(&self) -> usize {
        let arc_bytes = |text: &Option<Arc<str>>| {
            text.as_ref().map_or(0, |text| text.len())
        };

        arc_bytes(&self.prompt_text)
            + arc_bytes(&self.header_text)
            + arc_bytes(&self.output_text)
            + self
                .command
                .as_ref()
                .map_or(0, |command| command.text.len())
    }

    pub(super) fn mark_prompt_start(&mut self, point: Point) {
        self.prompt_start = Some(point);
    }

    pub(super) fn mark_prompt_end(&mut self, surface: &Surface, end: Point) {
        let Some(start) = self.prompt_start.take() else {
            return;
        };

        self.prompt_text = extract_range(surface, start, end).map(Arc::from);
    }

    pub(super) fn capture_header(
        &mut self,
        surface: &Surface,
        command: Option<&str>,
    ) {
        self.header_text = surface_text(surface).map(Arc::from);
        self.output_starts_on_new_line =
            surface.grid().cursor.point.column.0 == 0;
        if let Some(command) = command {
            self.command = Some(CommandRecord::from_shell(command));
        }
    }

    pub(super) fn freeze(&mut self, whole_text: Option<&Arc<str>>) {
        let (Some(whole_text), Some(header_text)) =
            (whole_text, self.header_text.as_ref())
        else {
            return;
        };
        let Some(output) = whole_text.strip_prefix(header_text.as_ref()) else {
            return;
        };

        let output = if self.output_starts_on_new_line {
            output.strip_prefix('\n').unwrap_or(output)
        } else {
            output
        };
        self.output_text = (!output.is_empty()).then(|| Arc::from(output));
    }
}

pub(super) fn surface_text(surface: &Surface) -> Option<String> {
    let grid = surface.grid();
    let columns = surface.columns();
    if columns == 0 {
        return None;
    }

    let history_lines = grid.history_size();
    let screen_lines = grid.screen_lines();
    let start = grid.topmost_line().0;
    let end = screen_lines as i32;
    let mut lines = Vec::with_capacity(history_lines + screen_lines);
    for line_value in start..end {
        let mut buffer = String::with_capacity(columns);
        for column in 0..columns {
            let cell = &grid[crate::Line(line_value)][Column(column)];
            if !cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                buffer.push(cell.c);
            }
        }
        lines.push(buffer.trim_end_matches(' ').to_string());
    }

    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    let text = lines.join("\n");
    (!text.is_empty()).then_some(text)
}

fn extract_range(
    surface: &Surface,
    start: Point,
    end: Point,
) -> Option<String> {
    if end < start || surface.columns() == 0 {
        return None;
    }

    let grid = surface.grid();
    let mut lines = Vec::new();
    for line_value in start.line.0..=end.line.0 {
        let first_column = if line_value == start.line.0 {
            start.column.0
        } else {
            0
        };
        let end_column = if line_value == end.line.0 {
            end.column.0
        } else {
            surface.columns()
        };
        if first_column >= end_column {
            continue;
        }

        let mut buffer = String::with_capacity(end_column - first_column);
        for column in first_column..end_column {
            let cell = &grid[crate::Line(line_value)][Column(column)];
            if !cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                buffer.push(cell.c);
            }
        }
        if line_value == end.line.0 {
            lines.push(buffer);
        } else {
            lines.push(buffer.trim_end_matches(' ').to_string());
        }
    }

    let text = lines.join("\n");
    (!text.is_empty()).then_some(text)
}
