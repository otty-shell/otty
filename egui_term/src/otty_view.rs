use egui::{
    epaint::RectShape, Align2, Id, Painter, Pos2, Rect, Response, Shape, Vec2,
    Widget,
};
use egui::{Key, Modifiers, MouseWheelUnit, PointerButton};

use crate::otty_backend::{
    BackendCommand, LinkAction, MouseButton, RenderDamage, RenderRow,
    RenderableContent, TerminalBackend,
};
use crate::otty_bindings::{Binding, BindingAction, BindingsLayout, InputKind};
use crate::otty_theme::TerminalTheme;
use crate::types::Size;
use crate::TerminalFont;
use otty_libterm::escape::{Color, StdColor};
use otty_libterm::TerminalMode;
use std::sync::{Arc, Mutex};

const EGUI_TERM_WIDGET_ID_PREFIX: &str = "egui_term::otty::";

#[derive(Debug, Clone)]
enum InputAction {
    BackendCall(BackendCommand),
    WriteToClipboard(String),
    Ignore,
}

#[derive(Clone)]
pub struct TerminalViewState {
    is_dragged: bool,
    scroll_pixels: f32,
    current_mouse_position_on_grid: (i32, usize),
    row_shapes: Arc<Mutex<Vec<Vec<Shape>>>>,
    cached_origin: Option<Pos2>,
    cached_cell_size: (u16, u16),
    last_revision: u64,
}

impl Default for TerminalViewState {
    fn default() -> Self {
        Self {
            is_dragged: false,
            scroll_pixels: 0.0,
            current_mouse_position_on_grid: (0, 0),
            row_shapes: Arc::new(Mutex::new(Vec::new())),
            cached_origin: None,
            cached_cell_size: (0, 0),
            last_revision: 0,
        }
    }
}

pub struct TerminalView<'a> {
    widget_id: Id,
    size: Vec2,
    has_focus: bool,
    backend: &'a mut TerminalBackend,
    font: TerminalFont,
    theme: TerminalTheme,
    bindings_layout: BindingsLayout,
}

impl<'a> TerminalView<'a> {
    pub fn new(ui: &mut egui::Ui, backend: &'a mut TerminalBackend) -> Self {
        let widget_id = ui.make_persistent_id(format!(
            "{}{}",
            EGUI_TERM_WIDGET_ID_PREFIX,
            backend.id()
        ));

        Self {
            widget_id,
            size: ui.available_size(),
            has_focus: true,
            backend,
            theme: TerminalTheme::default(),
            font: TerminalFont::default(),
            bindings_layout: BindingsLayout::default(),
        }
    }

    pub fn set_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn set_focus(mut self, focus: bool) -> Self {
        self.has_focus = focus;
        self
    }

    pub fn set_theme(mut self, theme: TerminalTheme) -> Self {
        self.theme = theme;
        self
    }

    #[inline]
    pub fn add_bindings(
        mut self,
        bindings: Vec<(Binding<InputKind>, BindingAction)>,
    ) -> Self {
        self.bindings_layout.add_bindings(bindings);
        self
    }

    fn focus(self, layout: &Response) -> Self {
        if self.has_focus {
            layout.request_focus();
        } else {
            layout.surrender_focus();
        }

        self
    }

    fn resize(self, layout: &Response) -> Self {
        self.backend.process_command(BackendCommand::Resize(
            Size::from(layout.rect.size()),
            self.font.font_measure(&layout.ctx),
        ));

        self
    }

    fn process_input(
        self,
        layout: &Response,
        state: &mut TerminalViewState,
    ) -> Self {
        let has_focus = layout.has_focus();
        let pointer_over = layout.contains_pointer();

        if !has_focus && !pointer_over {
            return self;
        }

        let modifiers = layout.ctx.input(|i| i.modifiers);
        let events = layout.ctx.input(|i| i.events.clone());
        for event in events {
            let mut input_actions = vec![];

            match event {
                egui::Event::Text(_)
                | egui::Event::Key { .. }
                | egui::Event::Copy
                | egui::Event::Paste(_)
                    if has_focus =>
                {
                    input_actions.push(process_keyboard_event(
                        event,
                        self.backend,
                        &self.bindings_layout,
                        modifiers,
                    ))
                },
                egui::Event::MouseWheel { unit, delta, .. } if pointer_over => {
                    input_actions.push(process_mouse_wheel(
                        state,
                        self.font.font_type().size,
                        unit,
                        delta,
                    ))
                },
                egui::Event::PointerButton {
                    button,
                    pressed,
                    modifiers,
                    pos,
                    ..
                } if pointer_over => input_actions.push(process_button_click(
                    state,
                    layout,
                    self.backend,
                    &self.bindings_layout,
                    button,
                    pos,
                    &modifiers,
                    pressed,
                )),
                egui::Event::PointerMoved(pos) if pointer_over => {
                    input_actions = process_mouse_move(
                        state,
                        layout,
                        self.backend,
                        pos,
                        &modifiers,
                    );
                },
                _ => {},
            };

            for action in input_actions {
                match action {
                    InputAction::BackendCall(cmd) => {
                        self.backend.process_command(cmd);
                    },
                    InputAction::WriteToClipboard(data) => {
                        layout.ctx.copy_text(data);
                    },
                    InputAction::Ignore => {},
                }
            }
        }

        self
    }

    fn show(
        self,
        state: &mut TerminalViewState,
        layout: &Response,
        painter: &Painter,
    ) {
        let content = self.backend.sync();
        paint_grid(painter, &layout.rect, &self.theme, &content, state);
    }
}

impl Widget for TerminalView<'_> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (layout, painter) =
            ui.allocate_painter(self.size, egui::Sense::click());

        let widget_id = self.widget_id;
        let mut state = ui.memory(|m| {
            m.data
                .get_temp::<TerminalViewState>(widget_id)
                .unwrap_or_default()
        });

        self.focus(&layout)
            .resize(&layout)
            .process_input(&layout, &mut state)
            .show(&mut state, &layout, &painter);

        ui.memory_mut(|m| m.data.insert_temp(widget_id, state));
        layout
    }
}

fn paint_grid(
    painter: &Painter,
    rect: &Rect,
    theme: &TerminalTheme,
    content: &RenderableContent,
    state: &mut TerminalViewState,
) {
    let bg = theme.resolve(Color::Std(StdColor::Background));
    painter.add(Shape::Rect(RectShape::filled(
        *rect,
        egui::CornerRadius::ZERO,
        bg,
    )));

    let mut cache = state
        .row_shapes
        .lock()
        .expect("row cache poisoning while rendering terminal");

    if content.rows == 0 || content.columns == 0 {
        cache.clear();
        return;
    }

    let cell_w = content.cell_width as f32;
    let cell_h = content.cell_height as f32;

    let geometry_changed = cache.len() != content.rows
        || state.cached_origin != Some(rect.min)
        || state.cached_cell_size.0 != content.cell_width
        || state.cached_cell_size.1 != content.cell_height;

    if geometry_changed {
        *cache = vec![Vec::new(); content.rows];
        state.cached_origin = Some(rect.min);
        state.cached_cell_size = (content.cell_width, content.cell_height);
        state.last_revision = 0;
    }

    let mut dirty_rows = Vec::new();
    if geometry_changed {
        dirty_rows = (0..content.rows).collect();
    } else if state.last_revision != content.revision {
        dirty_rows = match &content.damage {
            RenderDamage::None => {
                state.last_revision = content.revision;
                Vec::new()
            },
            RenderDamage::Full => (0..content.rows).collect(),
            RenderDamage::Partial(lines) => lines.clone(),
        };
    }

    if !dirty_rows.is_empty() {
        dirty_rows.retain(|&row| row < cache.len());
        dirty_rows.sort_unstable();
        dirty_rows.dedup();
        for row_idx in dirty_rows {
            let row = content.row(row_idx);
            cache[row_idx] = rebuild_row_shapes(
                painter, row_idx, row, rect.min, cell_w, cell_h, theme,
            );
        }
        state.last_revision = content.revision;
    }

    for row_shapes in cache.iter() {
        for shape in row_shapes {
            painter.add(shape.clone());
        }
    }
}

fn rebuild_row_shapes(
    painter: &Painter,
    row_idx: usize,
    row: &RenderRow,
    origin: Pos2,
    cell_w: f32,
    cell_h: f32,
    theme: &TerminalTheme,
) -> Vec<Shape> {
    let mut shapes = Vec::new();
    let y = origin.y + row_idx as f32 * cell_h;

    for (col_idx, cell) in row.cells().iter().enumerate() {
        let x = origin.x + col_idx as f32 * cell_w;
        let mut fg = theme.resolve(cell.attributes.foreground);
        let mut bg = theme.resolve(cell.attributes.background);

        if cell.attributes.reverse {
            std::mem::swap(&mut fg, &mut bg);
        }

        if bg != theme.resolve(Color::Std(StdColor::Background)) {
            shapes.push(Shape::Rect(RectShape::filled(
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(cell_w, cell_h)),
                egui::CornerRadius::ZERO,
                bg,
            )));
        }

        if !cell.is_blank() {
            painter.fonts_mut(|fonts| {
                shapes.push(Shape::text(
                    fonts,
                    Pos2 {
                        x: x + (cell_w / 2.0),
                        y,
                    },
                    Align2::CENTER_TOP,
                    cell.ch,
                    egui::FontId::monospace(cell_h * 0.85),
                    fg,
                ));
            });
        }
    }

    shapes
}

fn process_keyboard_event(
    event: egui::Event,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    modifiers: Modifiers,
) -> InputAction {
    match event {
        egui::Event::Text(text) => {
            process_text_event(&text, modifiers, backend, bindings_layout)
        },
        egui::Event::Paste(text) => InputAction::BackendCall(
            #[cfg(not(any(target_os = "ios", target_os = "macos")))]
            if modifiers.contains(Modifiers::COMMAND | Modifiers::SHIFT) {
                BackendCommand::Write(text.as_bytes().to_vec())
            } else {
                // Hotfix - Send ^V when there's not selection on view.
                BackendCommand::Write([0x16].to_vec())
            },
            #[cfg(any(target_os = "ios", target_os = "macos"))]
            {
                BackendCommand::Write(text.as_bytes().to_vec())
            },
        ),
        egui::Event::Copy => {
            InputAction::Ignore
            // #[cfg(not(any(target_os = "ios", target_os = "macos")))]
            // if modifiers.contains(Modifiers::COMMAND | Modifiers::SHIFT) {
            //     let content = backend.selectable_content();
            //     InputAction::WriteToClipboard(content)
            // } else {
            //     // Hotfix - Send ^C when there's not selection on view.
            //     InputAction::BackendCall(BackendCommand::Write([0x3].to_vec()))
            // }
            // #[cfg(any(target_os = "ios", target_os = "macos"))]
            // {
            //     let content = backend.selectable_content();
            //     InputAction::WriteToClipboard(content)
            // }
        },
        egui::Event::Key {
            key,
            pressed,
            modifiers,
            ..
        } => process_keyboard_key(
            backend,
            bindings_layout,
            key,
            modifiers,
            pressed,
        ),
        _ => InputAction::Ignore,
    }
}

fn process_text_event(
    text: &str,
    modifiers: Modifiers,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
) -> InputAction {
    if let Some(key) = Key::from_name(text) {
        if bindings_layout.get_action(
            InputKind::KeyCode(key),
            modifiers,
            backend.terminal_mode(),
        ) == BindingAction::Ignore
        {
            InputAction::BackendCall(BackendCommand::Write(
                text.as_bytes().to_vec(),
            ))
        } else {
            InputAction::Ignore
        }
    } else {
        InputAction::BackendCall(BackendCommand::Write(
            text.as_bytes().to_vec(),
        ))
    }
}

fn process_keyboard_key(
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    key: Key,
    modifiers: Modifiers,
    pressed: bool,
) -> InputAction {
    if !pressed {
        return InputAction::Ignore;
    }

    let terminal_mode = backend.terminal_mode();
    let binding_action = bindings_layout.get_action(
        InputKind::KeyCode(key),
        modifiers,
        terminal_mode,
    );

    match binding_action {
        BindingAction::Char(c) => {
            let mut buf = [0, 0, 0, 0];
            let str = c.encode_utf8(&mut buf);
            InputAction::BackendCall(BackendCommand::Write(
                str.as_bytes().to_vec(),
            ))
        },
        BindingAction::Esc(seq) => InputAction::BackendCall(
            BackendCommand::Write(seq.as_bytes().to_vec()),
        ),
        _ => InputAction::Ignore,
    }
}

fn process_mouse_wheel(
    state: &mut TerminalViewState,
    font_size: f32,
    unit: MouseWheelUnit,
    delta: Vec2,
) -> InputAction {
    match unit {
        MouseWheelUnit::Line => {
            let lines = delta.y.signum() * delta.y.abs().ceil();
            InputAction::BackendCall(BackendCommand::Scroll(lines as i32))
        },
        MouseWheelUnit::Point => {
            state.scroll_pixels -= delta.y;
            let lines = (state.scroll_pixels / font_size).trunc();
            state.scroll_pixels %= font_size;
            if lines != 0.0 {
                InputAction::BackendCall(BackendCommand::Scroll(-lines as i32))
            } else {
                InputAction::Ignore
            }
        },
        MouseWheelUnit::Page => InputAction::Ignore,
    }
}

fn process_button_click(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    button: PointerButton,
    position: Pos2,
    modifiers: &Modifiers,
    pressed: bool,
) -> InputAction {
    match button {
        PointerButton::Primary => process_left_button(
            state,
            layout,
            backend,
            bindings_layout,
            position,
            modifiers,
            pressed,
        ),
        _ => InputAction::Ignore,
    }
}

fn process_left_button(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    position: Pos2,
    modifiers: &Modifiers,
    pressed: bool,
) -> InputAction {
    let terminal_mode = backend.terminal_mode();
    if terminal_mode.intersects(TerminalMode::MOUSE_MODE) {
        InputAction::BackendCall(BackendCommand::MouseReport {
            button: MouseButton::LeftButton,
            modifiers: *modifiers,
            position: state.current_mouse_position_on_grid,
            is_pressed: pressed,
        })
    } else if pressed {
        process_left_button_pressed(state, layout, position)
    } else {
        process_left_button_released(
            state,
            layout,
            backend,
            bindings_layout,
            position,
            modifiers,
        )
    }
}

fn process_left_button_pressed(
    state: &mut TerminalViewState,
    layout: &Response,
    position: Pos2,
) -> InputAction {
    state.is_dragged = true;
    // InputAction::BackendCall(build_start_select_command(layout, position))
    InputAction::Ignore
}

fn process_left_button_released(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    bindings_layout: &BindingsLayout,
    _position: Pos2,
    modifiers: &Modifiers,
) -> InputAction {
    state.is_dragged = false;
    if layout.double_clicked() || layout.triple_clicked() {
        InputAction::Ignore
        // InputAction::BackendCall(build_start_select_command(layout, position))
    } else {
        let terminal_content = backend.last_content();
        let binding_action = bindings_layout.get_action(
            InputKind::Mouse(PointerButton::Primary),
            *modifiers,
            terminal_content.terminal_mode,
        );

        if binding_action == BindingAction::LinkOpen {
            InputAction::BackendCall(BackendCommand::ProcessLink {
                action: LinkAction::Open,
                position: state.current_mouse_position_on_grid,
            })
        } else {
            InputAction::Ignore
        }
    }
}

// fn build_start_select_command(
//     layout: &Response,
//     cursor_position: Pos2,
// ) -> BackendCommand {
//     let selection_type = if layout.double_clicked() {
//         SelectionType::Semantic
//     } else if layout.triple_clicked() {
//         SelectionType::Lines
//     } else {
//         SelectionType::Simple
//     };

//     BackendCommand::SelectStart(
//         selection_type,
//         cursor_position.x - layout.rect.min.x,
//         cursor_position.y - layout.rect.min.y,
//     )
// }

fn process_mouse_move(
    state: &mut TerminalViewState,
    layout: &Response,
    backend: &TerminalBackend,
    position: Pos2,
    modifiers: &Modifiers,
) -> Vec<InputAction> {
    let cursor_x = position.x - layout.rect.min.x;
    let cursor_y = position.y - layout.rect.min.y;
    state.current_mouse_position_on_grid = TerminalBackend::selection_point(
        cursor_x,
        cursor_y,
        &backend.terminal_size(),
        backend.last_content().display_offset,
    );

    let mut actions = vec![];
    // Handle command or selection update based on terminal mode and modifiers
    if state.is_dragged {
        let terminal_mode = backend.terminal_mode();
        let cmd = if terminal_mode.contains(TerminalMode::MOUSE_MOTION)
            && modifiers.is_none()
        {
            InputAction::BackendCall(BackendCommand::MouseReport {
                button: MouseButton::LeftMove,
                modifiers: *modifiers,
                position: state.current_mouse_position_on_grid,
                is_pressed: true,
            })
        } else {
            // InputAction::BackendCall(BackendCommand::SelectUpdate(
            //     cursor_x, cursor_y,
            // ))

            InputAction::Ignore
        };

        actions.push(cmd);
    }

    // Handle link hover if applicable
    if modifiers.command_only() {
        actions.push(InputAction::BackendCall(BackendCommand::ProcessLink {
            action: LinkAction::Hover,
            position: state.current_mouse_position_on_grid,
        }));
    }

    actions
}
