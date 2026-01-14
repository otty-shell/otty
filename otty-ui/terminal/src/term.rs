use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::process::ExitStatus;
use std::sync::Arc;

use iced::Size;
use iced::Subscription;
use iced::futures::stream::BoxStream;
use iced::futures::{SinkExt, StreamExt};
use iced::widget::canvas::Cache;
use log::debug;
use otty_libterm::SnapshotArc;
use otty_libterm::TerminalEvent;
use otty_libterm::surface::{
    BlockSnapshot, Point, SelectionType, SnapshotOwned,
};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Receiver};

use crate::bindings::{Binding, BindingAction, BindingsLayout, InputKind};
use crate::engine::MouseButton;
use crate::font::TermFont;
use crate::settings::{FontSettings, Settings, ThemeSettings};
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
pub enum Event {
    Redraw {
        id: u64,
    },
    ContentSync {
        id: u64,
        frame: Arc<SnapshotOwned>,
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
    Ignore {
        id: u64,
    },
}

impl Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Event::*;

        match self {
            Redraw { id } => f.write_fmt(format_args!("Event::Redraw {id}")),
            ContentSync {
                id,
                ..
            } => f.write_fmt(format_args!("Event::ContentSync {id}")),
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
            Ignore { id } => f.write_fmt(format_args!("Event::Ignore id: {id}")),
        }
    }
}

impl Event {
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
            Ignore { id } => id,
        }
    }

    fn from_terminal_event(id: u64, event: TerminalEvent) -> Event {
        match event {
            TerminalEvent::ChildExit { status } => Event::Shutdown {
                id,
                exit_status: status,
            },
            TerminalEvent::Frame { frame } => Event::ContentSync { id, frame },
            TerminalEvent::TitleChanged { title } => {
                Event::TitleChanged { id, title }
            },
            TerminalEvent::ResetTitle => Event::ResetTitle { id },
            _ => Event::Ignore { id },
        }
    }
}

pub struct Terminal {
    pub id: u64,
    widget_id: iced::widget::Id,
    pub(crate) font: TermFont,
    pub(crate) theme: Theme,
    pub(crate) cache: Cache,
    pub(crate) bindings: BindingsLayout,
    pub(crate) engine: engine::Engine,
    block_ui_mode: BlockUiMode,
    backend_event_rx: Arc<Mutex<Receiver<TerminalEvent>>>,
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

impl Terminal {
    pub fn new(id: u64, settings: Settings) -> error::Result<Self> {
        let (backend_event_tx, backend_event_rx) = mpsc::channel(100);
        let theme = Theme::new(settings.theme);
        let font = TermFont::new(settings.font);
        let engine = engine::Engine::new(backend_event_tx, settings.backend)?;

        Ok(Self {
            id,
            widget_id: iced::widget::Id::unique(),
            font,
            theme,
            bindings: BindingsLayout::default(),
            cache: Cache::default(),
            engine,
            block_ui_mode: BlockUiMode::Internal,
            backend_event_rx: Arc::new(Mutex::new(backend_event_rx)),
        })
    }

    pub fn widget_id(&self) -> &iced::widget::Id {
        &self.widget_id
    }

    /// Borrow the latest render snapshot shared by the terminal engine.
    ///
    /// The returned [`SnapshotArc`] reflects the most recent `ContentSync`
    /// event handled by this terminal instance. Clones of this [`Arc`] remain
    /// valid even after newer frames arrive, but they keep pointing to the
    /// captured frame and never update in place.
    pub fn snapshot_arc(&self) -> SnapshotArc {
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

    /// Return textual contents for the provided block id from the latest frame.
    ///
    /// Returns `None` when the block is not found or has no textual payload
    /// (e.g. prompt blocks).
    pub fn block_text(&self, block_id: &str) -> Option<String> {
        let snapshot = self.engine.snapshot();
        snapshot.block_text(block_id)
    }

    /// Return the prompt/input line for the provided block id from the latest
    /// frame, if available.
    pub fn block_prompt_text(&self, block_id: &str) -> Option<String> {
        let snapshot = self.engine.snapshot();
        snapshot.block_prompt_text(block_id)
    }

    pub fn subscription(&self) -> Subscription<Event> {
        let data = TerminalSubscriptionData {
            id: self.id,
            event_receiver: self.backend_event_rx.clone(),
        };

        Subscription::run_with(data, terminal_subscription_stream)
    }

    pub fn handle(&mut self, event: Event) {
        use Event::*;

        match event {
            Redraw { .. } => self.cache.clear(),
            ContentSync { frame, .. } => self.content_sync(frame),
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
        bindings: Vec<(Binding<InputKind>, BindingAction)>,
    ) {
        self.bindings.add_bindings(bindings);
        self.cache.clear();
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

    fn content_sync(&mut self, frame: Arc<SnapshotOwned>) {
        self.engine.sync_snapshot(frame);
        self.cache.clear();
    }
}

fn terminal_subscription_stream(
    data: &TerminalSubscriptionData,
) -> BoxStream<'static, Event> {
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
    use super::*;
    use crate::settings::Settings;
    use otty_libterm::surface::{
        BlockKind, BlockMeta, Column, Dimensions, Line, Surface, SurfaceConfig,
        SurfaceModel,
    };

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

    fn snapshot_with_blocks() -> Arc<SnapshotOwned> {
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
        Arc::new(snapshot)
    }

    fn set_line(surface: &mut Surface, line: usize, text: &str) {
        for (column, ch) in text.chars().enumerate() {
            surface.grid_mut()[Line(line as i32)][Column(column)].c = ch;
        }
    }

    fn snapshot_with_block_text() -> Arc<SnapshotOwned> {
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
        Arc::new(snapshot)
    }

    fn terminal_with_snapshot(snapshot: Arc<SnapshotOwned>) -> Terminal {
        let settings = Settings::default();
        let theme = Theme::new(settings.theme.clone());
        let font = TermFont::new(settings.font.clone());
        let (_, backend_event_rx) = mpsc::channel(1);
        Terminal {
            id: 7,
            widget_id: iced::widget::Id::unique(),
            font,
            theme,
            cache: Cache::default(),
            bindings: BindingsLayout::default(),
            engine: engine::Engine::with_snapshot_for_test(
                Arc::clone(&snapshot),
                settings.backend.size,
            ),
            block_ui_mode: BlockUiMode::Internal,
            backend_event_rx: Arc::new(Mutex::new(backend_event_rx)),
        }
    }

    #[test]
    fn snapshot_arc_returns_latest_frame() {
        let snapshot = snapshot_with_blocks();
        let terminal = terminal_with_snapshot(Arc::clone(&snapshot));

        let latest = terminal.snapshot_arc();
        assert!(Arc::ptr_eq(&latest, &snapshot));
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
