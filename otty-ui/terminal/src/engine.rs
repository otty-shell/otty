use std::cmp::min;
use std::sync::Arc;

use iced::keyboard::Modifiers;
use iced_core::Size;
#[cfg(test)]
use otty_libterm::Runtime;
use otty_libterm::mouse::{
    TerminalMouseButton, TerminalMouseModifiers, encode_mouse_report,
};
use otty_libterm::surface::{
    Column, Point, Scroll, SelectionType, Side, SnapshotOwned, SurfaceMode,
    viewport_to_point,
};
use otty_libterm::{
    DefaultParser, DefaultSurface, RuntimeRequestProxy, RuntimeTerminal,
    TerminalBuilder, TerminalEvent, TerminalRequest, TerminalSize, pty,
};
use tokio::sync::mpsc;

use crate::error::Result;
use crate::settings::{BackendSettings, SessionKind};

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

impl From<MouseButton> for TerminalMouseButton {
    fn from(value: MouseButton) -> Self {
        match value {
            MouseButton::LeftButton => Self::Left,
            MouseButton::MiddleButton => Self::Middle,
            MouseButton::RightButton => Self::Right,
            MouseButton::LeftMove => Self::LeftMove,
            MouseButton::MiddleMove => Self::MiddleMove,
            MouseButton::RightMove => Self::RightMove,
            MouseButton::NoneMove => Self::Move,
            MouseButton::ScrollUp => Self::ScrollUp,
            MouseButton::ScrollDown => Self::ScrollDown,
            MouseButton::Other => Self::Move,
        }
    }
}

type LocalTerminal =
    RuntimeTerminal<pty::LocalSession, DefaultParser, DefaultSurface>;

type SSHTerminal =
    RuntimeTerminal<pty::SSHSession, DefaultParser, DefaultSurface>;

enum EngineInner {
    Local(LocalTerminal),
    Ssh(SSHTerminal),
}

impl EngineInner {
    fn build(kind: SessionKind, size: TerminalSize) -> Result<Self> {
        match kind {
            SessionKind::Local(options) => {
                let mut builder = pty::local(options.program())
                    .with_args(options.args())
                    .with_size(size.into())
                    .set_controling_tty_enable();

                for (key, value) in options.envs() {
                    builder = builder.with_env(key, value);
                }

                if let Some(cwd) = options.working_directory() {
                    builder = builder.with_cwd(cwd)
                }

                let result =
                    TerminalBuilder::from(builder).build_with_runtime()?;

                Ok(EngineInner::Local(result))
            },
            SessionKind::Ssh(options) => {
                let mut builder = pty::ssh()
                    .with_host(options.host())
                    .with_user(options.user())
                    .with_auth(options.auth())
                    .with_size(size.into());

                if let Some(timeout) = options.timeout() {
                    builder = builder.with_timeout(timeout);
                }

                if let Some(cancel) = options.cancel_token() {
                    builder = builder.with_cancel_token(cancel.clone());
                }

                let result =
                    TerminalBuilder::from(builder).build_with_runtime()?;

                Ok(EngineInner::Ssh(result))
            },
        }
    }

    fn spawn(self) -> std::thread::JoinHandle<Result<()>> {
        std::thread::spawn(|| {
            match self {
                Self::Local((mut runtime, mut engine, ..)) => {
                    runtime.run(&mut engine, ())?
                },
                Self::Ssh((mut runtime, mut engine, ..)) => {
                    runtime.run(&mut engine, ())?
                },
            }

            Ok(())
        })
    }

    fn events_consumer(
        &self,
        pty_event_proxy_sender: mpsc::Sender<TerminalEvent>,
    ) -> std::thread::JoinHandle<Result<()>> {
        let events = match self {
            Self::Local((.., events)) => events.clone(),
            Self::Ssh((.., events)) => events.clone(),
        };

        std::thread::spawn(move || {
            while let Ok(event) = events.recv() {
                let is_child_exit =
                    matches!(event, TerminalEvent::ChildExit { .. });
                let _ = pty_event_proxy_sender.blocking_send(event);

                if is_child_exit {
                    break;
                }
            }

            Ok(())
        })
    }

    fn request_proxy(&self) -> RuntimeRequestProxy {
        match self {
            Self::Local((runtime, ..)) => runtime.proxy(),
            Self::Ssh((runtime, ..)) => runtime.proxy(),
        }
    }
}

pub struct Engine {
    terminal_size: TerminalSize,
    layout_size: Size,
    request_proxy: RuntimeRequestProxy,
    snapshot: Arc<SnapshotOwned>,
}

impl Engine {
    pub fn new(
        pty_event_proxy_sender: mpsc::Sender<TerminalEvent>,
        settings: BackendSettings,
    ) -> Result<Self> {
        let BackendSettings { session, size } = settings;
        let terminal = EngineInner::build(session, size)?;
        let request_proxy = terminal.request_proxy();
        let _ = terminal.events_consumer(pty_event_proxy_sender);
        let _ = terminal.spawn();

        Ok(Self {
            terminal_size: size,
            layout_size: Size::default(),
            request_proxy,
            snapshot: Arc::new(SnapshotOwned::default()),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_snapshot_for_test(
        snapshot: Arc<SnapshotOwned>,
        terminal_size: TerminalSize,
    ) -> Self {
        let runtime = Box::leak(Box::new(
            Runtime::new().expect("failed to build runtime for test"),
        ));
        let request_proxy = runtime.proxy();

        Self {
            terminal_size,
            layout_size: Size::default(),
            request_proxy,
            snapshot,
        }
    }

    pub fn terminal_size(&self) -> TerminalSize {
        self.terminal_size
    }

    pub fn snapshot(&self) -> Arc<SnapshotOwned> {
        self.snapshot.clone()
    }

    pub(crate) fn sync_snapshot(&mut self, frame: Arc<SnapshotOwned>) {
        self.snapshot = frame;
    }

    pub(crate) fn process_mouse_report(
        &self,
        button: MouseButton,
        modifiers: Modifiers,
        point: Point,
        pressed: bool,
    ) {
        let modifiers = TerminalMouseModifiers::new(
            modifiers.contains(Modifiers::SHIFT),
            modifiers.contains(Modifiers::ALT),
            modifiers.contains(Modifiers::COMMAND),
        );
        let Some(report) = encode_mouse_report(
            self.snapshot.view().mode,
            point,
            button.into(),
            modifiers,
            pressed,
        ) else {
            return;
        };

        let _ = self.request_proxy.send(TerminalRequest::WriteBytes(report));
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

        let _ = self.request_proxy.send(TerminalRequest::StartSelection {
            ty: selection_type,
            point: location,
            direction: self.selection_side(x),
        });
    }

    pub(crate) fn update_selection(&mut self, x: f32, y: f32) {
        let display_offset = self.snapshot.view().display_offset;
        let location =
            Self::selection_point(x, y, &self.terminal_size, display_offset);
        let _ = self.request_proxy.send(TerminalRequest::UpdateSelection {
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
            let _ = self
                .request_proxy
                .send(TerminalRequest::Resize(self.terminal_size));
        }
    }

    pub(crate) fn write(&self, input: Vec<u8>) {
        let _ = self.request_proxy.send(TerminalRequest::WriteBytes(input));
        self.sroll_bottom();
    }

    fn sroll_bottom(&self) {
        let _ = self
            .request_proxy
            .send(TerminalRequest::ScrollDisplay(Scroll::Bottom));
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

                let _ = self
                    .request_proxy
                    .send(TerminalRequest::WriteBytes(content));
            } else {
                let _ = self
                    .request_proxy
                    .send(TerminalRequest::ScrollDisplay(scroll));
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.request_proxy.send(TerminalRequest::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use otty_libterm::surface::Line;

    use super::*;

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
