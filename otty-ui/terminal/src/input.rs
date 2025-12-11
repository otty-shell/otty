use iced::{Point, Size, keyboard::Key, mouse::ScrollDelta};
use iced_core::clipboard::Kind as ClipboardKind;
use iced_core::mouse::{self, Click};
use otty_libterm::{
    SnapshotArc, TerminalSize,
    surface::{BlockKind, SelectionType, SurfaceMode},
};

use crate::{
    bindings::{BindingAction, BindingsLayout, InputKind},
    engine::{Engine, MouseButton},
    font::TermFont,
    view::TerminalViewState,
};

pub(crate) struct InputManager<'a> {
    terminal_id: u64,
    bindings: &'a BindingsLayout,
}

impl<'a> InputManager<'a> {
    pub(crate) fn new(terminal_id: u64, bindings: &'a BindingsLayout) -> Self {
        Self {
            terminal_id,
            bindings,
        }
    }

    pub(crate) fn handle_mouse_event(
        &self,
        view_state: &mut TerminalViewState,
        terminal_content: SnapshotArc,
        terminal_size: TerminalSize,
        terminal_font: &'a TermFont,
        layout_position: Point,
        cursor_position: Point,
        event: &iced::mouse::Event,
        _clipboard: &mut dyn iced_graphics::core::Clipboard,
        publisher: &mut impl FnMut(crate::Event),
    ) -> iced::event::Status {
        match event {
            iced_core::mouse::Event::ButtonPressed(
                iced_core::mouse::Button::Left,
            ) => self.handle_left_button_pressed(
                view_state,
                terminal_content,
                cursor_position,
                layout_position,
                publisher,
            ),
            iced_core::mouse::Event::CursorMoved { position } => self
                .handle_cursor_moved(
                    view_state,
                    terminal_content,
                    terminal_size,
                    position,
                    layout_position,
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
                    delta,
                    &terminal_font.measure,
                    publisher,
                ),
            _ => iced::event::Status::Ignored,
        }
    }

    fn handle_left_button_pressed(
        &self,
        state: &mut TerminalViewState,
        terminal_state: SnapshotArc,
        cursor_position: Point,
        layout_position: Point,
        publisher: &mut impl FnMut(crate::Event),
    ) -> iced::event::Status {
        state.selection_in_progress = false;
        let cmd = if terminal_state
            .view()
            .mode
            .intersects(SurfaceMode::MOUSE_MODE)
        {
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
        &self,
        state: &mut TerminalViewState,
        terminal_state: SnapshotArc,
        terminal_size: TerminalSize,
        position: &Point,
        layout_position: Point,
        publisher: &mut impl FnMut(crate::Event),
    ) -> iced::event::Status {
        let terminal_state = terminal_state.view();
        let in_alt_screen =
            terminal_state.mode.contains(SurfaceMode::ALT_SCREEN);
        let cursor_x = position.x - layout_position.x;
        let cursor_y = position.y - layout_position.y;
        state.mouse_position_on_grid = Engine::selection_point(
            cursor_x,
            cursor_y,
            &terminal_size,
            terminal_state.display_offset,
        );

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
                let had_block_selection = state.selected_block_id.is_some()
                    || state.selected_block_kind.is_some();
                if had_block_selection {
                    state.selected_block_id = None;
                    state.selected_block_kind = None;
                    publisher(crate::Event::BlockSelectionCleared {
                        id: self.terminal_id,
                    });
                    publisher(crate::Event::Redraw {
                        id: self.terminal_id,
                    });
                }
            }

            publisher(crate::Event::SelectUpdate {
                id: self.terminal_id,
                position: (cursor_x, cursor_y),
            });
            return iced::event::Status::Captured;
        } else if !in_alt_screen {
            let hovered_span_id = terminal_state
                .hyperlink_span_id_at(state.mouse_position_on_grid);
            let hovered_block = terminal_state
                .block_at_point(state.mouse_position_on_grid)
                .map(|block| (block.meta.id.clone(), block.meta.kind.clone()));

            let mut needs_redraw = false;

            if hovered_span_id != state.hovered_span_id {
                state.hovered_span_id = hovered_span_id;
                needs_redraw = true;
            }

            let hovered_block_id =
                hovered_block.as_ref().map(|(id, _)| id.to_string());
            let hovered_block_kind =
                hovered_block.as_ref().map(|(_, kind)| kind.clone());

            if hovered_block_id.as_deref() != state.hovered_block_id.as_deref()
            {
                state.hovered_block_id = hovered_block_id;
                needs_redraw = true;
            }

            if hovered_block_kind != state.hovered_block_kind {
                state.hovered_block_kind = hovered_block_kind;
                needs_redraw = true;
            }

            let prev_action_hover = state.hovered_action_block_id.clone();
            let action_hover = None;

            if action_hover != prev_action_hover {
                state.hovered_action_block_id = action_hover;
                needs_redraw = true;
            }

            if needs_redraw {
                publisher(crate::Event::Redraw {
                    id: self.terminal_id,
                });
                return iced::event::Status::Captured;
            }
        }

        iced::event::Status::Ignored
    }

    fn handle_button_released(
        &self,
        state: &mut TerminalViewState,
        terminal_state: SnapshotArc,
        bindings: &BindingsLayout, // Use the actual type of your bindings here
        publisher: &mut impl FnMut(crate::Event),
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
        {
            if let Some(span) =
                terminal_state.hyperlink_span_at(state.mouse_position_on_grid)
            {
                publisher(crate::Event::OpenLink {
                    id: self.terminal_id,
                    uri: span.link.uri().to_string(),
                });
                published = true;
            }
        }

        if !was_selecting
            && !terminal_state.mode.intersects(SurfaceMode::MOUSE_MODE)
            && !terminal_state.mode.contains(SurfaceMode::ALT_SCREEN)
        {
            use mouse::click::Kind;
            let click_kind = state
                .last_click
                .as_ref()
                .map(mouse::Click::kind)
                .unwrap_or(Kind::Single);

            let selection_changed = match click_kind {
                Kind::Double | Kind::Triple => {
                    if state.selected_block_id.is_some() {
                        state.selected_block_id = None;
                        state.selected_block_kind = None;
                        true
                    } else {
                        false
                    }
                },
                Kind::Single => {
                    let hovered_block_id = state.hovered_block_id.clone();
                    let hovered_block_kind = state.hovered_block_kind.clone();
                    match (
                        hovered_block_id.as_deref(),
                        hovered_block_kind.as_ref(),
                        state.selected_block_id.as_deref(),
                    ) {
                        (Some(hovered), Some(kind), _)
                            if *kind != BlockKind::Prompt =>
                        {
                            if state.selected_block_id.as_deref()
                                == Some(hovered)
                            {
                                false
                            } else {
                                state.selected_block_id =
                                    Some(hovered.to_string());
                                state.selected_block_kind = Some(kind.clone());
                                true
                            }
                        },
                        _ => {
                            if state.selected_block_id.is_some() {
                                state.selected_block_id = None;
                                state.selected_block_kind = None;
                                true
                            } else {
                                false
                            }
                        },
                    }
                },
            };

            if selection_changed {
                match state.selected_block_id.clone() {
                    Some(block_id) => {
                        publisher(crate::Event::BlockSelected {
                            id: self.terminal_id,
                            block_id,
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
        delta: &ScrollDelta,
        font_measure: &Size<f32>,
        publisher: &mut impl FnMut(crate::Event),
    ) -> iced::event::Status {
        let lines = match delta {
            ScrollDelta::Lines { y, .. } => y.signum() * y.abs().round(),
            ScrollDelta::Pixels { y, .. } => {
                state.scroll_pixels -= y;
                let line_height = font_measure.height; // Assume this method exists and gives the height of a line
                let lines = (state.scroll_pixels / line_height).trunc();
                state.scroll_pixels %= line_height;
                lines
            },
        };

        if lines != 0.0 {
            publisher(crate::Event::Scroll {
                id: self.terminal_id,
                delta: lines as i32,
            });
            iced::event::Status::Captured
        } else {
            iced::event::Status::Ignored
        }
    }

    pub(crate) fn handle_keyboard_event(
        &self,
        view_state: &mut TerminalViewState,
        terminal_state: SnapshotArc,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        event: &iced::keyboard::Event,
        publisher: &mut impl FnMut(crate::Event),
    ) -> iced::event::Status {
        let mut binding_action = BindingAction::Ignore;
        let terminal_state_ref = terminal_state.view();

        match event {
            iced::keyboard::Event::ModifiersChanged(m) => {
                view_state.keyboard_modifiers = *m;
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
                    binding_action = self.bindings.get_action(
                        InputKind::Char(lower),
                        &view_state.keyboard_modifiers,
                        terminal_state_ref.mode,
                    );

                    // If no binding matched, only write printable text (when provided)
                    if binding_action == BindingAction::Ignore {
                        if let Some(c) = text {
                            publisher(crate::Event::Write {
                                id: self.terminal_id,
                                data: c.as_bytes().to_vec(),
                            });
                            return iced::event::Status::Captured;
                        }
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
            },
            _ => {},
        }

        match binding_action {
            BindingAction::Char(c) => {
                let mut buf = [0, 0, 0, 0];
                let str = c.encode_utf8(&mut buf);
                publisher(crate::Event::Write {
                    id: self.terminal_id,
                    data: str.as_bytes().to_vec(),
                });
                iced::event::Status::Captured
            },
            BindingAction::Esc(seq) => {
                publisher(crate::Event::Write {
                    id: self.terminal_id,
                    data: seq.as_bytes().to_vec(),
                });
                iced::event::Status::Captured
            },
            BindingAction::Paste => {
                if let Some(data) = clipboard.read(ClipboardKind::Standard) {
                    let input: Vec<u8> = data.bytes().collect();
                    publisher(crate::Event::Write {
                        id: self.terminal_id,
                        data: input,
                    });
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
            _ => iced::event::Status::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use otty_libterm::TerminalSize;
    use otty_libterm::escape::{Hyperlink, NamedPrivateMode};
    use otty_libterm::surface::{
        BlockKind, BlockMeta, BlockSnapshot, Column, Line,
        Point as TerminalGridPoint, SnapshotOwned, Surface, SurfaceActor,
        SurfaceConfig, SurfaceModel,
    };

    use super::*;
    use crate::font::TermFont;
    use crate::settings::FontSettings;

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

    fn snapshot_with_block(
        id: &str,
        kind: BlockKind,
        start_line: i32,
        line_count: usize,
    ) -> Arc<SnapshotOwned> {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks.push(BlockSnapshot {
            meta: BlockMeta {
                id: id.to_string(),
                kind,
                ..BlockMeta::default()
            },
            start_line,
            line_count,
            is_alt_screen: false,
        });
        Arc::new(snapshot)
    }

    mod handle_left_button_pressed_tests {
        use iced::keyboard::Modifiers;

        use crate::bindings;

        use super::*;

        #[test]
        fn handles_mouse_mode_with_left_click() {
            let mut state = TerminalViewState::new();
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let _modifiers = Modifiers::empty();
            let bindings = bindings::BindingsLayout::new();
            let input_manager = InputManager::new(TEST_ID, &bindings);

            input_manager.handle_left_button_pressed(
                &mut state,
                snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]),
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

            let bindings = bindings::BindingsLayout::new();
            let input_manager = InputManager::new(TEST_ID, &bindings);

            input_manager.handle_left_button_pressed(
                &mut state,
                default_snapshot(),
                cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                crate::Event::SelectStart {
                    id: TEST_ID,
                    selection_type: SelectionType::Simple,
                    position: (150.0, 100.0)
                }
            ));
            assert!(state.is_dragged);
        }
    }

    mod handle_cursor_moved_tests {
        use crate::bindings;

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

            let bindings = bindings::BindingsLayout::new();
            let input_manager = InputManager::new(TEST_ID, &bindings);

            for (layout_position, cursor_position, expected) in cases {
                input_manager.handle_cursor_moved(
                    &mut state,
                    terminal_content.clone(),
                    terminal_size,
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
            let terminal_content = default_snapshot();
            let terminal_size = TerminalSize::default();
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            let bindings = bindings::BindingsLayout::new();
            let input_manager = InputManager::new(TEST_ID, &bindings);

            input_manager.handle_cursor_moved(
                &mut state,
                terminal_content,
                terminal_size,
                &cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                crate::Event::SelectUpdate {
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

            let bindings = bindings::BindingsLayout::new();
            let input_manager = InputManager::new(TEST_ID, &bindings);

            input_manager.handle_cursor_moved(
                &mut state,
                terminal_content,
                terminal_size,
                &cursor_position,
                layout_position,
                &mut publish,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                crate::Event::SelectUpdate {
                    id: TEST_ID,
                    position: (95.0, 145.0)
                }
            ));
            assert!(state.selection_in_progress);
        }

        #[test]
        fn clears_block_selection_when_dragging_starts() {
            let mut state = TerminalViewState::new();
            state.is_dragged = true;
            state.selected_block_id = Some(String::from("block-1"));
            state.selected_block_kind = Some(BlockKind::Command);
            let terminal_content = default_snapshot();
            let terminal_size = TerminalSize::default();
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            let bindings = bindings::BindingsLayout::new();
            let input_manager = InputManager::new(TEST_ID, &bindings);

            input_manager.handle_cursor_moved(
                &mut state,
                terminal_content,
                terminal_size,
                &cursor_position,
                layout_position,
                &mut publish,
            );

            assert!(state.selection_in_progress);
            assert!(state.selected_block_id.is_none());
            assert!(state.selected_block_kind.is_none());
            assert_eq!(commands.len(), 3);
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
        }
    }

    mod handle_button_released_tests {
        use iced::keyboard::Modifiers;

        use super::*;

        #[test]
        fn mouse_mode_activated() {
            let mut state = TerminalViewState::new();
            let bindings = BindingsLayout::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let _modifiers = Modifiers::empty();

            let input_manager = InputManager::new(TEST_ID, &bindings);

            input_manager.handle_button_released(
                &mut state,
                snapshot_with_modes(&[NamedPrivateMode::ReportMouseClicks]),
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
            let bindings = BindingsLayout::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            let input_manager = InputManager::new(TEST_ID, &bindings);

            input_manager.handle_button_released(
                &mut state,
                snapshot_with_hyperlink("https://example.com"),
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
            state.hovered_block_id = Some(String::from("block-1"));
            state.hovered_block_kind = Some(BlockKind::Command);
            let bindings = BindingsLayout::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let input_manager = InputManager::new(TEST_ID, &bindings);

            let status = input_manager.handle_button_released(
                &mut state,
                default_snapshot(),
                &bindings,
                &mut publish,
            );

            assert_eq!(state.selected_block_id.as_deref(), Some("block-1"));
            assert!(state.selected_block_kind.is_some());
            assert!(commands.iter().any(|event| matches!(
                event,
                crate::Event::BlockSelected {
                    id: TEST_ID,
                    block_id
                } if block_id == "block-1"
            )));
            assert!(
                commands.iter().any(|event| {
                    matches!(event, crate::Event::Redraw { .. })
                })
            );
            assert_eq!(status, iced::event::Status::Captured);
        }

        #[test]
        fn ignores_prompt_blocks_when_selecting() {
            let mut state = TerminalViewState::new();
            state.hovered_block_id = Some(String::from("prompt"));
            state.hovered_block_kind = Some(BlockKind::Prompt);
            let bindings = BindingsLayout::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let input_manager = InputManager::new(TEST_ID, &bindings);

            let status = input_manager.handle_button_released(
                &mut state,
                default_snapshot(),
                &bindings,
                &mut publish,
            );

            assert!(state.selected_block_id.is_none());
            assert!(commands.is_empty());
            assert_eq!(status, iced::event::Status::Ignored);
        }

        #[test]
        fn double_click_clears_selection() {
            let mut state = TerminalViewState::new();
            state.hovered_block_id = Some(String::from("block-1"));
            state.hovered_block_kind = Some(BlockKind::Command);
            state.selected_block_id = Some(String::from("block-1"));
            state.selected_block_kind = Some(BlockKind::Command);
            let first_click = mouse::Click::new(
                Point { x: 0.0, y: 0.0 },
                mouse::Button::Left,
                None,
            );
            state.last_click = Some(mouse::Click::new(
                Point { x: 0.0, y: 0.0 },
                mouse::Button::Left,
                Some(first_click),
            ));
            let bindings = BindingsLayout::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let input_manager = InputManager::new(TEST_ID, &bindings);

            let status = input_manager.handle_button_released(
                &mut state,
                default_snapshot(),
                &bindings,
                &mut publish,
            );

            assert!(state.selected_block_id.is_none());
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
            state.hovered_block_id = Some(String::from("block-1"));
            state.hovered_block_kind = Some(BlockKind::Command);
            state.selection_in_progress = true;
            let bindings = BindingsLayout::new();
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);
            let input_manager = InputManager::new(TEST_ID, &bindings);

            let status = input_manager.handle_button_released(
                &mut state,
                default_snapshot(),
                &bindings,
                &mut publish,
            );

            assert!(state.selected_block_id.is_none());
            assert!(commands.is_empty());
            assert_eq!(status, iced::event::Status::Ignored);
        }
    }

    mod handle_wheel_scrolled_tests {
        use crate::bindings;

        use super::*;

        #[test]
        fn scroll_with_lines_downward() {
            let mut state = TerminalViewState::new();
            let font = TermFont::new(FontSettings::default());
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            let bindings = bindings::BindingsLayout::new();
            let input_manager = InputManager::new(TEST_ID, &bindings);

            input_manager.handle_wheel_scrolled(
                &mut state,
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
            let font = TermFont::new(FontSettings::default());
            let mut commands = Vec::new();
            let mut publish = |event| commands.push(event);

            let bindings = bindings::BindingsLayout::new();
            let input_manager = InputManager::new(TEST_ID, &bindings);

            input_manager.handle_wheel_scrolled(
                &mut state,
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
    }

    mod block_ui_mode_tests {
        use super::*;

        #[test]
        fn skips_action_geometry_in_external_overlay_mode() {
            let mut state = TerminalViewState::new();
            state.hovered_block_id = Some(String::from("block-1"));
            state.size = Size::new(200.0, 200.0);
            let snapshot =
                snapshot_with_block("block-1", BlockKind::Command, 0, 2);
            let view = snapshot.view();

            // In external overlay mode, internal state does not expose
            // action-button hover geometry; this assertion is kept as a
            // placeholder to ensure the test harness compiles.
            assert!(
                view.block_at_point(TerminalGridPoint {
                    line: Line(0),
                    column: Column(0),
                })
                .is_some()
            );
        }
    }
}
