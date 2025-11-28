use std::sync::Arc;

use iced::alignment::{Horizontal, Vertical};
use iced::font::{Style as FontStyle, Weight as FontWeight};
use iced::mouse::{Cursor, ScrollDelta};
use iced::widget::canvas::{Path, Text};
use iced::widget::container;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};
use iced_core::clipboard::Kind as ClipboardKind;
use iced_core::keyboard::{Key, Modifiers};
use iced_core::mouse::{self, Click};
use iced_core::text::{LineHeight, Shaping};
use iced_core::widget::operation;
use iced_graphics::core::Widget;
use iced_graphics::core::widget::{Tree, tree};
use iced_graphics::geometry::Stroke;
use otty_libterm::TerminalSize;
use otty_libterm::escape::{self as ansi, CursorShape, StdColor};
use otty_libterm::surface::SelectionType;
use otty_libterm::surface::SurfaceMode;
use otty_libterm::surface::{Flags, Point as TerminalGridPoint, SnapshotOwned};

use crate::bindings::{BindingAction, BindingsLayout, InputKind};
use crate::engine::{Engine, MouseButton};
use crate::term::{Event, Terminal};
use crate::theme::TerminalStyle;

pub struct TerminalView<'a> {
    term: &'a Terminal,
}

impl<'a> TerminalView<'a> {
    pub fn show(term: &'a Terminal) -> Element<'a, Event> {
        container(Self { term })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| term.theme.container_style())
            .into()
    }

    pub fn focus<Message: 'static>(
        id: iced::widget::text_input::Id,
    ) -> iced::Task<Message> {
        iced::widget::text_input::focus(id)
    }

    fn is_cursor_in_layout(
        &self,
        cursor: Cursor,
        layout: iced_graphics::core::Layout<'_>,
    ) -> bool {
        if let Some(cursor_position) = cursor.position() {
            let layout_position = layout.position();
            let layout_size = layout.bounds();
            let is_triggered = cursor_position.x >= layout_position.x
                && cursor_position.y >= layout_position.y
                && cursor_position.x < (layout_position.x + layout_size.width)
                && cursor_position.y < (layout_position.y + layout_size.height);

            return is_triggered;
        }

        false
    }

    fn handle_mouse_event(
        &self,
        state: &mut TerminalViewState,
        layout_position: Point,
        cursor_position: Point,
        event: iced::mouse::Event,
        publisher: &mut impl FnMut(Event),
    ) -> iced::event::Status {
        let terminal_state = self.term.engine.snapshot();

        match event {
            iced_core::mouse::Event::ButtonPressed(
                iced_core::mouse::Button::Left,
            ) => Self::handle_left_button_pressed(
                self.term.id,
                state,
                terminal_state,
                cursor_position,
                layout_position,
                publisher,
            ),
            iced_core::mouse::Event::CursorMoved { position } => {
                Self::handle_cursor_moved(
                    self.term.id,
                    state,
                    &self.term.cache,
                    terminal_state,
                    self.term.engine.terminal_size(),
                    position,
                    layout_position,
                    publisher,
                )
            },
            iced_core::mouse::Event::ButtonReleased(
                iced_core::mouse::Button::Left,
            ) => Self::handle_button_released(
                self.term.id,
                state,
                terminal_state,
                &self.term.bindings,
                publisher,
            ),
            iced::mouse::Event::WheelScrolled { delta } => {
                Self::handle_wheel_scrolled(
                    self.term.id,
                    state,
                    delta,
                    &self.term.font.measure,
                    publisher,
                )
            },
            _ => iced::event::Status::Ignored,
        }
    }

    fn handle_left_button_pressed(
        id: u64,
        state: &mut TerminalViewState,
        terminal_state: Arc<SnapshotOwned>,
        cursor_position: Point,
        layout_position: Point,
        publisher: &mut impl FnMut(Event),
    ) -> iced::event::Status {
        let cmd = if terminal_state
            .view()
            .mode
            .intersects(SurfaceMode::MOUSE_MODE)
        {
            Event::MouseReport {
                id,
                button: MouseButton::LeftButton,
                modifiers: state.keyboard_modifiers,
                point: state.mouse_position_on_grid,
                pressed: true,
            }
        } else {
            let current_click = Click::new(
                cursor_position,
                mouse::Button::Left,
                state.last_click,
            );
            let selection_type = match current_click.kind() {
                mouse::click::Kind::Single => SelectionType::Simple,
                mouse::click::Kind::Double => SelectionType::Semantic,
                mouse::click::Kind::Triple => SelectionType::Lines,
            };
            state.last_click = Some(current_click);
            Event::SelectStart {
                id,
                selection_type,
                position: (
                    cursor_position.x - layout_position.x,
                    cursor_position.y - layout_position.y,
                ),
            }
        };
        publisher(cmd);
        state.is_dragged = true;
        iced::event::Status::Captured
    }

    fn handle_cursor_moved(
        id: u64,
        state: &mut TerminalViewState,
        cache: &iced::widget::canvas::Cache,
        terminal_state: Arc<SnapshotOwned>,
        terminal_size: TerminalSize,
        position: Point,
        layout_position: Point,
        publisher: &mut impl FnMut(Event),
    ) -> iced::event::Status {
        let terminal_state = terminal_state.view();
        let cursor_x = position.x - layout_position.x;
        let cursor_y = position.y - layout_position.y;
        state.mouse_position_on_grid = Engine::selection_point(
            cursor_x,
            cursor_y,
            &terminal_size,
            terminal_state.display_offset,
        );

        let hovered_span_id =
            terminal_state.hyperlink_span_id_at(state.mouse_position_on_grid);
        // Handle command or selection update based on terminal mode and modifiers
        if state.is_dragged {
            let terminal_mode = terminal_state.mode;
            let cmd = if terminal_mode.intersects(SurfaceMode::MOUSE_MOTION) {
                Event::MouseReport {
                    id,
                    button: MouseButton::LeftMove,
                    modifiers: state.keyboard_modifiers,
                    point: state.mouse_position_on_grid,
                    pressed: true,
                }
            } else {
                Event::SelectUpdate {
                    id,
                    position: (cursor_x, cursor_y),
                }
            };
            publisher(cmd);
            return iced::event::Status::Captured;
        }

        if hovered_span_id != state.hovered_span_id {
            state.hovered_span_id = hovered_span_id;
            cache.clear();
            return iced::event::Status::Captured;
        }

        iced::event::Status::Ignored
    }

    fn handle_button_released(
        id: u64,
        state: &mut TerminalViewState,
        terminal_state: Arc<SnapshotOwned>,
        bindings: &BindingsLayout, // Use the actual type of your bindings here
        publisher: &mut impl FnMut(Event),
    ) -> iced::event::Status {
        state.is_dragged = false;
        let mut published = false;

        let terminal_state = terminal_state.view();

        if terminal_state.mode.intersects(SurfaceMode::MOUSE_MODE) {
            publisher(Event::MouseReport {
                id,
                button: MouseButton::LeftButton,
                modifiers: state.keyboard_modifiers,
                point: state.mouse_position_on_grid,
                pressed: false,
            });
            published = true;
        }

        if bindings.get_action(
            InputKind::Mouse(iced_core::mouse::Button::Left),
            state.keyboard_modifiers,
            terminal_state.mode,
        ) == BindingAction::LinkOpen
        {
            if let Some(span) =
                terminal_state.hyperlink_span_at(state.mouse_position_on_grid)
            {
                publisher(Event::OpenLink {
                    id,
                    uri: span.link.uri().to_string(),
                });
                published = true;
            }
        }

        if published {
            iced::event::Status::Captured
        } else {
            iced::event::Status::Ignored
        }
    }

    fn handle_wheel_scrolled(
        id: u64,
        state: &mut TerminalViewState,
        delta: ScrollDelta,
        font_measure: &Size<f32>,
        publisher: &mut impl FnMut(Event),
    ) -> iced::event::Status {
        match delta {
            ScrollDelta::Lines { y, .. } => {
                let lines = y.signum() * y.abs().round();
                publisher(Event::Scroll {
                    id,
                    delta: lines as i32,
                });
                iced::event::Status::Captured
            },
            ScrollDelta::Pixels { y, .. } => {
                state.scroll_pixels -= y;
                let line_height = font_measure.height; // Assume this method exists and gives the height of a line
                let lines = (state.scroll_pixels / line_height).trunc();
                state.scroll_pixels %= line_height;
                if lines != 0.0 {
                    publisher(Event::Scroll {
                        id,
                        delta: lines as i32,
                    });
                    iced::event::Status::Captured
                } else {
                    iced::event::Status::Ignored
                }
            },
        }
    }

    fn handle_keyboard_event(
        &self,
        state: &mut TerminalViewState,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        event: iced::keyboard::Event,
        publisher: &mut impl FnMut(Event),
    ) -> iced::event::Status {
        let mut binding_action = BindingAction::Ignore;
        let last_content = self.term.engine.snapshot();
        match event {
            iced::keyboard::Event::ModifiersChanged(m) => {
                state.keyboard_modifiers = m;
            },
            iced::keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            } => match &key {
                // Use the physical character key for bindings even when text is None (e.g., Ctrl/Cmd combos)
                Key::Character(k) => {
                    let lower = k.to_ascii_lowercase();
                    binding_action = self.term.bindings.get_action(
                        InputKind::Char(lower),
                        state.keyboard_modifiers,
                        last_content.view().mode,
                    );

                    // If no binding matched, only write printable text (when provided)
                    if binding_action == BindingAction::Ignore {
                        if let Some(c) = text {
                            publisher(Event::Write {
                                id: self.term.id,
                                data: c.as_bytes().to_vec(),
                            });
                            return iced::event::Status::Captured;
                        }
                    }
                },
                Key::Named(code) => {
                    binding_action = self.term.bindings.get_action(
                        InputKind::KeyCode(*code),
                        modifiers,
                        last_content.view().mode,
                    );
                },
                _ => {},
            },
            _ => {},
        }

        match binding_action {
            BindingAction::Char(c) => {
                let mut buf = [0, 0, 0, 0];
                let str = c.encode_utf8(&mut buf);
                publisher(Event::Write {
                    id: self.term.id,
                    data: str.as_bytes().to_vec(),
                });
                return iced::event::Status::Captured;
            },
            BindingAction::Esc(seq) => {
                publisher(Event::Write {
                    id: self.term.id,
                    data: seq.as_bytes().to_vec(),
                });
                return iced::event::Status::Captured;
            },
            BindingAction::Paste => {
                if let Some(data) = clipboard.read(ClipboardKind::Standard) {
                    let input: Vec<u8> = data.bytes().collect();
                    publisher(Event::Write {
                        id: self.term.id,
                        data: input,
                    });
                    return iced::event::Status::Captured;
                }
            },
            BindingAction::Copy => {
                clipboard.write(
                    ClipboardKind::Standard,
                    self.term.engine.selectable_content(),
                );
            },
            _ => {},
        };

        iced::event::Status::Ignored
    }
}

impl Widget<Event, Theme, iced::Renderer> for TerminalView<'_> {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TerminalViewState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(TerminalViewState::new())
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &iced_core::layout::Limits,
    ) -> iced_core::layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        iced::advanced::layout::Node::new(size)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        _layout: iced_core::Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn operation::Operation,
    ) {
        let state = tree.state.downcast_mut::<TerminalViewState>();
        let wid = iced_core::widget::Id::from(self.term.widget_id());
        operation.focusable(state, Some(&wid));
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout,
        _cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<TerminalViewState>();
        let content = self.term.engine.snapshot();
        let view = content.view();
        let term_size = self.term.engine.terminal_size();
        let cell_width = term_size.cell_width as f32;
        let cell_height = term_size.cell_height as f32;
        let font_size = self.term.font.size;
        let font_scale_factor = self.term.font.scale_factor;
        let layout_offset_x = layout.position().x;
        let layout_offset_y = layout.position().y;

        let geom = self.term.cache.draw(renderer, viewport.size(), |frame| {
            // Precompute constants used in the inner loop
            let display_offset = view.display_offset as f32;
            let cell_size = Size::new(cell_width, cell_height);
            let half_w = cell_width * 0.5;
            let half_h = cell_height * 0.5;
            let hovered_span_id =
                view.hyperlink_span_id_at(state.mouse_position_on_grid);
            // We use the background pallete color as a default
            // because the widget global background color must be the same
            let default_bg = self
                .term
                .theme
                .get_color(ansi::Color::Std(StdColor::Background));

            let mut last_line: Option<i32> = None;
            let mut bg_batch_rect = BackgroundRect::default();

            for indexed in view.cells {
                // Compute per-cell geometry cheaply
                let line = indexed.point.line.0;
                let col = indexed.point.column.0 as f32;

                // Resolve position point for this cell
                let x = layout_offset_x + (col * cell_width);
                let y = layout_offset_y
                    + (((line as f32) + display_offset) * cell_height);
                let cell_center_y = y + half_h;
                let cell_center_x = x + half_w;

                // Resolve colors for this cell
                let mut fg = self.term.theme.get_color(indexed.cell.fg);
                let mut bg = self.term.theme.get_color(indexed.cell.bg);

                // If the new line was detected,
                // need to flush pending background rect and init the new one
                if last_line != Some(line) {
                    if bg_batch_rect.can_flush() {
                        let line = last_line.unwrap_or(line);
                        frame.fill(
                            &bg_batch_rect.build(line),
                            bg_batch_rect.color,
                        );
                    }

                    last_line = Some(line);
                    bg_batch_rect = BackgroundRect::default()
                        .with_cell_height(cell_height)
                        .with_display_offset(display_offset)
                        .with_layout_offset_y(layout_offset_y);
                }

                // Handle dim, inverse, and selected text
                if indexed.cell.flags.intersects(Flags::DIM | Flags::DIM_BOLD) {
                    fg.a *= 0.7;
                }
                if indexed.cell.flags.contains(Flags::INVERSE)
                    || content
                        .view()
                        .selection
                        .is_some_and(|r| r.contains(indexed.point))
                {
                    std::mem::swap(&mut fg, &mut bg);
                }

                // Batch draw backgrounds: skip default background (container already paints it)
                if bg != default_bg {
                    if bg_batch_rect.can_extend(bg, x) {
                        // Same color and contiguous: extend current run
                        bg_batch_rect.extend(cell_width);
                    } else {
                        // New colored run (or non-contiguous): flush previous run if any
                        if bg_batch_rect.can_flush() {
                            frame.fill(
                                &bg_batch_rect.build(line),
                                bg_batch_rect.color,
                            );
                        }

                        // Start a new run but do not draw yet; wait for potential extensions
                        bg_batch_rect = BackgroundRect::default()
                            .with_cell_height(cell_height)
                            .with_display_offset(display_offset)
                            .with_layout_offset_y(layout_offset_y)
                            .activate()
                            .with_color(bg)
                            .with_start_x(x)
                            .with_width(cell_width);
                    }
                } else if bg_batch_rect.can_flush() {
                    // Background returns to default, flush current background rect and init the new one
                    frame.fill(&bg_batch_rect.build(line), bg_batch_rect.color);

                    bg_batch_rect = BackgroundRect::default()
                        .with_cell_height(cell_height)
                        .with_display_offset(display_offset)
                        .with_layout_offset_y(layout_offset_y);
                }

                // Draw hovered hyperlink underline (rare; keep per-cell for correctness)
                if hovered_span_id.is_some_and(|target| {
                    view.hyperlink_span_id_at(indexed.point) == Some(target)
                }) {
                    let underline_height = y + cell_size.height;
                    let underline = Path::line(
                        Point::new(x, underline_height),
                        Point::new(x + cell_size.width, underline_height),
                    );
                    frame.stroke(
                        &underline,
                        Stroke::default()
                            .with_width(font_size * 0.15)
                            .with_color(fg),
                    );
                }

                // Handle cursor rendering
                if view.cursor.point == indexed.point
                    && !matches!(view.cursor.shape, CursorShape::Hidden) 
                {
                    let cursor_color =
                        self.term.theme.get_color(view.cursor.cell.fg);
                    let cursor_rect =
                        Path::rectangle(Point::new(x, y), cell_size);
                    frame.fill(&cursor_rect, cursor_color);
                }

                // Draw text
                if indexed.cell.c != ' ' && indexed.cell.c != '\t' {
                    if view.cursor.point == indexed.point
                        && !view.mode.contains(SurfaceMode::ALT_SCREEN)
                    {
                        fg = bg;
                    }
                    // Resolve font style (bold/italic) from cell flags
                    let mut font = self.term.font.font_type;
                    if indexed
                        .cell
                        .flags
                        .intersects(Flags::BOLD | Flags::DIM_BOLD)
                    {
                        font.weight = FontWeight::Bold;
                    }
                    if indexed.cell.flags.contains(Flags::ITALIC) {
                        font.style = FontStyle::Italic;
                    }
                    let text = Text {
                        content: indexed.cell.c.to_string(),
                        position: Point::new(cell_center_x, cell_center_y),
                        font,
                        size: iced_core::Pixels(font_size),
                        color: fg,
                        horizontal_alignment: Horizontal::Center,
                        vertical_alignment: Vertical::Center,
                        shaping: Shaping::Advanced,
                        line_height: LineHeight::Relative(font_scale_factor),
                    };
                    frame.fill_text(text);
                }
            }

            // Flush any remaining background run at the end
            if bg_batch_rect.can_flush() {
                frame.fill(
                    &bg_batch_rect.build(last_line.unwrap_or(0)),
                    bg_batch_rect.color,
                );
            }
        });

        use iced::advanced::graphics::geometry::Renderer as _;
        renderer.draw_geometry(geom);
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: iced::Event,
        layout: iced_graphics::core::Layout<'_>,
        cursor: Cursor,
        _renderer: &iced::Renderer,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
        _viewport: &Rectangle,
    ) -> iced::event::Status {
        let state = tree.state.downcast_mut::<TerminalViewState>();
        let layout_size = layout.bounds().size();
        if state.size != layout_size {
            state.size = layout_size;
            shell.publish(Event::Resize {
                id: self.term.id,
                layout_size: Some(layout_size),
                cell_size: Some(self.term.font.measure),
            });
        }

        if !state.is_focused {
            return iced::event::Status::Ignored;
        }

        let mut publish = |event: Event| {
            shell.publish(event);
        };

        match event {
            iced::Event::Mouse(mouse_event)
                if self.is_cursor_in_layout(cursor, layout) =>
            {
                self.handle_mouse_event(
                    state,
                    layout.position(),
                    cursor.position().unwrap(), // Assuming cursor position is always available here.
                    mouse_event,
                    &mut publish,
                )
            },
            iced::Event::Keyboard(keyboard_event) => self
                .handle_keyboard_event(
                    state,
                    clipboard,
                    keyboard_event,
                    &mut publish,
                ),
            _ => iced::event::Status::Ignored,
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: iced_core::Layout<'_>,
        cursor: iced_core::mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> iced_core::mouse::Interaction {
        let state = tree.state.downcast_ref::<TerminalViewState>();
        let mut cursor_mode = iced_core::mouse::Interaction::Idle;
        let snapshot = self.term.engine.snapshot();
        let view = snapshot.view();
        let terminal_mode = view.mode;
        if self.is_cursor_in_layout(cursor, layout)
            && !terminal_mode.contains(SurfaceMode::SGR_MOUSE)
        {
            cursor_mode = iced_core::mouse::Interaction::Text;
        }

        if self.is_cursor_in_layout(cursor, layout)
            && view
                .hyperlink_span_id_at(state.mouse_position_on_grid)
                .is_some()
        {
            cursor_mode = iced_core::mouse::Interaction::Pointer;
        }

        cursor_mode
    }
}

impl<'a> From<TerminalView<'a>> for Element<'a, Event, Theme, iced::Renderer> {
    fn from(widget: TerminalView<'a>) -> Self {
        Self::new(widget)
    }
}

#[derive(Debug, Clone)]
struct TerminalViewState {
    is_focused: bool,
    is_dragged: bool,
    last_click: Option<mouse::Click>,
    scroll_pixels: f32,
    keyboard_modifiers: Modifiers,
    size: Size<f32>,
    mouse_position_on_grid: TerminalGridPoint,
    hovered_span_id: Option<u32>,
}

impl TerminalViewState {
    fn new() -> Self {
        Self {
            is_focused: true,
            is_dragged: false,
            last_click: None,
            scroll_pixels: 0.0,
            keyboard_modifiers: Modifiers::empty(),
            size: Size::from([0.0, 0.0]),
            mouse_position_on_grid: TerminalGridPoint::default(),
            hovered_span_id: None,
        }
    }
}

impl Default for TerminalViewState {
    fn default() -> Self {
        Self::new()
    }
}

impl operation::Focusable for TerminalViewState {
    fn is_focused(&self) -> bool {
        self.is_focused
    }

    fn focus(&mut self) {
        self.is_focused = true;
    }

    fn unfocus(&mut self) {
        self.is_focused = false;
    }
}

#[derive(Default)]
struct BackgroundRect {
    display_offset: f32,
    cell_height: f32,
    layout_offset_y: f32,
    is_active: bool,
    color: Color,
    start_x: f32,
    width: f32,
}

impl BackgroundRect {
    fn with_display_offset(mut self, value: f32) -> Self {
        self.display_offset = value;
        self
    }

    fn with_cell_height(mut self, value: f32) -> Self {
        self.cell_height = value;
        self
    }

    fn with_layout_offset_y(mut self, value: f32) -> Self {
        self.layout_offset_y = value;
        self
    }

    fn with_width(mut self, value: f32) -> Self {
        self.width = value;
        self
    }

    fn with_start_x(mut self, value: f32) -> Self {
        self.start_x = value;
        self
    }

    fn with_color(mut self, value: Color) -> Self {
        self.color = value;
        self
    }

    fn activate(mut self) -> Self {
        self.is_active = true;
        self
    }

    fn build(&self, line: i32) -> Path {
        let flush_y = self.layout_offset_y
            + ((line as f32 + self.display_offset) * self.cell_height);
        Path::rectangle(
            Point::new(self.start_x, flush_y),
            Size::new(self.width, self.cell_height),
        )
    }

    fn can_flush(&self) -> bool {
        self.is_active && self.width > 0.0
    }

    fn can_extend(&self, bg: Color, x: f32) -> bool {
        self.is_active
            && bg == self.color
            && (self.start_x + self.width - x).abs() < f32::EPSILON
    }

    fn extend(&mut self, value: f32) {
        self.width += value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::TermFont;
    use crate::settings::FontSettings;
    use iced::widget::canvas::Cache;
    use otty_libterm::TerminalSize;
    use otty_libterm::escape::{Hyperlink, NamedPrivateMode};
    use otty_libterm::surface::{
        Column, Line, SnapshotOwned, Surface, SurfaceActor, SurfaceConfig,
        SurfaceModel,
    };

    const TEST_ID: u64 = 1;

    fn default_snapshot() -> Arc<SnapshotOwned> {
        Arc::new(SnapshotOwned::default())
    }

    fn snapshot_with_modes(modes: &[NamedPrivateMode]) -> Arc<SnapshotOwned> {
        let size = TerminalSize::default();
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        for mode in modes {
            surface.set_private_mode((*mode).into());
        }

        Arc::new(surface.snapshot_owned())
    }

    fn snapshot_with_hyperlink(uri: &str) -> Arc<SnapshotOwned> {
        let size = TerminalSize::default();
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        let link = Hyperlink {
            id: None,
            uri: uri.to_string(),
        };
        surface.grid_mut()[Line(0)][Column(0)].set_hyperlink(Some(link.into()));
        surface.grid_mut()[Line(0)][Column(0)].c = 'h';
        Arc::new(surface.snapshot_owned())
    }

    mod handle_left_button_pressed_tests {
        use super::*;

        #[test]
        fn handles_mouse_mode_with_left_click() {
            let mut state = TerminalViewState::new();
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let _modifiers = Modifiers::empty();

            TerminalView::handle_left_button_pressed(
                TEST_ID,
                &mut state,
                snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]),
                cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Event::MouseReport {
                    id: TEST_ID,
                    button: MouseButton::LeftButton,
                    modifiers: _modifiers,
                    point: TerminalGridPoint {
                        line: Line(0),
                        column: Column(0)
                    },
                    pressed: true,
                }
            ));
            assert!(state.is_dragged);
        }

        #[test]
        fn starts_simple_selection_with_left_click() {
            let cursor_position = Point { x: 200.0, y: 150.0 };
            let layout_position = Point { x: 50.0, y: 50.0 };

            let mut state = TerminalViewState::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            TerminalView::handle_left_button_pressed(
                TEST_ID,
                &mut state,
                default_snapshot(),
                cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Event::SelectStart {
                    id: TEST_ID,
                    selection_type: SelectionType::Simple,
                    position: (150.0, 100.0)
                }
            ));
            assert!(state.is_dragged);
        }
    }

    mod handle_cursor_moved_tests {
        use super::*;

        #[test]
        fn updates_mouse_position_on_grid() {
            let mut state = TerminalViewState::new();
            let terminal_content = default_snapshot();
            let terminal_size = TerminalSize::default();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let cases = vec![
                (
                    Point { x: 0.0, y: 0.0 },
                    Point { x: 1.0, y: 1.0 },
                    TerminalGridPoint {
                        line: Line(1),
                        column: Column(1),
                    },
                ),
                (
                    Point { x: 0.0, y: 0.0 },
                    Point { x: 79.0, y: 0.0 },
                    TerminalGridPoint {
                        line: Line(0),
                        column: Column(79),
                    },
                ),
                (
                    Point { x: 0.0, y: 0.0 },
                    Point {
                        x: 1000.0,
                        y: 1000.0,
                    },
                    TerminalGridPoint {
                        line: Line(49),
                        column: Column(79),
                    },
                ),
            ];

            for (layout_position, cursor_position, expected) in cases {
                TerminalView::handle_cursor_moved(
                    TEST_ID,
                    &mut state,
                    &Cache::default(),
                    terminal_content.clone(),
                    terminal_size,
                    cursor_position,
                    layout_position,
                    &mut publish,
                );

                assert_eq!(state.mouse_position_on_grid, expected);
            }
        }

        #[test]
        fn generates_drag_update_command_when_dragged() {
            let mut state = TerminalViewState::new();
            state.is_dragged = true; // Simulate an ongoing drag operation
            let terminal_content = default_snapshot();
            let terminal_size = TerminalSize::default();
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            TerminalView::handle_cursor_moved(
                TEST_ID,
                &mut state,
                &Cache::default(),
                terminal_content,
                terminal_size,
                cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Event::SelectUpdate {
                    id: TEST_ID,
                    position: (95.0, 145.0)
                }
            ));
        }

        #[test]
        fn selects_update_when_dragged_without_mouse_motion_mode() {
            let mut state = TerminalViewState::new();
            state.is_dragged = true; // Simulate an ongoing drag operation
            let terminal_content = default_snapshot();
            let terminal_size = TerminalSize::default();
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            TerminalView::handle_cursor_moved(
                TEST_ID,
                &mut state,
                &Cache::default(),
                terminal_content,
                terminal_size,
                cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Event::SelectUpdate {
                    id: TEST_ID,
                    position: (95.0, 145.0)
                }
            ));
        }
    }

    mod handle_button_released_tests {
        use super::*;

        #[test]
        fn mouse_mode_activated() {
            let mut state = TerminalViewState::new();
            let bindings = BindingsLayout::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let _modifiers = Modifiers::empty();

            TerminalView::handle_button_released(
                TEST_ID,
                &mut state,
                snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]),
                &bindings,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Event::MouseReport {
                    id: TEST_ID,
                    button: MouseButton::LeftButton,
                    modifiers: _modifiers,
                    point: TerminalGridPoint {
                        line: Line(0),
                        column: Column(0)
                    },
                    pressed: false
                }
            ));
        }

        #[test]
        fn publishes_open_link_event() {
            let mut state = TerminalViewState::new();
            state.keyboard_modifiers = Modifiers::COMMAND;
            state.mouse_position_on_grid = TerminalGridPoint {
                line: Line(0),
                column: Column(0),
            };
            let bindings = BindingsLayout::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            TerminalView::handle_button_released(
                TEST_ID,
                &mut state,
                snapshot_with_hyperlink("https://example.com"),
                &bindings,
                &mut publish,
            );

            assert!(commands.iter().any(|event| matches!(
                event,
                Event::OpenLink { uri, .. } if uri == "https://example.com"
            )));
        }
    }

    mod handle_wheel_scrolled_tests {
        use super::*;

        #[test]
        fn scroll_with_lines_downward() {
            let mut state = TerminalViewState::new();
            let font = TermFont::new(FontSettings::default());
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            TerminalView::handle_wheel_scrolled(
                TEST_ID,
                &mut state,
                ScrollDelta::Lines { y: 3.0, x: 0.0 }, // Scroll down 3 lines
                &font.measure,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Event::Scroll {
                    id: TEST_ID,
                    delta: 3
                }
            ));
        }

        #[test]
        fn scroll_with_lines_upward() {
            let mut state = TerminalViewState::new();
            let font = TermFont::new(FontSettings::default());
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            TerminalView::handle_wheel_scrolled(
                TEST_ID,
                &mut state,
                ScrollDelta::Lines { y: -2.0, x: 0.0 },
                &font.measure,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Event::Scroll {
                    id: TEST_ID,
                    delta: -2
                }
            ));
        }
    }
}
