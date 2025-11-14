use std::cmp::min;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use anyhow::Context;
use egui::{Context as EguiContext, Modifiers};

use otty_libterm::{
    escape::{self, KeyboardMode},
    pty::{self, PtySize},
    surface::{
        SurfaceSnapshotSource,
        point_to_viewport, Cell, Dimensions, Flags, Scroll, Surface,
        SurfaceConfig,
    },
    LibTermError, Runtime, RuntimeRequestProxy, Terminal, TerminalClient,
    TerminalEvent, TerminalMode, TerminalOptions, TerminalRequest,
    TerminalSnapshot,
};

use crate::backend::settings::BackendSettings;
use crate::types::Size;

type EscapeParser = escape::Parser<escape::vte::Parser>;

#[derive(Debug, Clone)]
pub enum BackendCommand {
    Write(Vec<u8>),
    Scroll(i32),
    Resize(Size, Size),
    MouseReport {
        button: MouseButton,
        modifiers: Modifiers,
        position: (i32, usize),
        is_pressed: bool,
    },
    ProcessLink {
        action: LinkAction,
        position: (i32, usize),
    },
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

#[derive(Debug, Clone)]
pub enum MouseMode {
    Sgr,
    Normal(bool),
}

impl From<TerminalMode> for MouseMode {
    fn from(term_mode: TerminalMode) -> Self {
        if term_mode.contains(TerminalMode::SGR_MOUSE) {
            MouseMode::Sgr
        } else if term_mode.contains(TerminalMode::UTF8_MOUSE) {
            MouseMode::Normal(true)
        } else {
            MouseMode::Normal(false)
        }
    }
}

#[derive(Debug, Clone)]
pub enum LinkAction {
    Clear,
    Hover,
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderCellAttributes {
    pub foreground: escape::Color,
    pub background: escape::Color,
    pub reverse: bool,
}

impl Default for RenderCellAttributes {
    fn default() -> Self {
        Self {
            foreground: escape::Color::Std(escape::StdColor::Foreground),
            background: escape::Color::Std(escape::StdColor::Background),
            reverse: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderCell {
    pub ch: char,
    pub attributes: RenderCellAttributes,
}

impl RenderCell {
    fn blank() -> Self {
        Self {
            ch: ' ',
            attributes: RenderCellAttributes::default(),
        }
    }

    fn from_cell(cell: &Cell) -> Self {
        let mut ch = cell.c;
        if cell.flags.intersects(
            Flags::HIDDEN
                | Flags::WIDE_CHAR_SPACER
                | Flags::LEADING_WIDE_CHAR_SPACER,
        ) {
            ch = ' ';
        }

        Self {
            ch,
            attributes: RenderCellAttributes {
                foreground: cell.fg,
                background: cell.bg,
                reverse: cell.flags.contains(Flags::INVERSE),
            },
        }
    }

    pub fn is_blank(&self) -> bool {
        self.ch == ' '
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderRow {
    cells: Vec<RenderCell>,
}

impl RenderRow {
    fn new(columns: usize) -> Self {
        Self {
            cells: vec![RenderCell::blank(); columns],
        }
    }

    pub fn cells(&self) -> &[RenderCell] {
        &self.cells
    }

    fn cells_mut(&mut self) -> &mut [RenderCell] {
        &mut self.cells
    }

    fn len(&self) -> usize {
        self.cells.len()
    }
}

#[derive(Clone, Debug)]
pub enum RenderDamage {
    None,
    Full,
    Partial(Vec<usize>),
}

#[derive(Clone)]
pub struct RenderableContent {
    pub grid: Vec<RenderRow>,
    pub columns: usize,
    pub rows: usize,
    pub cell_width: u16,
    pub cell_height: u16,
    pub terminal_mode: TerminalMode,
    pub keyboard_mode: KeyboardMode,
    pub display_offset: usize,
    pub cursor_row: Option<usize>,
    pub cursor_col: Option<usize>,
    pub damage: RenderDamage,
    pub revision: u64,
}

impl RenderableContent {
    fn apply_snapshot(
        &mut self,
        snapshot: TerminalSnapshot,
        size: &TerminalSize,
    ) {
        let TerminalSnapshot {
            mut surface,
            terminal_mode,
            keyboard_mode,
        } = snapshot;

        let target_columns = size.num_cols as usize;
        let target_rows = size.num_lines as usize;
        let mut new_grid = vec![RenderRow::new(target_columns); target_rows];
        let mut prev_line: Option<i32> = None;
        let mut row_idx: usize = 0;

        while let Some(indexed) = surface.display_iter.next() {
            if row_idx >= target_rows {
                break;
            }

            let line = indexed.point.line.0;
            if prev_line.map_or(true, |prev| prev != line) {
                if prev_line.is_some() {
                    row_idx += 1;
                    if row_idx >= target_rows {
                        break;
                    }
                }
                prev_line = Some(line);
            }

            let column = indexed.point.column.0 as usize;
            if column < target_columns {
                new_grid[row_idx].cells_mut()[column] =
                    RenderCell::from_cell(indexed.cell);
            }
        }

        let cursor_point =
            if surface.cursor.shape != escape::CursorShape::Hidden {
                point_to_viewport(surface.display_offset, surface.cursor.point)
            } else {
                None
            };

        let geometry_changed =
            self.columns != target_columns || self.rows != target_rows;

        let damage = if geometry_changed || self.grid.len() != new_grid.len() {
            RenderDamage::Full
        } else {
            let mut dirty_rows = Vec::new();
            for (idx, (prev, next)) in
                self.grid.iter().zip(new_grid.iter()).enumerate()
            {
                if prev != next {
                    dirty_rows.push(idx);
                }
            }

            if dirty_rows.is_empty() {
                RenderDamage::None
            } else {
                RenderDamage::Partial(dirty_rows)
            }
        };

        self.grid = new_grid;
        self.columns = target_columns;
        self.rows = target_rows;
        self.cell_width = size.cell_width;
        self.cell_height = size.cell_height;
        self.terminal_mode = terminal_mode;
        self.keyboard_mode = keyboard_mode;
        self.display_offset = surface.display_offset;
        self.cursor_row = cursor_point.as_ref().map(|point| point.line);
        self.cursor_col = cursor_point.as_ref().map(|point| point.column.0);
        self.damage = damage;
        self.revision = self.revision.wrapping_add(1);
    }

    fn from_snapshot(snapshot: TerminalSnapshot, size: &TerminalSize) -> Self {
        let mut content = Self {
            grid: Vec::new(),
            columns: size.num_cols as usize,
            rows: size.num_lines as usize,
            cell_width: size.cell_width,
            cell_height: size.cell_height,
            terminal_mode: TerminalMode::default(),
            keyboard_mode: KeyboardMode::default(),
            display_offset: 0,
            cursor_row: None,
            cursor_col: None,
            damage: RenderDamage::Full,
            revision: 0,
        };
        content.apply_snapshot(snapshot, size);
        content
    }

    pub fn row(&self, index: usize) -> &RenderRow {
        &self.grid[index]
    }
}

impl Default for RenderableContent {
    fn default() -> Self {
        let size = TerminalSize::default();
        let dimensions = TerminalDimensions {
            columns: size.num_cols as usize,
            rows: size.num_lines as usize,
        };
        let surface = Surface::new(SurfaceConfig::default(), &dimensions);
        let snapshot = TerminalSnapshot::new(
            surface.capture_snapshot(),
            TerminalMode::default(),
            KeyboardMode::default(),
        );
        Self::from_snapshot(snapshot, &size)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalSize {
    pub cell_width: u16,
    pub cell_height: u16,
    pub num_cols: u16,
    pub num_lines: u16,
    pub layout_size: Size,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            cell_width: 8,
            cell_height: 16,
            num_cols: 80,
            num_lines: 24,
            layout_size: Size {
                width: 640.0,
                height: 384.0,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct TerminalDimensions {
    columns: usize,
    rows: usize,
}

impl Dimensions for TerminalDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

pub struct TerminalBackend {
    id: u64,
    size: TerminalSize,
    request_proxy: RuntimeRequestProxy,
    render_state: Arc<Mutex<RenderableContent>>,
    size_state: Arc<Mutex<TerminalSize>>,
    last_content: RenderableContent,
    _thread_handle: thread::JoinHandle<()>,
}

impl TerminalBackend {
    pub fn new(
        id: u64,
        app_context: EguiContext,
        settings: BackendSettings,
    ) -> anyhow::Result<Self> {
        let size = TerminalSize::default();
        let render_state = Arc::new(Mutex::new(RenderableContent::default()));
        let size_state = Arc::new(Mutex::new(size));

        let (proxy_tx, proxy_rx) = mpsc::channel();

        let render_state_thread = Arc::clone(&render_state);
        let size_state_thread = Arc::clone(&size_state);
        let ctx = app_context.clone();
        let settings_clone = settings.clone();

        let thread_handle = thread::Builder::new()
            .name(format!("otty_runtime_{id}"))
            .spawn(move || {
                if let Err(err) = run_terminal_thread(
                    settings_clone,
                    ctx,
                    proxy_tx,
                    render_state_thread,
                    size_state_thread,
                    size,
                ) {
                    eprintln!(
                        "OTTY backend runtime exited with error: {err:?}"
                    );
                }
            })
            .context("failed to spawn OTTY runtime thread")?;

        let request_proxy =
            proxy_rx.recv().context("failed to receive runtime proxy")?;
        let last_content = render_state.lock().unwrap().clone();

        Ok(Self {
            id,
            size,
            request_proxy,
            render_state,
            size_state,
            last_content,
            _thread_handle: thread_handle,
        })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn process_command(&mut self, cmd: BackendCommand) {
        let request = match cmd {
            BackendCommand::Write(bytes) => {
                if bytes.is_empty() {
                    return;
                }
                TerminalRequest::Write(bytes)
            },
            BackendCommand::Scroll(delta) => {
                if delta == 0 {
                    return;
                }
                let scroll = Scroll::Delta(delta);
                let mode = self.terminal_mode();
                if mode.contains(TerminalMode::ALT_SCREEN)
                    && mode.contains(TerminalMode::ALTERNATE_SCROLL)
                {
                    let line_cmd = if delta > 0 { b'A' } else { b'B' };
                    let mut content = vec![];

                    for _ in 0..delta.abs() {
                        content.push(0x1b);
                        content.push(b'O');
                        content.push(line_cmd);
                    }

                    TerminalRequest::Write(content)
                } else {
                    TerminalRequest::ScrollDisplay(scroll)
                }
            },
            BackendCommand::Resize(layout, font) => {
                if let Some(size) = compute_terminal_size(layout, font) {
                    // Avoid spamming resizes that reset display_offset when geometry didn't change.
                    let same_geometry = size.num_cols == self.size.num_cols
                        && size.num_lines == self.size.num_lines
                        && size.cell_width == self.size.cell_width
                        && size.cell_height == self.size.cell_height;

                    if same_geometry {
                        return;
                    }

                    self.size = size;
                    TerminalRequest::Resize(PtySize {
                        rows: size.num_lines,
                        cols: size.num_cols,
                        cell_width: size.cell_width,
                        cell_height: size.cell_height,
                    })
                } else {
                    return;
                }
            },
            BackendCommand::MouseReport {
                button,
                modifiers,
                position,
                is_pressed,
            } => {
                let report = self.process_mouse_report(
                    button, modifiers, position, is_pressed,
                );
                if let Some(value) = report {
                    TerminalRequest::Write(value)
                } else {
                    return;
                }
            },
            BackendCommand::ProcessLink { action, position } => {
                return;
            },
        };

        if let Err(err) = self.request_proxy.send(request) {
            eprintln!("failed to send terminal request: {err:?}");
        }
    }

    pub fn sync(&mut self) -> RenderableContent {
        if let Ok(content) = self.render_state.lock() {
            self.last_content = content.clone();
        }
        if let Ok(size) = self.size_state.lock() {
            self.size = *size;
        }
        self.last_content.clone()
    }

    pub fn terminal_mode(&self) -> TerminalMode {
        self.last_content.terminal_mode
    }

    pub fn last_content(&self) -> &RenderableContent {
        &self.last_content
    }

    pub fn selection_point(
        x: f32,
        y: f32,
        terminal_size: &TerminalSize,
        display_offset: usize,
    ) -> (i32, usize) {
        let col = (x as usize) / (terminal_size.cell_width as usize);
        let col = min(col, terminal_size.num_cols as usize - 1);

        let line = (y as usize) / (terminal_size.cell_height as usize);
        let line = min(line, terminal_size.num_lines as usize - 1);
        ((line + display_offset) as i32, col)
    }

    pub fn terminal_size(&self) -> TerminalSize {
        self.size
    }

    fn process_mouse_report(
        &self,
        button: MouseButton,
        modifiers: Modifiers,
        point: (i32, usize),
        pressed: bool,
    ) -> Option<Vec<u8>> {
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

        match MouseMode::from(self.last_content().terminal_mode) {
            MouseMode::Sgr => {
                Some(self.sgr_mouse_report(point, button as u8 + mods, pressed))
            },
            MouseMode::Normal(is_utf8) => {
                let report = if pressed {
                    self.normal_mouse_report(
                        point,
                        button as u8 + mods,
                        is_utf8,
                    )
                } else {
                    self.normal_mouse_report(point, 3 + mods, is_utf8)
                };

                if report.is_empty() {
                    None
                } else {
                    Some(report)
                }
            },
        }
    }

    fn sgr_mouse_report(
        &self,
        point: (i32, usize),
        button: u8,
        pressed: bool,
    ) -> Vec<u8> {
        let c = if pressed { 'M' } else { 'm' };

        format!("\x1b[<{};{};{}{}", button, point.1 + 1, point.0 + 1, c)
            .as_bytes()
            .to_vec()
    }

    fn normal_mouse_report(
        &self,
        point: (i32, usize),
        button: u8,
        is_utf8: bool,
    ) -> Vec<u8> {
        let (line, column) = point;
        let max_point = if is_utf8 { 2015 } else { 223 };

        if line >= max_point || column >= max_point as usize {
            return vec![];
        }

        let mut msg = vec![b'\x1b', b'[', b'M', 32 + button];

        let mouse_pos_encode = |pos: usize| -> Vec<u8> {
            let pos = 32 + 1 + pos;
            let first = 0xC0 + pos / 64;
            let second = 0x80 + (pos & 63);
            vec![first as u8, second as u8]
        };

        if is_utf8 && column >= 95 {
            msg.append(&mut mouse_pos_encode(column));
        } else {
            msg.push(32 + 1 + column as u8);
        }

        if is_utf8 && line >= 95 {
            msg.append(&mut mouse_pos_encode(line as usize));
        } else {
            msg.push(32 + 1 + line as u8);
        }

        msg.to_vec()
    }
}

fn run_terminal_thread(
    settings: BackendSettings,
    ctx: EguiContext,
    proxy_tx: mpsc::Sender<RuntimeRequestProxy>,
    render_state: Arc<Mutex<RenderableContent>>,
    size_state: Arc<Mutex<TerminalSize>>,
    initial_size: TerminalSize,
) -> anyhow::Result<()> {
    let surface_config = SurfaceConfig::default();
    let surface_dimensions = TerminalDimensions {
        columns: initial_size.num_cols as usize,
        rows: initial_size.num_lines as usize,
    };
    let surface = Surface::new(surface_config, &surface_dimensions);

    let mut builder = pty::unix(&settings.shell)
        .with_args(&settings.args)
        .with_size(PtySize {
            rows: initial_size.num_lines,
            cols: initial_size.num_cols,
            cell_width: initial_size.cell_width,
            cell_height: initial_size.cell_height,
        })
        .set_controling_tty_enable();

    if let Some(dir) = settings.working_directory.as_ref() {
        builder = builder.with_cwd(dir);
    }

    let session = builder
        .spawn()
        .context("failed to spawn OTTY terminal session")?;
    let parser: EscapeParser = Default::default();
    let options = TerminalOptions::default();

    let mut terminal = Terminal::new(session, surface, parser, options)
        .context("failed to construct terminal runtime")?;

    {
        let snapshot = terminal.snapshot();
        let size = size_state.lock().map(|s| *s).unwrap_or(initial_size);
        if let Ok(mut content) = render_state.lock() {
            content.apply_snapshot(snapshot, &size);
        }
    }

    let mut runtime =
        Runtime::new().context("failed to create terminal runtime")?;
    proxy_tx
        .send(runtime.proxy())
        .expect("failed to send runtime proxy");

    let event_handler = RuntimeEventHandler::new(render_state, size_state, ctx);
    terminal.set_event_client(event_handler);

    runtime.run(terminal, ())?;
    Ok(())
}

struct RuntimeEventHandler {
    render_state: Arc<Mutex<RenderableContent>>,
    size_state: Arc<Mutex<TerminalSize>>,
    ctx: EguiContext,
}

impl RuntimeEventHandler {
    fn new(
        render_state: Arc<Mutex<RenderableContent>>,
        size_state: Arc<Mutex<TerminalSize>>,
        ctx: EguiContext,
    ) -> Self {
        Self {
            render_state,
            size_state,
            ctx,
        }
    }

    fn update_render_state(
        &self,
        snapshot: TerminalSnapshot,
    ) -> Result<(), LibTermError> {
        let size = self.size_state.lock().map(|s| *s).unwrap_or_default();
        if let Ok(mut content) = self.render_state.lock() {
            content.apply_snapshot(snapshot, &size);
        }
        self.ctx.request_repaint();
        Ok(())
    }
}

impl TerminalClient for RuntimeEventHandler {
    fn handle_event(
        &mut self,
        event: TerminalEvent,
    ) -> Result<(), LibTermError> {
        match event {
            TerminalEvent::SurfaceChanged { snapshot } => {
                // Debug: track display_offset and history size while investigating scroll behavior
                // eprintln!(
                //     "[egui_term] surface changed: display_offset={}, history_size={}",
                //     snapshot.surface.display_offset, snapshot.surface.grid.history_size()
                // );
                self.update_render_state(snapshot)?;
            },
            TerminalEvent::ChildExit { status } => {
                eprintln!(
                    "OTTY backend child exited with: {:?}",
                    status.code()
                );
            },
            TerminalEvent::TitleChanged { .. }
            | TerminalEvent::Bell
            | TerminalEvent::CursorShapeChanged { .. }
            | TerminalEvent::CursorStyleChanged { .. }
            | TerminalEvent::CursorIconChanged { .. }
            | TerminalEvent::Hyperlink { .. } => {},
        }

        Ok(())
    }
}

fn compute_terminal_size(layout: Size, font: Size) -> Option<TerminalSize> {
    if font.width <= 0.0 || font.height <= 0.0 {
        return None;
    }

    let cell_width = font.width.floor();
    let cell_height = font.height.floor();
    if cell_width < 1.0 || cell_height < 1.0 {
        return None;
    }

    let cols = (layout.width / cell_width).floor() as u16;
    let rows = (layout.height / cell_height).floor() as u16;

    if cols == 0 || rows == 0 {
        return None;
    }

    Some(TerminalSize {
        cell_width: cell_width as u16,
        cell_height: cell_height as u16,
        num_cols: cols,
        num_lines: rows,
        layout_size: layout,
    })
}
