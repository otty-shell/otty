use std::any::Any;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use iced::alignment::Vertical;
use iced::font::{Style as FontStyle, Weight as FontWeight};
use iced::mouse::Cursor;
use iced::widget::canvas::{Path, Text};
use iced::widget::container;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};
use iced_core::clipboard::Kind as ClipboardKind;
use iced_core::keyboard::Modifiers;
use iced_core::mouse;
use iced_core::text::{Alignment, LineHeight, Shaping};
use iced_core::widget::operation;
use iced_graphics::core::Widget;
use iced_graphics::core::widget::{Tree, tree};
use iced_graphics::geometry::Stroke;
use otty_libterm::escape::{self as ansi, CursorShape, StdColor};
use otty_libterm::surface::SurfaceMode;
use otty_libterm::surface::{BlockKind, Flags, Point as TerminalGridPoint};

use crate::input::InputManager;
use crate::term::{BlockCommand, BlockUiMode, Event, Terminal};
use crate::theme::TerminalStyle;
use crate::{
    block_controls::BlockActionButtonGeometry,
    block_layout::{self, BlockRect},
};

#[derive(Default)]
struct BlockUiVisuals {
    highlights: Vec<(Point, Size)>,
    dividers: Vec<(Point, Point)>,
    action_buttons: Vec<BlockActionButtonGeometry>,
}

impl BlockUiVisuals {
    fn from_rects(
        rects: &[BlockRect],
        state: &TerminalViewState,
        _cell_height: f32,
        mode: BlockUiMode,
    ) -> Self {
        let mut visuals = BlockUiVisuals::default();
        let show_internal = matches!(mode, BlockUiMode::Internal);

        for rect in rects {
            if rect.rect.height <= 0.0 {
                continue;
            }

            let block_id = rect.block_id.as_str();
            let is_prompt = rect.kind == BlockKind::Prompt;

            if !is_prompt {
                let divider_y = rect.rect.y + rect.rect.height;
                visuals.dividers.push((
                    Point::new(rect.rect.x, divider_y),
                    Point::new(rect.rect.x + rect.rect.width, divider_y),
                ));
            }

            if !show_internal {
                continue;
            }

            if Some(block_id) == state.selected_block_id.as_deref()
                && !is_prompt
            {
                visuals.highlights.push((
                    Point::new(rect.rect.x, rect.rect.y),
                    Size::new(rect.rect.width, rect.rect.height),
                ));
            }
        }

        visuals
    }
}

pub struct TerminalView<'a> {
    term: &'a Terminal,
    input_manager: InputManager<'a>,
}

impl<'a> TerminalView<'a> {
    pub fn show(term: &'a Terminal) -> Element<'a, Event> {
        container(Self {
            term,
            input_manager: InputManager::new(
                term.id,
                &term.bindings,
            ),
        })
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| term.theme.container_style())
        .into()
    }

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
                if id == Some(&self.id) {
                    if let Some(state) =
                        state.downcast_mut::<TerminalViewState>()
                    {
                        state.queue_block_command(self.command.clone());
                    }
                }
            }
        }

        iced::advanced::widget::operate(CommandOperation { id, command: cmd })
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

    fn process_pending_commands(
        &self,
        view_state: &mut TerminalViewState,
        layout: iced_graphics::core::Layout<'_>,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
    ) {
        while let Some(command) = view_state.next_block_command() {
            self.apply_block_command(
                command, view_state, layout, clipboard, shell,
            );
        }
    }

    fn apply_block_command(
        &self,
        command: BlockCommand,
        view_state: &mut TerminalViewState,
        layout: iced_graphics::core::Layout<'_>,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
    ) {
        let snapshot = self.term.engine.snapshot();
        let view = snapshot.view();
        if view.mode.contains(SurfaceMode::ALT_SCREEN) {
            return;
        }

        match command {
            BlockCommand::Select(block_id) => {
                if self.select_block(&block_id, view_state) {
                    self.notify_block_selected(&block_id, shell);
                }
            },
            BlockCommand::SelectHovered => {
                if let Some(block_id) = view_state.hovered_block_id.clone() {
                    if self.select_block(&block_id, view_state) {
                        self.notify_block_selected(&block_id, shell);
                    }
                }
            },
            BlockCommand::CopySelection => {
                let content = view.selectable_content();
                if !content.is_empty() {
                    clipboard.write(ClipboardKind::Standard, content);
                }
            },
            BlockCommand::ClearSelection => {
                if view_state.selected_block_id.is_some()
                    || view_state.selected_block_kind.is_some()
                {
                    view_state.selected_block_id = None;
                    view_state.selected_block_kind = None;
                    shell.publish(Event::BlockSelectionCleared {
                        id: self.term.id,
                    });
                    shell.publish(Event::Redraw { id: self.term.id });
                }
            },
            BlockCommand::Copy(block_id) => {
                let selected = self.select_block(&block_id, view_state);
                if selected {
                    self.notify_block_selected(&block_id, shell);
                }

                if self.copy_block_to_clipboard(&block_id, clipboard) {
                    shell.publish(Event::BlockCopied {
                        id: self.term.id,
                        block_id,
                    });
                    shell.publish(Event::Redraw { id: self.term.id });
                } else if selected {
                    shell.publish(Event::Redraw { id: self.term.id });
                }
            },
            BlockCommand::CopyContent(block_id) => {
                let selected = self.select_block(&block_id, view_state);
                if selected {
                    self.notify_block_selected(&block_id, shell);
                }

                if self.copy_block_content_to_clipboard(&block_id, clipboard) {
                    shell.publish(Event::BlockCopied {
                        id: self.term.id,
                        block_id,
                    });
                    shell.publish(Event::Redraw { id: self.term.id });
                } else if selected {
                    shell.publish(Event::Redraw { id: self.term.id });
                }
            },
            BlockCommand::CopyPrompt(block_id) => {
                let selected = self.select_block(&block_id, view_state);
                if selected {
                    self.notify_block_selected(&block_id, shell);
                }

                if self.copy_block_prompt_to_clipboard(&block_id, clipboard) {
                    shell.publish(Event::BlockCopied {
                        id: self.term.id,
                        block_id,
                    });
                    shell.publish(Event::Redraw { id: self.term.id });
                } else if selected {
                    shell.publish(Event::Redraw { id: self.term.id });
                }
            },
            BlockCommand::CopyCommand(block_id) => {
                let selected = self.select_block(&block_id, view_state);
                if selected {
                    self.notify_block_selected(&block_id, shell);
                }

                if self.copy_block_command_to_clipboard(&block_id, clipboard) {
                    shell.publish(Event::BlockCopied {
                        id: self.term.id,
                        block_id,
                    });
                    shell.publish(Event::Redraw { id: self.term.id });
                } else if selected {
                    shell.publish(Event::Redraw { id: self.term.id });
                }
            },
            BlockCommand::ScrollTo(block_id) => {
                self.scroll_block_into_view(&block_id, layout, shell);
            },
            BlockCommand::PasteClipboard => {
                if let Some(data) = clipboard.read(ClipboardKind::Standard) {
                    let payload: Vec<u8> = data.bytes().collect();
                    if !payload.is_empty() {
                        shell.publish(Event::Write {
                            id: self.term.id,
                            data: payload,
                        });
                    }
                }
            },
        }
    }

    fn select_block(
        &self,
        block_id: &str,
        view_state: &mut TerminalViewState,
    ) -> bool {
        let snapshot = self.term.engine.snapshot();
        let view = snapshot.view();
        let block =
            view.blocks().iter().find(|block| block.meta.id == block_id);

        let Some(block) = block else {
            return false;
        };

        if block.meta.kind == BlockKind::Prompt {
            return false;
        }

        if view_state.selected_block_id.as_deref() == Some(block_id) {
            return false;
        }

        view_state.selected_block_id = Some(block.meta.id.clone());
        view_state.selected_block_kind = Some(block.meta.kind.clone());
        true
    }

    fn notify_block_selected(
        &self,
        block_id: &str,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
    ) {
        shell.publish(Event::BlockSelected {
            id: self.term.id,
            block_id: block_id.to_string(),
        });
        shell.publish(Event::Redraw { id: self.term.id });
    }

    fn copy_block_to_clipboard(
        &self,
        block_id: &str,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
    ) -> bool {
        let snapshot = self.term.engine.snapshot();
        if let Some(content) = snapshot.block_text(block_id) {
            clipboard.write(ClipboardKind::Standard, content);
            true
        } else {
            false
        }
    }

    fn copy_block_content_to_clipboard(
        &self,
        block_id: &str,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
    ) -> bool {
        let snapshot = self.term.engine.snapshot();
        let Some(raw) = snapshot.block_text(block_id) else {
            return false;
        };

        let mut lines = raw.split('\n');
        let _ = lines.next();
        let content: String = lines.collect::<Vec<_>>().join("\n");
        if content.is_empty() {
            return false;
        }

        clipboard.write(ClipboardKind::Standard, content);
        true
    }

    fn copy_block_prompt_to_clipboard(
        &self,
        block_id: &str,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
    ) -> bool {
        let snapshot = self.term.engine.snapshot();
        if let Some(content) = snapshot.block_prompt_text(block_id) {
            clipboard.write(ClipboardKind::Standard, content);
            true
        } else {
            false
        }
    }

    fn copy_block_command_to_clipboard(
        &self,
        block_id: &str,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
    ) -> bool {
        let snapshot = self.term.engine.snapshot();
        let view = snapshot.view();
        let block = match view.blocks().iter().find(|b| b.meta.id == block_id) {
            Some(block) => block,
            None => return false,
        };

        let cmd = match block.meta.cmd.as_deref() {
            Some(cmd) if !cmd.is_empty() => cmd.to_string(),
            _ => return false,
        };

        clipboard.write(ClipboardKind::Standard, cmd);
        true
    }

    fn scroll_block_into_view(
        &self,
        block_id: &str,
        layout: iced_graphics::core::Layout<'_>,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
    ) {
        let snapshot = self.term.engine.snapshot();
        let view = snapshot.view();
        let layout_bounds = layout.bounds();
        let layout_position = Point::new(layout_bounds.x, layout_bounds.y);
        let cell_height = self.term.engine.terminal_size().cell_height as f32;
        let rects = block_layout::block_rects(
            &view,
            layout_position,
            layout_bounds.size(),
            cell_height,
        );

        let Some(block_rect) = rects
            .into_iter()
            .find(|rect| rect.block_id.as_str() == block_id)
        else {
            return;
        };

        let viewport_top = layout_bounds.y;
        let viewport_bottom = viewport_top + layout_bounds.height;
        let block_top = block_rect.rect.y;
        let block_bottom = block_rect.rect.y + block_rect.rect.height;

        if block_top < viewport_top {
            let delta_lines =
                ((viewport_top - block_top) / cell_height).ceil() as i32;
            if delta_lines != 0 {
                shell.publish(Event::Scroll {
                    id: self.term.id,
                    delta: delta_lines,
                });
            }
        } else if block_bottom > viewport_bottom {
            let delta_lines =
                ((block_bottom - viewport_bottom) / cell_height).ceil() as i32;
            if delta_lines != 0 {
                shell.publish(Event::Scroll {
                    id: self.term.id,
                    delta: -delta_lines,
                });
            }
        }
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
            let layout_position = layout.position();
            let layout_offset_x = layout_position.x;
            let layout_offset_y = layout_position.y;
            let layout_bounds = layout.bounds();
            let display_offset = view.display_offset as f32;
            let half_h = cell_height * 0.5;
            let block_rects = if view.mode.contains(SurfaceMode::ALT_SCREEN) {
                Vec::new()
            } else {
                block_layout::block_rects(
                    &view,
                    layout_position,
                    layout_bounds.size(),
                    cell_height,
                )
            };

            // We use the background pallete color as a default
            // because the widget global background color must be the same
            let default_bg = self
                .term
                .theme
                .get_color(ansi::Color::Std(StdColor::Background));

            let hovered_span_id =
                view.hyperlink_span_id_at(state.mouse_position_on_grid);
            let block_ui_mode = self.term.block_ui_mode();
            let block_visuals = BlockUiVisuals::from_rects(
                &block_rects,
                state,
                cell_height,
                block_ui_mode,
            );
            let mut selected_color = self.term.theme.block_highlight_color();
            selected_color.a = selected_color.a.min(0.1);
            let mut divider_color = self.term.theme.block_highlight_color();
            divider_color.a = divider_color.a.min(0.1);

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
                        align_x: Alignment::Center,
                        align_y: Vertical::Center,
                        shaping: Shaping::Advanced,
                        line_height: LineHeight::Relative(font_scale_factor),
                        ..Default::default()
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

            for (origin, size) in &block_visuals.highlights {
                let rect = Path::rectangle(*origin, *size);
                frame.fill(&rect, selected_color);
            }

            for (start, end) in &block_visuals.dividers {
                let divider = Path::line(*start, *end);
                frame.stroke(
                    &divider,
                    Stroke::default().with_width(1.0).with_color(divider_color),
                );
            }

            for button in &block_visuals.action_buttons {
                let is_hovered = state.hovered_action_block_id.as_deref()
                    == Some(button.block_id.as_str());
                if is_hovered {
                    let mut bg_color = self.term.theme.block_highlight_color();
                    bg_color.a = 0.0;
                    let origin = Point::new(button.rect.x, button.rect.y);
                    let size = Size::new(button.rect.width, button.rect.height);
                    frame.fill_rectangle(origin, size, bg_color);
                }

                let mut dot_color = self
                    .term
                    .theme
                    .get_color(ansi::Color::Std(StdColor::Foreground));
                dot_color.a = 0.5;
                if is_hovered {
                    dot_color.a = 1.0;
                }

                let dot_radius =
                    (button.rect.height.min(button.rect.width) / 9.0).max(1.0);
                let center_x = button.rect.x + (button.rect.width / 2.0);
                let center_y = button.rect.y + (button.rect.height / 2.0);
                let spacing = button.rect.height / 3.5;
                for offset in [-1.0_f32, 0.0, 1.0] {
                    let y = center_y + (offset * spacing);
                    let dot = Path::circle(Point::new(center_x, y), dot_radius);
                    frame.fill(&dot, dot_color);
                }
            }
        });

        use iced::advanced::graphics::geometry::Renderer as _;
        renderer.draw_geometry(geom);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: iced_graphics::core::Layout<'_>,
        cursor: Cursor,
        _renderer: &iced::Renderer,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
        _viewport: &Rectangle,
    ) {
        let view_state = tree.state.downcast_mut::<TerminalViewState>();
        let terminal_state = self.term.engine.snapshot();
        let terminal_size = self.term.engine.terminal_size();
        let font = &self.term.font;
        let layout_size = layout.bounds().size();
        let terminal_id = self.term.id;

        view_state.flush_pending_resize(terminal_id, shell);
        self.process_pending_commands(view_state, layout, clipboard, shell);

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

        if !view_state.is_focused {
            return; // iced::event::Status::Ignored;
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
                    clipboard,
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
        };
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

        if matches!(self.term.block_ui_mode(), BlockUiMode::Internal)
            && self.is_cursor_in_layout(cursor, layout)
            && state.hovered_action_block_id.is_some()
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
    pub hovered_block_id: Option<String>,
    pub hovered_block_kind: Option<BlockKind>,
    pub selected_block_id: Option<String>,
    pub selected_block_kind: Option<BlockKind>,
    pub hovered_action_block_id: Option<String>,
    pub selection_in_progress: bool,
    pub terminal_id: Option<u64>,
    pending_block_commands: VecDeque<BlockCommand>,
    pending_resize: Option<Size<f32>>,
    pending_cell_size: Option<Size<f32>>,
    pending_resize_deadline: Option<Instant>,
    last_resize_sent_at: Option<Instant>,
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
            hovered_block_id: None,
            hovered_block_kind: None,
            selected_block_id: None,
            selected_block_kind: None,
            hovered_action_block_id: None,
            selection_in_progress: false,
            terminal_id: None,
            pending_block_commands: VecDeque::new(),
            pending_resize: None,
            pending_cell_size: None,
            pending_resize_deadline: None,
            last_resize_sent_at: None,
        }
    }

    fn queue_resize(
        &mut self,
        terminal_id: u64,
        layout_size: Size<f32>,
        cell_size: Size<f32>,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
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
            shell.request_redraw();
        }
    }

    fn publish_resize(
        &mut self,
        terminal_id: u64,
        layout_size: Size<f32>,
        cell_size: Size<f32>,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
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

    fn flush_pending_resize(
        &mut self,
        terminal_id: u64,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
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
                shell.request_redraw();
            }
        }
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

    fn make_block_rect(
        id: &str,
        kind: BlockKind,
        y: f32,
        height: f32,
    ) -> BlockRect {
        BlockRect {
            block_id: id.to_string(),
            kind,
            rect: Rectangle {
                x: 0.0,
                y,
                width: 100.0,
                height,
            },
        }
    }

    #[test]
    fn block_visuals_highlight_selected_block_in_internal_mode() {
        let rect = make_block_rect("cmd", BlockKind::Command, 10.0, 20.0);
        let mut state = TerminalViewState::new();
        state.selected_block_id = Some("cmd".into());
        let visuals = BlockUiVisuals::from_rects(
            &[rect],
            &state,
            14.0,
            BlockUiMode::Internal,
        );

        assert_eq!(visuals.highlights.len(), 1);
        assert_eq!(visuals.action_buttons.len(), 0);
        assert_eq!(visuals.dividers.len(), 1);
    }

    #[test]
    fn block_visuals_hide_highlights_and_buttons_in_external_mode() {
        let rect = make_block_rect("cmd", BlockKind::Command, 0.0, 30.0);
        let mut state = TerminalViewState::new();
        state.selected_block_id = Some("cmd".into());
        state.hovered_block_id = Some("cmd".into());
        let visuals = BlockUiVisuals::from_rects(
            &[rect],
            &state,
            16.0,
            BlockUiMode::ExternalOverlay,
        );

        assert!(visuals.highlights.is_empty());
        assert!(visuals.action_buttons.is_empty());
        assert_eq!(visuals.dividers.len(), 1);
    }
}
