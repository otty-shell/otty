use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::process::ExitStatus;
use std::sync::Arc;

use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::canvas::Cache;
use iced::{Size, Subscription};
use log::debug;
use otty_libterm::TerminalEvent;
use otty_libterm::surface::{
    BlockSnapshot, Point, SelectionType, SnapshotOwned,
};
use tokio::sync::Mutex;
use tokio::sync::mpsc::Receiver;

use crate::backend::{self, StartedProgram};
use crate::bindings::{Binding, BindingAction, BindingsLayout, InputKind};
use crate::engine::MouseButton;
use crate::font::TermFont;
use crate::settings::{
    BlockSelectionMode, FontSettings, Settings, ThemeSettings,
};
use crate::theme::{ColorPalette, Theme};
use crate::{engine, error};

/// Command that can be issued against a terminal block from the UI layer.
#[derive(Clone, Debug)]
pub enum BlockCommand {
    /// Select the block with the provided identifier.
    Select(String),
    /// Select the block that is currently hovered by the pointer, if any.
    SelectHovered,
    /// Copy the currently highlighted grid selection to the clipboard.
    CopySelection,
    /// Clear any active block selection.
    ClearSelection,
    /// Scroll the viewport so that the block becomes visible.
    ScrollTo(String),
    /// Copy textual contents of the block into the clipboard.
    Copy(String),
    /// Copy block contents without the leading prompt line.
    CopyContent(String),
    /// Copy only the prompt/input line for the block.
    CopyPrompt(String),
    /// Copy the parsed command line without the prompt prefix.
    CopyCommand(String),
    /// Paste clipboard contents into the focused terminal.
    PasteClipboard,
}

/// Mode describing how block-level UI chrome is rendered.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum BlockUiMode {
    /// TerminalView renders highlights and inline buttons itself.
    #[default]
    Internal,
    /// TerminalView omits inline chrome so an external overlay can draw it.
    ExternalOverlay,
}

#[derive(Clone)]
/// Events a terminal widget reports to its host.
///
/// `T` is the host application's own action type, defaulting to `()`
/// for embedders that register no application bindings.
pub enum Event<T = ()> {
    Redraw {
        id: u64,
    },
    /// The terminal has a newer snapshot. The event carries no snapshot: the
    /// terminal takes the newest one itself when it handles the event.
    ContentSync {
        id: u64,
    },
    Shutdown {
        id: u64,
        exit_status: ExitStatus,
    },
    Write {
        id: u64,
        data: Vec<u8>,
    },
    Scroll {
        id: u64,
        delta: i32,
    },
    SelectStart {
        id: u64,
        selection_type: SelectionType,
        position: (f32, f32),
    },
    SelectUpdate {
        id: u64,
        position: (f32, f32),
    },
    MouseReport {
        id: u64,
        button: MouseButton,
        modifiers: iced::keyboard::Modifiers,
        point: Point,
        pressed: bool,
    },
    OpenLink {
        id: u64,
        uri: String,
    },
    Resize {
        id: u64,
        layout_size: Option<Size>,
        cell_size: Option<Size>,
    },
    TitleChanged {
        id: u64,
        title: String,
    },
    ResetTitle {
        id: u64,
    },
    BlockSelected {
        id: u64,
        block_id: String,
    },
    BlockCopied {
        id: u64,
        block_id: String,
    },
    BlockSelectionCleared {
        id: u64,
    },
    /// An application-defined shortcut fired inside this terminal.
    ///
    /// Emitted when a key press matches a [`BindingAction::Action`]
    /// binding the host registered. The value is opaque here; the host
    /// decides what it means.
    BindingActionDispatched {
        id: u64,
        action: T,
    },
    Ignore {
        id: u64,
    },
}

impl<T: Debug> Debug for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Event::*;

        match self {
            Redraw { id } => f.write_fmt(format_args!("Event::Redraw {id}")),
            ContentSync { id } => f.write_fmt(format_args!("Event::ContentSync {id}")),
            Shutdown {
                id,
                exit_status,
            } => f.write_fmt(format_args!("Event::Shutdown id: {id}, exit_status: {exit_status}")),
            Write {
                id,
                data,
            } => f.write_fmt(format_args!("Event::Write id: {id}, data: {data:?}")),
            Scroll {
                id,
                delta,
            } => f.write_fmt(format_args!("Event::Scroll id: {id}, delta: {delta}")),
            SelectStart {
                id,
                selection_type,
                position,
            } => f.write_fmt(format_args!("Event::SelectStart id: {id}, ty: {selection_type:?}, pos: {position:?}")),
            SelectUpdate {
                id,
                position,
            } => f.write_fmt(format_args!("Event::SelectUpdate id: {id}, pos: {position:?}")),
            MouseReport {
                id,
                button,
                modifiers,
                point,
                pressed,
            } => f.write_fmt(format_args!("Event::MouseReport id: {id}, button: {button:?}, modifiers: {modifiers:?}, point: {point:?}, is_pressed: {pressed}")),
            Resize {
                id,
                layout_size,
                cell_size,
            } => f.write_fmt(format_args!("Event::Resize id: {id}, layout: {layout_size:?}, cell: {cell_size:?}")),
            TitleChanged {
                id,
                title,
            } => f.write_fmt(format_args!("Event::TitleChanged id: {id}, title: {title}")),
            OpenLink { id, uri } => f.write_fmt(format_args!("Event::OpenLink id: {id}, uri: {uri}")),
            ResetTitle { id } => f.write_fmt(format_args!("Event::ResetTitle id: {id}")),
            BlockSelected { id, block_id } => {
                f.write_fmt(format_args!("Event::BlockSelected id: {id}, block_id: {block_id}"))
            }
            BlockCopied { id, block_id } => {
                f.write_fmt(format_args!("Event::BlockCopied id: {id}, block_id: {block_id}"))
            }
            BlockSelectionCleared { id } => {
                f.write_fmt(format_args!("Event::BlockSelectionCleared id: {id}"))
            }
            BindingActionDispatched { id, action } => {
                f.write_fmt(format_args!(
                    "Event::BindingActionDispatched id: {id}, action: {action:?}"
                ))
            }
            Ignore { id } => f.write_fmt(format_args!("Event::Ignore id: {id}")),
        }
    }
}

impl<T> Event<T> {
    pub fn terminal_id(&self) -> &u64 {
        use Event::*;

        match self {
            Redraw { id, .. } => id,
            ContentSync { id, .. } => id,
            Shutdown { id, .. } => id,
            Write { id, .. } => id,
            Scroll { id, .. } => id,
            SelectStart { id, .. } => id,
            SelectUpdate { id, .. } => id,
            MouseReport { id, .. } => id,
            Resize { id, .. } => id,
            TitleChanged { id, .. } => id,
            OpenLink { id, .. } => id,
            ResetTitle { id } => id,
            BlockSelected { id, .. } => id,
            BlockCopied { id, .. } => id,
            BlockSelectionCleared { id } => id,
            BindingActionDispatched { id, .. } => id,
            Ignore { id } => id,
        }
    }

    fn from_terminal_event(id: u64, event: TerminalEvent) -> Event<T> {
        match event {
            TerminalEvent::ChildExit { status } => Event::Shutdown {
                id,
                exit_status: status,
            },
            TerminalEvent::SnapshotReady => Event::ContentSync { id },
            TerminalEvent::TitleChanged { title } => {
                Event::TitleChanged { id, title }
            },
            TerminalEvent::ResetTitle => Event::ResetTitle { id },
            _ => Event::Ignore { id },
        }
    }
}

pub struct Terminal<T = ()> {
    pub id: u64,
    widget_id: iced::widget::Id,
    pub(crate) font: TermFont,
    pub(crate) theme: Theme,
    pub(crate) cache: Cache,
    pub(crate) bindings: BindingsLayout<T>,
    pub(crate) engine: engine::Engine,
    block_selection_mode: BlockSelectionMode,
    block_ui_mode: BlockUiMode,
    backend_event_rx: Arc<Mutex<Receiver<TerminalEvent>>>,
}

impl<T> Terminal<T> {
    /// Start the program `settings` ask for and build its terminal.
    pub fn new(id: u64, settings: Settings) -> error::Result<Self> {
        let program = backend::start_program(&settings.backend)?;

        Ok(Self::with_program(id, settings, program))
    }

    /// Build the terminal of a program that is already started.
    ///
    /// [`Terminal::new`] starts the program itself and ends here. A host
    /// that starts programs its own way, or a test that puts a mock in
    /// place of one, comes here directly. The session of `settings` is not
    /// started again; of its backend part only the size is read.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_ui_term::settings::Settings;
    /// use otty_ui_term::{
    ///     SnapshotOwned, StartedProgram, Terminal, TerminalBackend,
    ///     TerminalRequest,
    /// };
    ///
    /// /// A program that ignores input and never has a new screen.
    /// struct Idle;
    ///
    /// impl TerminalBackend for Idle {
    ///     fn send(&self, _request: TerminalRequest) {}
    ///
    ///     fn take_snapshot(&self) -> Option<SnapshotOwned> {
    ///         None
    ///     }
    /// }
    ///
    /// let (program, _events_of_the_program) = StartedProgram::new(Box::new(Idle));
    /// let terminal: Terminal =
    ///     Terminal::with_program(1, Settings::default(), program);
    ///
    /// assert!(terminal.blocks().is_empty());
    /// ```
    pub fn with_program(
        id: u64,
        settings: Settings,
        program: StartedProgram,
    ) -> Self {
        let Settings {
            font,
            theme,
            backend,
            interaction,
        } = settings;
        let (program_backend, program_events) = program.into_parts();

        Self {
            id,
            widget_id: iced::widget::Id::unique(),
            font: TermFont::new(font),
            theme: Theme::new(theme),
            bindings: BindingsLayout::default(),
            cache: Cache::default(),
            engine: engine::Engine::new(program_backend, backend.size),
            block_selection_mode: interaction.block_selection_mode(),
            block_ui_mode: BlockUiMode::Internal,
            backend_event_rx: Arc::new(Mutex::new(program_events)),
        }
    }

    pub fn widget_id(&self) -> &iced::widget::Id {
        &self.widget_id
    }

    /// Borrow the snapshot the widget shows now.
    ///
    /// It is the snapshot taken on the most recent [`Event::ContentSync`];
    /// before the first one the screen is empty. The widget owns it; a newer
    /// snapshot replaces it on the next `ContentSync`.
    pub fn snapshot(&self) -> &SnapshotOwned {
        self.engine.snapshot()
    }

    /// Return a copy of block metadata captured in the latest snapshot.
    ///
    /// The returned vector is detached from the engine — mutations inside
    /// the terminal will only be visible in subsequent calls after a new
    /// snapshot arrives.
    pub fn blocks(&self) -> Vec<BlockSnapshot> {
        let snapshot = self.engine.snapshot();
        snapshot.view().blocks().to_vec()
    }

    /// Return textual contents for the provided block id from the latest
    /// snapshot.
    ///
    /// Returns `None` when the block is not found or has no textual payload
    /// (e.g. prompt blocks).
    pub fn block_text(&self, block_id: &str) -> Option<String> {
        let snapshot = self.engine.snapshot();
        snapshot.block_text(block_id)
    }

    /// Return the prompt/input line for the provided block id from the latest
    /// snapshot, if available.
    pub fn block_prompt_text(&self, block_id: &str) -> Option<String> {
        let snapshot = self.engine.snapshot();
        snapshot.block_prompt_text(block_id)
    }

    pub fn subscription(&self) -> Subscription<Event<T>>
    where
        T: Send + 'static,
    {
        let data = TerminalSubscriptionData {
            id: self.id,
            event_receiver: self.backend_event_rx.clone(),
        };

        Subscription::run_with(data, terminal_subscription_stream)
    }

    pub fn handle(&mut self, event: Event<T>) {
        use Event::*;

        match event {
            Redraw { .. } => self.cache.clear(),
            ContentSync { .. } => self.content_sync(),
            Write { data, .. } => {
                debug!(
                    "terminal {} write {} bytes: {:02X?} (utf8={})",
                    self.id,
                    data.len(),
                    data,
                    String::from_utf8_lossy(&data)
                );
                self.engine.write(data);
            },
            Scroll { delta, .. } => self.engine.scroll_delta(delta),
            SelectStart {
                selection_type,
                position,
                ..
            } => self.engine.start_selection(
                selection_type,
                position.0,
                position.1,
            ),
            SelectUpdate { position, .. } => {
                self.engine.update_selection(position.0, position.1)
            },
            MouseReport {
                button,
                modifiers,
                point,
                pressed,
                ..
            } => self
                .engine
                .process_mouse_report(button, modifiers, point, pressed),
            Resize {
                layout_size,
                cell_size,
                ..
            } => self.engine.resize(layout_size, cell_size),
            OpenLink { uri, .. } => {
                let _ = open::that_detached(uri);
            },
            _ => {},
        }
    }

    pub fn change_theme(&mut self, pallete: ColorPalette) {
        self.theme = Theme::new(ThemeSettings::new(Box::new(pallete)));
        self.cache.clear();
    }

    pub fn change_font(&mut self, options: FontSettings) {
        self.font = TermFont::new(options);
        self.engine.resize(None, Some(self.font.measure));
    }

    pub fn add_bindings(
        &mut self,
        bindings: Vec<(Binding<InputKind>, BindingAction<T>)>,
    ) {
        self.bindings.add_bindings(bindings);
        self.cache.clear();
    }

    /// Return how pointer clicks should affect block selection.
    pub fn block_selection_mode(&self) -> BlockSelectionMode {
        self.block_selection_mode
    }

    /// Return the current block UI rendering mode.
    pub fn block_ui_mode(&self) -> BlockUiMode {
        self.block_ui_mode
    }

    /// Override the block UI mode after construction.
    pub fn set_block_ui_mode(&mut self, mode: BlockUiMode) {
        if self.block_ui_mode != mode {
            self.block_ui_mode = mode;
            self.cache.clear();
        }
    }

    /// Builder-style helper to pick a block UI mode during initialization.
    pub fn with_block_ui_mode(mut self, mode: BlockUiMode) -> Self {
        self.block_ui_mode = mode;
        self.cache.clear();
        self
    }

    /// Take the newest snapshot of the terminal. The screen is drawn again
    /// only when there was one.
    fn content_sync(&mut self) {
        let snapshot_changed = self.engine.sync_newest_snapshot();
        if snapshot_changed {
            self.cache.clear();
        }
    }
}

#[cfg(test)]
impl<T> Terminal<T> {
    /// Build a terminal that already shows `snapshot`: its program, a mock,
    /// had it ready, takes any request and has nothing newer afterwards.
    pub(crate) fn with_snapshot_for_test(
        id: u64,
        snapshot: SnapshotOwned,
    ) -> Self {
        let mut waiting = Some(snapshot);
        let mut backend = crate::backend::MockTerminalBackend::new();
        backend.expect_send().return_const(());
        backend
            .expect_take_snapshot()
            .returning(move || waiting.take());
        let (program, _no_events) = StartedProgram::new(Box::new(backend));

        let mut terminal = Self::with_program(id, Settings::default(), program);
        terminal.content_sync();

        terminal
    }
}

#[derive(Clone)]
struct TerminalSubscriptionData {
    id: u64,
    event_receiver: Arc<Mutex<Receiver<TerminalEvent>>>,
}

impl Hash for TerminalSubscriptionData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

fn terminal_subscription_stream<T: Send + 'static>(
    data: &TerminalSubscriptionData,
) -> BoxStream<'static, Event<T>> {
    let id = data.id;
    let event_receiver = data.event_receiver.clone();
    iced::stream::channel(1000, async move |mut output| {
        let mut shutdown = false;
        loop {
            let mut event_receiver = event_receiver.lock().await;
            match event_receiver.recv().await {
                Some(event) => {
                    let event = Event::from_terminal_event(id, event);
                    if matches!(event, Event::Shutdown { .. }) {
                        shutdown = true;
                    }

                    output
                        .send(event)
                        .await
                        .unwrap_or_else(|_| {
                            panic!(
                                "iced_term stream {}: sending BackendEventReceived event is failed",
                                id
                            )
                        });
                },
                None => {
                    if !shutdown {
                        panic!(
                            "iced_term stream {}: terminal event channel closed unexpected",
                            id
                        );
                    }
                    break;
                },
            }
        }
    })
    .boxed()
}

#[cfg(test)]
mod tests {
    use mockall::Sequence;
    use otty_libterm::TerminalRequest;
    use otty_libterm::surface::{
        BlockKind, BlockMeta, Column, Dimensions, Line, Surface, SurfaceConfig,
        SurfaceModel,
    };

    use super::*;
    use crate::backend::MockTerminalBackend;

    struct TestDimensions {
        columns: usize,
        lines: usize,
    }

    impl TestDimensions {
        fn new(columns: usize, lines: usize) -> Self {
            Self { columns, lines }
        }
    }

    impl Dimensions for TestDimensions {
        fn total_lines(&self) -> usize {
            self.lines
        }

        fn screen_lines(&self) -> usize {
            self.lines
        }

        fn columns(&self) -> usize {
            self.columns
        }
    }

    fn snapshot_with_blocks() -> SnapshotOwned {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = vec![
            BlockSnapshot {
                meta: BlockMeta {
                    id: "block-1".into(),
                    kind: BlockKind::Command,
                    ..BlockMeta::default()
                },
                start_line: 0,
                line_count: 4,
                cached_text: None,
                is_alt_screen: false,
            },
            BlockSnapshot {
                meta: BlockMeta {
                    id: "block-2".into(),
                    kind: BlockKind::Prompt,
                    ..BlockMeta::default()
                },
                start_line: 4,
                line_count: 1,
                cached_text: None,
                is_alt_screen: false,
            },
        ];

        snapshot
    }

    fn set_line(surface: &mut Surface, line: usize, text: &str) {
        for (column, ch) in text.chars().enumerate() {
            surface.grid_mut()[Line(line as i32)][Column(column)].c = ch;
        }
    }

    fn snapshot_with_block_text() -> SnapshotOwned {
        let dims = TestDimensions::new(16, 3);
        let mut surface = Surface::new(SurfaceConfig::default(), &dims);
        set_line(&mut surface, 0, "echo  hi");
        set_line(&mut surface, 1, "done");

        let mut snapshot = surface.snapshot_owned();
        snapshot.blocks = vec![
            BlockSnapshot {
                meta: BlockMeta {
                    id: "block-1".into(),
                    kind: BlockKind::Command,
                    ..BlockMeta::default()
                },
                start_line: 0,
                line_count: 2,
                cached_text: None,
                is_alt_screen: false,
            },
            BlockSnapshot {
                meta: BlockMeta {
                    id: "prompt".into(),
                    kind: BlockKind::Prompt,
                    ..BlockMeta::default()
                },
                start_line: 2,
                line_count: 1,
                cached_text: None,
                is_alt_screen: false,
            },
        ];

        snapshot
    }

    /// Ids of the blocks in the snapshot `terminal` shows, to tell snapshots
    /// apart.
    fn shown_block_ids(terminal: &Terminal) -> Vec<String> {
        terminal
            .snapshot()
            .view()
            .blocks()
            .iter()
            .map(|block| block.meta.id.clone())
            .collect()
    }

    fn terminal_with_snapshot(snapshot: SnapshotOwned) -> Terminal {
        Terminal::with_snapshot_for_test(7, snapshot)
    }

    /// A terminal over `backend` whose program reports no events.
    fn silent_terminal_over(backend: MockTerminalBackend) -> Terminal {
        let (program, _no_events) = StartedProgram::new(Box::new(backend));

        Terminal::with_program(7, Settings::default(), program)
    }

    /// Let `backend` take the shutdown a terminal sends when it is dropped,
    /// after the requests of `requests`.
    fn expect_shutdown(
        backend: &mut MockTerminalBackend,
        requests: &mut Sequence,
    ) {
        backend
            .expect_send()
            .withf(|request| matches!(request, TerminalRequest::Shutdown))
            .times(1)
            .in_sequence(requests)
            .return_const(());
    }

    #[test]
    fn new_terminal_shows_an_empty_screen_and_asks_its_program_for_nothing() {
        let mut backend = MockTerminalBackend::new();
        expect_shutdown(&mut backend, &mut Sequence::new());

        // A take the mock does not expect would fail the test.
        let terminal = silent_terminal_over(backend);

        assert!(shown_block_ids(&terminal).is_empty());
    }

    #[test]
    fn content_sync_takes_the_newest_snapshot_once_and_shows_it() {
        let mut backend = MockTerminalBackend::new();
        backend
            .expect_take_snapshot()
            .times(1)
            .return_once(|| Some(snapshot_with_blocks()));
        expect_shutdown(&mut backend, &mut Sequence::new());
        let mut terminal = silent_terminal_over(backend);

        terminal.handle(Event::ContentSync { id: 7 });

        assert_eq!(shown_block_ids(&terminal), ["block-1", "block-2"]);
        assert_eq!(terminal.blocks().len(), 2);
    }

    #[test]
    fn content_sync_without_a_new_snapshot_keeps_the_shown_one() {
        let mut waiting = Some(snapshot_with_blocks());
        let mut backend = MockTerminalBackend::new();
        backend
            .expect_take_snapshot()
            .times(2)
            .returning(move || waiting.take());
        expect_shutdown(&mut backend, &mut Sequence::new());
        let mut terminal = silent_terminal_over(backend);
        terminal.handle(Event::ContentSync { id: 7 });

        // A second notification for a snapshot that was already taken.
        terminal.handle(Event::ContentSync { id: 7 });

        assert_eq!(shown_block_ids(&terminal), ["block-1", "block-2"]);
    }

    #[test]
    fn write_event_passes_the_input_to_the_program() {
        let mut requests = Sequence::new();
        let mut backend = MockTerminalBackend::new();
        backend
            .expect_send()
            .withf(|request| {
                matches!(
                    request,
                    TerminalRequest::WriteBytes(bytes) if bytes == b"ls\n"
                )
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        backend
            .expect_send()
            .withf(|request| {
                matches!(request, TerminalRequest::ScrollDisplay(_))
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        expect_shutdown(&mut backend, &mut requests);
        let mut terminal = silent_terminal_over(backend);

        terminal.handle(Event::Write {
            id: 7,
            data: b"ls\n".to_vec(),
        });
    }

    #[test]
    fn snapshot_ready_becomes_a_content_sync_of_the_terminal() {
        let event =
            Event::<()>::from_terminal_event(7, TerminalEvent::SnapshotReady);

        assert!(matches!(event, Event::ContentSync { id: 7 }));
        assert_eq!(format!("{event:?}"), "Event::ContentSync 7");
    }

    /// The events a started program hands over with its backend are the
    /// ones the widget subscribes to.
    #[tokio::test]
    async fn events_of_the_program_reach_the_subscription_of_its_terminal() {
        let mut backend = MockTerminalBackend::new();
        expect_shutdown(&mut backend, &mut Sequence::new());
        let (program, program_events) = StartedProgram::new(Box::new(backend));
        let terminal: Terminal =
            Terminal::with_program(7, Settings::default(), program);
        let data = TerminalSubscriptionData {
            id: terminal.id,
            event_receiver: terminal.backend_event_rx.clone(),
        };
        let mut stream = terminal_subscription_stream::<()>(&data);

        program_events
            .send(TerminalEvent::SnapshotReady)
            .await
            .expect("the terminal listens");
        let event = stream.next().await;

        assert!(matches!(event, Some(Event::ContentSync { id: 7 })));
    }

    #[test]
    fn snapshot_borrows_what_the_terminal_shows() {
        let terminal = terminal_with_snapshot(snapshot_with_blocks());

        assert_eq!(shown_block_ids(&terminal), ["block-1", "block-2"]);
    }

    #[test]
    fn blocks_returns_cloned_block_metadata() {
        let snapshot = snapshot_with_blocks();
        let terminal = terminal_with_snapshot(snapshot);

        let blocks = terminal.blocks();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].meta.id, "block-1");
        assert_eq!(blocks[1].meta.kind, BlockKind::Prompt);
    }

    #[test]
    fn block_text_returns_aggregated_text_for_known_block() {
        let snapshot = snapshot_with_block_text();
        let terminal = terminal_with_snapshot(snapshot);

        let text = terminal.block_text("block-1");
        assert_eq!(text.as_deref(), Some("echo  hi\ndone"));
    }

    #[test]
    fn block_text_returns_none_for_missing_or_prompt_block() {
        let snapshot = snapshot_with_block_text();
        let terminal = terminal_with_snapshot(snapshot);

        assert!(terminal.block_text("prompt").is_none());
        assert!(terminal.block_text("missing").is_none());
    }
}
