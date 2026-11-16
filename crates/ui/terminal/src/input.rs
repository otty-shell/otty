use iced::keyboard::Key;
use iced::keyboard::key::Physical;
use iced::mouse::ScrollDelta;
use iced::{Point, Size};
use iced_core::clipboard::Kind as ClipboardKind;
use iced_core::mouse::{self, Click};
use otty_libterm::surface::{
    SelectionType, SnapshotOwned, SnapshotView, SurfaceMode,
};

use crate::bindings::{BindingAction, BindingsLayout, InputKind};
use crate::engine::{Engine, MouseButton};
use crate::font::TermFont;
use crate::scroll_mark::{ScrollMark, drag_scroll_mark, take_scroll_mark};
use crate::settings::BlockSelectionMode;
use crate::view::TerminalViewState;

pub(crate) struct InputManager<'a, T = ()> {
    terminal_id: u64,
    bindings: &'a BindingsLayout<T>,
    block_selection_mode: BlockSelectionMode,
}

impl<'a, T: Clone + PartialEq> InputManager<'a, T> {
    pub(crate) fn new(
        terminal_id: u64,
        bindings: &'a BindingsLayout<T>,
        block_selection_mode: BlockSelectionMode,
    ) -> Self {
        Self {
            terminal_id,
            bindings,
            block_selection_mode,
        }
    }

    /// Handle a mouse event that happened with the pointer over the pane,
    /// or anywhere during a drag the terminal started.
    ///
    /// `engine` holds what the pane shows: the snapshot and where its rows
    /// lie. `text_origin` is the top left corner of the text of the
    /// terminal: the pane starts to the left of it, so the pointer may be
    /// at a negative distance from it. `scroll_mark` is the scroll mark of
    /// the pane, when the history does not fit on screen: its strip takes
    /// the pointer before the text does.
    pub(crate) fn handle_mouse_event(
        &self,
        view_state: &mut TerminalViewState,
        engine: &Engine,
        terminal_font: &TermFont,
        text_origin: Point,
        cursor_position: Point,
        scroll_mark: Option<&ScrollMark>,
        event: &iced::mouse::Event,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) -> iced::event::Status {
        let terminal_content = engine.snapshot();

        if let Some(status) = self.handle_scroll_mark_event(
            view_state,
            scroll_mark,
            cursor_position,
            event,
            &terminal_font.measure,
            publisher,
        ) {
            return status;
        }

        match event {
            iced_core::mouse::Event::ButtonPressed(
                iced_core::mouse::Button::Left,
            ) => self.handle_left_button_pressed(
                view_state,
                terminal_content,
                cursor_position,
                text_origin,
                publisher,
            ),
            iced_core::mouse::Event::CursorMoved { position } => self
                .handle_cursor_moved(
                    view_state,
                    engine,
                    position,
                    text_origin,
                    publisher,
                ),
            iced_core::mouse::Event::ButtonReleased(
                iced_core::mouse::Button::Left,
            ) => self.handle_button_released(
                view_state,
                terminal_content,
                self.bindings,
                publisher,
            ),
            iced::mouse::Event::WheelScrolled { delta } => self
                .handle_wheel_scrolled(
                    view_state,
                    terminal_content,
                    delta,
                    &terminal_font.measure,
                    publisher,
                ),
            _ => iced::event::Status::Ignored,
        }
    }

    /// Forget the pointer and the block under it once the pointer has left
    /// the pane, and report it and ask for a redraw if there was a block.
    /// The scroll mark narrows again.
    pub(crate) fn handle_pointer_left(
        &self,
        state: &mut TerminalViewState,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) {
        state.set_pointer_position(None);

        self.leave_scroll_strip(state, publisher);
        self.forget_hovered_block(state, publisher);
    }

    /// Note that the pointer is off the strip of the scroll mark, such as
    /// over a layer of the host, and ask for a redraw when the mark
    /// narrows.
    pub(crate) fn leave_scroll_strip(
        &self,
        state: &mut TerminalViewState,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) {
        self.set_pointer_on_scroll_strip(state, false, publisher);
    }

    /// Forget the block under the pointer, and report it and ask for a
    /// redraw if there was one.
    fn forget_hovered_block(
        &self,
        state: &mut TerminalViewState,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) {
        if state.hovered_block_key.is_none() {
            return;
        }

        state.hovered_block_key = None;
        publisher(crate::Event::BlockHovered {
            id: self.terminal_id,
            block_key: None,
        });
        publisher(crate::Event::Redraw {
            id: self.terminal_id,
        });
    }

    /// Note whether the pointer is on the strip of the scroll mark, and ask
    /// for a redraw when that changed: the mark widens or narrows.
    fn set_pointer_on_scroll_strip(
        &self,
        state: &mut TerminalViewState,
        is_on_strip: bool,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) {
        if state.set_pointer_on_scroll_strip(is_on_strip) {
            publisher(crate::Event::Redraw {
                id: self.terminal_id,
            });
        }
    }

    fn handle_left_button_pressed(
        &self,
        state: &mut TerminalViewState,
        terminal_state: &SnapshotOwned,
        cursor_position: Point,
        text_origin: Point,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) -> iced::event::Status {
        state.selection_in_progress = false;
        let is_mouse_mode = terminal_state
            .view()
            .mode
            .intersects(SurfaceMode::MOUSE_MODE);
        let cmd = if is_mouse_mode {
            crate::Event::MouseReport {
                id: self.terminal_id,
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
            crate::Event::SelectStart {
                id: self.terminal_id,
                selection_type,
                position: (
                    cursor_position.x - text_origin.x,
                    cursor_position.y - text_origin.y,
                ),
            }
        };
        publisher(cmd);
        if !is_mouse_mode {
            publisher(crate::Event::Redraw {
                id: self.terminal_id,
            });
        }
        state.is_dragged = true;
        iced::event::Status::Captured
    }

    fn handle_cursor_moved(
        &self,
        state: &mut TerminalViewState,
        engine: &Engine,
        position: &Point,
        text_origin: Point,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) -> iced::event::Status {
        let terminal_state = engine.snapshot().view();
        let is_full_screen = terminal_state.is_full_screen();
        let cursor_x = position.x - text_origin.x;
        let cursor_y = position.y - text_origin.y;
        state.set_pointer_position(Some(*position));
        state.mouse_position_on_grid =
            engine.selection_point(cursor_x, cursor_y);

        // Handle command or selection update based on terminal mode and modifiers
        if state.is_dragged {
            let terminal_mode = terminal_state.mode;
            if terminal_mode.intersects(SurfaceMode::MOUSE_MOTION) {
                publisher(crate::Event::MouseReport {
                    id: self.terminal_id,
                    button: MouseButton::LeftMove,
                    modifiers: state.keyboard_modifiers,
                    point: state.mouse_position_on_grid,
                    pressed: true,
                });
                return iced::event::Status::Captured;
            }

            if !state.selection_in_progress {
                state.selection_in_progress = true;
                self.clear_block_selection(state, publisher);
            }

            publisher(crate::Event::SelectUpdate {
                id: self.terminal_id,
                position: (cursor_x, cursor_y),
            });
            publisher(crate::Event::Redraw {
                id: self.terminal_id,
            });
            return iced::event::Status::Captured;
        } else if !is_full_screen
            && self.look_up_hover(state, &terminal_state, publisher)
        {
            publisher(crate::Event::Redraw {
                id: self.terminal_id,
            });
            return iced::event::Status::Captured;
        }

        iced::event::Status::Ignored
    }

    /// Find the block and the link under a pointer that rests over the
    /// pane again, because the content under it may have moved: the history
    /// scrolled, new output arrived, the pane was resized or a block was
    /// folded. The host learns the block when it changed, and a redraw is
    /// asked for only then.
    ///
    /// Nothing is looked up while the pointer is away from the pane, and,
    /// as for a pointer that moves, during a drag or in a full-screen
    /// program. A pointer on the strip of the scroll mark (`scroll_mark`,
    /// when the history does not fit on screen) hovers no block. `engine`
    /// holds what the pane shows now.
    pub(crate) fn handle_content_moved(
        &self,
        state: &mut TerminalViewState,
        engine: &Engine,
        text_origin: Point,
        scroll_mark: Option<&ScrollMark>,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) {
        let Some(position) = state.pointer_position() else {
            return;
        };
        let view = engine.snapshot().view();

        state.mouse_position_on_grid = engine.selection_point(
            position.x - text_origin.x,
            position.y - text_origin.y,
        );

        if state.is_dragged
            || state.scroll_mark_grab().is_some()
            || view.is_full_screen()
        {
            return;
        }

        // The mark may have come or gone under the pointer.
        let is_on_strip =
            scroll_mark.is_some_and(|mark| mark.strip().contains(position));
        self.set_pointer_on_scroll_strip(state, is_on_strip, publisher);

        if is_on_strip {
            self.forget_hovered_block(state, publisher);
            return;
        }

        if self.look_up_hover(state, &view, publisher) {
            publisher(crate::Event::Redraw {
                id: self.terminal_id,
            });
        }
    }

    fn handle_button_released(
        &self,
        state: &mut TerminalViewState,
        terminal_state: &SnapshotOwned,
        bindings: &BindingsLayout<T>,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) -> iced::event::Status {
        state.is_dragged = false;
        let was_selecting = state.selection_in_progress;
        state.selection_in_progress = false;
        let mut published = false;

        let terminal_state = terminal_state.view();

        if terminal_state.mode.intersects(SurfaceMode::MOUSE_MODE) {
            publisher(crate::Event::MouseReport {
                id: self.terminal_id,
                button: MouseButton::LeftButton,
                modifiers: state.keyboard_modifiers,
                point: state.mouse_position_on_grid,
                pressed: false,
            });
            published = true;
        }

        if bindings.get_action(
            InputKind::Mouse(iced_core::mouse::Button::Left),
            &state.keyboard_modifiers,
            terminal_state.mode,
        ) == BindingAction::LinkOpen
            && let Some(span) =
                terminal_state.hyperlink_span_at(state.mouse_position_on_grid)
        {
            publisher(crate::Event::OpenLink {
                id: self.terminal_id,
                uri: span.link.uri().to_string(),
            });
            published = true;
        }

        if self.block_selection_mode == BlockSelectionMode::PrimaryClick
            && !was_selecting
            && !terminal_state.mode.intersects(SurfaceMode::MOUSE_MODE)
            && !terminal_state.is_full_screen()
        {
            use mouse::click::Kind;
            let click_kind = state
                .last_click
                .as_ref()
                .map(mouse::Click::kind)
                .unwrap_or(Kind::Single);

            let selection_changed = match click_kind {
                Kind::Double | Kind::Triple => {
                    state.selected_block_key.take().is_some()
                },
                // Only a command block is ever hovered, so the click
                // selects the block under the pointer or, away from the
                // blocks, clears the selection.
                Kind::Single => {
                    let hovered_block_key = state.hovered_block_key;
                    let changed = state.selected_block_key != hovered_block_key;
                    state.selected_block_key = hovered_block_key;

                    changed
                },
            };

            if selection_changed {
                match state.selected_block_key {
                    Some(block_key) => {
                        publisher(crate::Event::BlockSelected {
                            id: self.terminal_id,
                            block_key,
                        });
                    },
                    None => {
                        publisher(crate::Event::BlockSelectionCleared {
                            id: self.terminal_id,
                        });
                    },
                }
                publisher(crate::Event::Redraw {
                    id: self.terminal_id,
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
        &self,
        state: &mut TerminalViewState,
        terminal_state: &SnapshotOwned,
        delta: &ScrollDelta,
        font_measure: &Size<f32>,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) -> iced::event::Status {
        let lines = wheel_lines(state, delta, font_measure);

        if lines == 0.0 {
            return iced::event::Status::Ignored;
        }

        // A program that asked for mouse reports gets the wheel as buttons
        // 64 and 65, one report per line, and scrolls its own output.
        if terminal_state
            .view()
            .mode
            .intersects(SurfaceMode::MOUSE_MODE)
        {
            let button = if lines > 0.0 {
                MouseButton::ScrollUp
            } else {
                MouseButton::ScrollDown
            };

            for _ in 0..lines.abs() as u32 {
                publisher(crate::Event::MouseReport {
                    id: self.terminal_id,
                    button: button.clone(),
                    modifiers: state.keyboard_modifiers,
                    point: state.mouse_position_on_grid,
                    pressed: true,
                });
            }

            return iced::event::Status::Captured;
        }

        publisher(crate::Event::Scroll {
            id: self.terminal_id,
            delta: lines as i32,
        });

        iced::event::Status::Captured
    }

    /// Handle `event` when it belongs to the scroll mark, and return
    /// `None` for one the terminal handles as usual.
    ///
    /// While the mark is dragged, a move scrolls the history so that the
    /// mark follows the pointer, wherever the pointer is, and the release
    /// lets it go. Over the strip of the mark a press takes the mark, the
    /// pointer hovers no block and the wheel scrolls the history. Nothing
    /// of it starts a text selection, selects a block or reaches a program
    /// as a mouse report. A text selection dragged over the strip stays a
    /// text selection.
    fn handle_scroll_mark_event(
        &self,
        state: &mut TerminalViewState,
        scroll_mark: Option<&ScrollMark>,
        cursor_position: Point,
        event: &iced::mouse::Event,
        font_measure: &Size<f32>,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) -> Option<iced::event::Status> {
        let is_on_strip = scroll_mark
            .is_some_and(|mark| mark.strip().contains(cursor_position));
        self.set_pointer_on_scroll_strip(state, is_on_strip, publisher);

        if let Some(grab) = state.scroll_mark_grab() {
            return drag_scroll_mark(
                self.terminal_id,
                state,
                scroll_mark,
                grab,
                event,
                publisher,
            );
        }

        let mark = scroll_mark.filter(|_| is_on_strip && !state.is_dragged)?;

        match event {
            iced_core::mouse::Event::ButtonPressed(
                iced_core::mouse::Button::Left,
            ) => {
                take_scroll_mark(
                    self.terminal_id,
                    state,
                    mark,
                    cursor_position.y,
                    publisher,
                );
                Some(iced::event::Status::Captured)
            },
            iced_core::mouse::Event::CursorMoved { position } => {
                state.set_pointer_position(Some(*position));
                self.forget_hovered_block(state, publisher);
                Some(iced::event::Status::Ignored)
            },
            iced::mouse::Event::WheelScrolled { delta } => {
                let lines = wheel_lines(state, delta, font_measure);
                if lines == 0.0 {
                    return Some(iced::event::Status::Ignored);
                }

                publisher(crate::Event::Scroll {
                    id: self.terminal_id,
                    delta: lines as i32,
                });
                Some(iced::event::Status::Captured)
            },
            // A release with no press of the terminal before it, such as
            // the end of a press the host took.
            iced_core::mouse::Event::ButtonReleased(
                iced_core::mouse::Button::Left,
            ) => Some(iced::event::Status::Ignored),
            _ => None,
        }
    }

    pub(crate) fn handle_keyboard_event(
        &self,
        view_state: &mut TerminalViewState,
        terminal_state: &SnapshotOwned,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        event: &iced::keyboard::Event,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) -> iced::event::Status {
        let mut binding_action = BindingAction::Ignore;
        let mut is_repeat = false;
        let terminal_state_ref = terminal_state.view();

        match event {
            iced::keyboard::Event::ModifiersChanged(m) => {
                view_state.keyboard_modifiers = *m;
            },
            iced::keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                text,
                repeat,
                ..
            } => {
                is_repeat = *repeat;

                // Bindings on a physical key come first: they match the key
                // whatever character the layout and Shift make it type.
                if let Physical::Code(code) = physical_key {
                    binding_action = self.bindings.get_action(
                        InputKind::Physical(*code),
                        modifiers,
                        terminal_state_ref.mode,
                    );
                }

                match &key {
                    // A physical binding matched; the character does not
                    // matter any more.
                    _ if binding_action != BindingAction::Ignore => {},
                    // Use the physical character key for bindings even when text is None (e.g., Ctrl/Cmd combos)
                    Key::Character(k) => {
                        let lower = k.to_ascii_lowercase();
                        // Read the modifiers off the event rather than the
                        // cached state: a pane created by a split starts
                        // empty and never sees a ModifiersChanged, because
                        // the modifiers did not change while it was born.
                        binding_action = self.bindings.get_action(
                            InputKind::Char(lower),
                            modifiers,
                            terminal_state_ref.mode,
                        );

                        // If no binding matched, write the text reported by
                        // the platform. Application shortcuts do not reach
                        // this branch because the host registers an action
                        // binding for them.
                        if binding_action == BindingAction::Ignore
                            && let Some(c) = text
                        {
                            self.write_typed(
                                view_state,
                                c.as_bytes().to_vec(),
                                publisher,
                            );
                            return iced::event::Status::Captured;
                        }
                    },
                    Key::Named(code) => {
                        binding_action = self.bindings.get_action(
                            InputKind::KeyCode(*code),
                            modifiers,
                            terminal_state_ref.mode,
                        );
                    },
                    _ => {},
                }
            },
            _ => {},
        }

        match binding_action {
            BindingAction::Char(c) => {
                let mut buf = [0, 0, 0, 0];
                let str = c.encode_utf8(&mut buf);
                self.write_typed(
                    view_state,
                    str.as_bytes().to_vec(),
                    publisher,
                );
                iced::event::Status::Captured
            },
            BindingAction::Esc(seq) => {
                self.write_typed(
                    view_state,
                    seq.as_bytes().to_vec(),
                    publisher,
                );
                iced::event::Status::Captured
            },
            BindingAction::Paste => {
                if let Some(data) = crate::clipboard::read_paste_text(clipboard)
                {
                    let input = crate::paste::paste_input(
                        &data,
                        terminal_state_ref.mode,
                    );
                    self.write_typed(view_state, input, publisher);
                    iced::event::Status::Captured
                } else {
                    iced::event::Status::Ignored
                }
            },
            BindingAction::Copy => {
                clipboard.write(
                    ClipboardKind::Standard,
                    terminal_state_ref.selectable_content(),
                );
                iced::event::Status::Ignored
            },
            // Auto-repeat is dropped here rather than in the host: an
            // application action fires once per press, and a split action
            // would otherwise spawn a pane, terminal and shell process for
            // every repeat event while the key is held.
            BindingAction::Action(action) if !is_repeat => {
                publisher(crate::Event::BindingActionDispatched {
                    id: self.terminal_id,
                    action,
                });
                iced::event::Status::Captured
            },
            _ => iced::event::Status::Ignored,
        }
    }

    /// End the block selection, if a block is selected, and tell the host,
    /// with a redraw. Typing, starting a text selection and the block
    /// command of the host to clear the selection end it.
    pub(crate) fn clear_block_selection(
        &self,
        state: &mut TerminalViewState,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) {
        if state.selected_block_key.take().is_none() {
            return;
        }

        publisher(crate::Event::BlockSelectionCleared {
            id: self.terminal_id,
        });
        publisher(crate::Event::Redraw {
            id: self.terminal_id,
        });
    }

    /// Send typed `data` to the program. It goes to the prompt or to the
    /// running program, so the block selection ends.
    fn write_typed(
        &self,
        state: &mut TerminalViewState,
        data: Vec<u8>,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) {
        self.clear_block_selection(state, publisher);
        publisher(crate::Event::Write {
            id: self.terminal_id,
            data,
        });
    }

    /// Find the link and the block at `state.mouse_position_on_grid` in
    /// `view` and keep them. The host shows controls for the block under
    /// the pointer, so it learns when that block changes. Returns whether
    /// anything changed, that is whether the pane must be drawn again.
    fn look_up_hover(
        &self,
        state: &mut TerminalViewState,
        view: &SnapshotView<'_>,
        publisher: &mut impl FnMut(crate::Event<T>),
    ) -> bool {
        let hovered_span_id =
            view.hyperlink_span_id_at(state.mouse_position_on_grid);
        // Only command blocks are blocks for the user: not the prompt, and
        // not the block a terminal starts with.
        let hovered_block = view
            .block_at_point(state.mouse_position_on_grid)
            .filter(|block| block.is_command());
        let hovered_block_key = hovered_block.map(|block| block.key);

        let mut changed = false;

        if hovered_span_id != state.hovered_span_id {
            state.hovered_span_id = hovered_span_id;
            changed = true;
        }

        if hovered_block_key != state.hovered_block_key {
            state.hovered_block_key = hovered_block_key;
            publisher(crate::Event::BlockHovered {
                id: self.terminal_id,
                block_key: hovered_block_key,
            });
            changed = true;
        }

        changed
    }
}

/// How many lines the wheel turned by `delta`: whole lines, upward
/// positive. A touchpad reports pixels, which add up in `state` until they
/// make a line.
fn wheel_lines(
    state: &mut TerminalViewState,
    delta: &ScrollDelta,
    font_measure: &Size<f32>,
) -> f32 {
    match delta {
        ScrollDelta::Lines { y, .. } => y.signum() * y.abs().round(),
        ScrollDelta::Pixels { y, .. } => {
            state.scroll_pixels -= y;
            let line_height = font_measure.height;
            let lines = (state.scroll_pixels / line_height).trunc();
            state.scroll_pixels %= line_height;
            lines
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_libterm::TerminalSize;
    use otty_libterm::escape::{Hyperlink, NamedPrivateMode};
    use otty_libterm::surface::{
        BlockKey, BlockKind, BlockMeta, BlockSnapshot, Column, Line,
        Point as TerminalGridPoint, SnapshotOwned, Surface, SurfaceActor,
        SurfaceConfig, SurfaceModel,
    };

    use super::*;
    use crate::font::TermFont;
    use crate::settings::FontSettings;
    use crate::testing::engine_showing;

    const TEST_ID: u64 = 1;

    /// The keys of the blocks of the tests.
    const START: BlockKey = BlockKey::new(0);
    const BLOCK_1: BlockKey = BlockKey::new(1);
    const LS: BlockKey = BlockKey::new(2);
    const PWD: BlockKey = BlockKey::new(3);
    const OLDER: BlockKey = BlockKey::new(4);
    const PROMPT: BlockKey = BlockKey::new(5);
    const CARGO: BlockKey = BlockKey::new(6);
    const VIM: BlockKey = BlockKey::new(7);

    fn default_snapshot() -> SnapshotOwned {
        SnapshotOwned::default()
    }

    fn snapshot_with_modes(modes: &[NamedPrivateMode]) -> SnapshotOwned {
        let size = TerminalSize::default();
        let mut surface = Surface::new(SurfaceConfig::default(), &size);

        for mode in modes {
            surface.set_private_mode((*mode).into());
        }

        surface.snapshot_owned()
    }

    fn snapshot_with_hyperlink(uri: &str) -> SnapshotOwned {
        let size = TerminalSize::default();
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        let link = Hyperlink {
            id: None,
            uri: uri.to_string(),
        };
        surface.grid_mut()[Line(0)][Column(0)].set_hyperlink(Some(link.into()));
        surface.grid_mut()[Line(0)][Column(0)].c = 'h';

        surface.snapshot_owned()
    }

    fn snapshot_with_block(
        key: BlockKey,
        kind: BlockKind,
        start_line: i32,
        line_count: usize,
    ) -> SnapshotOwned {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks.push(BlockSnapshot {
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
        });

        snapshot
    }

    mod handle_keyboard_event_tests {
        use iced::keyboard::key::{Code, Physical};
        use iced::keyboard::{Event, Key, Location, Modifiers};

        use super::*;
        use crate::bindings;
        use crate::bindings::KeyboardBinding;

        fn key_pressed(character: &str, modifiers: Modifiers) -> Event {
            key_pressed_with_text(character, Some(character), modifiers)
        }

        /// Build a key press whose `text` differs from the character key,
        /// as happens when the platform applies a Ctrl transformation or
        /// produces no text at all.
        fn key_pressed_with_text(
            character: &str,
            text: Option<&str>,
            modifiers: Modifiers,
        ) -> Event {
            let key = Key::Character(character.into());

            Event::KeyPressed {
                key: key.clone(),
                modified_key: key,
                physical_key: Physical::Code(Code::KeyD),
                location: Location::Standard,
                modifiers,
                text: text.map(Into::into),
                repeat: false,
            }
        }

        /// Build an auto-repeat key press, as the platform delivers while
        /// the key is held down.
        fn key_pressed_repeat(character: &str, modifiers: Modifiers) -> Event {
            let Event::KeyPressed {
                key,
                modified_key,
                physical_key,
                location,
                modifiers,
                text,
                ..
            } = key_pressed(character, modifiers)
            else {
                unreachable!("key_pressed always builds a KeyPressed event")
            };

            Event::KeyPressed {
                key,
                modified_key,
                physical_key,
                location,
                modifiers,
                text,
                repeat: true,
            }
        }

        /// A stand-in for a host application's action type; the terminal
        /// crate never interprets it.
        #[derive(Clone, Debug, PartialEq, Eq)]
        enum TestAction {
            Split,
        }

        /// Build the binding layout an application registers when it wants
        /// a shortcut to surface as
        /// [`crate::Event::BindingActionDispatched`].
        fn bindings_with_action() -> bindings::BindingsLayout<TestAction> {
            let mut layout = bindings::BindingsLayout::new();
            layout.add_bindings(crate::generate_bindings!(
                KeyboardBinding;
                "d", Modifiers::LOGO; BindingAction::Action(TestAction::Split);
            ));

            layout
        }

        fn dispatch(event: &Event) -> (iced::event::Status, Vec<crate::Event>) {
            let cached = match event {
                Event::KeyPressed { modifiers, .. } => *modifiers,
                _ => Modifiers::empty(),
            };

            dispatch_with(event, cached, bindings::BindingsLayout::new())
        }

        /// Dispatch against a custom binding layout, so a test can register
        /// an application action the way the app does at startup.
        fn dispatch_with_bindings(
            event: &Event,
            bindings: bindings::BindingsLayout<TestAction>,
        ) -> (iced::event::Status, Vec<crate::Event<TestAction>>) {
            let cached = match event {
                Event::KeyPressed { modifiers, .. } => *modifiers,
                _ => Modifiers::empty(),
            };

            dispatch_with(event, cached, bindings)
        }

        /// Dispatch with an explicit cached modifier state, so a test can
        /// reproduce a freshly split pane whose view state has not seen a
        /// `ModifiersChanged` yet.
        fn dispatch_with_stale_modifiers<T: Clone + PartialEq>(
            event: &Event,
            cached: Modifiers,
            bindings: bindings::BindingsLayout<T>,
        ) -> (iced::event::Status, Vec<crate::Event<T>>) {
            dispatch_with(event, cached, bindings)
        }

        fn dispatch_with<T: Clone + PartialEq>(
            event: &Event,
            cached: Modifiers,
            bindings: bindings::BindingsLayout<T>,
        ) -> (iced::event::Status, Vec<crate::Event<T>>) {
            let mut state = TerminalViewState::new();
            state.keyboard_modifiers = cached;
            let mut clipboard = iced_core::clipboard::Null;
            let mut commands = Vec::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            let status = {
                let mut publish = |event| commands.push(event);
                input_manager.handle_keyboard_event(
                    &mut state,
                    &default_snapshot(),
                    &mut clipboard,
                    event,
                    &mut publish,
                )
            };

            (status, commands)
        }

        #[cfg(target_os = "macos")]
        struct FileUrlClipboard {
            value: String,
        }

        #[cfg(target_os = "macos")]
        impl iced_core::clipboard::Clipboard for FileUrlClipboard {
            fn read(&self, kind: ClipboardKind) -> Option<String> {
                (kind == ClipboardKind::Standard).then(|| self.value.clone())
            }

            fn write(&mut self, _kind: ClipboardKind, _contents: String) {}
        }

        #[cfg(target_os = "macos")]
        #[test]
        fn macos_file_url_paste_prefers_posix_path() {
            let event = key_pressed("v", Modifiers::COMMAND);
            let mut state = TerminalViewState::new();
            let mut clipboard = FileUrlClipboard {
                value: String::from(
                    "file:///Users/example/My%20File%20%E3%83%86%E3%82%B9%E3%83%88.txt",
                ),
            };
            let mut commands = Vec::new();
            let bindings: BindingsLayout = BindingsLayout::default();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );
            let mut publish = |event| commands.push(event);

            let status = input_manager.handle_keyboard_event(
                &mut state,
                &default_snapshot(),
                &mut clipboard,
                &event,
                &mut publish,
            );

            assert_eq!(status, iced::event::Status::Captured);
            assert!(commands.iter().any(|event| matches!(
                event,
                crate::Event::Write { id: TEST_ID, data }
                    if data == "/Users/example/My File テスト.txt".as_bytes()
            )));
        }

        #[test]
        fn a_bound_action_is_published_to_the_application() {
            let event = key_pressed("d", Modifiers::LOGO);

            let (status, commands) =
                dispatch_with_bindings(&event, bindings_with_action());

            assert_eq!(status, iced::event::Status::Captured);
            assert!(
                commands.iter().any(|event| matches!(
                    event,
                    crate::Event::BindingActionDispatched {
                        id: TEST_ID,
                        action,
                    }
                        if *action == TestAction::Split
                )),
                "a bound action must surface as a binding-action event, got \
                 {commands:?}"
            );
            assert!(
                !commands
                    .iter()
                    .any(|event| matches!(event, crate::Event::Write { .. })),
                "an action binding must not also reach the pty, got {commands:?}"
            );
        }

        #[test]
        fn a_repeated_action_is_dropped() {
            // Holding the shortcut down would otherwise spawn one pane,
            // terminal and shell process per repeat event.
            let event = key_pressed_repeat("d", Modifiers::LOGO);

            let (status, commands) =
                dispatch_with_bindings(&event, bindings_with_action());

            assert_eq!(status, iced::event::Status::Ignored);
            assert!(
                !commands.iter().any(|event| matches!(
                    event,
                    crate::Event::BindingActionDispatched { .. }
                )),
                "auto-repeat must not publish an action, got {commands:?}"
            );
        }

        #[test]
        fn unbound_logo_modified_character_writes_reported_text() {
            let event = key_pressed("d", Modifiers::LOGO);

            let (status, commands) = dispatch(&event);

            assert_eq!(status, iced::event::Status::Captured);
            assert!(
                commands.iter().any(|event| matches!(
                    event,
                    crate::Event::Write { id: TEST_ID, data } if data == b"d"
                )),
                "an unbound key must preserve the text reported by the platform, \
                 got {commands:?}"
            );
        }

        #[test]
        fn bound_action_uses_event_modifiers_when_the_cache_is_stale() {
            // A pane created by the split itself starts with empty
            // modifiers and never sees a ModifiersChanged, because the
            // modifiers did not change while it was being created.
            let event = key_pressed("d", Modifiers::LOGO);

            let (status, commands) = dispatch_with_stale_modifiers(
                &event,
                Modifiers::empty(),
                bindings_with_action(),
            );

            assert_eq!(status, iced::event::Status::Captured);
            assert!(
                commands.iter().any(|event| matches!(
                    event,
                    crate::Event::BindingActionDispatched {
                        id: TEST_ID,
                        action,
                    }
                        if *action == TestAction::Split
                )),
                "the event modifiers must match the action binding even when \
                 the cached modifiers are stale, got {commands:?}"
            );
            assert!(
                !commands
                    .iter()
                    .any(|event| matches!(event, crate::Event::Write { .. })),
                "a matched action must not also reach the pty, got {commands:?}"
            );
        }

        #[test]
        fn plain_character_is_written_to_the_pty() {
            let event = key_pressed("d", Modifiers::empty());

            let (status, commands) = dispatch(&event);

            assert_eq!(status, iced::event::Status::Captured);
            assert!(commands.iter().any(|event| matches!(
                event,
                crate::Event::Write { id: TEST_ID, data } if data == b"d"
            )));
        }

        #[test]
        fn ctrl_modified_character_matches_its_binding() {
            // Ctrl+D is bound to EOF in the binding table on every
            // platform; the logo guard must not shadow it.
            let event = key_pressed("d", Modifiers::CTRL);

            let (status, commands) = dispatch(&event);

            assert_eq!(status, iced::event::Status::Captured);
            assert!(
                commands.iter().any(|event| matches!(
                    event,
                    crate::Event::Write { id: TEST_ID, data } if data == b"\x04"
                )),
                "Ctrl+D must send EOF through its binding, got {commands:?}"
            );
        }

        /// Build the binding layout an application registers for a shortcut
        /// on the `1` key of the digit row, matched by its place on the
        /// keyboard.
        fn bindings_with_digit_action() -> bindings::BindingsLayout<TestAction>
        {
            let mut layout = bindings::BindingsLayout::new();
            layout.add_bindings(vec![(
                KeyboardBinding {
                    target: InputKind::Physical(Code::Digit1),
                    modifiers: Modifiers::CTRL | Modifiers::SHIFT,
                    mode_include: SurfaceMode::empty(),
                    mode_exclude: SurfaceMode::empty(),
                },
                BindingAction::Action(TestAction::Split),
            )]);

            layout
        }

        /// Build a press of the physical key `code` that reports
        /// `character`, as a layout and the held modifiers make it.
        fn physical_key_pressed(
            code: Code,
            character: &str,
            modifiers: Modifiers,
        ) -> Event {
            let key = Key::Character(character.into());

            Event::KeyPressed {
                key: key.clone(),
                modified_key: key,
                physical_key: Physical::Code(code),
                location: Location::Standard,
                modifiers,
                text: Some(character.into()),
                repeat: false,
            }
        }

        #[test]
        fn a_physical_binding_matches_whatever_character_the_key_types() {
            // US layout without Shift, US layout with Shift, French AZERTY.
            for character in ["1", "!", "&"] {
                let event = physical_key_pressed(
                    Code::Digit1,
                    character,
                    Modifiers::CTRL | Modifiers::SHIFT,
                );

                let (status, commands) = dispatch_with_bindings(
                    &event,
                    bindings_with_digit_action(),
                );

                assert_eq!(status, iced::event::Status::Captured);
                assert!(
                    matches!(
                        commands.as_slice(),
                        [crate::Event::BindingActionDispatched {
                            id: TEST_ID,
                            action: TestAction::Split,
                        }]
                    ),
                    "{character:?} on the 1 key must run the action and \
                     reach no pty, got {commands:?}"
                );
            }
        }

        #[test]
        fn a_physical_binding_needs_its_exact_modifiers() {
            let event =
                physical_key_pressed(Code::Digit1, "1", Modifiers::CTRL);

            let (status, commands) =
                dispatch_with_bindings(&event, bindings_with_digit_action());

            assert_eq!(status, iced::event::Status::Captured);
            assert!(
                matches!(
                    commands.as_slice(),
                    [crate::Event::Write { id: TEST_ID, data }] if data == b"1"
                ),
                "Ctrl 1 must keep reaching the shell, got {commands:?}"
            );
        }

        #[test]
        fn the_same_character_on_another_key_does_not_match_a_physical_binding()
        {
            // The keypad 1 types "1" too, but it is not the digit row key.
            let event = physical_key_pressed(
                Code::Numpad1,
                "1",
                Modifiers::CTRL | Modifiers::SHIFT,
            );

            let (_, commands) =
                dispatch_with_bindings(&event, bindings_with_digit_action());

            assert!(
                !commands.iter().any(|event| matches!(
                    event,
                    crate::Event::BindingActionDispatched { .. }
                )),
                "the keypad 1 must not run the digit row action, got \
                 {commands:?}"
            );
        }

        #[test]
        fn character_bindings_still_match_next_to_physical_ones() {
            // Ctrl M is CR in the terminal's own table.
            let event = physical_key_pressed(Code::KeyM, "m", Modifiers::CTRL);

            let (status, commands) =
                dispatch_with_bindings(&event, bindings_with_digit_action());

            assert_eq!(status, iced::event::Status::Captured);
            assert!(
                matches!(
                    commands.as_slice(),
                    [crate::Event::Write { id: TEST_ID, data }] if data == b"\r"
                ),
                "Ctrl M must keep sending CR, got {commands:?}"
            );
        }

        #[test]
        fn unbound_ctrl_character_still_writes_its_text() {
            // Ctrl+9 has no binding; whatever text the platform
            // delivers must reach the pty unfiltered.
            let event = key_pressed("9", Modifiers::CTRL);

            let (status, commands) = dispatch(&event);

            assert_eq!(status, iced::event::Status::Captured);
            assert!(
                commands.iter().any(|event| matches!(
                    event,
                    crate::Event::Write { id: TEST_ID, data } if data == b"9"
                )),
                "an unbound Ctrl combination must keep its text fallback, \
                 got {commands:?}"
            );
        }

        /// Press `event` while the block `ls` is selected; return what the
        /// terminal published and whether `ls` is still selected.
        fn press_with_selected_block(
            event: &Event,
            bindings: bindings::BindingsLayout<TestAction>,
        ) -> (Vec<crate::Event<TestAction>>, bool) {
            let mut state = TerminalViewState::new();
            state.selected_block_key = Some(LS);
            let mut clipboard = iced_core::clipboard::Null;
            let mut published = Vec::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            {
                let mut publish = |event| published.push(event);
                let _ = input_manager.handle_keyboard_event(
                    &mut state,
                    &default_snapshot(),
                    &mut clipboard,
                    event,
                    &mut publish,
                );
            }

            let is_selected = state.selected_block_key.is_some();
            (published, is_selected)
        }

        fn clears_the_selection(
            published: &[crate::Event<TestAction>],
        ) -> bool {
            published.iter().any(|event| {
                matches!(
                    event,
                    crate::Event::BlockSelectionCleared { id: TEST_ID }
                )
            })
        }

        /// Typing a command, a letter or a named key such as Enter, ends
        /// the block selection, as the text goes to the prompt.
        #[test]
        fn a_key_written_to_the_program_clears_the_block_selection() {
            let enter = Event::KeyPressed {
                key: Key::Named(iced::keyboard::key::Named::Enter),
                modified_key: Key::Named(iced::keyboard::key::Named::Enter),
                physical_key: Physical::Code(Code::Enter),
                location: Location::Standard,
                modifiers: Modifiers::empty(),
                text: None,
                repeat: false,
            };

            for event in [key_pressed("l", Modifiers::empty()), enter] {
                let (published, is_selected) = press_with_selected_block(
                    &event,
                    bindings::BindingsLayout::new(),
                );

                assert!(!is_selected, "{event:?}");
                assert!(clears_the_selection(&published), "{published:?}");
                assert!(
                    published.iter().any(|event| matches!(
                        event,
                        crate::Event::Write { .. }
                    )),
                    "{published:?}"
                );
            }
        }

        /// An app shortcut writes nothing, so a block selected before it,
        /// for example with the block keys, stays selected.
        #[test]
        fn an_app_shortcut_keeps_the_block_selection() {
            let event = key_pressed("d", Modifiers::LOGO);

            let (published, is_selected) =
                press_with_selected_block(&event, bindings_with_action());

            assert!(is_selected);
            assert!(!clears_the_selection(&published), "{published:?}");
        }
    }

    mod handle_left_button_pressed_tests {
        use iced::keyboard::Modifiers;

        use super::*;
        use crate::bindings;

        #[test]
        fn handles_mouse_mode_with_left_click() {
            let mut state = TerminalViewState::new();
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let _modifiers = Modifiers::empty();
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_left_button_pressed(
                &mut state,
                &snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]),
                cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                crate::Event::MouseReport {
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

            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_left_button_pressed(
                &mut state,
                &default_snapshot(),
                cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 2);
            assert!(matches!(
                commands[0],
                crate::Event::SelectStart {
                    id: TEST_ID,
                    selection_type: SelectionType::Simple,
                    position: (150.0, 100.0)
                }
            ));
            assert!(matches!(commands[1], crate::Event::Redraw { .. }));
            assert!(state.is_dragged);
        }
    }

    mod handle_cursor_moved_tests {
        use super::*;
        use crate::bindings;

        #[test]
        fn updates_mouse_position_on_grid() {
            let mut state = TerminalViewState::new();
            let engine =
                engine_showing(default_snapshot(), TerminalSize::default());
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

            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            for (layout_position, cursor_position, expected) in cases {
                input_manager.handle_cursor_moved(
                    &mut state,
                    &engine,
                    &cursor_position,
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
            let engine =
                engine_showing(default_snapshot(), TerminalSize::default());
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_cursor_moved(
                &mut state,
                &engine,
                &cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 2);
            assert!(matches!(
                commands[0],
                crate::Event::SelectUpdate {
                    id: TEST_ID,
                    position: (95.0, 145.0)
                }
            ));
            assert!(matches!(commands[1], crate::Event::Redraw { .. }));
        }

        #[test]
        fn selects_update_when_dragged_without_mouse_motion_mode() {
            let mut state = TerminalViewState::new();
            state.is_dragged = true; // Simulate an ongoing drag operation
            let engine =
                engine_showing(default_snapshot(), TerminalSize::default());
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_cursor_moved(
                &mut state,
                &engine,
                &cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 2);
            assert!(matches!(
                commands[0],
                crate::Event::SelectUpdate {
                    id: TEST_ID,
                    position: (95.0, 145.0)
                }
            ));
            assert!(matches!(commands[1], crate::Event::Redraw { .. }));
            assert!(state.selection_in_progress);
        }

        #[test]
        fn clears_block_selection_when_dragging_starts() {
            let mut state = TerminalViewState::new();
            state.is_dragged = true;
            state.selected_block_key = Some(BLOCK_1);
            let engine =
                engine_showing(default_snapshot(), TerminalSize::default());
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_cursor_moved(
                &mut state,
                &engine,
                &cursor_position,
                layout_position,
                &mut publish,
            );

            assert!(state.selection_in_progress);
            assert!(state.selected_block_key.is_none());
            assert_eq!(commands.len(), 4);
            assert!(matches!(
                commands[0],
                crate::Event::BlockSelectionCleared { .. }
            ));
            assert!(matches!(commands[1], crate::Event::Redraw { .. }));
            assert!(matches!(
                commands[2],
                crate::Event::SelectUpdate {
                    id: TEST_ID,
                    position: (95.0, 145.0)
                }
            ));
            assert!(matches!(commands[3], crate::Event::Redraw { .. }));
        }
    }

    mod hovered_block_tests {
        use super::*;
        use crate::bindings;

        /// A grid of 8 by 16 px cells.
        fn grid() -> TerminalSize {
            TerminalSize {
                cell_width: 8,
                cell_height: 16,
                cols: 80,
                rows: 50,
            }
        }

        /// Move the pointer to `cursor_position` in a pane whose text
        /// starts at the origin and return what the terminal published.
        fn move_pointer(
            state: &mut TerminalViewState,
            snapshot: &SnapshotOwned,
            cursor_position: Point,
        ) -> Vec<crate::Event> {
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_cursor_moved(
                state,
                &engine_showing(snapshot.clone(), grid()),
                &cursor_position,
                Point::ORIGIN,
                &mut publish,
            );

            commands
        }

        fn leave_pane(state: &mut TerminalViewState) -> Vec<crate::Event> {
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_pointer_left(state, &mut publish);

            commands
        }

        #[test]
        fn reports_the_block_and_redraws_when_the_pointer_enters_it() {
            let mut state = TerminalViewState::new();
            let snapshot =
                snapshot_with_block(BLOCK_1, BlockKind::Command, 0, 2);

            let commands =
                move_pointer(&mut state, &snapshot, Point::new(4.0, 4.0));

            assert_eq!(state.hovered_block_key, Some(BLOCK_1));
            assert!(
                matches!(
                    commands.as_slice(),
                    [
                        crate::Event::BlockHovered {
                            id: TEST_ID,
                            block_key: Some(BLOCK_1),
                        },
                        crate::Event::Redraw { id: TEST_ID },
                    ]
                ),
                "got {commands:?}"
            );
        }

        #[test]
        fn does_not_redraw_while_the_pointer_stays_in_the_block() {
            let mut state = TerminalViewState::new();
            let snapshot =
                snapshot_with_block(BLOCK_1, BlockKind::Command, 0, 2);
            move_pointer(&mut state, &snapshot, Point::new(4.0, 4.0));

            let commands =
                move_pointer(&mut state, &snapshot, Point::new(6.0, 4.0));

            assert_eq!(state.hovered_block_key, Some(BLOCK_1));
            assert!(commands.is_empty(), "got {commands:?}");
        }

        #[test]
        fn reports_no_block_and_redraws_when_the_pointer_moves_off_every_block()
        {
            let mut state = TerminalViewState::new();
            let snapshot =
                snapshot_with_block(BLOCK_1, BlockKind::Command, 0, 2);
            move_pointer(&mut state, &snapshot, Point::new(4.0, 4.0));
            let below_the_block = Point::new(4.0, 900.0);

            let commands = move_pointer(&mut state, &snapshot, below_the_block);

            assert_eq!(state.hovered_block_key, None);
            assert!(
                matches!(
                    commands.as_slice(),
                    [
                        crate::Event::BlockHovered {
                            id: TEST_ID,
                            block_key: None,
                        },
                        crate::Event::Redraw { id: TEST_ID },
                    ]
                ),
                "got {commands:?}"
            );
        }

        /// The text starts to the right of the pane's left edge. A pointer
        /// between the two is over the first column of the line.
        #[test]
        fn a_pointer_left_of_the_text_is_over_the_first_column() {
            let mut state = TerminalViewState::new();
            let snapshot =
                snapshot_with_block(BLOCK_1, BlockKind::Command, 0, 2);
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );
            let text_origin = Point::new(16.0, 0.0);
            let cursor_position = Point::new(3.0, 20.0);

            input_manager.handle_cursor_moved(
                &mut state,
                &engine_showing(snapshot, grid()),
                &cursor_position,
                text_origin,
                &mut |_| {},
            );

            assert_eq!(
                state.mouse_position_on_grid,
                TerminalGridPoint {
                    line: Line(1),
                    column: Column(0),
                }
            );
            assert_eq!(state.hovered_block_key, Some(BLOCK_1));
        }

        #[test]
        fn leaving_the_pane_forgets_the_hovered_block_reports_it_and_redraws() {
            let mut state = TerminalViewState::new();
            state.hovered_block_key = Some(BLOCK_1);

            let commands = leave_pane(&mut state);

            assert_eq!(state.hovered_block_key, None);
            assert!(
                matches!(
                    commands.as_slice(),
                    [
                        crate::Event::BlockHovered {
                            id: TEST_ID,
                            block_key: None,
                        },
                        crate::Event::Redraw { id: TEST_ID },
                    ]
                ),
                "got {commands:?}"
            );
        }

        #[test]
        fn leaving_the_pane_with_no_hovered_block_does_not_redraw() {
            let mut state = TerminalViewState::new();

            let commands = leave_pane(&mut state);

            assert!(commands.is_empty(), "got {commands:?}");
        }

        /// The block a terminal starts with holds what the shell printed
        /// before its first prompt. No command started it, so it is not a
        /// block for the user, even when it holds text.
        #[test]
        fn the_start_block_is_not_hovered_under_the_pointer() {
            let mut state = TerminalViewState::new();
            let snapshot = snapshot_with_block(START, BlockKind::Start, 0, 2);

            let commands =
                move_pointer(&mut state, &snapshot, Point::new(4.0, 20.0));

            assert_eq!(state.hovered_block_key, None);
            assert!(commands.is_empty(), "got {commands:?}");
        }

        /// After `ls` the first block that can be hovered, and so clicked
        /// to select, is `ls`.
        #[test]
        fn after_ls_the_first_block_that_is_hovered_and_selected_is_ls() {
            let mut state = TerminalViewState::new();
            let mut snapshot =
                snapshot_with_block(START, BlockKind::Start, 0, 1);
            snapshot.blocks.extend(
                snapshot_with_block(LS, BlockKind::Command, 1, 2).blocks,
            );
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            move_pointer(&mut state, &snapshot, Point::new(4.0, 4.0));
            assert_eq!(state.hovered_block_key, None);
            input_manager.handle_button_released(
                &mut state,
                &snapshot,
                &bindings,
                &mut |_| {},
            );
            assert_eq!(state.selected_block_key, None);

            move_pointer(&mut state, &snapshot, Point::new(4.0, 20.0));
            assert_eq!(state.hovered_block_key, Some(LS));
            input_manager.handle_button_released(
                &mut state,
                &snapshot,
                &bindings,
                &mut |_| {},
            );
            assert_eq!(state.selected_block_key, Some(LS));
        }
    }

    /// The scroll mark at the right edge of the pane: the pointer takes it
    /// anywhere in the right inset, the strip it runs in.
    mod scroll_mark_tests {
        use iced::Rectangle;
        use iced::mouse::ScrollDelta;

        use super::*;
        use crate::bindings;
        use crate::scroll_mark::ScrollMark;

        /// A grid of 36 columns and 8 rows of 8 by 16 px cells.
        const GRID: TerminalSize = TerminalSize {
            cell_width: 8,
            cell_height: 16,
            cols: 36,
            rows: 8,
        };
        /// The pane: 16 px wider than the grid at each side, as high as
        /// it, at the top left corner of the window.
        const PANE: Rectangle = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 128.0,
        };
        /// The part of the pane that holds the text.
        const TEXT_AREA: Rectangle = Rectangle {
            x: 16.0,
            y: 0.0,
            width: 288.0,
            height: 128.0,
        };
        /// A place in the right inset, the strip of the mark.
        const STRIP_X: f32 = 310.0;

        /// A screen of [`GRID`] that printed `lines` lines, with `modes`
        /// on. With 40 lines, 32 are in the history above: the mark is
        /// 48 px high and travels 80 px over 32 lines, 2.5 px a line. At
        /// the bottom of the history it covers 80 to 128 px.
        fn history(lines: usize, modes: &[NamedPrivateMode]) -> SnapshotOwned {
            let mut surface = Surface::new(SurfaceConfig::default(), &GRID);
            for _ in 1..lines {
                surface.line_feed();
            }
            for mode in modes {
                surface.set_private_mode((*mode).into());
            }

            surface.snapshot_owned()
        }

        /// Give `event` to the terminal showing `snapshot` with the pointer
        /// at `position`, and return what it published.
        fn send(
            state: &mut TerminalViewState,
            snapshot: &SnapshotOwned,
            position: Point,
            event: mouse::Event,
        ) -> Vec<crate::Event> {
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );
            let font = TermFont::new(FontSettings::default());
            let mark = ScrollMark::of_view(PANE, TEXT_AREA, &snapshot.view());

            let _ = input_manager.handle_mouse_event(
                state,
                &engine_showing(snapshot.clone(), GRID),
                &font,
                TEXT_AREA.position(),
                position,
                mark.as_ref(),
                &event,
                &mut publish,
            );

            commands
        }

        fn press() -> mouse::Event {
            mouse::Event::ButtonPressed(mouse::Button::Left)
        }

        fn release() -> mouse::Event {
            mouse::Event::ButtonReleased(mouse::Button::Left)
        }

        /// Move the pointer to `position` and return what was published.
        fn move_to(
            state: &mut TerminalViewState,
            snapshot: &SnapshotOwned,
            position: Point,
        ) -> Vec<crate::Event> {
            let moved = mouse::Event::CursorMoved { position };

            send(state, snapshot, position, moved)
        }

        /// The scroll offsets the terminal asked for, in order.
        fn offsets(commands: &[crate::Event]) -> Vec<usize> {
            commands
                .iter()
                .filter_map(|command| match command {
                    crate::Event::ScrollToOffset { offset, .. } => {
                        Some(*offset)
                    },
                    _ => None,
                })
                .collect()
        }

        /// Whether `commands` ask to go one step past the top of the
        /// history.
        fn asks_past_the_top(commands: &[crate::Event]) -> bool {
            commands.iter().any(|command| {
                matches!(command, crate::Event::ScrollPastTheTop { .. })
            })
        }

        /// A press on the strip right at its top puts the top of the mark
        /// there: the top of its way, past the top of the history.
        #[test]
        fn a_press_at_the_top_of_the_strip_goes_past_the_top_of_the_history() {
            let snapshot = history(40, &[]);
            let mut state = TerminalViewState::new();

            let pressed =
                send(&mut state, &snapshot, Point::new(STRIP_X, 1.0), press());

            assert!(offsets(&pressed).is_empty(), "got {pressed:?}");
            assert!(asks_past_the_top(&pressed), "got {pressed:?}");
        }

        /// Whether `commands` start or extend a text selection, select a
        /// block or tell a program about the mouse.
        fn touches_text_blocks_or_programs(commands: &[crate::Event]) -> bool {
            commands.iter().any(|command| {
                matches!(
                    command,
                    crate::Event::SelectStart { .. }
                        | crate::Event::SelectUpdate { .. }
                        | crate::Event::BlockSelected { .. }
                        | crate::Event::BlockSelectionCleared { .. }
                        | crate::Event::MouseReport { .. }
                )
            })
        }

        #[test]
        fn a_press_on_the_mark_and_a_move_by_n_px_scroll_by_as_many_lines() {
            let snapshot = history(40, &[]);
            let mut state = TerminalViewState::new();

            let pressed = send(
                &mut state,
                &snapshot,
                Point::new(STRIP_X, 100.0),
                press(),
            );
            // 10 px up is 4 lines, 25 px up is 10 lines.
            let first =
                move_to(&mut state, &snapshot, Point::new(STRIP_X, 90.0));
            let second =
                move_to(&mut state, &snapshot, Point::new(STRIP_X, 75.0));

            assert!(offsets(&pressed).is_empty(), "got {pressed:?}");
            assert_eq!(offsets(&first), [4]);
            assert_eq!(offsets(&second), [10]);
            assert!(state.scroll_mark_grab().is_some());
        }

        #[test]
        fn a_press_on_the_strip_above_the_mark_centers_the_mark_under_the_pointer()
         {
            let snapshot = history(40, &[]);
            let mut state = TerminalViewState::new();

            // The mark covers 80 to 128 px. Centered at 44 px, its top is
            // at 20 px: a quarter of its way, 8 of the 32 lines.
            let pressed =
                send(&mut state, &snapshot, Point::new(STRIP_X, 44.0), press());
            // The drag goes on from there, holding the mark at its middle.
            let moved =
                move_to(&mut state, &snapshot, Point::new(STRIP_X, 54.0));

            assert_eq!(offsets(&pressed), [24]);
            assert_eq!(offsets(&moved), [20]);
        }

        #[test]
        fn a_press_in_the_strip_starts_no_text_selection_and_selects_no_block()
        {
            let mut snapshot = history(40, &[]);
            snapshot.blocks =
                snapshot_with_block(LS, BlockKind::Command, 0, 8).blocks;
            let mut state = TerminalViewState::new();
            state.hovered_block_key = Some(LS);

            let mut published = send(
                &mut state,
                &snapshot,
                Point::new(STRIP_X, 100.0),
                press(),
            );
            published.extend(move_to(
                &mut state,
                &snapshot,
                Point::new(STRIP_X, 90.0),
            ));
            published.extend(send(
                &mut state,
                &snapshot,
                Point::new(STRIP_X, 90.0),
                release(),
            ));

            assert!(
                !touches_text_blocks_or_programs(&published),
                "got {published:?}"
            );
            assert!(!state.is_dragged);
            assert_eq!(state.selected_block_key, None);
            assert_eq!(state.scroll_mark_grab(), None);
        }

        /// A program that reads the mouse in the normal screen, such as
        /// `fzf --height 40%`: the drag scrolls the history of OTTY and
        /// tells the program nothing.
        #[test]
        fn a_drag_of_the_mark_sends_nothing_to_a_program_that_reads_the_mouse()
        {
            let snapshot = history(
                40,
                &[
                    NamedPrivateMode::ReportMouseClicks,
                    NamedPrivateMode::ReportCellMouseMotion,
                ],
            );
            let mut state = TerminalViewState::new();

            let mut published = send(
                &mut state,
                &snapshot,
                Point::new(STRIP_X, 100.0),
                press(),
            );
            published.extend(move_to(
                &mut state,
                &snapshot,
                Point::new(STRIP_X, 90.0),
            ));
            published.extend(send(
                &mut state,
                &snapshot,
                Point::new(STRIP_X, 90.0),
                release(),
            ));

            assert!(
                !touches_text_blocks_or_programs(&published),
                "got {published:?}"
            );
            assert_eq!(offsets(&published), [4]);
        }

        #[test]
        fn the_drag_of_the_mark_goes_on_away_from_the_pane_and_a_release_there_ends_it()
         {
            let snapshot = history(40, &[]);
            let mut state = TerminalViewState::new();
            let outside = Point::new(500.0, -40.0);
            send(&mut state, &snapshot, Point::new(STRIP_X, 100.0), press());

            let dragged = move_to(&mut state, &snapshot, outside);
            let released = send(&mut state, &snapshot, outside, release());
            let after =
                move_to(&mut state, &snapshot, Point::new(STRIP_X, 60.0));

            // Above the pane the mark is at the top of its way: the view
            // goes to the top of the history and one step past it.
            assert!(offsets(&dragged).is_empty(), "got {dragged:?}");
            assert!(asks_past_the_top(&dragged), "got {dragged:?}");
            assert_eq!(state.scroll_mark_grab(), None);
            assert!(
                released
                    .iter()
                    .any(|event| matches!(event, crate::Event::Redraw { .. })),
                "got {released:?}"
            );
            assert!(offsets(&after).is_empty(), "got {after:?}");
        }

        #[test]
        fn with_no_history_to_scroll_a_press_at_the_right_edge_means_the_last_column()
         {
            let snapshot = history(3, &[]);
            let mut state = TerminalViewState::new();
            let at_the_right_edge = Point::new(STRIP_X, 8.0);

            move_to(&mut state, &snapshot, at_the_right_edge);
            let pressed =
                send(&mut state, &snapshot, at_the_right_edge, press());

            assert_eq!(state.mouse_position_on_grid.column, Column(35));
            assert!(
                pressed.iter().any(|event| matches!(
                    event,
                    crate::Event::SelectStart { .. }
                )),
                "got {pressed:?}"
            );
            assert!(state.is_dragged);
        }

        #[test]
        fn the_pointer_on_the_strip_hovers_no_block_and_widens_the_mark() {
            let mut snapshot = history(40, &[]);
            snapshot.blocks =
                snapshot_with_block(LS, BlockKind::Command, 0, 8).blocks;
            let mut state = TerminalViewState::new();
            move_to(&mut state, &snapshot, Point::new(100.0, 40.0));
            assert_eq!(state.hovered_block_key, Some(LS));
            assert!(!state.is_scroll_mark_active());

            let on_strip =
                move_to(&mut state, &snapshot, Point::new(STRIP_X, 40.0));

            assert_eq!(state.hovered_block_key, None);
            assert!(state.is_scroll_mark_active());
            assert!(
                on_strip.iter().any(|event| matches!(
                    event,
                    crate::Event::BlockHovered {
                        block_key: None,
                        ..
                    }
                )),
                "got {on_strip:?}"
            );
            assert!(
                on_strip
                    .iter()
                    .any(|event| matches!(event, crate::Event::Redraw { .. })),
                "got {on_strip:?}"
            );

            move_to(&mut state, &snapshot, Point::new(100.0, 40.0));
            assert!(!state.is_scroll_mark_active());
            assert_eq!(state.hovered_block_key, Some(LS));
        }

        /// The strip belongs to OTTY: the wheel over it scrolls the
        /// history, also while a program reads the mouse.
        #[test]
        fn the_wheel_over_the_strip_scrolls_the_history() {
            let snapshot = history(40, &[NamedPrivateMode::ReportMouseClicks]);
            let mut state = TerminalViewState::new();
            let wheel = mouse::Event::WheelScrolled {
                delta: ScrollDelta::Lines { x: 0.0, y: 2.0 },
            };

            let published =
                send(&mut state, &snapshot, Point::new(STRIP_X, 40.0), wheel);

            assert!(
                published.iter().any(|event| matches!(
                    event,
                    crate::Event::Scroll { delta: 2, .. }
                )),
                "got {published:?}"
            );
            assert!(
                !touches_text_blocks_or_programs(&published),
                "got {published:?}"
            );
        }
    }

    /// The content of the pane moves while the pointer rests: the history
    /// scrolls, new output arrives, the pane is resized or a block folds.
    mod content_moved_tests {
        use otty_libterm::surface::Scroll;

        use super::*;
        use crate::bindings;

        /// A grid of 36 columns and 4 lines of 8 by 16 px cells.
        const GRID: TerminalSize = TerminalSize {
            cell_width: 8,
            cell_height: 16,
            cols: 36,
            rows: 4,
        };

        /// A point over the second line of the pane, whose text starts at
        /// the origin.
        const OVER_THE_SECOND_LINE: Point = Point::new(20.0, 24.0);

        /// A block that holds `line_count` lines from `start_line`.
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
                ..BlockSnapshot::default()
            }
        }

        /// A screen of `GRID` with `blocks` and nothing scrolled.
        fn screen_with(blocks: Vec<BlockSnapshot>) -> SnapshotOwned {
            let mut snapshot = SnapshotOwned::default();
            snapshot.blocks = blocks;

            snapshot
        }

        /// A screen of `GRID` with 4 lines of history above it, scrolled
        /// `lines_up` lines up: `older` and `ls` are in the history, `pwd`
        /// and the prompt on the screen.
        fn history_scrolled_up(lines_up: i32) -> SnapshotOwned {
            let mut surface = Surface::new(SurfaceConfig::default(), &GRID);
            for _ in 0..7 {
                surface.line_feed();
            }
            surface.scroll_display(Scroll::Delta(lines_up));

            let mut snapshot = surface.snapshot_owned();
            snapshot.blocks = vec![
                block(OLDER, BlockKind::Command, -4, 2),
                block(LS, BlockKind::Command, -2, 2),
                block(PWD, BlockKind::Command, 0, 3),
                block(PROMPT, BlockKind::Prompt, 3, 1),
            ];

            snapshot
        }

        /// Move the pointer to `position` over a pane that shows
        /// `snapshot`.
        fn rest_pointer(
            state: &mut TerminalViewState,
            snapshot: &SnapshotOwned,
            position: Point,
        ) {
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_cursor_moved(
                state,
                &engine_showing(snapshot.clone(), GRID),
                &position,
                Point::ORIGIN,
                &mut |_| {},
            );
        }

        /// The pane shows `snapshot` now, and the pointer has not moved.
        /// Return what the terminal published.
        fn show(
            state: &mut TerminalViewState,
            snapshot: &SnapshotOwned,
        ) -> Vec<crate::Event> {
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_content_moved(
                state,
                &engine_showing(snapshot.clone(), GRID),
                Point::ORIGIN,
                None,
                &mut publish,
            );

            commands
        }

        fn leave_pane(state: &mut TerminalViewState) {
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_pointer_left(state, &mut |_| {});
        }

        /// Whether `commands` report `block_key` as hovered and ask for one
        /// redraw, and nothing else.
        fn reports_hovered(
            commands: &[crate::Event],
            block_key: BlockKey,
        ) -> bool {
            matches!(
                commands,
                [
                    crate::Event::BlockHovered {
                        id: TEST_ID,
                        block_key: Some(hovered),
                    },
                    crate::Event::Redraw { id: TEST_ID },
                ] if *hovered == block_key
            )
        }

        #[test]
        fn the_block_the_history_scrolls_under_a_resting_pointer_is_hovered() {
            let mut state = TerminalViewState::new();
            rest_pointer(
                &mut state,
                &history_scrolled_up(0),
                OVER_THE_SECOND_LINE,
            );
            assert_eq!(state.hovered_block_key, Some(PWD));

            let commands = show(&mut state, &history_scrolled_up(2));

            assert_eq!(state.hovered_block_key, Some(LS));
            assert_eq!(
                state.mouse_position_on_grid,
                TerminalGridPoint {
                    line: Line(-1),
                    column: Column(2),
                }
            );
            assert!(reports_hovered(&commands, LS), "got {commands:?}");
        }

        #[test]
        fn the_block_new_output_pushes_under_a_resting_pointer_is_hovered() {
            let mut state = TerminalViewState::new();
            let before = screen_with(vec![
                block(LS, BlockKind::Command, 0, 2),
                block(CARGO, BlockKind::Command, 2, 2),
            ]);
            rest_pointer(&mut state, &before, OVER_THE_SECOND_LINE);
            assert_eq!(state.hovered_block_key, Some(LS));
            // `cargo` printed two more lines and pushed `ls` up.
            let after = screen_with(vec![
                block(LS, BlockKind::Command, -2, 2),
                block(CARGO, BlockKind::Command, 0, 4),
            ]);

            let commands = show(&mut state, &after);

            assert_eq!(state.hovered_block_key, Some(CARGO));
            assert!(reports_hovered(&commands, CARGO), "got {commands:?}");
        }

        #[test]
        fn content_that_keeps_the_same_block_under_the_pointer_asks_for_nothing()
         {
            let mut state = TerminalViewState::new();
            let before = screen_with(vec![
                block(CARGO, BlockKind::Command, 0, 3),
                block(PROMPT, BlockKind::Prompt, 3, 1),
            ]);
            rest_pointer(&mut state, &before, OVER_THE_SECOND_LINE);
            // One more line of output: `cargo` moved up, but its second
            // line is still under the pointer.
            let after = screen_with(vec![
                block(CARGO, BlockKind::Command, -1, 4),
                block(PROMPT, BlockKind::Prompt, 3, 1),
            ]);

            let commands = show(&mut state, &after);

            assert_eq!(state.hovered_block_key, Some(CARGO));
            assert!(commands.is_empty(), "got {commands:?}");
        }

        #[test]
        fn content_that_moves_while_the_pointer_is_away_from_the_pane_hovers_nothing()
         {
            let mut state = TerminalViewState::new();
            rest_pointer(
                &mut state,
                &history_scrolled_up(0),
                OVER_THE_SECOND_LINE,
            );
            leave_pane(&mut state);

            let commands = show(&mut state, &history_scrolled_up(2));

            assert_eq!(state.hovered_block_key, None);
            assert!(commands.is_empty(), "got {commands:?}");
        }

        /// A full-screen program has no blocks to hover, as when the pointer
        /// moves over it.
        #[test]
        fn a_full_screen_program_under_a_resting_pointer_hovers_nothing() {
            let mut state = TerminalViewState::new();
            rest_pointer(
                &mut state,
                &screen_with(vec![]),
                OVER_THE_SECOND_LINE,
            );
            let mut vim = snapshot_with_modes(&[
                NamedPrivateMode::SwapScreenAndSetRestoreCursor,
            ]);
            vim.blocks = vec![block(VIM, BlockKind::Command, 0, 4)];

            let commands = show(&mut state, &vim);

            assert_eq!(state.hovered_block_key, None);
            assert!(commands.is_empty(), "got {commands:?}");
        }

        /// While a text selection is dragged, the block under the pointer
        /// stays the one it was, as when the pointer moves.
        #[test]
        fn content_that_moves_during_a_drag_keeps_the_hovered_block() {
            let mut state = TerminalViewState::new();
            rest_pointer(
                &mut state,
                &history_scrolled_up(0),
                OVER_THE_SECOND_LINE,
            );
            state.is_dragged = true;

            let commands = show(&mut state, &history_scrolled_up(2));

            assert_eq!(state.hovered_block_key, Some(PWD));
            assert!(commands.is_empty(), "got {commands:?}");
        }
    }

    mod handle_button_released_tests {
        use iced::keyboard::Modifiers;

        use super::*;

        #[test]
        fn mouse_mode_activated() {
            let mut state = TerminalViewState::new();
            let bindings = BindingsLayout::<()>::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let _modifiers = Modifiers::empty();

            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_button_released(
                &mut state,
                &snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]),
                &bindings,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                crate::Event::MouseReport {
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
            let bindings = BindingsLayout::<()>::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_button_released(
                &mut state,
                &snapshot_with_hyperlink("https://example.com"),
                &bindings,
                &mut publish,
            );

            assert!(commands.iter().any(|event| matches!(
                event,
                crate::Event::OpenLink { uri, .. } if uri == "https://example.com"
            )));
        }

        #[test]
        fn selects_block_on_simple_click() {
            let mut state = TerminalViewState::new();
            state.hovered_block_key = Some(BLOCK_1);
            let bindings = BindingsLayout::<()>::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            let status = input_manager.handle_button_released(
                &mut state,
                &default_snapshot(),
                &bindings,
                &mut publish,
            );

            assert_eq!(state.selected_block_key, Some(BLOCK_1));
            assert!(commands.iter().any(|event| matches!(
                event,
                crate::Event::BlockSelected {
                    id: TEST_ID,
                    block_key: BLOCK_1
                }
            )));
            assert!(
                commands.iter().any(|event| {
                    matches!(event, crate::Event::Redraw { .. })
                })
            );
            assert_eq!(status, iced::event::Status::Captured);
        }

        #[test]
        fn does_not_select_block_on_click_in_command_only_mode() {
            let mut state = TerminalViewState::new();
            state.hovered_block_key = Some(BLOCK_1);
            let bindings = BindingsLayout::<()>::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::CommandOnly,
            );

            let status = input_manager.handle_button_released(
                &mut state,
                &default_snapshot(),
                &bindings,
                &mut publish,
            );

            assert!(state.selected_block_key.is_none());
            assert!(commands.is_empty());
            assert_eq!(status, iced::event::Status::Ignored);
        }

        #[test]
        fn double_click_clears_selection() {
            let mut state = TerminalViewState::new();
            state.hovered_block_key = Some(BLOCK_1);
            state.selected_block_key = Some(BLOCK_1);
            let first_click = mouse::Click::new(
                Point { x: 0.0, y: 0.0 },
                mouse::Button::Left,
                None,
            );
            let deadline = std::time::Instant::now()
                + std::time::Duration::from_millis(300);
            state.last_click = Some(loop {
                let click = mouse::Click::new(
                    Point { x: 0.0, y: 0.0 },
                    mouse::Button::Left,
                    Some(first_click),
                );
                if click.kind() == mouse::click::Kind::Double {
                    break click;
                }

                assert!(
                    std::time::Instant::now() < deadline,
                    "test clock did not advance within the double-click window"
                );
                std::thread::yield_now();
            });
            let bindings = BindingsLayout::<()>::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            let status = input_manager.handle_button_released(
                &mut state,
                &default_snapshot(),
                &bindings,
                &mut publish,
            );

            assert!(state.selected_block_key.is_none());
            assert!(
                commands.iter().any(|event| {
                    matches!(event, crate::Event::Redraw { .. })
                })
            );
            assert_eq!(status, iced::event::Status::Captured);
        }

        #[test]
        fn selection_click_is_ignored_when_text_was_selected() {
            let mut state = TerminalViewState::new();
            state.hovered_block_key = Some(BLOCK_1);
            state.selection_in_progress = true;
            let bindings = BindingsLayout::<()>::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            let status = input_manager.handle_button_released(
                &mut state,
                &default_snapshot(),
                &bindings,
                &mut publish,
            );

            assert!(state.selected_block_key.is_none());
            assert!(commands.is_empty());
            assert_eq!(status, iced::event::Status::Ignored);
        }
    }

    mod handle_wheel_scrolled_tests {
        use iced::keyboard::Modifiers;

        use super::*;
        use crate::bindings;

        /// Turn the wheel by `delta` over a program in the state of
        /// `snapshot`, and return what the terminal published.
        fn turn_wheel(
            state: &mut TerminalViewState,
            snapshot: &SnapshotOwned,
            delta: ScrollDelta,
        ) -> (iced::event::Status, Vec<crate::Event>) {
            let font = TermFont::new(FontSettings::default());
            let mut commands = Vec::new();
            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            let status = {
                let mut publish = |event| commands.push(event);
                input_manager.handle_wheel_scrolled(
                    state,
                    snapshot,
                    &delta,
                    &font.measure,
                    &mut publish,
                )
            };

            (status, commands)
        }

        #[test]
        fn scroll_with_lines_downward() {
            let mut state = TerminalViewState::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let font = TermFont::new(FontSettings::default());

            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_wheel_scrolled(
                &mut state,
                &default_snapshot(),
                &ScrollDelta::Lines { y: 3.0, x: 0.0 }, // Scroll down 3 lines
                &font.measure,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                crate::Event::Scroll {
                    id: TEST_ID,
                    delta: 3
                }
            ));
        }

        #[test]
        fn scroll_with_lines_upward() {
            let mut state = TerminalViewState::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let font = TermFont::new(FontSettings::default());

            let bindings = bindings::BindingsLayout::<()>::new();
            let input_manager = InputManager::new(
                TEST_ID,
                &bindings,
                BlockSelectionMode::PrimaryClick,
            );

            input_manager.handle_wheel_scrolled(
                &mut state,
                &default_snapshot(),
                &ScrollDelta::Lines { y: -2.0, x: 0.0 },
                &font.measure,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                crate::Event::Scroll {
                    id: TEST_ID,
                    delta: -2
                }
            ));
        }

        #[test]
        fn wheel_up_sends_one_report_per_line_to_a_program_that_asked() {
            let mut state = TerminalViewState::new();
            let snapshot =
                snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]);

            let (status, commands) = turn_wheel(
                &mut state,
                &snapshot,
                ScrollDelta::Lines { y: 3.0, x: 0.0 },
            );

            assert_eq!(status, iced::event::Status::Captured);
            assert_eq!(commands.len(), 3, "got {commands:?}");
            for command in &commands {
                assert!(
                    matches!(
                        command,
                        crate::Event::MouseReport {
                            id: TEST_ID,
                            button: MouseButton::ScrollUp,
                            pressed: true,
                            ..
                        }
                    ),
                    "expected a wheel up report, got {command:?}"
                );
            }
        }

        #[test]
        fn wheel_down_sends_one_report_per_line_to_a_program_that_asked() {
            let mut state = TerminalViewState::new();
            let snapshot =
                snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]);

            let (status, commands) = turn_wheel(
                &mut state,
                &snapshot,
                ScrollDelta::Lines { y: -2.0, x: 0.0 },
            );

            assert_eq!(status, iced::event::Status::Captured);
            assert_eq!(commands.len(), 2, "got {commands:?}");
            for command in &commands {
                assert!(
                    matches!(
                        command,
                        crate::Event::MouseReport {
                            id: TEST_ID,
                            button: MouseButton::ScrollDown,
                            pressed: true,
                            ..
                        }
                    ),
                    "expected a wheel down report, got {command:?}"
                );
            }
        }

        #[test]
        fn a_wheel_report_carries_the_cell_under_the_pointer_and_the_modifiers()
        {
            let mut state = TerminalViewState::new();
            state.keyboard_modifiers = Modifiers::CTRL | Modifiers::SHIFT;
            state.mouse_position_on_grid = TerminalGridPoint {
                line: Line(4),
                column: Column(9),
            };
            let snapshot =
                snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]);

            let (_, commands) = turn_wheel(
                &mut state,
                &snapshot,
                ScrollDelta::Lines { y: 1.0, x: 0.0 },
            );

            assert!(
                matches!(
                    commands.as_slice(),
                    [crate::Event::MouseReport {
                        modifiers,
                        point: TerminalGridPoint {
                            line: Line(4),
                            column: Column(9),
                        },
                        ..
                    }] if *modifiers == Modifiers::CTRL | Modifiers::SHIFT
                ),
                "the report must carry the cell and the held keys, got \
                 {commands:?}"
            );
        }

        #[test]
        fn a_wheel_step_shorter_than_a_line_sends_no_report() {
            let mut state = TerminalViewState::new();
            let snapshot =
                snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]);

            let (status, commands) = turn_wheel(
                &mut state,
                &snapshot,
                ScrollDelta::Pixels { y: 1.0, x: 0.0 },
            );

            assert_eq!(status, iced::event::Status::Ignored);
            assert!(commands.is_empty(), "got {commands:?}");
        }

        #[test]
        fn the_wheel_scrolls_the_history_when_the_program_did_not_ask() {
            let mut state = TerminalViewState::new();

            let (status, commands) = turn_wheel(
                &mut state,
                &default_snapshot(),
                ScrollDelta::Lines { y: 3.0, x: 0.0 },
            );

            assert_eq!(status, iced::event::Status::Captured);
            assert!(
                matches!(
                    commands.as_slice(),
                    [crate::Event::Scroll {
                        id: TEST_ID,
                        delta: 3,
                    }]
                ),
                "got {commands:?}"
            );
        }
    }
}
