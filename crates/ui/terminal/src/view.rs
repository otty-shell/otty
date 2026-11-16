use std::any::Any;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use iced::mouse::Cursor;
use iced::widget::canvas::Path;
use iced::widget::container;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};
use iced_core::input_method::{self, InputMethod, Purpose};
use iced_core::keyboard::Modifiers;
use iced_core::mouse;
use iced_core::widget::operation;
use iced_graphics::core::Widget;
use iced_graphics::core::widget::{Tree, tree};
use iced_graphics::geometry::Stroke;
use otty_libterm::escape::{self as ansi, CursorShape, StdColor};
use otty_libterm::surface::{
    BlockKey, Flags, Point as TerminalGridPoint, SurfaceMode,
};

use crate::block_layout;
use crate::block_paint::{BlockBackground, block_paints};
use crate::input::InputManager;
use crate::scroll_mark::ScrollMark;
use crate::term::{BlockCommand, Event, Terminal};
use crate::text::{
    TextDrawConfig, TextDrawState, build_text_runs, draw_text_runs,
};
use crate::theme::{self, TerminalStyle};

mod block_commands;

/// Space between the left or the right edge of the pane and the text of
/// the terminal, in pixels. There is none at the top and the bottom. Block
/// backgrounds and lines are painted across it, from edge to edge; a host
/// that lays its own parts over the terminal lines them up with the text by
/// it.
pub const TEXT_INSET_X: f32 = 16.0;

pub struct TerminalView<'a, T = ()> {
    term: &'a Terminal<T>,
    input_manager: InputManager<'a, T>,
}

impl<'a, T: Clone + PartialEq + 'a> TerminalView<'a, T> {
    /// Build the view of `term`. It fills the pane it is put in: the text
    /// starts 16 px from the left and the right edge, and the backgrounds
    /// and lines of the blocks reach the edges.
    pub fn show(term: &'a Terminal<T>) -> Element<'a, Event<T>> {
        container(Self {
            term,
            input_manager: InputManager::new(
                term.id,
                &term.bindings,
                term.block_selection_mode(),
            ),
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| term.theme.container_style())
        .into()
    }
}

/// Operations that address a terminal view by its widget id.
///
/// These take no `self` and never touch the action type, so they live on
/// the default instantiation rather than on `TerminalView<'_, T>`. A
/// generic impl would force every call site to name a type parameter the
/// body ignores, since nothing in the signatures could infer it. Callers
/// embedding any `T` still write `TerminalView::command(id, cmd)`.
impl TerminalView<'_> {
    pub fn focus<Message: 'static>(
        id: iced::widget::Id,
    ) -> iced::Task<Message> {
        iced::widget::operation::focus(id)
    }

    pub fn command<Message: 'static + Send>(
        id: iced::widget::Id,
        cmd: BlockCommand,
    ) -> iced::Task<Message> {
        struct CommandOperation {
            id: iced::widget::Id,
            command: BlockCommand,
        }

        impl<Message: 'static> operation::Operation<Message> for CommandOperation {
            fn traverse(
                &mut self,
                operate: &mut dyn FnMut(&mut dyn operation::Operation<Message>),
            ) {
                operate(self);
            }

            fn custom(
                &mut self,
                id: Option<&iced::widget::Id>,
                _bounds: Rectangle,
                state: &mut dyn Any,
            ) {
                if id == Some(&self.id)
                    && let Some(state) =
                        state.downcast_mut::<TerminalViewState>()
                {
                    state.queue_block_command(self.command.clone());
                }
            }
        }

        iced::advanced::widget::operate(CommandOperation { id, command: cmd })
    }
}

impl<T: Clone + PartialEq> TerminalView<'_, T> {
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

impl<T: Clone + PartialEq> Widget<Event<T>, Theme, iced::Renderer>
    for TerminalView<'_, T>
{
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
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &iced_core::layout::Limits,
    ) -> iced_core::layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        iced::advanced::layout::Node::new(size)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: iced_core::Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn operation::Operation,
    ) {
        let state = tree.state.downcast_mut::<TerminalViewState>();
        let wid = self.term.widget_id();
        operation.focusable(Some(wid), layout.bounds(), state);
        operation.custom(Some(wid), layout.bounds(), state);
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
        let pane = layout.bounds();
        let text_area = text_area(pane);
        let text_origin = text_area.position();
        let layout_offset_x = text_origin.x;
        let layout_offset_y = text_origin.y;
        // Where every row is: on the line grid, with the space around the
        // blocks, and past the top of the history when the view is there.
        let rows = self.term.engine.row_layout();
        // Blocks are placed in the whole pane, so that their backgrounds
        // and lines reach its edges. A full-screen program has no blocks.
        let block_rects = if view.is_full_screen() {
            Vec::new()
        } else {
            block_layout::block_rects(rows, &view, pane.position(), pane.size())
        };
        let block_paints = block_paints(
            &block_rects,
            state.selected_block_key,
            state.hovered_block_key,
        );
        // A full-screen program has no history to show a place in. The
        // mark is wide while the pointer is on its strip or drags it.
        let is_scroll_mark_active = state.is_scroll_mark_active();
        let scroll_mark = ScrollMark::of_view(pane, text_area, &view)
            .map(|mark| mark.rect(is_scroll_mark_active));
        let scroll_mark_color = if is_scroll_mark_active {
            self.term.theme.scroll_mark_active()
        } else {
            self.term.theme.scroll_mark()
        };
        let hovered_span_id =
            view.hyperlink_span_id_at(state.mouse_position_on_grid);
        let text_runs = build_text_runs(
            &view,
            &self.term.theme,
            self.term.font.font_type,
            hovered_span_id,
            !view.is_full_screen(),
        );

        // Shapes are drawn in window coordinates, so the frame clip must be
        // the visible part of the pane in window coordinates too.
        let clip = pane
            .intersection(viewport)
            .unwrap_or(Rectangle::new(pane.position(), Size::ZERO));

        let geom = self.term.cache.draw_with_bounds(renderer, clip, |frame| {
            // We use the background pallete color as a default
            // because the widget global background color must be the same
            let default_bg = self
                .term
                .theme
                .get_color(ansi::Color::Std(StdColor::Background));

            // Block backgrounds lie under the cells: a cell with its own
            // background is painted over them, a cell with the default
            // background lets them show.
            for paint in &block_paints {
                if let Some(background) = paint.background() {
                    let rect = paint.rect();
                    frame.fill_rectangle(
                        rect.position(),
                        rect.size(),
                        block_background_color(&self.term.theme, background),
                    );
                }
            }

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
                // The lines OTTY adds to the blocks are not drawn selected.
                let is_selected = view.is_selected(indexed.point);

                // Compute per-cell geometry cheaply
                let line = indexed.point.line.0;
                let col = indexed.point.column.0 as f32;

                // Resolve position point for this cell
                let x = layout_offset_x + (col * cell_width);
                let y = layout_offset_y + rows.line_top(line);
                let cell_render_width = if is_wide_char {
                    cell_width * 2.0
                } else {
                    cell_width
                };
                let cell_size = Size::new(cell_render_width, cell_height);

                // Resolve colors for this cell
                let mut fg = self.term.theme.get_color(indexed.cell.fg);
                let mut bg = self.term.theme.get_color(indexed.cell.bg);

                // If the new line was detected,
                // need to flush pending background rect and init the new one
                if last_line != Some(line) {
                    if bg_batch_rect.can_flush() {
                        frame.fill(&bg_batch_rect.build(), bg_batch_rect.color);
                    }

                    last_line = Some(line);
                    bg_batch_rect =
                        BackgroundRect::default().on_row(y, cell_height);
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
                                &bg_batch_rect.build(),
                                bg_batch_rect.color,
                            );
                        }

                        // Start a new run but do not draw yet; wait for potential extensions
                        bg_batch_rect = BackgroundRect::default()
                            .on_row(y, cell_height)
                            .activate()
                            .with_color(bg)
                            .with_start_x(x)
                            .with_width(cell_render_width);
                    }
                } else if bg_batch_rect.can_flush() {
                    // Background returns to default, flush current background rect and init the new one
                    frame.fill(&bg_batch_rect.build(), bg_batch_rect.color);

                    bg_batch_rect =
                        BackgroundRect::default().on_row(y, cell_height);
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
            }

            // Flush any remaining background run at the end
            if bg_batch_rect.can_flush() {
                frame.fill(&bg_batch_rect.build(), bg_batch_rect.color);
            }

            // The stripe and the line are painted after the cells, so a
            // cell with its own background does not break the line. The
            // text is drawn later and stays above both.
            for paint in &block_paints {
                if let Some(stripe) = paint.failed_stripe() {
                    frame.fill_rectangle(
                        stripe.position(),
                        stripe.size(),
                        self.term.theme.block_failed_stripe(),
                    );
                }

                if let Some(line) = paint.line() {
                    frame.fill_rectangle(
                        line.position(),
                        line.size(),
                        self.term.theme.block_line(),
                    );
                }
            }

            // The scroll mark lies over everything the widget paints, in the
            // right inset where no text is drawn.
            if let Some(mark) = scroll_mark {
                frame.fill_rectangle(
                    mark.position(),
                    mark.size(),
                    scroll_mark_color,
                );
            }
        });

        use iced::advanced::graphics::geometry::Renderer as _;
        renderer.draw_geometry(geom);
        let text_config = TextDrawConfig::new(
            text_origin,
            rows,
            cell_width,
            cell_height,
            font_size,
            font_scale_factor,
        );
        draw_text_runs(
            renderer,
            &text_runs,
            &text_config,
            text_area,
            &state.text_draw_state,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: iced_graphics::core::Layout<'_>,
        cursor: Cursor,
        _renderer: &iced::Renderer,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event<T>>,
        _viewport: &Rectangle,
    ) {
        let view_state = tree.state.downcast_mut::<TerminalViewState>();
        let terminal_state = self.term.engine.snapshot();
        let terminal_size = self.term.engine.terminal_size();
        let rows = self.term.engine.row_layout();
        let font = &self.term.font;
        // The grid of the terminal fills the text area, not the whole pane.
        let pane = layout.bounds();
        let text_area = text_area(pane);
        let layout_size = text_area.size();
        let text_origin = text_area.position();
        let terminal_id = self.term.id;
        let scroll_mark =
            ScrollMark::of_view(pane, text_area, &terminal_state.view());

        view_state.flush_pending_resize(terminal_id, shell);
        self.process_pending_commands(view_state, clipboard, shell);

        if view_state.size != layout_size
            || view_state.terminal_id != Some(terminal_id)
        {
            view_state.size = layout_size;
            view_state.terminal_id = Some(terminal_id);
            view_state.queue_resize(
                terminal_id,
                layout_size,
                self.term.font.measure,
                shell,
            );
        }

        // Keep the platform input method (IME) enabled while the terminal is
        // focused so composed input such as Chinese, Japanese or Korean is
        // delivered as `InputMethod` events instead of being lost. The cursor
        // rectangle anchors the candidate window to the terminal caret. This is
        // re-requested on every update because iced resets the input method
        // state each cycle.
        if view_state.is_focused {
            let cursor_point = terminal_state.view().cursor.point;
            let cell_width = terminal_size.cell_width as f32;
            let cell_height = terminal_size.cell_height as f32;

            let cursor_area = Rectangle::new(
                Point::new(
                    text_origin.x + cursor_point.column.0 as f32 * cell_width,
                    text_origin.y + rows.line_top(cursor_point.line.0),
                ),
                Size::new(cell_width, cell_height),
            );

            shell.request_input_method(&InputMethod::<String>::Enabled {
                cursor: cursor_area,
                purpose: Purpose::Terminal,
                // The composition string is rendered by the platform IME's own
                // candidate window, so no over-the-spot preedit is requested.
                preedit: None,
            });
        }

        let mut publish = |event: Event<T>| {
            shell.publish(event);
        };

        // The widget sees a new snapshot first here, on the next event it
        // gets after the host took it: at the latest the redraw before the
        // next frame. The content may have moved under a pointer that did
        // not, so the block under the pointer is looked up again.
        let snapshot_version = self.term.engine.snapshot_version();
        if view_state.hover_snapshot_version != snapshot_version {
            view_state.hover_snapshot_version = snapshot_version;
            self.input_manager.handle_content_moved(
                view_state,
                &self.term.engine,
                text_origin,
                scroll_mark.as_ref(),
                &mut publish,
            );
        }

        let is_in_pane = self.is_cursor_in_layout(cursor, layout);
        let is_layer_over_pane = is_under_a_layer(cursor, layout);
        // A drag the terminal started (a text selection, a press sent to a
        // program, or the scroll mark) keeps the pointer until the button is
        // released, wherever the pointer goes: over a layer of the host, out
        // of the pane or out of the window. The release ends it there.
        let has_drag = is_drag_on(view_state);
        // The wheel scrolls the history under a layer of the host too, such
        // as the fold arrow of a block: the layer does not use it.
        let is_wheel_under_a_layer = is_layer_over_pane
            && matches!(
                event,
                iced::Event::Mouse(mouse::Event::WheelScrolled { .. })
            );

        let status = match event {
            iced::Event::Mouse(mouse_event)
                if is_in_pane || has_drag || is_wheel_under_a_layer =>
            {
                if !view_state.is_focused
                    && !matches!(
                        mouse_event,
                        iced::mouse::Event::CursorMoved { .. }
                    )
                {
                    return;
                }

                // The window may have lost the pointer during a drag; the
                // release still ends the drag, where the pointer was last
                // seen.
                let Some(cursor_position) =
                    cursor.land().position().or(view_state.pointer_position())
                else {
                    return;
                };

                // Mouse events stay uncaptured even when handled here: the
                // host follows the pointer over the terminal too, for
                // example to place a context menu or to drag a pane.
                let _ = self.input_manager.handle_mouse_event(
                    view_state,
                    &self.term.engine,
                    font,
                    text_origin,
                    cursor_position,
                    scroll_mark.as_ref(),
                    mouse_event,
                    &mut publish,
                );

                // A drag that ended away from the pane leaves no pointer
                // over it.
                if !is_in_pane && !is_layer_over_pane && !is_drag_on(view_state)
                {
                    self.input_manager
                        .handle_pointer_left(view_state, &mut publish);
                }
                iced::event::Status::Ignored
            },
            // A layer of the host over the pane has the pointer, such as the
            // fold arrow of a block. The pointer is still over the pane, so
            // the block under the layer stays hovered: forgetting it would
            // hide the arrow under the pointer, and the next move would
            // show it again. The pointer is kept, for a lookup after the
            // content moves. It is off the strip of the scroll mark.
            iced::Event::Mouse(_) if is_layer_over_pane => {
                view_state.set_pointer_position(cursor.land().position());
                self.input_manager
                    .leave_scroll_strip(view_state, &mut publish);
                iced::event::Status::Ignored
            },
            // The pointer is not over the pane any more.
            iced::Event::Mouse(_) => {
                self.input_manager
                    .handle_pointer_left(view_state, &mut publish);
                iced::event::Status::Ignored
            },
            iced::Event::Keyboard(keyboard_event) if view_state.is_focused => {
                self.input_manager.handle_keyboard_event(
                    view_state,
                    terminal_state,
                    clipboard,
                    keyboard_event,
                    &mut publish,
                )
            },
            iced::Event::InputMethod(input_method::Event::Commit(text))
                if view_state.is_focused =>
            {
                // Typed text ends the block selection, as a typed key does.
                self.input_manager
                    .clear_block_selection(view_state, &mut publish);
                publish(Event::Write {
                    id: terminal_id,
                    data: text.as_bytes().to_vec(),
                });
                iced::event::Status::Captured
            },
            _ => iced::event::Status::Ignored,
        };

        // A key the terminal used, as typed text or as a binding it reported,
        // must not reach the host's keyboard listeners (such as
        // `iced::keyboard::listen`) too, or a shortcut would run twice.
        if status == iced::event::Status::Captured {
            shell.capture_event();
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

        // The strip of the scroll mark is no text: the pointer keeps its
        // arrow there and while it drags the mark.
        let pane = layout.bounds();
        let is_on_scroll_strip = cursor.position().is_some_and(|position| {
            ScrollMark::of_view(pane, text_area(pane), &view)
                .is_some_and(|mark| mark.strip().contains(position))
        });
        if is_on_scroll_strip || state.scroll_mark_grab().is_some() {
            return cursor_mode;
        }

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

impl<'a, T: Clone + PartialEq + 'a> From<TerminalView<'a, T>>
    for Element<'a, Event<T>, Theme, iced::Renderer>
{
    fn from(widget: TerminalView<'a, T>) -> Self {
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
    /// Where the pointer was last seen over the pane, in window
    /// coordinates; `None` while it is away from the pane.
    pointer_position: Option<Point>,
    pub mouse_position_on_grid: TerminalGridPoint,
    pub hovered_span_id: Option<u32>,
    pub hovered_block_key: Option<BlockKey>,
    pub selected_block_key: Option<BlockKey>,
    pub selection_in_progress: bool,
    pub terminal_id: Option<u64>,
    /// Version of the snapshot the block under the pointer was last looked
    /// up in, see [`crate::engine::Engine::snapshot_version`].
    hover_snapshot_version: u64,
    /// How far below the top of the scroll mark the pointer holds it, in
    /// pixels, while it drags the mark.
    scroll_mark_grab: Option<f32>,
    /// Whether the pointer is on the strip of the scroll mark, the right
    /// inset of the pane.
    is_pointer_on_scroll_strip: bool,
    pending_block_commands: VecDeque<BlockCommand>,
    pending_resize: Option<Size<f32>>,
    pending_cell_size: Option<Size<f32>>,
    pending_resize_deadline: Option<Instant>,
    last_resize_sent_at: Option<Instant>,
    text_draw_state: TextDrawState,
}

impl TerminalViewState {
    /// Where the pointer was last seen over the pane, in window
    /// coordinates; `None` while it is away from the pane.
    pub(crate) fn pointer_position(&self) -> Option<Point> {
        self.pointer_position
    }

    /// How far below the top of the scroll mark the pointer holds it, in
    /// pixels; `None` while the mark is not dragged.
    pub(crate) fn scroll_mark_grab(&self) -> Option<f32> {
        self.scroll_mark_grab
    }

    /// Whether the scroll mark is drawn wide and bright: the pointer is on
    /// its strip or drags it.
    pub(crate) fn is_scroll_mark_active(&self) -> bool {
        self.scroll_mark_grab.is_some() || self.is_pointer_on_scroll_strip
    }

    pub(crate) fn new() -> Self {
        Self {
            is_focused: true,
            is_dragged: false,
            last_click: None,
            scroll_pixels: 0.0,
            keyboard_modifiers: Modifiers::empty(),
            size: Size::from([0.0, 0.0]),
            pointer_position: None,
            mouse_position_on_grid: TerminalGridPoint::default(),
            hovered_span_id: None,
            hovered_block_key: None,
            selected_block_key: None,
            selection_in_progress: false,
            terminal_id: None,
            hover_snapshot_version: 0,
            scroll_mark_grab: None,
            is_pointer_on_scroll_strip: false,
            pending_block_commands: VecDeque::new(),
            pending_resize: None,
            pending_cell_size: None,
            pending_resize_deadline: None,
            last_resize_sent_at: None,
            text_draw_state: TextDrawState::new(),
        }
    }

    fn queue_resize<T>(
        &mut self,
        terminal_id: u64,
        layout_size: Size<f32>,
        cell_size: Size<f32>,
        shell: &mut iced_graphics::core::Shell<'_, Event<T>>,
    ) {
        const THROTTLE: Duration = Duration::from_millis(33);
        let now = Instant::now();
        let should_send = self
            .last_resize_sent_at
            .map(|last| now.saturating_duration_since(last) >= THROTTLE)
            .unwrap_or(true);

        if should_send {
            self.publish_resize(
                terminal_id,
                layout_size,
                cell_size,
                shell,
                now,
            );
        } else {
            self.pending_resize = Some(layout_size);
            self.pending_cell_size = Some(cell_size);
            self.pending_resize_deadline =
                self.last_resize_sent_at.map(|last| last + THROTTLE);
            if let Some(deadline) = self.pending_resize_deadline {
                shell.request_redraw_at(deadline);
            }
        }
    }

    fn publish_resize<T>(
        &mut self,
        terminal_id: u64,
        layout_size: Size<f32>,
        cell_size: Size<f32>,
        shell: &mut iced_graphics::core::Shell<'_, Event<T>>,
        now: Instant,
    ) {
        self.last_resize_sent_at = Some(now);
        self.pending_resize = None;
        self.pending_cell_size = None;
        self.pending_resize_deadline = None;
        shell.publish(Event::Resize {
            id: terminal_id,
            layout_size: Some(layout_size),
            cell_size: Some(cell_size),
        });
    }

    fn flush_pending_resize<T>(
        &mut self,
        terminal_id: u64,
        shell: &mut iced_graphics::core::Shell<'_, Event<T>>,
    ) {
        if let (Some(layout_size), Some(cell_size), Some(deadline)) = (
            self.pending_resize,
            self.pending_cell_size,
            self.pending_resize_deadline,
        ) {
            if Instant::now() >= deadline {
                self.publish_resize(
                    terminal_id,
                    layout_size,
                    cell_size,
                    shell,
                    Instant::now(),
                );
            } else {
                shell.request_redraw_at(deadline);
            }
        }
    }

    /// Note where the pointer is over the pane, in window coordinates, or
    /// `None` when it left the pane.
    pub(crate) fn set_pointer_position(&mut self, position: Option<Point>) {
        self.pointer_position = position;
    }

    /// Take the scroll mark `grab` px below its top: the drag starts.
    pub(crate) fn grab_scroll_mark(&mut self, grab: f32) {
        self.scroll_mark_grab = Some(grab);
    }

    /// Let go of the scroll mark: the drag ends.
    pub(crate) fn release_scroll_mark(&mut self) {
        self.scroll_mark_grab = None;
    }

    /// Note whether the pointer is on the strip of the scroll mark, and
    /// return whether that changed, so the mark must be drawn again.
    pub(crate) fn set_pointer_on_scroll_strip(&mut self, is_on: bool) -> bool {
        let is_changed = self.is_pointer_on_scroll_strip != is_on;
        self.is_pointer_on_scroll_strip = is_on;

        is_changed
    }

    fn queue_block_command(&mut self, command: BlockCommand) {
        self.pending_block_commands.push_back(command);
    }

    fn next_block_command(&mut self) -> Option<BlockCommand> {
        self.pending_block_commands.pop_front()
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

/// A run of cells of one background color on one row, painted at once.
#[derive(Default)]
struct BackgroundRect {
    /// The top of the row, in window coordinates.
    top: f32,
    height: f32,
    is_active: bool,
    color: Color,
    start_x: f32,
    width: f32,
}

impl BackgroundRect {
    /// Place the run on the row whose top is `top`, `height` high.
    fn on_row(mut self, top: f32, height: f32) -> Self {
        self.top = top;
        self.height = height;
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

    fn build(&self) -> Path {
        Path::rectangle(
            Point::new(self.start_x, self.top),
            Size::new(self.width, self.height),
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

/// The part of the pane that holds the text of the terminal: the pane
/// without the inset at its left and its right.
fn text_area(pane: Rectangle) -> Rectangle {
    let inset = TEXT_INSET_X.min(pane.width / 2.0);

    Rectangle {
        x: pane.x + inset,
        width: pane.width - inset * 2.0,
        ..pane
    }
}

/// Whether a drag the terminal started is on: a press in the text (a text
/// selection, or a press sent to a program) or on the scroll mark.
fn is_drag_on(state: &TerminalViewState) -> bool {
    state.is_dragged || state.scroll_mark_grab().is_some()
}

/// Whether the pointer is over the pane but a layer above the terminal has
/// it: iced then gives the terminal a levitating cursor.
fn is_under_a_layer(
    cursor: Cursor,
    layout: iced_graphics::core::Layout<'_>,
) -> bool {
    cursor.is_levitating() && cursor.land().is_over(layout.bounds())
}

/// The color the theme gives to a block background.
fn block_background_color(
    theme: &theme::Theme,
    background: BlockBackground,
) -> Color {
    match background {
        BlockBackground::Selected => theme.block_selected_background(),
        BlockBackground::Hovered => theme.block_hovered_background(),
        BlockBackground::Failed => theme.block_failed_background(),
    }
}

#[cfg(test)]
mod tests {
    use iced_core::keyboard::key::{NativeCode, Physical};
    use iced_core::keyboard::{self, Key, Location};
    use iced_core::renderer::Headless;
    use otty_libterm::TerminalSize;
    use otty_libterm::escape::NamedPrivateMode;
    use otty_libterm::surface::{
        BlockKind, BlockMeta, BlockSnapshot, Column, Line, Scroll,
        SnapshotOwned, Surface, SurfaceActor, SurfaceConfig, SurfaceModel,
    };

    use super::*;
    use crate::bindings::{BindingAction, InputKind, KeyboardBinding};
    use crate::testing::{HistoryBlock, history_snapshot};

    /// A stand-in for the host application's action type; the terminal
    /// never interprets it.
    #[derive(Clone, Debug, PartialEq)]
    enum TestAction {
        Split,
    }

    const VIEW_SIZE: Size = Size::new(640.0, 480.0);

    /// A grid of 36 columns and 4 rows of 8 by 16 px cells. It fills the
    /// text area of a pane of `PANE_SIZE`.
    const PANE_GRID: TerminalSize = TerminalSize {
        cell_width: 8,
        cell_height: 16,
        cols: 36,
        rows: 4,
    };

    /// A pane as wide as the grid plus 16 px at each side, and as high as
    /// the grid.
    const PANE_SIZE: Size = Size::new(320.0, 64.0);

    /// The color the window has where the terminal paints nothing.
    const UNPAINTED: [u8; 4] = [255, 255, 255, 255];

    const CELL_BG: ansi::Color = ansi::Color::TrueColor(ansi::Rgb {
        r: 200,
        g: 30,
        b: 40,
    });

    /// A picture of a terminal pane that sits at the top left corner of
    /// the window.
    struct PanePicture {
        pixels: Vec<u8>,
    }

    impl PanePicture {
        /// The RGBA pixel `x` px from the left and `y` px from the top
        /// edge of the pane.
        fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
            let at = (y * PANE_SIZE.width as usize + x) * 4;
            [
                self.pixels[at],
                self.pixels[at + 1],
                self.pixels[at + 2],
                self.pixels[at + 3],
            ]
        }
    }

    /// Give `event` to a focused terminal view, with the pointer over it,
    /// and return whether the view captured it. A captured event never
    /// reaches the host's own listeners, such as `iced::keyboard::listen`.
    fn is_captured_by_a_focused_terminal(event: iced::Event) -> bool {
        let mut term = Terminal::<TestAction>::with_snapshot_for_test(
            7,
            SnapshotOwned::default(),
        );
        term.add_bindings(vec![(
            KeyboardBinding {
                target: InputKind::Char(String::from("d")),
                modifiers: Modifiers::LOGO,
                mode_include: SurfaceMode::empty(),
                mode_exclude: SurfaceMode::empty(),
            },
            BindingAction::Action(TestAction::Split),
        )]);
        let mut view = TerminalView {
            term: &term,
            input_manager: InputManager::new(
                term.id,
                &term.bindings,
                term.block_selection_mode(),
            ),
        };
        let mut tree = Tree::new(
            &view as &dyn Widget<Event<TestAction>, Theme, iced::Renderer>,
        );
        tree.state.downcast_mut::<TerminalViewState>().is_focused = true;
        let node = iced_core::layout::Node::new(VIEW_SIZE);
        let pointer = Cursor::Available(Point::new(320.0, 240.0));
        let renderer = iced::futures::executor::block_on(
            <iced::Renderer as Headless>::new(
                iced::Font::DEFAULT,
                iced::Pixels(16.0),
                Some("tiny-skia"),
            ),
        )
        .expect("a headless renderer");
        let mut published = Vec::new();
        let mut shell = iced_graphics::core::Shell::new(&mut published);

        view.update(
            &mut tree,
            &event,
            iced_core::Layout::new(&node),
            pointer,
            &renderer,
            &mut iced_core::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(VIEW_SIZE),
        );

        shell.is_event_captured()
    }

    fn key_press(
        character: &str,
        text: Option<&str>,
        modifiers: Modifiers,
    ) -> iced::Event {
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Character(character.into()),
            modified_key: Key::Character(character.into()),
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers,
            text: text.map(Into::into),
            repeat: false,
        })
    }

    /// Give `event` to the view of `term` that fills a pane of `pane_size`
    /// at the top left corner of the window, with the pointer at `pointer`
    /// and `state` as the state of the view. Return what the view
    /// published and the state it left.
    fn update_view(
        term: &Terminal<TestAction>,
        pane_size: Size,
        event: iced::Event,
        pointer: Cursor,
        state: TerminalViewState,
    ) -> (Vec<Event<TestAction>>, TerminalViewState) {
        let mut view = TerminalView {
            term,
            input_manager: InputManager::new(
                term.id,
                &term.bindings,
                term.block_selection_mode(),
            ),
        };
        let mut tree = Tree::new(
            &view as &dyn Widget<Event<TestAction>, Theme, iced::Renderer>,
        );
        *tree.state.downcast_mut::<TerminalViewState>() = state;
        let node = iced_core::layout::Node::new(pane_size);
        let renderer = iced::futures::executor::block_on(
            <iced::Renderer as Headless>::new(
                iced::Font::DEFAULT,
                iced::Pixels(16.0),
                Some("tiny-skia"),
            ),
        )
        .expect("a headless renderer");
        let mut published = Vec::new();
        let mut shell = iced_graphics::core::Shell::new(&mut published);

        view.update(
            &mut tree,
            &event,
            iced_core::Layout::new(&node),
            pointer,
            &renderer,
            &mut iced_core::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(pane_size),
        );

        drop(shell);
        let state = tree.state.downcast_ref::<TerminalViewState>().clone();
        (published, state)
    }

    /// A terminal with an empty screen.
    fn empty_terminal() -> Terminal<TestAction> {
        Terminal::with_snapshot_for_test(7, SnapshotOwned::default())
    }

    /// An empty screen of `PANE_GRID`.
    fn pane_surface() -> Surface {
        Surface::new(SurfaceConfig::default(), &PANE_GRID)
    }

    /// The keys of the blocks of the tests.
    const START: BlockKey = BlockKey::new(0);
    const LS: BlockKey = BlockKey::new(1);
    const PWD: BlockKey = BlockKey::new(2);
    const PROMPT: BlockKey = BlockKey::new(3);
    const SLEEP: BlockKey = BlockKey::new(4);
    const FALSE: BlockKey = BlockKey::new(5);
    const VIM: BlockKey = BlockKey::new(6);

    /// A block that holds `line_count` lines of the pane from `start_line`.
    fn block(
        key: BlockKey,
        kind: BlockKind,
        start_line: i32,
        line_count: usize,
    ) -> BlockSnapshot {
        BlockSnapshot {
            key,
            meta: BlockMeta {
                kind,
                ..BlockMeta::default()
            },
            start_line,
            line_count,
            cached_text: None,
            has_status_line: false,
            is_status_line_visible: false,
            ..BlockSnapshot::default()
        }
    }

    /// The same block after its command ended with `exit_code`.
    fn finished(mut block: BlockSnapshot, exit_code: i32) -> BlockSnapshot {
        block.meta.is_finished = true;
        block.meta.exit_code = Some(exit_code);

        block
    }

    /// A terminal that shows `snapshot` in a pane of `PANE_SIZE`.
    fn pane_terminal(snapshot: SnapshotOwned) -> Terminal<TestAction> {
        pane_terminal_with_snapshots(vec![snapshot])
    }

    /// A terminal that shows the first of `snapshots` in a pane of
    /// `PANE_SIZE` and the next one after every [`Event::ContentSync`].
    fn pane_terminal_with_snapshots(
        snapshots: Vec<SnapshotOwned>,
    ) -> Terminal<TestAction> {
        let mut term =
            Terminal::<TestAction>::with_snapshots_for_test(7, snapshots);
        let text_size = text_area(Rectangle::with_size(PANE_SIZE)).size();
        term.engine
            .resize(Some(text_size), Some(Size::new(8.0, 16.0)));

        term
    }

    /// Draw `term` in a pane of `PANE_SIZE` with `state` as the state of
    /// its view, in a white window.
    fn picture_of_pane(
        term: &Terminal<TestAction>,
        state: TerminalViewState,
    ) -> PanePicture {
        let view = TerminalView {
            term,
            input_manager: InputManager::new(
                term.id,
                &term.bindings,
                term.block_selection_mode(),
            ),
        };
        let mut tree = Tree::new(
            &view as &dyn Widget<Event<TestAction>, Theme, iced::Renderer>,
        );
        *tree.state.downcast_mut::<TerminalViewState>() = state;
        let node = iced_core::layout::Node::new(PANE_SIZE);
        let mut renderer = iced::futures::executor::block_on(
            <iced::Renderer as Headless>::new(
                iced::Font::DEFAULT,
                iced::Pixels(16.0),
                Some("tiny-skia"),
            ),
        )
        .expect("a headless renderer");

        view.draw(
            &tree,
            &mut renderer,
            &Theme::Dark,
            &iced::advanced::renderer::Style::default(),
            iced_core::Layout::new(&node),
            Cursor::Unavailable,
            &Rectangle::with_size(PANE_SIZE),
        );

        let window = Size::new(PANE_SIZE.width as u32, PANE_SIZE.height as u32);
        PanePicture {
            pixels: renderer.screenshot(window, 1.0, Color::WHITE),
        }
    }

    /// Draw a terminal whose every cell has `cell_bg` at window position
    /// (300, 60), size 800x400, with the given `viewport`, and return the
    /// RGBA pixel in the middle of its bottom-right cell from a screenshot
    /// taken at `scale`. The text is 16 px in from the left and the right
    /// edge, so the grid is 96 columns wide and its last column ends 16 px
    /// before the right edge of the pane.
    fn bottom_right_cell_pixel(
        cell_bg: ansi::Color,
        viewport: Rectangle,
        scale: f32,
    ) -> [u8; 4] {
        let grid = TerminalSize {
            cell_width: 8,
            cell_height: 16,
            cols: 96,
            rows: 25,
        };
        let mut surface = Surface::new(SurfaceConfig::default(), &grid);
        for line in 0..grid.rows {
            for column in 0..grid.cols {
                surface.grid_mut()[Line(i32::from(line))]
                    [Column(usize::from(column))]
                .bg = cell_bg;
            }
        }
        let mut term = Terminal::<TestAction>::with_snapshot_for_test(
            7,
            surface.snapshot_owned(),
        );
        term.engine
            .resize(Some(Size::new(768.0, 400.0)), Some(Size::new(8.0, 16.0)));
        let view = TerminalView {
            term: &term,
            input_manager: InputManager::new(
                term.id,
                &term.bindings,
                term.block_selection_mode(),
            ),
        };
        let tree = Tree::new(
            &view as &dyn Widget<Event<TestAction>, Theme, iced::Renderer>,
        );
        let node = iced_core::layout::Node::new(Size::new(800.0, 400.0))
            .move_to(Point::new(300.0, 60.0));
        let mut renderer = iced::futures::executor::block_on(
            <iced::Renderer as Headless>::new(
                iced::Font::DEFAULT,
                iced::Pixels(16.0),
                Some("tiny-skia"),
            ),
        )
        .expect("a headless renderer");

        view.draw(
            &tree,
            &mut renderer,
            &Theme::Dark,
            &iced::advanced::renderer::Style::default(),
            iced_core::Layout::new(&node),
            Cursor::Unavailable,
            &viewport,
        );

        let window = Size::new(
            (1100.0 * scale).round() as u32,
            (460.0 * scale).round() as u32,
        );
        let pixels = renderer.screenshot(window, scale, Color::WHITE);
        let x = (1080.0 * scale) as usize;
        let y = (452.0 * scale) as usize;
        let at = (y * window.width as usize + x) * 4;
        [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
    }

    #[test]
    fn a_typed_letter_is_captured() {
        let letter = key_press("d", Some("d"), Modifiers::empty());

        assert!(is_captured_by_a_focused_terminal(letter));
    }

    /// The host gets the key once, as the binding event, and not a second
    /// time from its own keyboard listener.
    #[test]
    fn a_bound_action_key_is_captured() {
        let action = key_press("d", None, Modifiers::LOGO);

        assert!(is_captured_by_a_focused_terminal(action));
    }

    /// The copy key copies the selection but leaves the key to the host.
    #[test]
    fn the_copy_key_is_not_captured() {
        let modifiers = if cfg!(target_os = "macos") {
            Modifiers::COMMAND
        } else {
            Modifiers::SHIFT | Modifiers::COMMAND
        };
        let copy = key_press("c", None, modifiers);

        assert!(!is_captured_by_a_focused_terminal(copy));
    }

    #[test]
    fn composed_text_is_captured() {
        let commit = iced::Event::InputMethod(input_method::Event::Commit(
            String::from("日本"),
        ));

        assert!(is_captured_by_a_focused_terminal(commit));
    }

    /// The host follows the pointer around the terminal, to place its menus
    /// and to drag panes, so it must still get mouse events.
    #[test]
    fn a_press_in_the_terminal_is_not_captured() {
        let press = iced::Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ));
        let moved = iced::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(320.0, 240.0),
        });

        assert!(!is_captured_by_a_focused_terminal(press));
        assert!(!is_captured_by_a_focused_terminal(moved));
    }

    #[test]
    fn the_text_area_is_the_pane_without_16_px_at_the_left_and_the_right() {
        let pane =
            Rectangle::new(Point::new(300.0, 60.0), Size::new(800.0, 400.0));

        let text = text_area(pane);

        assert_eq!(
            text,
            Rectangle::new(Point::new(316.0, 60.0), Size::new(768.0, 400.0))
        );
    }

    #[test]
    fn the_text_area_of_a_pane_narrower_than_the_insets_is_empty() {
        let pane =
            Rectangle::new(Point::new(300.0, 60.0), Size::new(20.0, 400.0));

        let text = text_area(pane);

        assert_eq!(text.width, 0.0);
        assert_eq!(text.height, 400.0);
    }

    /// The number of columns and rows follows the text area, not the pane.
    #[test]
    fn the_terminal_is_resized_to_the_text_area() {
        let moved = iced::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(320.0, 240.0),
        });

        let (published, _) = update_view(
            &empty_terminal(),
            VIEW_SIZE,
            moved,
            Cursor::Available(Point::new(320.0, 240.0)),
            TerminalViewState::new(),
        );

        let resized_to = published.iter().find_map(|event| match event {
            Event::Resize { layout_size, .. } => *layout_size,
            _ => None,
        });
        assert_eq!(
            resized_to,
            Some(Size::new(VIEW_SIZE.width - 32.0, VIEW_SIZE.height))
        );
    }

    #[test]
    fn the_pointer_is_mapped_to_the_cell_under_it_in_the_text_area() {
        let position = Point::new(16.0 + 2.5 * 8.0, 1.5 * 16.0);
        let moved = iced::Event::Mouse(mouse::Event::CursorMoved { position });
        let term = pane_terminal(pane_surface().snapshot_owned());

        let (_, state) = update_view(
            &term,
            PANE_SIZE,
            moved,
            Cursor::Available(position),
            TerminalViewState::new(),
        );

        assert_eq!(
            state.mouse_position_on_grid,
            TerminalGridPoint {
                line: Line(1),
                column: Column(2),
            }
        );
    }

    #[test]
    fn the_hovered_block_is_forgotten_when_the_pointer_leaves_the_pane() {
        let outside = Point::new(VIEW_SIZE.width + 5.0, 240.0);
        let moved =
            iced::Event::Mouse(mouse::Event::CursorMoved { position: outside });
        let mut state = TerminalViewState::new();
        state.hovered_block_key = Some(LS);

        let (published, state) = update_view(
            &empty_terminal(),
            VIEW_SIZE,
            moved,
            Cursor::Available(outside),
            state,
        );

        assert_eq!(state.hovered_block_key, None);
        assert!(
            published
                .iter()
                .any(|event| matches!(event, Event::Redraw { id: 7 })),
            "got {published:?}"
        );
    }

    /// A layer of the host over the pane, such as the fold arrow of a
    /// block, takes the pointer. The block under it must stay hovered, or
    /// the host would hide the arrow under the pointer.
    #[test]
    fn the_hovered_block_stays_while_a_layer_over_the_pane_has_the_pointer() {
        let over_the_arrow = Point::new(8.0, 24.0);
        let moved = iced::Event::Mouse(mouse::Event::CursorMoved {
            position: over_the_arrow,
        });
        let mut snapshot = pane_surface().snapshot_owned();
        snapshot.blocks = vec![
            block(LS, BlockKind::Command, 0, 2),
            block(PROMPT, BlockKind::Prompt, 2, 1),
        ];
        let term = pane_terminal(snapshot);
        let mut state = TerminalViewState::new();
        state.hovered_block_key = Some(LS);

        let (published, state) = update_view(
            &term,
            PANE_SIZE,
            moved,
            Cursor::Levitating(over_the_arrow),
            state,
        );

        assert_eq!(state.hovered_block_key, Some(LS));
        assert!(
            !published
                .iter()
                .any(|event| matches!(event, Event::BlockHovered { .. })),
            "got {published:?}"
        );
    }

    /// The event iced gives every widget before it draws a frame.
    fn redraw_requested() -> iced::Event {
        iced::Event::Window(
            iced::window::Event::RedrawRequested(Instant::now()),
        )
    }

    /// `ls` on the first two lines of the pane and `pwd` on the next two,
    /// or, after `pwd` printed two more lines, `ls` above the pane and
    /// `pwd` on all four lines.
    fn ls_and_pwd(is_ls_pushed_up: bool) -> SnapshotOwned {
        let mut snapshot = pane_surface().snapshot_owned();
        snapshot.blocks = if is_ls_pushed_up {
            vec![
                block(LS, BlockKind::Command, -2, 2),
                block(PWD, BlockKind::Command, 0, 4),
            ]
        } else {
            vec![
                block(LS, BlockKind::Command, 0, 2),
                block(PWD, BlockKind::Command, 2, 2),
            ]
        };

        snapshot
    }

    /// The widget learns about a new snapshot on the next event it gets,
    /// at the latest the redraw before the next frame.
    #[test]
    fn a_new_snapshot_under_a_still_pointer_hovers_the_block_now_under_it() {
        let over_the_second_line = Point::new(40.0, 24.0);
        let pointer = Cursor::Available(over_the_second_line);
        let moved = iced::Event::Mouse(mouse::Event::CursorMoved {
            position: over_the_second_line,
        });
        let mut term = pane_terminal_with_snapshots(vec![
            ls_and_pwd(false),
            ls_and_pwd(true),
        ]);
        let (_, state) = update_view(
            &term,
            PANE_SIZE,
            moved,
            pointer,
            TerminalViewState::new(),
        );
        assert_eq!(state.hovered_block_key, Some(LS));
        term.handle(Event::ContentSync { id: 7 });

        let (published, state) =
            update_view(&term, PANE_SIZE, redraw_requested(), pointer, state);

        assert_eq!(state.hovered_block_key, Some(PWD));
        assert!(
            published.iter().any(|event| matches!(
                event,
                Event::BlockHovered {
                    id: 7,
                    block_key: Some(PWD)
                }
            )),
            "got {published:?}"
        );
        assert!(
            published
                .iter()
                .any(|event| matches!(event, Event::Redraw { id: 7 })),
            "got {published:?}"
        );
    }

    /// A text selection dragged from the terminal onto a layer over the
    /// pane, such as the fold arrow of a block, still follows the pointer,
    /// and the release there ends it. It is no click on a block.
    #[test]
    fn a_text_selection_dragged_onto_a_layer_over_the_pane_ends_where_it_is_released()
     {
        let over_the_arrow = Point::new(8.0, 8.0);
        let pointer = Cursor::Levitating(over_the_arrow);
        let moved = iced::Event::Mouse(mouse::Event::CursorMoved {
            position: over_the_arrow,
        });
        let released = iced::Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ));
        let term = pane_terminal(ls_and_pwd(false));
        let mut state = TerminalViewState::new();
        state.hovered_block_key = Some(LS);
        state.is_dragged = true;
        state.selection_in_progress = true;

        let (dragged, state) =
            update_view(&term, PANE_SIZE, moved, pointer, state);
        let (published, state) =
            update_view(&term, PANE_SIZE, released, pointer, state);

        assert!(
            dragged.iter().any(|event| matches!(
                event,
                Event::SelectUpdate { position, .. } if *position == (-8.0, 8.0)
            )),
            "got {dragged:?}"
        );
        assert!(!state.is_dragged);
        assert!(!state.selection_in_progress);
        assert_eq!(state.selected_block_key, None);
        assert!(
            !published
                .iter()
                .any(|event| matches!(event, Event::BlockSelected { .. })),
            "got {published:?}"
        );
    }

    #[test]
    fn a_fold_command_asks_the_program_to_fold_the_block() {
        run_block_command_expecting(BlockCommand::ToggleFold(LS), |request| {
            matches!(
                request,
                otty_libterm::TerminalRequest::ToggleBlockFold {
                    block_key: LS
                }
            )
        });
    }

    /// A history of 8 lines in the pane of 4 rows, at its bottom: `ls`
    /// (its status line and 2 lines) is above the pane, `pwd` (its status
    /// line and 3 lines) holds the top rows without its status line, and
    /// the prompt is the bottom row.
    fn ls_above_the_pane() -> SnapshotOwned {
        let mut snapshot = pane_surface_with_history().snapshot_owned();
        let in_history = |mut block: BlockSnapshot, lines, has_status_line| {
            block.history_line_count = lines;
            block.has_status_line = has_status_line;
            block
        };
        snapshot.blocks = vec![
            in_history(block(LS, BlockKind::Command, 0, 0), 3, true),
            in_history(block(PWD, BlockKind::Command, 0, 3), 4, true),
            in_history(block(PROMPT, BlockKind::Prompt, 3, 1), 1, false),
        ];

        snapshot
    }

    /// The scroll requests the terminal sends to its program when its
    /// widget runs `command` on `snapshot` and the host hands the scroll
    /// the widget asks for back to the terminal.
    fn scrolls_after(
        command: BlockCommand,
        snapshot: SnapshotOwned,
    ) -> Vec<Scroll> {
        use std::sync::{Arc, Mutex};

        let sent = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&sent);
        let mut program = crate::program::MockTerminalProgram::new();
        program.expect_send().returning(move |request| {
            if let otty_libterm::TerminalRequest::ScrollDisplay(scroll) =
                request
            {
                recorded.lock().expect("the requests").push(scroll);
            }
        });
        let mut waiting = Some(snapshot);
        program
            .expect_take_snapshot()
            .returning(move || waiting.take());
        let (started, _no_events) =
            crate::program::StartedProgram::new(Box::new(program));
        let mut term = Terminal::<TestAction>::with_program(
            7,
            crate::settings::Settings::default(),
            started,
        );
        let text_size = text_area(Rectangle::with_size(PANE_SIZE)).size();
        term.engine
            .resize(Some(text_size), Some(Size::new(8.0, 16.0)));
        term.handle(Event::ContentSync { id: 7 });
        let mut state = TerminalViewState::new();
        state.queue_block_command(command);

        let (published, _) = update_view(
            &term,
            PANE_SIZE,
            iced::Event::Window(iced::window::Event::RedrawRequested(
                Instant::now(),
            )),
            Cursor::Unavailable,
            state,
        );
        for event in published {
            if matches!(
                event,
                Event::ScrollToOffset { .. } | Event::ScrollPastTheTop { .. }
            ) {
                term.handle(event);
            }
        }

        sent.lock().expect("the requests").clone()
    }

    /// The header button and `Ctrl Shift ↑`/`↓` bring `ls` into view: the
    /// widget works out the place once from the lines of the blocks in the
    /// history. `ls` is the first block, and at the top of the history the
    /// gaps still cut its start, so the view goes to the top of the history
    /// and one step past it, where its gap and status line start at the top
    /// edge.
    #[test]
    fn a_scroll_to_command_scrolls_the_history_once_to_the_start_of_the_block()
    {
        let snapshot = ls_above_the_pane();
        assert_eq!(snapshot.view().size.total_lines, 8);

        let scrolls = scrolls_after(BlockCommand::ScrollTo(LS), snapshot);

        assert!(matches!(scrolls.as_slice(), [Scroll::Top]), "{scrolls:?}");
    }

    /// `pwd` holds the top rows under `ls`; its start is 3 lines up, below
    /// the top of the history, so the jump is the usual absolute scroll.
    #[test]
    fn a_scroll_to_command_below_the_top_scrolls_to_an_offset() {
        let scrolls =
            scrolls_after(BlockCommand::ScrollTo(PWD), ls_above_the_pane());

        assert!(
            matches!(scrolls.as_slice(), [Scroll::Offset(_)]),
            "{scrolls:?}"
        );
    }

    /// A new terminal after `ls` and `pwd`: as many lines as rows, so the
    /// history can not scroll, and the gaps cut `ls` at the top. The jump
    /// to `ls` ends past the top, without asking the history to move.
    #[test]
    fn a_scroll_to_the_first_block_of_a_short_history_ends_past_the_top() {
        let snapshot = history_snapshot(
            4,
            &[
                HistoryBlock::command(LS, 2),
                HistoryBlock::command(PWD, 1),
                HistoryBlock::plain(PROMPT, 1),
            ],
            0,
        );
        let term = pane_terminal(snapshot);
        let mut state = TerminalViewState::new();
        state.queue_block_command(BlockCommand::ScrollTo(LS));

        let (published, _) = update_view(
            &term,
            PANE_SIZE,
            iced::Event::Window(iced::window::Event::RedrawRequested(
                Instant::now(),
            )),
            Cursor::Unavailable,
            state,
        );

        assert!(
            published.iter().any(|event| matches!(
                event,
                Event::ScrollPastTheTop { id: 7 }
            )),
            "{published:?}"
        );
    }

    #[test]
    fn a_scroll_to_command_for_a_block_the_pane_shows_whole_moves_nothing() {
        let scrolls =
            scrolls_after(BlockCommand::ScrollTo(PROMPT), ls_above_the_pane());

        assert!(scrolls.is_empty(), "{scrolls:?}");
    }

    /// A screen whose shell asked for bracketed paste and waits at the
    /// prompt under the finished `ls` block, whose command is `command`.
    fn ls_finished_at_the_prompt(command: &str) -> SnapshotOwned {
        let mut surface = pane_surface();
        surface.set_private_mode(NamedPrivateMode::BracketedPaste.into());
        let mut snapshot = surface.snapshot_owned();
        let mut ls = finished(block(LS, BlockKind::Command, 0, 2), 0);
        ls.meta.cmd = Some(String::from(command));
        snapshot.blocks = vec![ls, block(PROMPT, BlockKind::Prompt, 2, 1)];

        snapshot
    }

    /// Run `command` in the widget of `term` and return what it published.
    fn run_block_command(
        term: &Terminal<TestAction>,
        command: BlockCommand,
    ) -> Vec<Event<TestAction>> {
        let mut state = TerminalViewState::new();
        state.queue_block_command(command);
        let moved = iced::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(40.0, 8.0),
        });

        let (published, _) = update_view(
            term,
            PANE_SIZE,
            moved,
            Cursor::Available(Point::new(40.0, 8.0)),
            state,
        );

        published
    }

    /// What the widget wrote to its program.
    fn written(published: &[Event<TestAction>]) -> Vec<Vec<u8>> {
        published
            .iter()
            .filter_map(|event| match event {
                Event::Write { data, .. } => Some(data.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_run_again_command_pastes_the_command_of_the_block_at_the_prompt_and_presses_enter()
     {
        let term = pane_terminal(ls_finished_at_the_prompt(
            "for i in 1 2\ndo echo $i; done",
        ));

        let published = run_block_command(&term, BlockCommand::RunAgain(LS));

        assert_eq!(
            written(&published),
            [
                b"\x05\x15\x1b[200~for i in 1 2\ndo echo $i; done\x1b[201~\r"
                    .to_vec()
            ]
        );
    }

    #[test]
    fn a_run_again_command_writes_nothing_while_a_command_runs() {
        let mut snapshot = ls_finished_at_the_prompt("ls");
        snapshot.blocks[1] = block(SLEEP, BlockKind::Command, 2, 1);
        let term = pane_terminal(snapshot);

        let published = run_block_command(&term, BlockCommand::RunAgain(LS));

        assert!(written(&published).is_empty(), "got {published:?}");
    }

    /// `ls`, then `pwd` in a shell started inside the first one, which
    /// counts its blocks from 1 again: both blocks have the shell id
    /// `cmd-1`. The prompt is under them.
    fn ls_and_pwd_with_one_shell_id() -> SnapshotOwned {
        let mut ls = finished(block(LS, BlockKind::Command, 0, 1), 0);
        ls.meta.cmd = Some(String::from("ls"));
        let mut pwd = finished(block(PWD, BlockKind::Command, 1, 1), 0);
        pwd.meta.cmd = Some(String::from("pwd"));
        for block in [&mut ls, &mut pwd] {
            block.meta.id = String::from("cmd-1");
        }

        let mut snapshot = pane_surface().snapshot_owned();
        snapshot.blocks = vec![ls, pwd, block(PROMPT, BlockKind::Prompt, 2, 1)];

        snapshot
    }

    /// Owner, issue 13: of two blocks with one shell id, a command acts on
    /// the block its key names, the older one as well as the newer one.
    #[test]
    fn of_two_blocks_with_one_shell_id_a_command_acts_on_the_block_of_its_key()
    {
        let term = pane_terminal(ls_and_pwd_with_one_shell_id());

        let selected = run_block_command(&term, BlockCommand::Select(LS));
        let ran_ls = run_block_command(&term, BlockCommand::RunAgain(LS));
        let ran_pwd = run_block_command(&term, BlockCommand::RunAgain(PWD));

        assert!(
            selected.iter().any(|event| matches!(
                event,
                Event::BlockSelected {
                    id: 7,
                    block_key: LS
                }
            )),
            "got {selected:?}"
        );
        assert_eq!(written(&ran_ls), [b"\x05\x15ls\r".to_vec()]);
        assert_eq!(written(&ran_pwd), [b"\x05\x15pwd\r".to_vec()]);
    }

    /// Run `command` in the widget of a terminal whose program expects one
    /// request for which `expected` is true.
    fn run_block_command_expecting(
        command: BlockCommand,
        expected: fn(&otty_libterm::TerminalRequest) -> bool,
    ) {
        let mut waiting = Some(pane_surface().snapshot_owned());
        let mut program = crate::program::MockTerminalProgram::new();
        program
            .expect_take_snapshot()
            .returning(move || waiting.take());
        program
            .expect_send()
            .withf(expected)
            .times(1)
            .return_const(());
        program
            .expect_send()
            .withf(|request| {
                matches!(request, otty_libterm::TerminalRequest::Shutdown)
            })
            .return_const(());
        let (started, _no_events) =
            crate::program::StartedProgram::new(Box::new(program));
        let mut term = Terminal::<TestAction>::with_program(
            7,
            crate::settings::Settings::default(),
            started,
        );
        term.handle(Event::ContentSync { id: 7 });
        let mut state = TerminalViewState::new();
        state.queue_block_command(command);
        let moved = iced::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(40.0, 8.0),
        });

        let _ = update_view(
            &term,
            PANE_SIZE,
            moved,
            Cursor::Available(Point::new(40.0, 8.0)),
            state,
        );
    }

    #[test]
    fn cells_are_drawn_16_px_from_the_left_and_the_right_edge_of_the_pane() {
        let mut surface = pane_surface();
        for column in 0..PANE_GRID.cols {
            surface.grid_mut()[Line(1)][Column(usize::from(column))].bg =
                CELL_BG;
        }
        let term = pane_terminal(surface.snapshot_owned());

        let picture = picture_of_pane(&term, TerminalViewState::new());

        // The second row of cells is 16 to 32 px from the top.
        assert_eq!(picture.pixel(15, 24), UNPAINTED);
        assert_eq!(picture.pixel(16, 24), [200, 30, 40, 255]);
        assert_eq!(picture.pixel(303, 24), [200, 30, 40, 255]);
        assert_eq!(picture.pixel(304, 24), UNPAINTED);
        // There is no inset at the top and the bottom.
        assert_eq!(picture.pixel(160, 15), UNPAINTED);
        assert_eq!(picture.pixel(160, 16), [200, 30, 40, 255]);
    }

    /// `ls` holds the second and the third row and the prompt the bottom
    /// row, as the history stacks them: the gaps put the rows of `ls` at
    /// 3..19 and 19..35 px, and the prompt at 48..64 px.
    fn ls_over_the_prompt(ls: BlockSnapshot) -> Vec<BlockSnapshot> {
        vec![ls, block(PROMPT, BlockKind::Prompt, 3, 1)]
    }

    #[test]
    fn a_hovered_block_is_painted_from_edge_to_edge_under_the_cells() {
        let mut surface = pane_surface();
        surface.grid_mut()[Line(2)][Column(2)].bg = CELL_BG;
        let mut snapshot = surface.snapshot_owned();
        snapshot.blocks =
            ls_over_the_prompt(block(LS, BlockKind::Command, 1, 2));
        let term = pane_terminal(snapshot);
        let hovered = term.theme.block_hovered_background().into_rgba8();
        let line = term.theme.block_line().into_rgba8();
        let mut state = TerminalViewState::new();
        state.hovered_block_key = Some(LS);

        let picture = picture_of_pane(&term, state);

        // The background reaches both edges.
        assert_eq!(picture.pixel(0, 24), hovered);
        assert_eq!(picture.pixel(319, 24), hovered);
        // A cell with its own background is painted over it.
        assert_eq!(picture.pixel(16 + 2 * 8 + 4, 24), [200, 30, 40, 255]);
        assert_eq!(picture.pixel(16 + 3 * 8 + 4, 24), hovered);
        // It covers the gap under the rows of the block: 6 px, and the
        // line, the lowest row of pixels of the block. The gap above the
        // first row reaches over the top edge.
        assert_eq!(picture.pixel(0, 0), hovered);
        assert_eq!(picture.pixel(0, 35), hovered);
        assert_eq!(picture.pixel(0, 40), hovered);
        assert_eq!(picture.pixel(0, 41), line);
        assert_eq!(picture.pixel(319, 41), line);
        // The prompt block, the last one, has no background and no line,
        // in its gap and on its row.
        assert_eq!(picture.pixel(0, 42), UNPAINTED);
        assert_eq!(picture.pixel(0, 63), UNPAINTED);
    }

    #[test]
    fn a_selected_block_is_painted_with_the_selected_background() {
        let mut snapshot = pane_surface().snapshot_owned();
        snapshot.blocks =
            ls_over_the_prompt(block(LS, BlockKind::Command, 1, 2));
        let term = pane_terminal(snapshot);
        let selected = term.theme.block_selected_background().into_rgba8();
        let mut state = TerminalViewState::new();
        state.selected_block_key = Some(LS);
        state.hovered_block_key = Some(LS);

        let picture = picture_of_pane(&term, state);

        assert_eq!(picture.pixel(160, 24), selected);
    }

    #[test]
    fn a_failed_block_has_its_background_and_a_stripe_at_the_left_edge() {
        let mut snapshot = pane_surface().snapshot_owned();
        snapshot.blocks = ls_over_the_prompt(finished(
            block(FALSE, BlockKind::Command, 1, 2),
            1,
        ));
        let term = pane_terminal(snapshot);
        let failed = term.theme.block_failed_background().into_rgba8();
        let stripe = term.theme.block_failed_stripe().into_rgba8();

        let picture = picture_of_pane(&term, TerminalViewState::new());

        assert_eq!(picture.pixel(0, 24), stripe);
        assert_eq!(picture.pixel(1, 24), stripe);
        assert_eq!(picture.pixel(2, 24), failed);
        assert_eq!(picture.pixel(319, 24), failed);
        // The stripe runs over the gap under the rows too.
        assert_eq!(picture.pixel(0, 38), stripe);
    }

    #[test]
    fn a_full_screen_program_shows_no_block_paint() {
        let mut surface = pane_surface();
        surface.set_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );
        let mut snapshot = surface.snapshot_owned();
        snapshot.blocks = vec![
            finished(block(VIM, BlockKind::Command, 0, 2), 1),
            block(PROMPT, BlockKind::Prompt, 2, 1),
        ];
        let term = pane_terminal(snapshot);
        let mut state = TerminalViewState::new();
        state.selected_block_key = Some(VIM);

        let picture = picture_of_pane(&term, state);

        assert_eq!(picture.pixel(0, 24), UNPAINTED);
        assert_eq!(picture.pixel(160, 24), UNPAINTED);
        assert_eq!(picture.pixel(0, 31), UNPAINTED);
    }

    /// A screen of `PANE_GRID` that has printed 8 lines: 4 are on screen
    /// and 4 have scrolled into the history above.
    fn pane_surface_with_history() -> Surface {
        let mut surface = pane_surface();
        for _ in 0..7 {
            surface.line_feed();
        }

        surface
    }

    #[test]
    fn a_pane_with_history_above_has_a_scroll_mark_in_its_right_inset() {
        let term = pane_terminal(pane_surface_with_history().snapshot_owned());
        let mark = term.theme.scroll_mark().into_rgba8();

        let picture = picture_of_pane(&term, TerminalViewState::new());

        // Half of the history is on screen, but the mark is never under
        // 48 px. At the bottom of the history it ends at the bottom edge.
        assert_eq!(picture.pixel(316, 16), mark);
        assert_eq!(picture.pixel(319, 63), mark);
        assert_eq!(picture.pixel(315, 40), UNPAINTED);
        assert_eq!(picture.pixel(318, 15), UNPAINTED);
    }

    #[test]
    fn scrolled_to_the_top_the_scroll_mark_starts_at_the_top_edge() {
        let mut surface = pane_surface_with_history();
        surface.scroll_display(Scroll::Top);
        let term = pane_terminal(surface.snapshot_owned());
        let mark = term.theme.scroll_mark().into_rgba8();

        let picture = picture_of_pane(&term, TerminalViewState::new());

        assert_eq!(picture.pixel(318, 0), mark);
        assert_eq!(picture.pixel(318, 47), mark);
        assert_eq!(picture.pixel(318, 48), UNPAINTED);
    }

    /// The wheel at the top of the history takes the step past it: the
    /// mark stays at the top edge.
    #[test]
    fn past_the_top_the_scroll_mark_starts_at_the_top_edge() {
        let snapshot = history_snapshot(
            4,
            &[
                HistoryBlock::command(LS, 3),
                HistoryBlock::command(PWD, 4),
                HistoryBlock::plain(PROMPT, 1),
            ],
            4,
        );
        let mut term = pane_terminal(snapshot);
        term.handle(Event::Scroll { id: 7, delta: 1 });
        assert!(term.engine.is_past_the_top());
        let mark = term.theme.scroll_mark().into_rgba8();

        let picture = picture_of_pane(&term, TerminalViewState::new());

        assert_eq!(picture.pixel(318, 0), mark);
        assert_eq!(picture.pixel(318, 47), mark);
        assert_eq!(picture.pixel(318, 48), UNPAINTED);
    }

    #[test]
    fn a_pane_whose_history_fits_on_screen_has_no_scroll_mark() {
        let term = pane_terminal(pane_surface().snapshot_owned());

        let picture = picture_of_pane(&term, TerminalViewState::new());

        assert_eq!(picture.pixel(318, 8), UNPAINTED);
        assert_eq!(picture.pixel(318, 56), UNPAINTED);
    }

    #[test]
    fn a_full_screen_program_has_no_scroll_mark() {
        let mut surface = pane_surface_with_history();
        surface.set_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );
        let term = pane_terminal(surface.snapshot_owned());

        let picture = picture_of_pane(&term, TerminalViewState::new());

        assert_eq!(picture.pixel(318, 8), UNPAINTED);
        assert_eq!(picture.pixel(318, 56), UNPAINTED);
    }

    #[test]
    fn the_scroll_mark_is_8_px_wide_and_brighter_while_the_pointer_is_on_its_strip()
     {
        let term = pane_terminal(pane_surface_with_history().snapshot_owned());
        let active = term.theme.scroll_mark_active().into_rgba8();
        let mut state = TerminalViewState::new();
        state.set_pointer_on_scroll_strip(true);

        let picture = picture_of_pane(&term, state);

        assert_ne!(active, term.theme.scroll_mark().into_rgba8());
        assert_eq!(picture.pixel(312, 40), active);
        assert_eq!(picture.pixel(319, 40), active);
        assert_eq!(picture.pixel(311, 40), UNPAINTED);
    }

    #[test]
    fn the_scroll_mark_stays_wide_while_it_is_dragged() {
        let term = pane_terminal(pane_surface_with_history().snapshot_owned());
        let active = term.theme.scroll_mark_active().into_rgba8();
        let mut state = TerminalViewState::new();
        state.grab_scroll_mark(10.0);

        let picture = picture_of_pane(&term, state);

        assert_eq!(picture.pixel(312, 40), active);
    }

    /// Issue 09: the terminal believed a drag was still on after the button
    /// was released outside the pane.
    #[test]
    fn a_text_selection_released_outside_the_pane_ends_there() {
        let outside = Point::new(PANE_SIZE.width + 40.0, 200.0);
        let back_in_the_pane = Point::new(40.0, 24.0);
        let released = iced::Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ));
        let moved_back = iced::Event::Mouse(mouse::Event::CursorMoved {
            position: back_in_the_pane,
        });
        let term = pane_terminal(ls_and_pwd(false));
        let mut state = TerminalViewState::new();
        state.is_dragged = true;
        state.selection_in_progress = true;
        state.set_pointer_position(Some(outside));

        let (_, state) = update_view(
            &term,
            PANE_SIZE,
            released,
            Cursor::Available(outside),
            state,
        );
        assert!(!state.is_dragged);
        assert!(!state.selection_in_progress);
        assert_eq!(state.pointer_position(), None);

        let (published, _) = update_view(
            &term,
            PANE_SIZE,
            moved_back,
            Cursor::Available(back_in_the_pane),
            state,
        );
        assert!(
            !published
                .iter()
                .any(|event| matches!(event, Event::SelectUpdate { .. })),
            "got {published:?}"
        );
    }

    #[test]
    fn a_text_selection_dragged_outside_the_pane_follows_the_pointer() {
        let below = Point::new(40.0, PANE_SIZE.height + 30.0);
        let moved =
            iced::Event::Mouse(mouse::Event::CursorMoved { position: below });
        let term = pane_terminal(ls_and_pwd(false));
        let mut state = TerminalViewState::new();
        state.is_dragged = true;
        state.selection_in_progress = true;

        let (published, state) = update_view(
            &term,
            PANE_SIZE,
            moved,
            Cursor::Available(below),
            state,
        );

        assert!(
            published.iter().any(|event| matches!(
                event,
                Event::SelectUpdate { position, .. } if *position == (24.0, 94.0)
            )),
            "got {published:?}"
        );
        assert!(state.is_dragged);
    }

    #[test]
    fn a_release_after_the_window_lost_the_pointer_still_ends_a_drag() {
        let released = iced::Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ));
        let term = pane_terminal(pane_surface_with_history().snapshot_owned());
        let mut text = TerminalViewState::new();
        text.is_dragged = true;
        text.set_pointer_position(Some(Point::new(500.0, 30.0)));
        let mut mark = TerminalViewState::new();
        mark.grab_scroll_mark(10.0);
        mark.set_pointer_position(Some(Point::new(500.0, 30.0)));

        let (_, text) = update_view(
            &term,
            PANE_SIZE,
            released.clone(),
            Cursor::Unavailable,
            text,
        );
        let (_, mark) =
            update_view(&term, PANE_SIZE, released, Cursor::Unavailable, mark);

        assert!(!text.is_dragged);
        assert_eq!(mark.scroll_mark_grab(), None);
    }

    /// The history of [`pane_surface_with_history`] has 4 lines above the
    /// screen; the mark is 48 px high in a 64 px pane, so it travels 16 px,
    /// 4 px a line.
    #[test]
    fn a_drag_of_the_scroll_mark_goes_on_outside_the_pane_and_a_release_there_ends_it()
     {
        let above = Point::new(PANE_SIZE.width + 40.0, -30.0);
        let moved =
            iced::Event::Mouse(mouse::Event::CursorMoved { position: above });
        let released = iced::Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        ));
        let term = pane_terminal(pane_surface_with_history().snapshot_owned());
        let mut state = TerminalViewState::new();
        state.grab_scroll_mark(10.0);

        let (dragged, state) = update_view(
            &term,
            PANE_SIZE,
            moved,
            Cursor::Available(above),
            state,
        );
        let (_, state) = update_view(
            &term,
            PANE_SIZE,
            released,
            Cursor::Available(above),
            state,
        );

        // Above the pane the mark is at the top of its way: the top of the
        // history and the step past it.
        assert!(
            dragged.iter().any(|event| matches!(
                event,
                Event::ScrollPastTheTop { id: 7 }
            )),
            "got {dragged:?}"
        );
        assert_eq!(state.scroll_mark_grab(), None);
        assert!(!state.is_scroll_mark_active());
    }

    #[test]
    fn a_press_on_the_scroll_mark_strip_takes_the_mark_and_starts_no_selection()
    {
        let on_the_mark = Point::new(310.0, 40.0);
        let pressed = iced::Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ));
        let term = pane_terminal(pane_surface_with_history().snapshot_owned());

        let (published, state) = update_view(
            &term,
            PANE_SIZE,
            pressed,
            Cursor::Available(on_the_mark),
            TerminalViewState::new(),
        );

        assert_eq!(state.scroll_mark_grab(), Some(24.0));
        assert!(!state.is_dragged);
        assert!(
            !published
                .iter()
                .any(|event| matches!(event, Event::SelectStart { .. })),
            "got {published:?}"
        );
    }

    /// Issue 09: the wheel did nothing over the fold arrow, because the
    /// terminal ignored mouse events under a layer of the host.
    #[test]
    fn the_wheel_over_a_layer_over_the_pane_scrolls_the_history() {
        let over_the_arrow = Point::new(8.0, 24.0);
        let wheel = iced::Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 3.0 },
        });
        let term = pane_terminal(ls_and_pwd(false));

        let (published, _) = update_view(
            &term,
            PANE_SIZE,
            wheel,
            Cursor::Levitating(over_the_arrow),
            TerminalViewState::new(),
        );

        assert!(
            published.iter().any(|event| matches!(
                event,
                Event::Scroll { id: 7, delta: 3 }
            )),
            "got {published:?}"
        );
    }

    /// `Starting...` printed before the first prompt, in the block every
    /// terminal starts with, and the prompt under it.
    fn start_text_and_prompt() -> SnapshotOwned {
        let mut surface = pane_surface();
        for (column, c) in "Starting".chars().enumerate() {
            surface.grid_mut()[Line(0)][Column(column)].c = c;
        }
        let mut snapshot = surface.snapshot_owned();
        snapshot.blocks = vec![
            block(START, BlockKind::Start, 0, 2),
            block(PROMPT, BlockKind::Prompt, 2, 1),
        ];

        snapshot
    }

    #[test]
    fn a_start_block_with_text_has_no_block_paint() {
        let term = pane_terminal(start_text_and_prompt());
        let mut state = TerminalViewState::new();
        state.hovered_block_key = Some(START);
        state.selected_block_key = Some(START);

        let picture = picture_of_pane(&term, state);

        // No background from edge to edge and no line under it.
        assert_eq!(picture.pixel(0, 8), UNPAINTED);
        assert_eq!(picture.pixel(319, 24), UNPAINTED);
        assert_eq!(picture.pixel(0, 31), UNPAINTED);
    }

    /// The right-click menu acts on the block the click selects: the start
    /// block is never selected, so the menu offers no block items for it.
    #[test]
    fn the_start_block_is_not_selected_by_a_command() {
        let term = pane_terminal(start_text_and_prompt());
        let moved = iced::Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(40.0, 8.0),
        });
        let mut by_id = TerminalViewState::new();
        by_id.queue_block_command(BlockCommand::Select(START));
        let mut hovered = TerminalViewState::new();
        hovered.hovered_block_key = Some(START);
        hovered.queue_block_command(BlockCommand::SelectHovered);

        for state in [by_id, hovered] {
            let (published, state) = update_view(
                &term,
                PANE_SIZE,
                moved.clone(),
                Cursor::Available(Point::new(40.0, 8.0)),
                state,
            );

            assert_eq!(state.selected_block_key, None);
            assert!(
                !published
                    .iter()
                    .any(|event| matches!(event, Event::BlockSelected { .. })),
                "got {published:?}"
            );
        }
    }

    #[test]
    fn pending_resize_schedules_redraw_at_throttle_deadline() {
        let mut state = TerminalViewState::new();
        let mut events: Vec<Event> = Vec::new();
        let mut shell = iced_graphics::core::Shell::new(&mut events);
        let cell_size = Size::new(8.0, 16.0);

        state.queue_resize(7, Size::new(640.0, 480.0), cell_size, &mut shell);
        state.queue_resize(7, Size::new(641.0, 480.0), cell_size, &mut shell);

        let Some(deadline) = state.pending_resize_deadline else {
            panic!("expected pending resize deadline");
        };
        assert_eq!(
            shell.redraw_request(),
            iced_core::window::RedrawRequest::At(deadline)
        );

        drop(shell);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn a_terminal_in_a_clipped_container_draws_its_bottom_right_cell() {
        // A clipped container, such as the app's main slot, passes its own
        // bounds down as the viewport.
        let main_slot =
            Rectangle::new(Point::new(300.0, 60.0), Size::new(800.0, 400.0));
        let cell_bg = ansi::Color::TrueColor(ansi::Rgb {
            r: 200,
            g: 30,
            b: 40,
        });

        let pixel = bottom_right_cell_pixel(cell_bg, main_slot, 1.75);

        assert_eq!(pixel, [200, 30, 40, 255]);
    }
}
