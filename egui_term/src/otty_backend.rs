use std::cmp::min;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use anyhow::Context;
use egui::{Context as EguiContext, Modifiers};

use otty_libterm::surface::{Column, Point, SelectionType, Side};
use otty_libterm::{
    escape::{self, KeyboardMode},
    pty::{self, PtySize},
    surface::{
        point_to_viewport, viewport_to_point, Cell, Dimensions, Flags, Scroll,
        Surface, SurfaceConfig, SurfaceMode, SurfaceSnapshotSource,
    },
    Error, Runtime, RuntimeRequestProxy, Terminal, TerminalClient,
    TerminalEvent, TerminalOptions, TerminalRequest, TerminalSize,
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
    SelectStart(SelectionType, f32, f32),
    SelectUpdate(f32, f32),
    MouseReport {
        button: MouseButton,
        modifiers: Modifiers,
        position: Point,
        is_pressed: bool,
    },
    ProcessLink {
        action: LinkAction,
        position: Point,
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
    pub selected: bool,
}

impl Default for RenderCellAttributes {
    fn default() -> Self {
        Self {
            foreground: escape::Color::Std(escape::StdColor::Foreground),
            background: escape::Color::Std(escape::StdColor::Background),
            reverse: false,
            selected: false,
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

    fn from_cell(cell: &Cell, selected: bool) -> Self {
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
                selected,
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
    pub terminal_mode: SurfaceMode,
    pub keyboard_mode: KeyboardMode,
    pub display_offset: usize,
    pub cursor_row: Option<usize>,
    pub cursor_col: Option<usize>,
    pub damage: RenderDamage,
    pub revision: u64,
    pub selected_text: String,
}

impl RenderableContent {
    fn apply_snapshot(
        &mut self,
        snapshot: TerminalSnapshot,
        size: &TerminalSize,
    ) {
        let TerminalSnapshot { mut surface, .. } = snapshot;

        let target_columns = size.cols as usize;
        let target_rows = size.rows as usize;
        let mut new_grid = vec![RenderRow::new(target_columns); target_rows];
        let mut prev_line: Option<i32> = None;
        let mut row_idx: usize = 0;
        let selection = surface.selection;
        let cursor_snapshot = surface.cursor;
        let mut selected_text = String::new();

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
                let is_selected = selection.as_ref().is_some_and(|range| {
                    range.contains_cell(
                        &indexed,
                        cursor_snapshot.point,
                        cursor_snapshot.shape,
                    )
                });

                let cell = RenderCell::from_cell(indexed.cell, is_selected);
                if is_selected {
                    selected_text.push(cell.ch);
                }

                new_grid[row_idx].cells_mut()[column] = cell;
            }
        }

        let cursor_point =
            if cursor_snapshot.shape != escape::CursorShape::Hidden {
                point_to_viewport(surface.display_offset, cursor_snapshot.point)
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
        self.display_offset = surface.display_offset;
        self.terminal_mode = surface.mode;
        self.cursor_row = cursor_point.as_ref().map(|point| point.line);
        self.cursor_col = cursor_point.as_ref().map(|point| point.column.0);
        self.damage = damage;
        self.revision = self.revision.wrapping_add(1);
        self.selected_text = selected_text;
    }

    fn from_snapshot(snapshot: TerminalSnapshot, size: &TerminalSize) -> Self {
        let mut content = Self {
            grid: Vec::new(),
            columns: size.cols as usize,
            rows: size.rows as usize,
            cell_width: size.cell_width,
            cell_height: size.cell_height,
            terminal_mode: SurfaceMode::default(),
            keyboard_mode: KeyboardMode::default(),
            display_offset: 0,
            cursor_row: None,
            cursor_col: None,
            damage: RenderDamage::Full,
            revision: 0,
            selected_text: String::new(),
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
            columns: size.cols as usize,
            rows: size.rows as usize,
        };
        let surface = Surface::new(SurfaceConfig::default(), &dimensions);
        let snapshot = TerminalSnapshot::new(surface.capture_snapshot(), size);
        Self::from_snapshot(snapshot, &size)
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
                if mode.contains(SurfaceMode::ALT_SCREEN)
                    && mode.contains(SurfaceMode::ALTERNATE_SCROLL)
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
                    let same_geometry = size.cols == self.size.cols
                        && size.rows == self.size.rows
                        && size.cell_width == self.size.cell_width
                        && size.cell_height == self.size.cell_height;

                    if same_geometry {
                        return;
                    }

                    self.size = size;
                    self.update_shared_size(size);
                    TerminalRequest::Resize(size)
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
            BackendCommand::SelectStart(selection_type, x, y) => {
                let point = Self::selection_point(
                    x,
                    y,
                    &self.size,
                    self.last_content.display_offset,
                );
                TerminalRequest::StartSelection {
                    ty: selection_type,
                    point,
                    direction: self.selection_side(x),
                }
            },
            BackendCommand::SelectUpdate(x, y) => {
                let point = Self::selection_point(
                    x,
                    y,
                    &self.size,
                    self.last_content.display_offset,
                );

                TerminalRequest::UpdateSelection {
                    point,
                    direction: self.selection_side(x),
                }
            },
            BackendCommand::ProcessLink { .. } => {
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

    pub fn terminal_mode(&self) -> SurfaceMode {
        self.last_content.terminal_mode
    }

    pub fn last_content(&self) -> &RenderableContent {
        &self.last_content
    }

    pub fn selectable_content(&self) -> String {
        self.last_content.selected_text.clone()
    }

    pub fn terminal_size(&self) -> TerminalSize {
        self.size
    }

    fn update_shared_size(&self, size: TerminalSize) {
        if let Ok(mut shared_size) = self.size_state.lock() {
            *shared_size = size;
        }
    }

    fn process_mouse_report(
        &self,
        button: MouseButton,
        modifiers: Modifiers,
        point: Point,
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
        point: Point,
        button: u8,
        pressed: bool,
    ) -> Vec<u8> {
        let c = if pressed { 'M' } else { 'm' };

        format!(
            "\x1b[<{};{};{}{}",
            button,
            point.column.0 + 1,
            point.line.0 + 1,
            c
        )
        .as_bytes()
        .to_vec()
    }

    fn normal_mouse_report(
        &self,
        point: Point,
        button: u8,
        is_utf8: bool,
    ) -> Vec<u8> {
        let Point { line, column } = point;
        let line_idx = line.0.max(0) as usize;
        let column_idx = column.0;
        let max_point = if is_utf8 { 2015 } else { 223 };

        if line_idx >= max_point || column_idx >= max_point {
            return vec![];
        }

        let mut msg = vec![b'\x1b', b'[', b'M', 32 + button];

        let mouse_pos_encode = |pos: usize| -> Vec<u8> {
            let pos = 32 + 1 + pos;
            let first = 0xC0 + pos / 64;
            let second = 0x80 + (pos & 63);
            vec![first as u8, second as u8]
        };

        if is_utf8 && column_idx >= 95 {
            msg.append(&mut mouse_pos_encode(column_idx));
        } else {
            msg.push(32 + 1 + column_idx as u8);
        }

        if is_utf8 && line_idx >= 95 {
            msg.append(&mut mouse_pos_encode(line_idx));
        } else {
            msg.push(32 + 1 + line_idx as u8);
        }

        msg.to_vec()
    }

    fn selection_side(&self, x: f32) -> Side {
        let cell_x = x as usize % self.size.cell_width as usize;
        let half_cell_width = (self.size.cell_width as f32 / 2.0) as usize;

        if cell_x > half_cell_width {
            Side::Right
        } else {
            Side::Left
        }
    }

    pub fn selection_point(
        x: f32,
        y: f32,
        terminal_size: &TerminalSize,
        display_offset: usize,
    ) -> Point {
        let col = (x as usize) / (terminal_size.cell_width as usize);
        let col = min(col, terminal_size.cols as usize - 1);

        let line = (y as usize) / (terminal_size.cell_height as usize);
        let line = min(line, terminal_size.rows as usize - 1);

        let viewport_point = Point::<usize>::new(line, Column(col));
        viewport_to_point(display_offset, viewport_point)
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
        columns: initial_size.cols as usize,
        rows: initial_size.rows as usize,
    };
    let surface = Surface::new(surface_config, &surface_dimensions);

    let mut builder = pty::unix(&settings.shell)
        .with_args(&settings.args)
        .with_size(PtySize {
            rows: initial_size.rows,
            cols: initial_size.cols,
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
    ) -> Result<(), Error> {
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
    ) -> Result<(), Error> {
        match event {
            TerminalEvent::SurfaceChanged { snapshot } => {
                self.update_render_state(snapshot)?;
            },
            TerminalEvent::ChildExit { status } => {
                eprintln!(
                    "OTTY backend child exited with: {:?}",
                    status.code()
                );
            },
            TerminalEvent::TitleChanged { .. }
            | TerminalEvent::ResetTitle
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
        rows,
        cols,
    })
}
