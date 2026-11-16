use std::cmp::min;

use iced::keyboard::Modifiers;
use iced_core::Size;
use otty_libterm::surface::{
    BlockKey, Column, Line, Point, Scroll, SelectionType, Side, SnapshotOwned,
    SurfaceMode,
};
use otty_libterm::{TerminalRequest, TerminalSize};

use crate::program::TerminalProgram;
use crate::row_layout::{RowLayout, is_at_the_top};

#[derive(Debug, Clone)]
pub enum MouseMode {
    Sgr,
    Normal(bool),
}

impl From<SurfaceMode> for MouseMode {
    fn from(term_mode: SurfaceMode) -> Self {
        if term_mode.contains(SurfaceMode::SGR_MOUSE) {
            MouseMode::Sgr
        } else if term_mode.contains(SurfaceMode::UTF8_MOUSE) {
            MouseMode::Normal(true)
        } else {
            MouseMode::Normal(false)
        }
    }
}

#[derive(Debug, Clone)]
pub enum MouseButton {
    LeftButton = 0,
    MiddleButton = 1,
    RightButton = 2,
    LeftMove = 32,
    MiddleMove = 33,
    RightMove = 34,
    NoneMove = 35,
    ScrollUp = 64,
    ScrollDown = 65,
    Other = 99,
}

/// The widget's side of a terminal: the snapshot it shows, the size of its
/// grid, where the view is past the top of the history, and the requests it
/// sends to the program behind it.
///
/// The step past the top is the widget's own: the model of the program
/// counts lines and knows nothing of the gaps between the blocks. At the
/// top of the history (or in a history that can not scroll) the gaps may
/// cut the first block; the user scrolling further up then takes one step
/// past the top, and the rows are laid out from the top edge (see
/// [`RowLayout`]). Scrolling down, a scroll to an offset, typing, a resize
/// and new lines in the history leave it.
pub struct Engine {
    terminal_size: TerminalSize,
    layout_size: Size,
    program: Box<dyn TerminalProgram>,
    snapshot: SnapshotOwned,
    snapshot_version: u64,
    /// Whether the user asked to go one step past the top of the history.
    /// It shows only while the view is at the top of the history and the
    /// gaps cut the first block there.
    is_past_the_top_asked: bool,
    /// The rows of the snapshot shown now, laid out for the cell height and
    /// the step past the top. They are laid out again when one of these
    /// changes, see [`Engine::lay_out_rows`].
    rows: RowLayout,
}

impl Engine {
    pub fn terminal_size(&self) -> TerminalSize {
        self.terminal_size
    }

    /// Borrow the snapshot the widget shows now.
    pub fn snapshot(&self) -> &SnapshotOwned {
        &self.snapshot
    }

    /// Return the version of the snapshot the widget shows now. It changes
    /// with every new snapshot, so the view can tell that the content under
    /// the pointer may have moved.
    pub(crate) fn snapshot_version(&self) -> u64 {
        self.snapshot_version
    }

    /// Whether the pane shows the view one step past the top of the
    /// history now, with the rows laid out from the top edge.
    pub(crate) fn is_past_the_top(&self) -> bool {
        self.rows.is_past_the_top()
    }

    /// The rows of the snapshot shown now, with the gaps around the blocks
    /// and the step past the top of the history when the view takes it.
    /// Everything that turns a row into a pixel or back uses it.
    pub(crate) fn row_layout(&self) -> &RowLayout {
        &self.rows
    }

    /// Build the engine of a terminal `size` big whose program is reached
    /// through `program`. It shows an empty screen until the first snapshot
    /// is taken.
    pub(crate) fn new(
        program: Box<dyn TerminalProgram>,
        size: TerminalSize,
    ) -> Self {
        let snapshot = SnapshotOwned::default();
        let rows = RowLayout::of_view(
            &snapshot.view(),
            f32::from(size.cell_height),
            false,
        );

        Self {
            terminal_size: size,
            layout_size: Size::default(),
            program,
            snapshot,
            snapshot_version: 0,
            is_past_the_top_asked: false,
            rows,
        }
    }

    /// Show the newest snapshot of the program, if it has one that was not
    /// taken yet. Returns whether the shown snapshot changed. New lines in
    /// the history (output, a fold) move the view, so they leave the step
    /// past the top; a snapshot with the same lines keeps it.
    pub(crate) fn sync_newest_snapshot(&mut self) -> bool {
        let Some(snapshot) = self.program.take_snapshot() else {
            return false;
        };

        if history_lines(&snapshot) != history_lines(&self.snapshot) {
            self.is_past_the_top_asked = false;
        }

        self.snapshot = snapshot;
        self.snapshot_version = self.snapshot_version.wrapping_add(1);
        self.lay_out_rows();

        true
    }

    /// Lay out the rows of the snapshot shown now again, for the cell
    /// height and the step past the top the engine has now. Every change of
    /// one of them, or of the snapshot, ends here.
    fn lay_out_rows(&mut self) {
        self.rows = RowLayout::of_view(
            &self.snapshot.view(),
            f32::from(self.terminal_size.cell_height),
            self.is_past_the_top_asked,
        );
    }

    /// Note whether the user asks for the step past the top of the history,
    /// and lay the rows out for it.
    fn ask_past_the_top(&mut self, is_asked: bool) {
        self.is_past_the_top_asked = is_asked;
        self.lay_out_rows();
    }

    /// Pass `request` to the program behind the terminal.
    fn send(&self, request: TerminalRequest) {
        self.program.send(request);
    }

    pub(crate) fn process_mouse_report(
        &self,
        button: MouseButton,
        modifiers: Modifiers,
        point: Point,
        pressed: bool,
    ) {
        let mut mods = 0;
        if modifiers.contains(Modifiers::SHIFT) {
            mods += 4;
        }
        if modifiers.contains(Modifiers::ALT) {
            mods += 8;
        }
        if modifiers.contains(Modifiers::COMMAND) {
            mods += 16;
        }

        match MouseMode::from(self.snapshot.view().mode) {
            MouseMode::Sgr => {
                self.sgr_mouse_report(point, button as u8 + mods, pressed)
            },
            MouseMode::Normal(is_utf8) => {
                if pressed {
                    self.normal_mouse_report(
                        point,
                        button as u8 + mods,
                        is_utf8,
                    )
                } else {
                    self.normal_mouse_report(point, 3 + mods, is_utf8)
                }
            },
        }
    }

    fn sgr_mouse_report(&self, point: Point, button: u8, pressed: bool) {
        let c = if pressed { 'M' } else { 'm' };

        let msg = format!(
            "\x1b[<{};{};{}{}",
            button,
            point.column + 1,
            point.line + 1,
            c
        )
        .as_bytes()
        .to_vec();

        self.send(TerminalRequest::WriteBytes(msg));
    }

    fn normal_mouse_report(&self, point: Point, button: u8, is_utf8: bool) {
        let Point { line, column } = point;
        let max_point = if is_utf8 { 2015 } else { 223 };

        if line >= max_point || column >= max_point {
            return;
        }

        let mut msg = vec![b'\x1b', b'[', b'M', 32 + button];

        let mouse_pos_encode = |pos: usize| -> Vec<u8> {
            let pos = 32 + 1 + pos;
            let first = 0xC0 + pos / 64;
            let second = 0x80 + (pos & 63);
            vec![first as u8, second as u8]
        };

        if is_utf8 && column >= Column(95) {
            msg.append(&mut mouse_pos_encode(column.0));
        } else {
            msg.push(32 + 1 + column.0 as u8);
        }

        if is_utf8 && line >= 95 {
            msg.append(&mut mouse_pos_encode(line.0 as usize));
        } else {
            msg.push(32 + 1 + line.0 as u8);
        }

        self.send(TerminalRequest::WriteBytes(msg));
    }

    pub(crate) fn start_selection(
        &mut self,
        selection_type: SelectionType,
        x: f32,
        y: f32,
    ) {
        let location = self.selection_point(x, y);

        self.send(TerminalRequest::StartSelection {
            ty: selection_type,
            point: location,
            direction: self.selection_side(x),
        });
    }

    /// Ask the program to fold the block with `block_key`, or to unfold it
    /// when it is folded.
    pub(crate) fn toggle_block_fold(&self, block_key: BlockKey) {
        self.send(TerminalRequest::ToggleBlockFold { block_key });
    }

    pub(crate) fn update_selection(&mut self, x: f32, y: f32) {
        let location = self.selection_point(x, y);
        self.send(TerminalRequest::UpdateSelection {
            point: location,
            direction: self.selection_side(x),
        });
    }

    /// The cell under a pointer `x` px right of the left edge of the grid
    /// and `y` px below its top, where the rows shown now lie, with the
    /// space around the blocks: a pointer in a gap is over the nearest row
    /// of the block the gap belongs to. A pointer past the grid is over its
    /// last column or its bottom row.
    pub(crate) fn selection_point(&self, x: f32, y: f32) -> Point {
        let size = &self.terminal_size;
        let col = (x as usize) / (size.cell_width as usize);
        let col = min(Column(col), Column(size.cols as usize - 1));

        let bottom_line = i32::from(size.rows) - 1 - self.rows.display_offset();
        let line = self.rows.line_at(y).min(bottom_line);

        Point::new(Line(line), col)
    }

    /// The half of a cell a pointer `x` px from the left edge of the grid is
    /// over. The pane is wider than the grid, so the pointer can be past
    /// the last column: there it is at the right side of that column.
    fn selection_side(&self, x: f32) -> Side {
        let grid_width = f32::from(self.terminal_size.cols)
            * f32::from(self.terminal_size.cell_width);
        if x >= grid_width {
            return Side::Right;
        }

        let cell_x = x as usize % self.terminal_size.cell_width as usize;
        let half_cell_width =
            (self.terminal_size.cell_width as f32 / 2.0) as usize;

        if cell_x > half_cell_width {
            Side::Right
        } else {
            Side::Left
        }
    }

    /// Fit the grid to the area of `layout_size` with cells of
    /// `font_measure`, either of them kept when `None`, and tell the program
    /// the new grid. The rows move, so the view leaves the step past the
    /// top of the history.
    pub(crate) fn resize(
        &mut self,
        layout_size: Option<Size<f32>>,
        font_measure: Option<Size<f32>>,
    ) {
        if let Some(size) = layout_size {
            self.layout_size.height = size.height;
            self.layout_size.width = size.width;
        };

        if let Some(size) = font_measure {
            self.terminal_size.cell_height = size.height as u16;
            self.terminal_size.cell_width = size.width as u16;
        }
        self.ask_past_the_top(false);

        let lines = (self.layout_size.height
            / self.terminal_size.cell_height as f32)
            .floor() as u16;
        let cols = (self.layout_size.width
            / self.terminal_size.cell_width as f32)
            .floor() as u16;
        if lines > 0 && cols > 0 {
            self.terminal_size.rows = lines;
            self.terminal_size.cols = cols;
            self.send(TerminalRequest::Resize(self.terminal_size));
        }
    }

    /// Send typed `input` to the program and show the bottom of the
    /// history, where it goes: the view leaves the step past the top.
    pub(crate) fn write(&mut self, input: Vec<u8>) {
        self.ask_past_the_top(false);

        self.send(TerminalRequest::WriteBytes(input));
        self.sroll_bottom();
    }

    fn sroll_bottom(&self) {
        self.send(TerminalRequest::ScrollDisplay(Scroll::Bottom));
    }

    /// Ask the program to show the history `offset` lines up from its
    /// bottom, or its top when it is shorter, with the bottom row at the
    /// bottom edge: the view leaves the step past the top. A drag of the
    /// scroll mark asks for the place it reached on every move, so the
    /// request names the place and not a step: the snapshot the widget
    /// shows may not have caught up with the last move yet.
    pub(crate) fn scroll_to_offset(&mut self, offset: usize) {
        self.ask_past_the_top(false);

        self.send(TerminalRequest::ScrollDisplay(Scroll::Offset(offset)));
    }

    /// Show the top of the history and one step past it: the scroll mark
    /// at the top of its way, or a jump to a block whose start shows only
    /// there. The program is asked for the top of the history unless the
    /// view is there already; the step shows once it is, when the gaps cut
    /// the first block there.
    pub(crate) fn scroll_past_the_top(&mut self) {
        self.ask_past_the_top(true);

        if !is_at_the_top(&self.snapshot.view()) {
            self.send(TerminalRequest::ScrollDisplay(Scroll::Top));
        }
    }

    /// Scroll the history by `delta_value` lines, up when it is positive,
    /// as the wheel does. A full-screen program that asked for it gets the
    /// lines as arrow keys instead.
    ///
    /// At the top of the history, when the gaps cut the first block there,
    /// a scroll up takes the step past the top and asks the program for
    /// nothing; past the top a scroll up goes no further. A scroll down
    /// leaves the step: its first line undoes it, and the rest scroll the
    /// history.
    pub(crate) fn scroll_delta(&mut self, delta_value: i32) {
        if delta_value == 0 {
            return;
        }

        // A full-screen program that asked for it gets the wheel as arrow
        // keys.
        let view = self.snapshot.view();
        if view.is_full_screen()
            && view.mode.contains(SurfaceMode::ALTERNATE_SCROLL)
        {
            self.send_arrow_keys(delta_value);
            return;
        }

        if delta_value > 0 {
            self.scroll_up(delta_value);
        } else {
            self.scroll_down(delta_value);
        }
    }

    /// Scroll the history up by `lines` lines. At the top of the history,
    /// when the gaps cut the first block there, take the step past the top
    /// instead; past the top go no further.
    fn scroll_up(&mut self, lines: i32) {
        let cell_height = f32::from(self.terminal_size.cell_height);
        if self.is_past_the_top() {
            return;
        }

        if RowLayout::can_go_past_the_top(&self.snapshot.view(), cell_height) {
            self.ask_past_the_top(true);
            return;
        }

        self.send(TerminalRequest::ScrollDisplay(Scroll::Delta(lines)));
    }

    /// Scroll the history down by `-delta_value` lines. Past the top the
    /// first line leaves the step and the rest scroll the history.
    fn scroll_down(&mut self, delta_value: i32) {
        let was_past_the_top = self.is_past_the_top();

        self.ask_past_the_top(false);

        let delta = if was_past_the_top {
            delta_value + 1
        } else {
            delta_value
        };
        if delta != 0 {
            self.send(TerminalRequest::ScrollDisplay(Scroll::Delta(delta)));
        }
    }

    /// Send `delta_value` lines of the wheel to a full-screen program as
    /// arrow keys: up for a positive one, down for a negative one.
    fn send_arrow_keys(&self, delta_value: i32) {
        let line_cmd = if delta_value > 0 { b'A' } else { b'B' };
        let mut content = vec![];

        for _ in 0..delta_value.abs() {
            content.push(0x1b);
            content.push(b'O');
            content.push(line_cmd);
        }

        self.send(TerminalRequest::WriteBytes(content));
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.send(TerminalRequest::Shutdown);
    }
}

/// How many lines the history of `snapshot` has, on screen or not: the
/// lines of all its blocks.
fn history_lines(snapshot: &SnapshotOwned) -> usize {
    snapshot
        .view()
        .blocks()
        .iter()
        .map(|block| block.history_line_count)
        .sum()
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;
    use otty_libterm::surface::Line;

    use super::*;
    use crate::program::MockTerminalProgram;
    use crate::testing::engine_showing;

    /// An engine over `program`. Dropping an engine shuts its program down,
    /// so that last request is expected here once, after `requests`.
    fn engine_over(
        mut program: MockTerminalProgram,
        requests: &mut Sequence,
    ) -> Engine {
        program
            .expect_send()
            .withf(|request| matches!(request, TerminalRequest::Shutdown))
            .times(1)
            .in_sequence(requests)
            .return_const(());

        Engine::new(Box::new(program), TerminalSize::default())
    }

    /// The snapshot of a screen of `size` with `lines_up` lines of
    /// history above it, scrolled up by all of them.
    fn scrolled_up_by(lines_up: usize, size: TerminalSize) -> SnapshotOwned {
        use otty_libterm::surface::{
            Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
        };

        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        for _ in 1..usize::from(size.rows) + lines_up {
            surface.line_feed();
        }
        surface.scroll_display(Scroll::Delta(lines_up as i32));

        surface.snapshot_owned()
    }

    /// A snapshot `columns` wide, to tell snapshots apart.
    fn snapshot_of_width(columns: u16) -> SnapshotOwned {
        use otty_libterm::surface::{Surface, SurfaceConfig, SurfaceModel};

        let size = TerminalSize {
            cols: columns,
            ..TerminalSize::default()
        };

        Surface::new(SurfaceConfig::default(), &size).snapshot_owned()
    }

    #[test]
    fn sync_newest_snapshot_shows_the_snapshot_the_program_has() {
        let mut program = MockTerminalProgram::new();
        program
            .expect_take_snapshot()
            .times(1)
            .return_once(|| Some(snapshot_of_width(90)));
        let mut engine = engine_over(program, &mut Sequence::new());
        assert_ne!(engine.snapshot().view().size.columns, 90);

        let changed = engine.sync_newest_snapshot();

        assert!(changed);
        assert_eq!(engine.snapshot().view().size.columns, 90);
    }

    #[test]
    fn sync_newest_snapshot_keeps_the_shown_one_when_the_program_has_nothing_new()
     {
        let mut waiting = Some(snapshot_of_width(90));
        let mut program = MockTerminalProgram::new();
        program
            .expect_take_snapshot()
            .times(2)
            .returning(move || waiting.take());
        let mut engine = engine_over(program, &mut Sequence::new());
        assert!(engine.sync_newest_snapshot());

        let changed = engine.sync_newest_snapshot();

        assert!(!changed);
        assert_eq!(engine.snapshot().view().size.columns, 90);
    }

    /// The view looks up the block under the pointer again when the version
    /// changes, so it changes with every new snapshot and only then.
    #[test]
    fn every_new_snapshot_gets_a_new_version() {
        let mut waiting =
            vec![snapshot_of_width(90), snapshot_of_width(91)].into_iter();
        let mut program = MockTerminalProgram::new();
        program
            .expect_take_snapshot()
            .times(3)
            .returning(move || waiting.next());
        let mut engine = engine_over(program, &mut Sequence::new());
        let empty_screen = engine.snapshot_version();

        engine.sync_newest_snapshot();
        let first = engine.snapshot_version();
        engine.sync_newest_snapshot();
        let second = engine.snapshot_version();
        engine.sync_newest_snapshot();

        assert_ne!(first, empty_screen);
        assert_ne!(second, first);
        assert_eq!(engine.snapshot_version(), second);
    }

    #[test]
    fn write_sends_the_input_and_scrolls_to_the_bottom() {
        let mut requests = Sequence::new();
        let mut program = MockTerminalProgram::new();
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::WriteBytes(bytes) if bytes == b"ls\n"
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::ScrollDisplay(Scroll::Bottom)
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        let mut engine = engine_over(program, &mut requests);

        engine.write(b"ls\n".to_vec());
    }

    #[test]
    fn resize_sends_the_grid_that_fits_the_layout() {
        let mut requests = Sequence::new();
        let mut program = MockTerminalProgram::new();
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::Resize(size)
                        if size.cols == 100
                            && size.rows == 25
                            && size.cell_width == 8
                            && size.cell_height == 16
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        let mut engine = engine_over(program, &mut requests);

        engine
            .resize(Some(Size::new(800.0, 400.0)), Some(Size::new(8.0, 16.0)));

        assert_eq!(engine.terminal_size().cols, 100);
        assert_eq!(engine.terminal_size().rows, 25);
    }

    #[test]
    fn resize_to_less_than_one_cell_sends_nothing() {
        let program = MockTerminalProgram::new();
        let mut engine = engine_over(program, &mut Sequence::new());
        let size_before = engine.terminal_size();

        engine.resize(Some(Size::new(4.0, 4.0)), Some(Size::new(8.0, 16.0)));

        assert_eq!(engine.terminal_size().cols, size_before.cols);
        assert_eq!(engine.terminal_size().rows, size_before.rows);
    }

    #[test]
    fn scroll_delta_scrolls_the_display() {
        let mut requests = Sequence::new();
        let mut program = MockTerminalProgram::new();
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::ScrollDisplay(Scroll::Delta(3))
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        let mut engine = engine_over(program, &mut requests);

        engine.scroll_delta(3);
        // No lines to scroll by is no request.
        engine.scroll_delta(0);
    }

    #[test]
    fn scroll_to_offset_puts_the_display_that_many_lines_up() {
        let mut requests = Sequence::new();
        let mut program = MockTerminalProgram::new();
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::ScrollDisplay(Scroll::Offset(12))
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        let mut engine = engine_over(program, &mut requests);

        engine.scroll_to_offset(12);
    }

    #[test]
    fn a_wheel_report_is_written_in_the_sgr_format() {
        use otty_libterm::escape::NamedPrivateMode;
        use otty_libterm::surface::{
            Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
        };

        let size = TerminalSize::default();
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        surface.set_private_mode(NamedPrivateMode::ReportMouseClicks.into());
        surface.set_private_mode(NamedPrivateMode::SgrMouse.into());

        let mut requests = Sequence::new();
        let mut program = MockTerminalProgram::new();
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::WriteBytes(bytes)
                        if bytes == b"\x1b[<64;10;5M"
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        let mut engine = engine_over(program, &mut requests);
        engine.snapshot = surface.snapshot_owned();

        engine.process_mouse_report(
            MouseButton::ScrollUp,
            Modifiers::empty(),
            Point::new(Line(4), Column(9)),
            true,
        );
    }

    #[test]
    fn selection_requests_carry_the_cell_under_the_pointer() {
        let mut requests = Sequence::new();
        let mut program = MockTerminalProgram::new();
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::StartSelection {
                        ty: SelectionType::Simple,
                        point,
                        direction: Side::Left,
                    } if point.line == Line(0) && point.column == Column(0)
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::UpdateSelection {
                        point,
                        direction: Side::Right,
                    } if point.line == Line(1) && point.column == Column(2)
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        let mut engine = engine_over(program, &mut requests);
        engine.resize(None, Some(Size::new(8.0, 16.0)));

        engine.start_selection(SelectionType::Simple, 1.0, 1.0);
        engine.update_selection(22.0, 17.0);
    }

    /// The pane is wider than the grid: its text starts further in, and
    /// the pointer can be on either side of it.
    #[test]
    fn a_pointer_beside_the_grid_selects_the_nearest_column_from_its_outer_side()
     {
        let mut requests = Sequence::new();
        let mut program = MockTerminalProgram::new();
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::StartSelection {
                        ty: SelectionType::Simple,
                        point,
                        direction: Side::Left,
                    } if point.line == Line(0) && point.column == Column(0)
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::UpdateSelection {
                        point,
                        direction: Side::Right,
                    } if point.line == Line(0) && point.column == Column(79)
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        let mut engine = engine_over(program, &mut requests);
        engine.resize(None, Some(Size::new(8.0, 16.0)));
        let left_of_the_grid = -10.0;
        // The grid is 80 columns of 8 px. Two px past it would be the left
        // half of a column, if there were one.
        let right_of_the_grid = 642.0;

        engine.start_selection(SelectionType::Simple, left_of_the_grid, 1.0);
        engine.update_selection(right_of_the_grid, 1.0);
    }

    #[test]
    fn mouse_press_is_reported_to_the_program() {
        let mut requests = Sequence::new();
        let mut program = MockTerminalProgram::new();
        program
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::WriteBytes(bytes)
                        if bytes == b"\x1b[M\x20\x22\x22"
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        let engine = engine_over(program, &mut requests);

        engine.process_mouse_report(
            MouseButton::LeftButton,
            Modifiers::empty(),
            Point::new(Line(1), Column(1)),
            true,
        );
    }

    #[test]
    fn dropping_the_engine_shuts_its_program_down() {
        let program = MockTerminalProgram::new();
        let engine = engine_over(program, &mut Sequence::new());

        // The mock fails the test when it is dropped without the shutdown
        // request `engine_over` expects.
        drop(engine);
    }

    #[test]
    fn test_selection_point_basic() {
        let terminal_size = TerminalSize {
            cols: 80,
            rows: 24,
            cell_width: 8,
            cell_height: 16,
        };

        let engine = engine_showing(SnapshotOwned::default(), terminal_size);

        let point = engine.selection_point(16.0, 32.0);

        // x=16 / cell_width=8 = col 2
        // y=32 / cell_height=16 = line 2
        assert_eq!(point.column, Column(2));
        assert_eq!(point.line, Line(2));
    }

    #[test]
    fn test_selection_point_with_offset() {
        let terminal_size = TerminalSize {
            cols: 80,
            rows: 24,
            cell_width: 8,
            cell_height: 16,
        };

        let engine =
            engine_showing(scrolled_up_by(5, terminal_size), terminal_size);

        let point = engine.selection_point(0.0, 0.0);

        // The top row is 5 lines up in the history.
        assert_eq!(point.line, Line(-5));
        assert_eq!(point.column, Column(0));
    }

    #[test]
    fn test_selection_point_boundary_clamp() {
        let terminal_size = TerminalSize {
            cols: 80,
            rows: 24,
            cell_width: 8,
            cell_height: 16,
        };

        let engine = engine_showing(SnapshotOwned::default(), terminal_size);

        // Large coordinates should be clamped to terminal bounds
        let point = engine.selection_point(10000.0, 10000.0);

        assert!(point.column.0 < terminal_size.cols as usize);
        assert!(point.line.0 < terminal_size.rows as i32);
    }

    #[test]
    fn test_selection_point_at_zero() {
        let terminal_size = TerminalSize {
            cols: 80,
            rows: 24,
            cell_width: 8,
            cell_height: 16,
        };

        let engine = engine_showing(SnapshotOwned::default(), terminal_size);

        let point = engine.selection_point(0.0, 0.0);

        assert_eq!(point.column, Column(0));
    }

    /// Issue 07: three blocks of one row each at the bottom of 5 rows of
    /// 16 px. The gaps push the upper rows up, so the pointer finds the
    /// rows where the gaps put them, not on the line grid.
    #[test]
    fn a_pointer_over_a_block_pushed_up_by_the_gaps_is_over_its_row() {
        use otty_libterm::surface::{BlockKind, BlockSnapshot};

        let terminal_size = TerminalSize {
            cols: 80,
            rows: 5,
            cell_width: 8,
            cell_height: 16,
        };
        let mut snapshot = scrolled_up_by(0, terminal_size);
        for line in 2..5 {
            let mut block = BlockSnapshot {
                start_line: line,
                line_count: 1,
                ..BlockSnapshot::default()
            };
            block.meta.kind = BlockKind::Command;
            snapshot.blocks.push(block);
        }
        let engine = engine_showing(snapshot, terminal_size);

        // The rows are at 6, 35 and 64 px: 13 px between them. 25 and 55
        // are in the gaps below the first and the second block.
        let lines = [10.0, 25.0, 40.0, 55.0, 70.0]
            .map(|y| engine.selection_point(0.0, y).line);

        assert_eq!(lines, [Line(2), Line(2), Line(3), Line(3), Line(4)]);
    }

    /// The step past the top of the history: the view at the top of the
    /// history (or of a history that can not scroll) asked to go further
    /// up while the gaps cut the top of the first block.
    mod past_the_top_tests {
        use std::sync::{Arc, Mutex};

        use otty_libterm::surface::BlockKey;

        use super::*;
        use crate::row_layout::BLOCK_GAP;
        use crate::testing::{HistoryBlock, history_snapshot};

        /// The pane of the tests: 4 rows of 16 px, 64 px high.
        const ROWS: usize = 4;
        const PANE_HEIGHT: f32 = 64.0;
        const LS: BlockKey = BlockKey::new(1);
        const PWD: BlockKey = BlockKey::new(2);
        const SEQ: BlockKey = BlockKey::new(3);
        const PROMPT: BlockKey = BlockKey::new(4);

        /// A new terminal after `ls` and `pwd`: 4 lines, as many as the
        /// pane has rows, so the history can not scroll. The gaps push the
        /// status line of `ls` over the top edge.
        fn short_history_cut_at_the_top() -> SnapshotOwned {
            history_snapshot(
                ROWS,
                &[
                    HistoryBlock::command(LS, 2),
                    HistoryBlock::command(PWD, 1),
                    HistoryBlock::plain(PROMPT, 1),
                ],
                0,
            )
        }

        /// The same terminal after `pwd` printed one more line.
        fn short_history_with_one_line_more() -> SnapshotOwned {
            history_snapshot(
                ROWS,
                &[
                    HistoryBlock::command(LS, 2),
                    HistoryBlock::command(PWD, 2),
                    HistoryBlock::plain(PROMPT, 1),
                ],
                0,
            )
        }

        /// `ls` alone over the prompt: 2 lines under 2 empty rows. The
        /// gaps push only an empty row over the top edge.
        fn short_history_cut_nowhere() -> SnapshotOwned {
            history_snapshot(
                ROWS,
                &[HistoryBlock::command(LS, 1), HistoryBlock::plain(PROMPT, 1)],
                0,
            )
        }

        /// A history of 10 lines (`ls`, `seq` and the prompt) scrolled up
        /// by `offset` lines; its top is 6 lines up.
        fn long_history(offset: usize) -> SnapshotOwned {
            history_snapshot(
                ROWS,
                &[
                    HistoryBlock::command(LS, 3),
                    HistoryBlock::command(SEQ, 6),
                    HistoryBlock::plain(PROMPT, 1),
                ],
                offset,
            )
        }

        /// An engine of rows 16 px high that shows the first of
        /// `snapshots` and takes the next one on every sync, and the
        /// scroll requests it sends, written out.
        fn engine_recording_scrolls(
            snapshots: Vec<SnapshotOwned>,
        ) -> (Engine, Arc<Mutex<Vec<String>>>) {
            let sent = Arc::new(Mutex::new(Vec::new()));
            let recorded = Arc::clone(&sent);
            let mut program = MockTerminalProgram::new();
            program.expect_send().returning(move |request| {
                if let TerminalRequest::ScrollDisplay(scroll) = request {
                    let mut sent = recorded.lock().expect("the requests");
                    sent.push(format!("{scroll:?}"));
                }
            });
            let mut waiting = snapshots.into_iter();
            program
                .expect_take_snapshot()
                .returning(move || waiting.next());
            let size = TerminalSize {
                cell_width: 8,
                cell_height: 16,
                cols: 36,
                rows: 4,
            };

            let mut engine = Engine::new(Box::new(program), size);
            engine.sync_newest_snapshot();

            (engine, sent)
        }

        fn requests(sent: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
            sent.lock().expect("the requests").clone()
        }

        /// The bottom of the row of the grid line `line`.
        fn bottom_of(engine: &Engine, line: i32) -> f32 {
            engine.row_layout().line_top(line) + 16.0
        }

        #[test]
        fn in_a_short_history_cut_at_the_top_the_wheel_up_shows_the_first_block_whole()
         {
            let (mut engine, sent) =
                engine_recording_scrolls(vec![short_history_cut_at_the_top()]);
            assert!(engine.row_layout().line_top(0) < 0.0);

            engine.scroll_delta(1);

            assert!(engine.is_past_the_top());
            assert_eq!(engine.row_layout().line_top(0), BLOCK_GAP);
            // The history can not scroll: nothing is asked of the program.
            assert!(requests(&sent).is_empty());
        }

        #[test]
        fn past_the_top_the_wheel_down_brings_the_prompt_back_to_the_bottom_edge()
         {
            let (mut engine, sent) =
                engine_recording_scrolls(vec![short_history_cut_at_the_top()]);
            engine.scroll_delta(1);

            engine.scroll_delta(-1);

            assert!(!engine.is_past_the_top());
            assert_eq!(bottom_of(&engine, 3), PANE_HEIGHT);
            assert!(requests(&sent).is_empty());
        }

        #[test]
        fn past_the_top_the_wheel_up_goes_no_further() {
            let (mut engine, sent) =
                engine_recording_scrolls(vec![short_history_cut_at_the_top()]);
            engine.scroll_delta(1);

            engine.scroll_delta(3);

            assert!(engine.is_past_the_top());
            assert_eq!(engine.row_layout().line_top(0), BLOCK_GAP);
            assert!(requests(&sent).is_empty());
        }

        /// One line below the top the wheel scrolls the history, as
        /// everywhere: the rows keep to the bottom edge. Only the wheel at
        /// the top takes the step past it.
        #[test]
        fn in_a_long_history_the_step_past_the_top_comes_after_the_last_line() {
            let (mut engine, sent) = engine_recording_scrolls(vec![
                long_history(5),
                long_history(6),
            ]);

            engine.scroll_delta(1);
            let one_line_below = engine.is_past_the_top();
            engine.sync_newest_snapshot();
            let at_the_top = bottom_of(&engine, 3 - 6);
            engine.scroll_delta(1);

            assert!(!one_line_below);
            assert_eq!(at_the_top, PANE_HEIGHT);
            assert!(engine.is_past_the_top());
            assert_eq!(engine.row_layout().line_top(-6), BLOCK_GAP);
            assert_eq!(requests(&sent), ["Delta(1)"]);
        }

        /// The first wheel line down undoes the step past the top; the
        /// rest scrolls the history.
        #[test]
        fn past_the_top_the_wheel_down_by_three_lines_scrolls_the_history_by_two()
         {
            let (mut engine, sent) =
                engine_recording_scrolls(vec![long_history(6)]);
            engine.scroll_delta(1);

            engine.scroll_delta(-3);

            assert!(!engine.is_past_the_top());
            assert_eq!(requests(&sent), ["Delta(-2)"]);
        }

        #[test]
        fn with_nothing_cut_at_the_top_the_wheel_up_takes_no_step_past_it() {
            let (mut engine, sent) =
                engine_recording_scrolls(vec![short_history_cut_nowhere()]);
            let before = engine.row_layout().clone();

            engine.scroll_delta(1);

            assert!(!engine.is_past_the_top());
            assert_eq!(engine.row_layout(), &before);
            // As before: the history is asked, and can not scroll.
            assert_eq!(requests(&sent), ["Delta(1)"]);
        }

        #[test]
        fn new_output_past_the_top_brings_the_rows_back_to_the_bottom_edge() {
            let (mut engine, _sent) = engine_recording_scrolls(vec![
                short_history_cut_at_the_top(),
                short_history_with_one_line_more(),
            ]);
            engine.scroll_delta(1);

            engine.sync_newest_snapshot();

            assert!(!engine.is_past_the_top());
            assert_eq!(bottom_of(&engine, 3), PANE_HEIGHT);
        }

        /// A snapshot with the same lines, such as the clock of a running
        /// command, moves nothing.
        #[test]
        fn a_new_snapshot_with_the_same_lines_keeps_the_view_past_the_top() {
            let (mut engine, _sent) = engine_recording_scrolls(vec![
                short_history_cut_at_the_top(),
                short_history_cut_at_the_top(),
            ]);
            engine.scroll_delta(1);

            engine.sync_newest_snapshot();

            assert!(engine.is_past_the_top());
        }

        /// A jump to the first block, or the scroll mark at the top of its
        /// way, from the bottom of a long history: the history goes to its
        /// top, and the view is past it once the history is there.
        #[test]
        fn a_scroll_past_the_top_from_the_bottom_goes_to_the_top_and_past_it() {
            let (mut engine, sent) = engine_recording_scrolls(vec![
                long_history(0),
                long_history(6),
            ]);

            engine.scroll_past_the_top();
            let before_the_history_moved = engine.is_past_the_top();
            engine.sync_newest_snapshot();

            assert!(!before_the_history_moved);
            assert!(engine.is_past_the_top());
            assert_eq!(engine.row_layout().line_top(-6), BLOCK_GAP);
            assert_eq!(requests(&sent), ["Top"]);
        }

        #[test]
        fn a_scroll_past_the_top_of_a_short_history_asks_the_history_nothing() {
            let (mut engine, sent) =
                engine_recording_scrolls(vec![short_history_cut_at_the_top()]);

            engine.scroll_past_the_top();

            assert!(engine.is_past_the_top());
            assert!(requests(&sent).is_empty());
        }

        #[test]
        fn a_scroll_to_an_offset_leaves_the_step_past_the_top() {
            let (mut engine, sent) =
                engine_recording_scrolls(vec![long_history(6)]);
            engine.scroll_delta(1);

            engine.scroll_to_offset(6);

            assert!(!engine.is_past_the_top());
            assert_eq!(requests(&sent), ["Offset(6)"]);
        }

        #[test]
        fn typing_leaves_the_step_past_the_top() {
            let (mut engine, _sent) =
                engine_recording_scrolls(vec![short_history_cut_at_the_top()]);
            engine.scroll_delta(1);

            engine.write(b"l".to_vec());

            assert!(!engine.is_past_the_top());
        }

        #[test]
        fn a_resize_leaves_the_step_past_the_top() {
            let (mut engine, _sent) =
                engine_recording_scrolls(vec![short_history_cut_at_the_top()]);
            engine.scroll_delta(1);

            engine.resize(None, Some(Size::new(8.0, 16.0)));

            assert!(!engine.is_past_the_top());
        }

        /// A click past the top lands on the row the pane shows there.
        #[test]
        fn past_the_top_the_pointer_is_over_the_rows_laid_out_from_the_top() {
            let (mut engine, _sent) =
                engine_recording_scrolls(vec![short_history_cut_at_the_top()]);
            engine.scroll_delta(1);

            let point = engine.selection_point(0.0, BLOCK_GAP + 1.0);

            assert_eq!(point.line, Line(0));
        }
    }
}
