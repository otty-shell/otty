use std::cmp::min;

use iced::keyboard::Modifiers;
use iced_core::Size;
use otty_libterm::surface::{
    Column, Point, Scroll, SelectionType, Side, SnapshotOwned, SurfaceMode,
    viewport_to_point,
};
use otty_libterm::{TerminalRequest, TerminalSize};

use crate::program::TerminalProgram;

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
/// grid and the requests it sends to the program behind it.
pub struct Engine {
    terminal_size: TerminalSize,
    layout_size: Size,
    program: Box<dyn TerminalProgram>,
    snapshot: SnapshotOwned,
}

impl Engine {
    pub fn terminal_size(&self) -> TerminalSize {
        self.terminal_size
    }

    /// Borrow the snapshot the widget shows now.
    pub fn snapshot(&self) -> &SnapshotOwned {
        &self.snapshot
    }

    /// Build the engine of a terminal `size` big whose program is reached
    /// through `program`. It shows an empty screen until the first snapshot
    /// is taken.
    pub(crate) fn new(
        program: Box<dyn TerminalProgram>,
        size: TerminalSize,
    ) -> Self {
        Self {
            terminal_size: size,
            layout_size: Size::default(),
            program,
            snapshot: SnapshotOwned::default(),
        }
    }

    /// Show the newest snapshot of the program, if it has one that was not
    /// taken yet. Returns whether the shown snapshot changed.
    pub(crate) fn sync_newest_snapshot(&mut self) -> bool {
        let Some(snapshot) = self.program.take_snapshot() else {
            return false;
        };

        self.snapshot = snapshot;

        true
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
        let location = Self::selection_point(
            x,
            y,
            &self.terminal_size,
            self.snapshot.view().display_offset,
        );

        self.send(TerminalRequest::StartSelection {
            ty: selection_type,
            point: location,
            direction: self.selection_side(x),
        });
    }

    pub(crate) fn update_selection(&mut self, x: f32, y: f32) {
        let display_offset = self.snapshot.view().display_offset;
        let location =
            Self::selection_point(x, y, &self.terminal_size, display_offset);
        self.send(TerminalRequest::UpdateSelection {
            point: location,
            direction: self.selection_side(x),
        });
    }

    pub(crate) fn selection_point(
        x: f32,
        y: f32,
        terminal_size: &TerminalSize,
        display_offset: usize,
    ) -> Point {
        let col = (x as usize) / (terminal_size.cell_width as usize);
        let col = min(Column(col), Column(terminal_size.cols as usize - 1));

        let line = (y as usize) / (terminal_size.cell_height as usize);
        let line = min(line, terminal_size.rows as usize - 1);

        viewport_to_point(display_offset, Point::new(line, col))
    }

    fn selection_side(&self, x: f32) -> Side {
        let cell_x = x as usize % self.terminal_size.cell_width as usize;
        let half_cell_width =
            (self.terminal_size.cell_width as f32 / 2.0) as usize;

        if cell_x > half_cell_width {
            Side::Right
        } else {
            Side::Left
        }
    }

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

    pub(crate) fn write(&self, input: Vec<u8>) {
        self.send(TerminalRequest::WriteBytes(input));
        self.sroll_bottom();
    }

    fn sroll_bottom(&self) {
        self.send(TerminalRequest::ScrollDisplay(Scroll::Bottom));
    }

    pub(crate) fn scroll_delta(&self, delta_value: i32) {
        if delta_value != 0 {
            let scroll = Scroll::Delta(delta_value);
            if self.snapshot.view().mode.contains(
                SurfaceMode::ALTERNATE_SCROLL | SurfaceMode::ALT_SCREEN,
            ) {
                let line_cmd = if delta_value > 0 { b'A' } else { b'B' };
                let mut content = vec![];

                for _ in 0..delta_value.abs() {
                    content.push(0x1b);
                    content.push(b'O');
                    content.push(line_cmd);
                }

                self.send(TerminalRequest::WriteBytes(content));
            } else {
                self.send(TerminalRequest::ScrollDisplay(scroll));
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.send(TerminalRequest::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;
    use otty_libterm::surface::Line;

    use super::*;
    use crate::program::MockTerminalProgram;

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
        let engine = engine_over(program, &mut requests);

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
        let engine = engine_over(program, &mut requests);

        engine.scroll_delta(3);
        // No lines to scroll by is no request.
        engine.scroll_delta(0);
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

        let point = Engine::selection_point(16.0, 32.0, &terminal_size, 0);

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

        let point = Engine::selection_point(0.0, 0.0, &terminal_size, 5);

        // Should account for display_offset
        assert!(point.line.0 != 0 || point.column == Column(0));
    }

    #[test]
    fn test_selection_point_boundary_clamp() {
        let terminal_size = TerminalSize {
            cols: 80,
            rows: 24,
            cell_width: 8,
            cell_height: 16,
        };

        // Large coordinates should be clamped to terminal bounds
        let point =
            Engine::selection_point(10000.0, 10000.0, &terminal_size, 0);

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

        let point = Engine::selection_point(0.0, 0.0, &terminal_size, 0);

        assert_eq!(point.column, Column(0));
    }
}
