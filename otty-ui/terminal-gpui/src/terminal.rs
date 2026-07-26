use std::collections::VecDeque;
use std::io;
use std::ops::Range;
use std::sync::Arc;

use cursor_icon::CursorIcon;
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle as PointerCursorStyle,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render, ScrollDelta,
    ScrollWheelEvent, SharedString, Styled, Subscription, UTF16Selection,
    Window, div, point, px, size,
};
use otty_libterm::mouse::{
    TerminalMouseButton, TerminalMouseModifiers, encode_mouse_report,
};
use otty_libterm::surface::{
    BlockSnapshot, Column, Line, Point, Scroll, SelectionType, Side,
    SnapshotOwned, SurfaceMode,
};
use otty_libterm::{
    ChannelSendError, SnapshotArc, TerminalEvent as CoreEvent, TerminalHandle,
    TerminalRequest, TerminalSize,
};

use crate::actions::{
    ClearSelection, Copy, Paste, ScrollPageDown, ScrollPageUp, ScrollToBottom,
    ScrollToTop, SelectAll,
};
use crate::config::ConfigChange;
use crate::input::{byte_range_to_utf16, utf16_range_to_byte};
use crate::terminal_element::TerminalElement;
use crate::{
    BackendError, BackendGeneration, BackendSession, BackendState,
    BindingAction, BlockId, BlockTextPart, CellMetrics, ContextMenuPolicy,
    CopySource, HitTarget, LinkPolicy, OperationError, TerminalAppearance,
    TerminalBackend, TerminalBehavior, TerminalBindings, TerminalConfig,
    TerminalEvent, TerminalFont, TerminalGeometry, TerminalTheme,
};

const MAX_PENDING_REQUESTS: usize = 1024;

/// Embeddable GPUI terminal entity with replaceable backend ownership.
pub struct Terminal {
    config: TerminalConfig,
    focus_handle: FocusHandle,
    generation: BackendGeneration,
    backend_state: BackendState,
    handle: Option<TerminalHandle>,
    snapshot: SnapshotArc,
    title: Option<SharedString>,
    selected_block: Option<BlockId>,
    terminal_size: TerminalSize,
    last_bounds: Option<Bounds<Pixels>>,
    cell_metrics: Option<CellMetrics>,
    marked_text: String,
    marked_selection_utf16: Range<usize>,
    local_selection_drag: bool,
    copy_after_selection_frame: bool,
    scroll_remainder: f32,
    terminal_pointer_cursor: PointerCursorStyle,
    pointer_cursor: PointerCursorStyle,
    pending_requests: VecDeque<TerminalRequest>,
    flushing_requests: bool,
    release_subscription: Option<Subscription>,
}

impl Terminal {
    /// Current presentation and interaction configuration.
    pub fn config(&self) -> &TerminalConfig {
        &self.config
    }

    /// Lifecycle state of the active backend generation.
    pub fn backend_state(&self) -> &BackendState {
        &self.backend_state
    }

    /// Generation identifying the current backend start attempt.
    pub fn backend_generation(&self) -> BackendGeneration {
        self.generation
    }

    /// Current terminal title, when set by the backend.
    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|title| title.as_ref())
    }

    /// Cheaply clone the latest immutable terminal frame.
    pub fn snapshot_arc(&self) -> SnapshotArc {
        Arc::clone(&self.snapshot)
    }

    /// Block metadata contained in the latest frame.
    pub fn blocks(&self) -> &[BlockSnapshot] {
        &self.snapshot.blocks
    }

    /// Resolve the full textual content of a block.
    pub fn block_text(&self, id: &BlockId) -> Option<String> {
        self.snapshot.block_text(id.as_str())
    }

    /// Whether the latest frame contains a non-empty selection.
    pub fn has_selection(&self) -> bool {
        self.snapshot.view().selection.is_some()
    }

    pub(crate) fn focus_handle_for_element(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    pub(crate) fn marked_text(&self) -> &str {
        &self.marked_text
    }

    pub(crate) fn selected_block(&self) -> Option<&BlockId> {
        self.selected_block.as_ref()
    }

    /// Create a terminal and start its backend away from the UI thread.
    pub fn new(
        config: TerminalConfig,
        backend: impl TerminalBackend,
        cx: &mut Context<Self>,
    ) -> Self {
        let release_subscription = cx.on_release(|terminal, cx| {
            terminal.send_shutdown_async(cx);
        });
        let terminal = Self {
            config,
            focus_handle: cx.focus_handle(),
            generation: BackendGeneration::initial(),
            backend_state: BackendState::Starting,
            handle: None,
            snapshot: Arc::new(SnapshotOwned::default()),
            title: None,
            selected_block: None,
            terminal_size: TerminalSize::default(),
            last_bounds: None,
            cell_metrics: None,
            marked_text: String::new(),
            marked_selection_utf16: 0..0,
            local_selection_drag: false,
            copy_after_selection_frame: false,
            scroll_remainder: 0.0,
            terminal_pointer_cursor: PointerCursorStyle::IBeam,
            pointer_cursor: PointerCursorStyle::IBeam,
            pending_requests: VecDeque::new(),
            flushing_requests: false,
            release_subscription: Some(release_subscription),
        };

        terminal.start_backend(Box::new(backend), terminal.generation, cx);
        terminal
    }

    /// Replace the active backend without replacing this GPUI entity.
    pub fn replace_backend(
        &mut self,
        backend: impl TerminalBackend,
        cx: &mut Context<Self>,
    ) -> BackendGeneration {
        self.send_shutdown_async(cx);
        self.generation = self.generation.next();
        self.clear_backend_state();
        self.set_backend_state(BackendState::Starting, cx);
        self.start_backend(Box::new(backend), self.generation, cx);

        self.generation
    }

    /// Request idempotent asynchronous shutdown of the active backend.
    pub fn shutdown_backend(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.backend_state,
            BackendState::Stopping | BackendState::Exited(_)
        ) {
            return;
        }

        self.send_shutdown_async(cx);
        self.set_backend_state(BackendState::Stopping, cx);
    }

    /// Replace all frontend configuration with a single invalidation.
    pub fn set_config(
        &mut self,
        config: TerminalConfig,
        cx: &mut Context<Self>,
    ) {
        let change = ConfigChange::between(&self.config, &config);
        self.config = config;

        if change.needs_notify() {
            cx.notify();
        }
    }

    /// Replace the terminal paint theme.
    pub fn set_theme(&mut self, theme: TerminalTheme, cx: &mut Context<Self>) {
        self.set_config(
            TerminalConfig::new(
                theme,
                self.config.font().clone(),
                *self.config.appearance(),
                self.config.behavior().clone(),
                self.config.bindings().clone(),
            ),
            cx,
        );
    }

    /// Replace font metrics and shaping settings.
    pub fn set_font(&mut self, font: TerminalFont, cx: &mut Context<Self>) {
        self.set_config(
            TerminalConfig::new(
                self.config.theme().clone(),
                font,
                *self.config.appearance(),
                self.config.behavior().clone(),
                self.config.bindings().clone(),
            ),
            cx,
        );
    }

    /// Replace frame, padding, and corner settings.
    pub fn set_appearance(
        &mut self,
        appearance: TerminalAppearance,
        cx: &mut Context<Self>,
    ) {
        self.set_config(
            TerminalConfig::new(
                self.config.theme().clone(),
                self.config.font().clone(),
                appearance,
                self.config.behavior().clone(),
                self.config.bindings().clone(),
            ),
            cx,
        );
    }

    /// Replace terminal interaction policies.
    pub fn set_behavior(
        &mut self,
        behavior: TerminalBehavior,
        cx: &mut Context<Self>,
    ) {
        self.set_config(
            TerminalConfig::new(
                self.config.theme().clone(),
                self.config.font().clone(),
                *self.config.appearance(),
                behavior,
                self.config.bindings().clone(),
            ),
            cx,
        );
    }

    /// Replace terminal-local key bindings without restarting the backend.
    pub fn set_bindings(
        &mut self,
        bindings: TerminalBindings,
        cx: &mut Context<Self>,
    ) {
        self.set_config(
            TerminalConfig::new(
                self.config.theme().clone(),
                self.config.font().clone(),
                *self.config.appearance(),
                self.config.behavior().clone(),
                bindings,
            ),
            cx,
        );
    }

    /// Write UTF-8 text to the active terminal backend.
    pub fn write_text(
        &mut self,
        text: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError> {
        self.write_bytes(text.as_bytes(), cx)
    }

    /// Write exact bytes to the active terminal backend.
    pub fn write_bytes(
        &mut self,
        bytes: &[u8],
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError> {
        self.queue_request(TerminalRequest::WriteBytes(bytes.to_vec()), cx)
    }

    /// Copy the current selection to the native clipboard.
    pub fn copy_selection(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError> {
        let text = self.snapshot.view().selectable_content();
        if text.is_empty() {
            return Err(OperationError::ContentUnavailable);
        }

        cx.write_to_clipboard(ClipboardItem::new_string(text));
        cx.emit(TerminalEvent::Copied {
            source: CopySource::Selection,
        });
        Ok(())
    }

    /// Paste native clipboard text with bracketed-paste protocol handling.
    pub fn paste(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError> {
        let text = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .ok_or(OperationError::ContentUnavailable)?;
        let bytes = if self
            .snapshot
            .view()
            .mode
            .contains(SurfaceMode::BRACKETED_PASTE)
        {
            format!("\x1b[200~{text}\x1b[201~").into_bytes()
        } else {
            text.into_bytes()
        };

        self.write_bytes(&bytes, cx)
    }

    /// Clear the grid selection without changing backend contents.
    pub fn clear_selection(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError> {
        self.queue_request(TerminalRequest::ClearSelection, cx)?;
        if self.selected_block.take().is_some() {
            cx.emit(TerminalEvent::BlockSelectionChanged { block_id: None });
        }
        cx.notify();
        Ok(())
    }

    /// Select a non-prompt block for internal or external host chrome.
    pub fn select_block(
        &mut self,
        id: &BlockId,
        cx: &mut Context<Self>,
    ) -> bool {
        if !block_operations_allowed(self.snapshot.view().mode) {
            return false;
        }
        let Some(block) = self
            .blocks()
            .iter()
            .find(|block| block.meta.id == id.as_str())
        else {
            return false;
        };
        if block.meta.kind == otty_libterm::surface::BlockKind::Prompt
            || self.selected_block.as_ref() == Some(id)
        {
            return false;
        }

        self.selected_block = Some(id.clone());
        cx.emit(TerminalEvent::BlockSelectionChanged {
            block_id: Some(id.clone()),
        });
        cx.notify();
        true
    }

    /// Scroll the viewport until the named block starts at the top.
    pub fn scroll_to_block(
        &mut self,
        id: &BlockId,
        cx: &mut Context<Self>,
    ) -> bool {
        if !block_operations_allowed(self.snapshot.view().mode) {
            return false;
        }
        let Some(block) = self
            .blocks()
            .iter()
            .find(|block| block.meta.id == id.as_str())
        else {
            return false;
        };
        let delta = block.start_line;

        self.queue_request(
            TerminalRequest::ScrollDisplay(Scroll::Delta(delta)),
            cx,
        )
        .is_ok()
    }

    /// Copy one semantic portion of a block to the native clipboard.
    pub fn copy_block(
        &mut self,
        id: &BlockId,
        part: BlockTextPart,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError> {
        let view = self.snapshot.view();
        if !block_operations_allowed(view.mode) {
            return Err(OperationError::ContentUnavailable);
        }
        let text = match part {
            BlockTextPart::All => view.block_text(id.as_str()),
            BlockTextPart::Content => {
                view.block_text(id.as_str()).and_then(|raw| {
                    let content =
                        raw.split_once('\n').map_or("", |(_, output)| output);
                    (!content.is_empty()).then(|| content.to_string())
                })
            },
            BlockTextPart::Prompt => view.block_prompt_text(id.as_str()),
            BlockTextPart::Command => view
                .blocks()
                .iter()
                .find(|block| block.meta.id == id.as_str())
                .and_then(|block| block.meta.cmd.clone())
                .filter(|command| !command.is_empty()),
        }
        .ok_or(OperationError::ContentUnavailable)?;

        cx.write_to_clipboard(ClipboardItem::new_string(text));
        cx.emit(TerminalEvent::Copied {
            source: CopySource::Block {
                block_id: id.clone(),
                part,
            },
        });
        Ok(())
    }

    fn start_backend(
        &self,
        backend: Box<dyn TerminalBackend>,
        generation: BackendGeneration,
        cx: &mut Context<Self>,
    ) {
        let initial_size = self.terminal_size;
        let start = cx
            .background_executor()
            .spawn(async move { backend.start(initial_size) });

        cx.spawn(async move |terminal, cx| {
            let result = start.await;
            let _ = terminal.update(cx, |terminal, cx| {
                terminal.install_backend(generation, result, cx);
            });
        })
        .detach();
    }

    fn install_backend(
        &mut self,
        generation: BackendGeneration,
        result: Result<BackendSession, BackendError>,
        cx: &mut Context<Self>,
    ) {
        if generation != self.generation {
            if let Ok(session) = result {
                let (handle, _events, run) = session.into_parts();
                let _ = handle.send(TerminalRequest::Shutdown);
                let _ = std::thread::Builder::new()
                    .name(format!("otty-terminal-stale-{}", generation.value()))
                    .spawn(move || {
                        let _ = run();
                    });
            }
            return;
        }

        let session = match result {
            Ok(session) => session,
            Err(error) => {
                self.set_backend_state(
                    BackendState::Failed(Arc::new(error)),
                    cx,
                );
                return;
            },
        };
        let (handle, events, run) = session.into_parts();
        let thread = std::thread::Builder::new()
            .name(format!("otty-terminal-{}", generation.value()))
            .spawn(move || {
                if let Err(error) = run() {
                    log::error!("terminal backend runtime failed: {error}");
                }
            });
        if let Err(error) = thread {
            let _ = handle.send(TerminalRequest::Shutdown);
            self.set_backend_state(
                BackendState::Failed(Arc::new(BackendError::ThreadSpawn(
                    error,
                ))),
                cx,
            );
            return;
        }

        self.handle = Some(handle);
        self.set_backend_state(BackendState::Running, cx);
        self.flush_requests(cx);

        cx.spawn(async move |terminal, cx| {
            while let Ok(event) = events.recv_async().await {
                if terminal
                    .update(cx, |terminal, cx| {
                        terminal.apply_core_event(generation, event, cx);
                    })
                    .is_err()
                {
                    return;
                }
            }

            let _ = terminal.update(cx, |terminal, cx| {
                terminal.backend_channel_closed(generation, cx);
            });
        })
        .detach();
    }

    fn apply_core_event(
        &mut self,
        generation: BackendGeneration,
        event: CoreEvent,
        cx: &mut Context<Self>,
    ) {
        if generation != self.generation {
            return;
        }

        match event {
            CoreEvent::Frame { frame } => {
                let had_selection = self.has_selection();
                self.snapshot = frame;
                let has_selection = self.has_selection();
                if had_selection != has_selection {
                    cx.emit(TerminalEvent::SelectionChanged { has_selection });
                }
                if self.copy_after_selection_frame {
                    self.copy_after_selection_frame = false;
                    if has_selection {
                        let _ = self.copy_selection(cx);
                    }
                }
                cx.notify();
            },
            CoreEvent::ChildExit { status } => {
                self.handle = None;
                self.set_backend_state(BackendState::Exited(status), cx);
            },
            CoreEvent::TitleChanged { title } => {
                let title = SharedString::from(title);
                self.title = Some(title.clone());
                cx.emit(TerminalEvent::TitleChanged(Some(title)));
                cx.notify();
            },
            CoreEvent::ResetTitle => {
                self.title = None;
                cx.emit(TerminalEvent::TitleChanged(None));
                cx.notify();
            },
            CoreEvent::Bell => self.emit_bell(cx),
            CoreEvent::CursorIconChanged { icon } => {
                let cursor = pointer_cursor_for_terminal_icon(icon);
                self.terminal_pointer_cursor = cursor;
                self.pointer_cursor = cursor;
                cx.notify();
            },
            _ => cx.notify(),
        }

        self.flush_requests(cx);
    }

    fn backend_channel_closed(
        &mut self,
        generation: BackendGeneration,
        cx: &mut Context<Self>,
    ) {
        if generation != self.generation
            || !backend_channel_close_is_failure(&self.backend_state)
        {
            return;
        }

        self.handle = None;
        let error = io::Error::new(
            io::ErrorKind::BrokenPipe,
            "terminal backend event channel closed",
        );
        self.set_backend_state(
            BackendState::Failed(Arc::new(BackendError::external(error))),
            cx,
        );
    }

    fn set_backend_state(
        &mut self,
        state: BackendState,
        cx: &mut Context<Self>,
    ) {
        self.backend_state = state.clone();
        cx.emit(TerminalEvent::BackendStateChanged {
            generation: self.generation,
            state,
        });
        cx.notify();
    }

    fn clear_backend_state(&mut self) {
        self.handle = None;
        self.snapshot = Arc::new(SnapshotOwned::default());
        self.title = None;
        self.selected_block = None;
        self.marked_text.clear();
        self.marked_selection_utf16 = 0..0;
        self.local_selection_drag = false;
        self.copy_after_selection_frame = false;
        self.scroll_remainder = 0.0;
        self.terminal_pointer_cursor = PointerCursorStyle::IBeam;
        self.pointer_cursor = PointerCursorStyle::IBeam;
        self.pending_requests.clear();
        self.flushing_requests = false;
    }

    fn send_shutdown(&self) {
        if let Some(handle) = &self.handle {
            let _ = handle.send(TerminalRequest::Shutdown);
        }
    }

    fn send_shutdown_async(&mut self, cx: &App) {
        let Some(handle) = self.handle.take() else {
            return;
        };

        cx.background_executor()
            .spawn(async move {
                let _ = handle.send_async(TerminalRequest::Shutdown).await;
            })
            .detach();
    }

    fn queue_request(
        &mut self,
        request: TerminalRequest,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError> {
        let handle = self
            .handle
            .as_ref()
            .ok_or(OperationError::BackendUnavailable)?;
        if !self.pending_requests.is_empty() {
            return self.push_pending(request, cx);
        }

        match handle.send(request.clone()) {
            Ok(()) => Ok(()),
            Err(ChannelSendError::Full) => self.push_pending(request, cx),
            Err(ChannelSendError::Disconnected) => {
                Err(OperationError::BackendDisconnected)
            },
        }
    }

    fn push_pending(
        &mut self,
        request: TerminalRequest,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError> {
        if self.pending_requests.len() >= MAX_PENDING_REQUESTS {
            return Err(OperationError::Backpressure);
        }

        self.pending_requests.push_back(request);
        self.flush_requests(cx);
        Ok(())
    }

    fn flush_requests(&mut self, cx: &mut Context<Self>) {
        if self.flushing_requests {
            return;
        }
        let Some(handle) = self.handle.clone() else {
            return;
        };
        let Some(request) = self.pending_requests.front().cloned() else {
            return;
        };

        self.flushing_requests = true;
        cx.spawn(async move |terminal, cx| {
            let result = handle.send_async(request).await;
            let _ = terminal.update(cx, |terminal, cx| {
                terminal.flushing_requests = false;
                match result {
                    Ok(()) => {
                        terminal.pending_requests.pop_front();
                        terminal.flush_requests(cx);
                    },
                    Err(ChannelSendError::Full) => terminal.flush_requests(cx),
                    Err(ChannelSendError::Disconnected) => {
                        terminal.pending_requests.clear();
                        terminal
                            .backend_channel_closed(terminal.generation, cx);
                    },
                }
            });
        })
        .detach();
    }

    fn emit_bell(&self, cx: &mut Context<Self>) {
        use crate::BellPolicy;

        match self.config.behavior().bell_policy() {
            BellPolicy::SystemAndEmit | BellPolicy::EmitOnly => {
                cx.emit(TerminalEvent::Bell);
            },
            BellPolicy::Disabled => {},
        }
    }

    pub(crate) fn update_layout(
        &mut self,
        bounds: Bounds<Pixels>,
        metrics: CellMetrics,
        cx: &mut Context<Self>,
    ) {
        let geometry = TerminalGeometry::new(
            f32::from(bounds.size.width),
            f32::from(bounds.size.height),
            0.0,
            0.0,
        );
        let terminal_size = geometry.terminal_size(metrics);
        let changed = !same_terminal_size(self.terminal_size, terminal_size);
        self.last_bounds = Some(bounds);
        self.cell_metrics = Some(metrics);

        if changed {
            self.terminal_size = terminal_size;
            let _ =
                self.queue_request(TerminalRequest::Resize(terminal_size), cx);
        }
    }

    fn candidate_bounds(&self) -> Option<Bounds<Pixels>> {
        let bounds = self.last_bounds?;
        let metrics = self.cell_metrics?;
        let view = self.snapshot.view();
        let cursor = view.cursor.point;
        let x = bounds.left() + px(cursor.column.0 as f32 * metrics.width());
        let y = bounds.top()
            + px((cursor.line.0 as f32 + view.display_offset as f32)
                * metrics.height());

        Some(Bounds::new(
            point(x, y),
            size(px(metrics.width()), px(metrics.height())),
        ))
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        let view = self.snapshot.view();
        let last_line = view.size.screen_lines.saturating_sub(1) as i32;
        let last_column = view.size.columns.saturating_sub(1);
        let start = Point::new(Line(-(view.display_offset as i32)), Column(0));
        let end = Point::new(Line(last_line), Column(last_column));
        let _ = self.queue_request(
            TerminalRequest::StartSelection {
                ty: SelectionType::Simple,
                point: start,
                direction: Side::Left,
            },
            cx,
        );
        let _ = self.queue_request(
            TerminalRequest::UpdateSelection {
                point: end,
                direction: Side::Right,
            },
            cx,
        );
    }

    fn scroll(&mut self, scroll: Scroll, cx: &mut Context<Self>) {
        let _ = self.queue_request(TerminalRequest::ScrollDisplay(scroll), cx);
    }

    fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mode = self.snapshot.view().mode;
        let Some(action) = self
            .config
            .bindings()
            .resolve(&event.keystroke, mode)
            .cloned()
        else {
            return;
        };

        match action {
            BindingAction::Bytes(bytes) => {
                let _ = self.write_bytes(&bytes, cx);
            },
            BindingAction::Copy => {
                let _ = self.copy_selection(cx);
            },
            BindingAction::Paste => {
                let _ = self.paste(cx);
            },
            BindingAction::Ignore => {},
        }
        cx.stop_propagation();
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.config.behavior().focus_on_click() {
            window.focus(&self.focus_handle);
        }

        let mode = self.snapshot.view().mode;
        let reporting =
            mode.intersects(SurfaceMode::MOUSE_MODE) && !event.modifiers.shift;
        if reporting {
            if let Some(button) = mouse_button(event.button) {
                self.send_mouse_report(
                    event.position,
                    button,
                    event.modifiers,
                    true,
                    cx,
                );
            }
            self.local_selection_drag = false;
            cx.stop_propagation();
            return;
        }

        match event.button {
            MouseButton::Left => {
                let Some(point) = self.grid_point(event.position, true) else {
                    return;
                };
                if event.modifiers.platform && self.activate_link(point, cx) {
                    cx.stop_propagation();
                    return;
                }
                let ty = if event.modifiers.alt {
                    SelectionType::Block
                } else {
                    match event.click_count {
                        2 => SelectionType::Semantic,
                        3.. => SelectionType::Lines,
                        _ => SelectionType::Simple,
                    }
                };
                let direction = self.selection_side(event.position);
                if self
                    .queue_request(
                        TerminalRequest::StartSelection {
                            ty,
                            point,
                            direction,
                        },
                        cx,
                    )
                    .is_ok()
                {
                    self.local_selection_drag = true;
                }
            },
            MouseButton::Middle
                if self.config.behavior().middle_click_paste() =>
            {
                let _ = self.paste(cx);
            },
            MouseButton::Right
                if self.config.behavior().context_menu_policy()
                    == ContextMenuPolicy::Emit =>
            {
                let target = self.hit_target(event.position);
                cx.emit(TerminalEvent::ContextMenuRequested {
                    position: event.position,
                    target,
                });
            },
            _ => {},
        }

        cx.stop_propagation();
    }

    fn on_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mode = self.snapshot.view().mode;
        if mode.intersects(SurfaceMode::MOUSE_MODE) && !event.modifiers.shift {
            if let Some(button) = mouse_button(event.button) {
                self.send_mouse_report(
                    event.position,
                    button,
                    event.modifiers,
                    false,
                    cx,
                );
            }
        } else if self.local_selection_drag {
            let final_selection_queued =
                self.grid_point(event.position, true).is_some_and(|point| {
                    self.queue_request(
                        TerminalRequest::UpdateSelection {
                            point,
                            direction: self.selection_side(event.position),
                        },
                        cx,
                    )
                    .is_ok()
                });
            self.copy_after_selection_frame = final_selection_queued
                && self.config.behavior().copy_on_select();
        }

        self.local_selection_drag = false;
        cx.stop_propagation();
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_pointer_cursor(
            event.position,
            event.modifiers.platform,
            cx,
        );

        let mode = self.snapshot.view().mode;
        if mode.intersects(SurfaceMode::MOUSE_MODE) && !event.modifiers.shift {
            let button = match event.pressed_button {
                Some(MouseButton::Left)
                    if mode.contains(SurfaceMode::MOUSE_DRAG) =>
                {
                    Some(TerminalMouseButton::LeftMove)
                },
                Some(MouseButton::Middle)
                    if mode.contains(SurfaceMode::MOUSE_DRAG) =>
                {
                    Some(TerminalMouseButton::MiddleMove)
                },
                Some(MouseButton::Right)
                    if mode.contains(SurfaceMode::MOUSE_DRAG) =>
                {
                    Some(TerminalMouseButton::RightMove)
                },
                None if mode.contains(SurfaceMode::MOUSE_MOTION) => {
                    Some(TerminalMouseButton::Move)
                },
                _ => None,
            };
            if let Some(button) = button {
                self.send_mouse_report(
                    event.position,
                    button,
                    event.modifiers,
                    true,
                    cx,
                );
            }
            return;
        }
        if !self.local_selection_drag || !event.dragging() {
            return;
        }
        let Some(point) = self.grid_point(event.position, true) else {
            return;
        };

        let _ = self.queue_request(
            TerminalRequest::UpdateSelection {
                point,
                direction: self.selection_side(event.position),
            },
            cx,
        );
        cx.stop_propagation();
    }

    fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mode = self.snapshot.view().mode;
        let delta = self.scroll_lines(event.delta);
        if delta == 0 {
            return;
        }

        if mode.intersects(SurfaceMode::MOUSE_MODE) && !event.modifiers.shift {
            let button = if delta > 0 {
                TerminalMouseButton::ScrollUp
            } else {
                TerminalMouseButton::ScrollDown
            };
            for _ in 0..delta.unsigned_abs() {
                self.send_mouse_report(
                    event.position,
                    button,
                    event.modifiers,
                    true,
                    cx,
                );
            }
        } else if mode
            .contains(SurfaceMode::ALTERNATE_SCROLL | SurfaceMode::ALT_SCREEN)
        {
            let command = if delta > 0 { b'A' } else { b'B' };
            let mut bytes =
                Vec::with_capacity(delta.unsigned_abs() as usize * 3);
            for _ in 0..delta.unsigned_abs() {
                bytes.extend_from_slice(&[0x1b, b'O', command]);
            }
            let _ = self.write_bytes(&bytes, cx);
        } else {
            self.scroll(Scroll::Delta(delta), cx);
        }

        cx.stop_propagation();
    }

    fn grid_point(
        &self,
        position: gpui::Point<Pixels>,
        include_display_offset: bool,
    ) -> Option<Point> {
        let bounds = self.last_bounds?;
        let metrics = self.cell_metrics?;
        let geometry = TerminalGeometry::new(
            f32::from(bounds.size.width),
            f32::from(bounds.size.height),
            0.0,
            0.0,
        );
        let x = f32::from(position.x - bounds.left());
        let y = f32::from(position.y - bounds.top());
        let display_offset = if include_display_offset {
            self.snapshot.view().display_offset
        } else {
            0
        };

        Some(geometry.point_to_grid(x, y, metrics, display_offset))
    }

    fn selection_side(&self, position: gpui::Point<Pixels>) -> Side {
        let Some(bounds) = self.last_bounds else {
            return Side::Left;
        };
        let Some(metrics) = self.cell_metrics else {
            return Side::Left;
        };
        let x = f32::from(position.x - bounds.left()).max(0.0);
        let cell_x = x % metrics.width();

        if cell_x > metrics.width() / 2.0 {
            Side::Right
        } else {
            Side::Left
        }
    }

    fn send_mouse_report(
        &mut self,
        position: gpui::Point<Pixels>,
        button: TerminalMouseButton,
        modifiers: gpui::Modifiers,
        pressed: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(point) = self.grid_point(position, false) else {
            return;
        };
        let modifiers = TerminalMouseModifiers::new(
            modifiers.shift,
            modifiers.alt,
            modifiers.control || modifiers.platform,
        );
        let Some(report) = encode_mouse_report(
            self.snapshot.view().mode,
            point,
            button,
            modifiers,
            pressed,
        ) else {
            return;
        };

        let _ = self.write_bytes(&report, cx);
    }

    fn activate_link(&mut self, point: Point, cx: &mut Context<Self>) -> bool {
        let Some(uri) = self
            .snapshot
            .view()
            .hyperlink_span_at(point)
            .map(|span| span.link.uri().to_string())
        else {
            return false;
        };

        match self.config.behavior().link_policy() {
            LinkPolicy::EmitOnly => {
                cx.emit(TerminalEvent::OpenLinkRequested { uri: uri.into() });
            },
            LinkPolicy::OpenAndEmit => {
                cx.open_url(&uri);
                cx.emit(TerminalEvent::OpenLinkRequested { uri: uri.into() });
            },
            LinkPolicy::Disabled => return false,
        }

        true
    }

    fn hit_target(&self, position: gpui::Point<Pixels>) -> HitTarget {
        let Some(point) = self.grid_point(position, true) else {
            return HitTarget::Terminal;
        };
        let view = self.snapshot.view();
        if let Some(link) = view.hyperlink_span_at(point) {
            return HitTarget::Link(link.link.uri().to_string().into());
        }
        if let Some(block) = view.block_at_point(point) {
            return HitTarget::Block(BlockId::from(block.meta.id.clone()));
        }

        HitTarget::Terminal
    }

    fn update_pointer_cursor(
        &mut self,
        position: gpui::Point<Pixels>,
        link_modifier: bool,
        cx: &mut Context<Self>,
    ) {
        let over_enabled_link = link_modifier
            && self.config.behavior().link_policy() != LinkPolicy::Disabled
            && self.grid_point(position, true).is_some_and(|point| {
                self.snapshot.view().hyperlink_span_at(point).is_some()
            });
        let cursor = if over_enabled_link {
            PointerCursorStyle::PointingHand
        } else {
            self.terminal_pointer_cursor
        };
        if cursor != self.pointer_cursor {
            self.pointer_cursor = cursor;
            cx.notify();
        }
    }

    fn scroll_lines(&mut self, delta: ScrollDelta) -> i32 {
        let multiplier = self.config.behavior().scroll_multiplier();
        let line_height = self
            .cell_metrics
            .map_or(self.config.font().size(), |metrics| metrics.height());

        scroll_delta_lines(
            delta,
            multiplier,
            line_height,
            &mut self.scroll_remainder,
        )
    }

    fn action_copy(
        &mut self,
        _: &Copy,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let _ = self.copy_selection(cx);
    }

    fn action_paste(
        &mut self,
        _: &Paste,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let _ = self.paste(cx);
    }

    fn action_select_all(
        &mut self,
        _: &SelectAll,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_all(cx);
    }

    fn action_clear_selection(
        &mut self,
        _: &ClearSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let _ = self.clear_selection(cx);
    }

    fn action_scroll_page_up(
        &mut self,
        _: &ScrollPageUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.scroll(Scroll::PageUp, cx);
    }

    fn action_scroll_page_down(
        &mut self,
        _: &ScrollPageDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.scroll(Scroll::PageDown, cx);
    }

    fn action_scroll_to_top(
        &mut self,
        _: &ScrollToTop,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.scroll(Scroll::Top, cx);
    }

    fn action_scroll_to_bottom(
        &mut self,
        _: &ScrollToBottom,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.scroll(Scroll::Bottom, cx);
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.send_shutdown();
        self.release_subscription.take();
    }
}

impl EventEmitter<TerminalEvent> for Terminal {}

impl Focusable for Terminal {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Terminal {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let appearance = self.config.appearance();
        let is_focused = self.focus_handle.is_focused(window);
        let border = if is_focused {
            self.config.theme().focused_border().hsla()
        } else {
            self.config.theme().border().hsla()
        };

        div()
            .id("otty-terminal")
            .size_full()
            .overflow_hidden()
            .track_focus(&self.focus_handle)
            .bg(self.config.theme().palette().background().hsla())
            .border(px(active_border_width(appearance, is_focused)))
            .border_color(border)
            .rounded(px(appearance.corner_radius()))
            .p(px(appearance.padding()))
            .cursor(self.pointer_cursor)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(Self::on_mouse_down),
            )
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_action(cx.listener(Self::action_copy))
            .on_action(cx.listener(Self::action_paste))
            .on_action(cx.listener(Self::action_select_all))
            .on_action(cx.listener(Self::action_clear_selection))
            .on_action(cx.listener(Self::action_scroll_page_up))
            .on_action(cx.listener(Self::action_scroll_page_down))
            .on_action(cx.listener(Self::action_scroll_to_top))
            .on_action(cx.listener(Self::action_scroll_to_bottom))
            .child(TerminalElement::new(cx.entity()))
    }
}

impl EntityInputHandler for Terminal {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = utf16_range_to_byte(&self.marked_text, range_utf16);
        actual_range
            .replace(byte_range_to_utf16(&self.marked_text, range.clone()));

        Some(self.marked_text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.marked_selection_utf16.clone(),
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        (!self.marked_text.is_empty())
            .then(|| 0..self.marked_text.encode_utf16().count())
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.marked_text.clear();
        self.marked_selection_utf16 = 0..0;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked_text.clear();
        self.marked_selection_utf16 = 0..0;
        if !new_text.is_empty() {
            let _ = self.write_text(new_text, cx);
        }
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked_text.clear();
        self.marked_text.push_str(new_text);
        let utf16_len = new_text.encode_utf16().count();
        let selected = new_selected_range_utf16.unwrap_or(utf16_len..utf16_len);
        self.marked_selection_utf16 =
            selected.start.min(utf16_len)..selected.end.min(utf16_len);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        self.candidate_bounds()
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(0)
    }
}

fn same_terminal_size(left: TerminalSize, right: TerminalSize) -> bool {
    left.cols == right.cols
        && left.rows == right.rows
        && left.cell_width == right.cell_width
        && left.cell_height == right.cell_height
}

fn mouse_button(button: MouseButton) -> Option<TerminalMouseButton> {
    match button {
        MouseButton::Left => Some(TerminalMouseButton::Left),
        MouseButton::Middle => Some(TerminalMouseButton::Middle),
        MouseButton::Right => Some(TerminalMouseButton::Right),
        MouseButton::Navigate(_) => None,
    }
}

fn active_border_width(
    appearance: &TerminalAppearance,
    is_focused: bool,
) -> f32 {
    if is_focused {
        appearance.focused_border_width()
    } else {
        appearance.border_width()
    }
}

fn block_operations_allowed(mode: SurfaceMode) -> bool {
    !mode.contains(SurfaceMode::ALT_SCREEN)
}

fn backend_channel_close_is_failure(state: &BackendState) -> bool {
    matches!(state, BackendState::Starting | BackendState::Running)
}

fn pointer_cursor_for_terminal_icon(icon: CursorIcon) -> PointerCursorStyle {
    match icon {
        CursorIcon::Pointer => PointerCursorStyle::PointingHand,
        CursorIcon::Text => PointerCursorStyle::IBeam,
        CursorIcon::Crosshair => PointerCursorStyle::Crosshair,
        CursorIcon::Grab => PointerCursorStyle::OpenHand,
        CursorIcon::Grabbing => PointerCursorStyle::ClosedHand,
        CursorIcon::WResize => PointerCursorStyle::ResizeLeft,
        CursorIcon::EResize => PointerCursorStyle::ResizeRight,
        CursorIcon::EwResize => PointerCursorStyle::ResizeLeftRight,
        CursorIcon::NResize => PointerCursorStyle::ResizeUp,
        CursorIcon::SResize => PointerCursorStyle::ResizeDown,
        CursorIcon::NsResize => PointerCursorStyle::ResizeUpDown,
        CursorIcon::NeswResize => PointerCursorStyle::ResizeUpLeftDownRight,
        CursorIcon::NwseResize => PointerCursorStyle::ResizeUpRightDownLeft,
        CursorIcon::ColResize => PointerCursorStyle::ResizeColumn,
        CursorIcon::RowResize => PointerCursorStyle::ResizeRow,
        CursorIcon::VerticalText => {
            PointerCursorStyle::IBeamCursorForVerticalLayout
        },
        _ => PointerCursorStyle::Arrow,
    }
}

fn scroll_delta_lines(
    delta: ScrollDelta,
    multiplier: f32,
    line_height: f32,
    remainder: &mut f32,
) -> i32 {
    match delta {
        ScrollDelta::Lines(delta) => (delta.y * multiplier).round() as i32,
        ScrollDelta::Pixels(delta) => {
            *remainder += f32::from(delta.y) * multiplier;
            let lines = (*remainder / line_height).trunc();
            *remainder %= line_height;
            lines as i32
        },
    }
}

#[cfg(test)]
mod tests {
    use cursor_icon::CursorIcon;
    use gpui::{CursorStyle, ScrollDelta, point, px};
    use otty_libterm::surface::SurfaceMode;

    use super::{
        active_border_width, backend_channel_close_is_failure,
        block_operations_allowed, pointer_cursor_for_terminal_icon,
        scroll_delta_lines,
    };
    use crate::{BackendState, TerminalAppearance};

    #[test]
    fn focused_terminal_uses_the_focused_border_width() {
        let appearance = TerminalAppearance::try_new(4.0, 1.0, 6.0, 3.0)
            .expect("valid appearance");

        assert_eq!(active_border_width(&appearance, false), 1.0);
        assert_eq!(active_border_width(&appearance, true), 3.0);
    }

    #[test]
    fn block_operations_are_disabled_on_the_alternate_screen() {
        assert!(block_operations_allowed(SurfaceMode::empty()));
        assert!(!block_operations_allowed(SurfaceMode::ALT_SCREEN));
    }

    #[test]
    fn expected_backend_channel_close_is_not_a_runtime_failure() {
        assert!(!backend_channel_close_is_failure(&BackendState::Stopping));
    }

    #[test]
    fn terminal_cursor_icons_map_to_available_gpui_cursor_styles() {
        assert_eq!(
            pointer_cursor_for_terminal_icon(CursorIcon::Pointer),
            CursorStyle::PointingHand
        );
        assert_eq!(
            pointer_cursor_for_terminal_icon(CursorIcon::Text),
            CursorStyle::IBeam
        );
        assert_eq!(
            pointer_cursor_for_terminal_icon(CursorIcon::Grab),
            CursorStyle::OpenHand
        );
    }

    #[test]
    fn line_scroll_preserves_gpui_wheel_direction() {
        let mut remainder = 0.0;

        assert_eq!(
            scroll_delta_lines(
                ScrollDelta::Lines(point(0.0, 3.0)),
                2.0,
                16.0,
                &mut remainder,
            ),
            6
        );
        assert_eq!(remainder, 0.0);
    }

    #[test]
    fn pixel_scroll_accumulates_partial_lines_in_gpui_wheel_direction() {
        let mut remainder = 0.0;

        assert_eq!(
            scroll_delta_lines(
                ScrollDelta::Pixels(point(px(0.0), px(6.0))),
                1.0,
                16.0,
                &mut remainder,
            ),
            0
        );
        assert_eq!(remainder, 6.0);
        assert_eq!(
            scroll_delta_lines(
                ScrollDelta::Pixels(point(px(0.0), px(10.0))),
                1.0,
                16.0,
                &mut remainder,
            ),
            1
        );
        assert_eq!(remainder, 0.0);
    }
}
