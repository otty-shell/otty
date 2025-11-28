use iced::alignment::{Horizontal, Vertical};
use iced::font::{Style as FontStyle, Weight as FontWeight};
use iced::mouse::Cursor;
use iced::widget::canvas::{Path, Text};
use iced::widget::container;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};
use iced_core::keyboard::Modifiers;
use iced_core::mouse;
use iced_core::text::{LineHeight, Shaping};
use iced_core::widget::operation;
use iced_graphics::core::Widget;
use iced_graphics::core::widget::{Tree, tree};
use iced_graphics::geometry::Stroke;
use otty_libterm::escape::{self as ansi, CursorShape, StdColor};
use otty_libterm::surface::SurfaceMode;
use otty_libterm::surface::{Flags, Point as TerminalGridPoint};

use crate::input::InputManager;
use crate::term::{Event, Terminal};
use crate::theme::TerminalStyle;

pub struct TerminalView<'a> {
    term: &'a Terminal,
    input_manager: InputManager<'a>,
}

impl<'a> TerminalView<'a> {
    pub fn show(term: &'a Terminal) -> Element<'a, Event> {
        container(Self {
            term,
            input_manager: InputManager::new(term.id, &term.bindings),
        })
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
        let geom = self.term.cache.draw(renderer, viewport.size(), |frame| {
            // Precompute constants used in the inner loop
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
            let display_offset = view.display_offset as f32;
            let half_h = cell_height * 0.5;

            // We use the background pallete color as a default
            // because the widget global background color must be the same
            let default_bg = self
                .term
                .theme
                .get_color(ansi::Color::Std(StdColor::Background));

            let hovered_span_id =
                view.hyperlink_span_id_at(state.mouse_position_on_grid);

            let mut last_line: Option<i32> = None;
            let mut bg_batch_rect = BackgroundRect::default();

            for indexed in view.cells {
                let flags = indexed.cell.flags;
                let is_wide_char_spacer =
                    flags.contains(Flags::WIDE_CHAR_SPACER);
                if is_wide_char_spacer {
                    continue;
                }

                let is_wide_char = flags.contains(Flags::WIDE_CHAR);
                let is_inverse = flags.contains(Flags::INVERSE);
                let is_dim = flags.intersects(Flags::DIM | Flags::DIM_BOLD);
                let is_selected =
                    view.selection.is_some_and(|r| r.contains(indexed.point));

                // Compute per-cell geometry cheaply
                let line = indexed.point.line.0;
                let col = indexed.point.column.0 as f32;

                // Resolve position point for this cell
                let x = layout_offset_x + (col * cell_width);
                let y = layout_offset_y
                    + (((line as f32) + display_offset) * cell_height);
                let cell_render_width = if is_wide_char {
                    cell_width * 2.0
                } else {
                    cell_width
                };
                let cell_size = Size::new(cell_render_width, cell_height);
                let cell_center_y = y + half_h;
                let cell_center_x = x + (cell_render_width * 0.5);

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
                if is_dim {
                    fg.a *= 0.7;
                }

                if is_inverse || is_selected {
                    std::mem::swap(&mut fg, &mut bg);
                }

                // Batch draw backgrounds: skip default background (container already paints it)
                if bg != default_bg {
                    if bg_batch_rect.can_extend(bg, x) {
                        // Same color and contiguous: extend current run
                        bg_batch_rect.extend(cell_render_width);
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
                            .with_width(cell_render_width);
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
                    if flags.intersects(Flags::BOLD | Flags::DIM_BOLD) {
                        font.weight = FontWeight::Bold;
                    }
                    if flags.contains(Flags::ITALIC) {
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
        let view_state = tree.state.downcast_mut::<TerminalViewState>();
        let terminal_state = self.term.engine.snapshot();
        let terminal_size = self.term.engine.terminal_size();
        let font = &self.term.font;
        let layout_size = layout.bounds().size();

        if view_state.size != layout_size {
            view_state.size = layout_size;
            shell.publish(Event::Resize {
                id: self.term.id,
                layout_size: Some(layout_size),
                cell_size: Some(self.term.font.measure),
            });
        }

        if !view_state.is_focused {
            return iced::event::Status::Ignored;
        }

        let mut publish = |event: Event| {
            shell.publish(event);
        };

        match event {
            iced::Event::Mouse(mouse_event)
                if self.is_cursor_in_layout(cursor, layout) =>
            {
                self.input_manager.handle_mouse_event(
                    view_state,
                    terminal_state,
                    terminal_size,
                    font,
                    layout.position(),
                    cursor.position().unwrap(), // Assuming cursor position is always available here.
                    mouse_event,
                    &mut publish,
                )
            },
            iced::Event::Keyboard(keyboard_event) => {
                self.input_manager.handle_keyboard_event(
                    view_state,
                    terminal_state,
                    clipboard,
                    keyboard_event,
                    &mut publish,
                )
            },
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
pub(crate) struct TerminalViewState {
    pub is_focused: bool,
    pub is_dragged: bool,
    pub last_click: Option<mouse::Click>,
    pub scroll_pixels: f32,
    pub keyboard_modifiers: Modifiers,
    pub size: Size<f32>,
    pub mouse_position_on_grid: TerminalGridPoint,
    pub hovered_span_id: Option<u32>,
}

impl TerminalViewState {
    pub(crate) fn new() -> Self {
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
