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
    BlockKey, BlockSnapshot, Point, SelectionType, SnapshotOwned,
};
use tokio::sync::Mutex;
use tokio::sync::mpsc::Receiver;

use crate::bindings::{Binding, BindingAction, BindingsLayout, InputKind};
use crate::block_layout::{self, BlockRect};
use crate::engine::MouseButton;
use crate::font::TermFont;
use crate::program::{self, StartedProgram};
use crate::settings::{
    BlockSelectionMode, FontSettings, Settings, ThemeSettings,
};
use crate::theme::{ColorPalette, Theme};
use crate::{engine, error};

/// Command that can be issued against a terminal block from the UI layer.
/// A block is named by its key ([`BlockKey`]), which never repeats in a
/// terminal.
#[derive(Clone, Debug)]
pub enum BlockCommand {
    /// Select the block with the provided key.
    Select(BlockKey),
    /// Select the block that is currently hovered by the pointer, if any.
    SelectHovered,
    /// Copy the currently highlighted grid selection to the clipboard.
    CopySelection,
    /// Clear any active block selection.
    ClearSelection,
    /// Bring the block with the provided key into view, also when it is
    /// off screen: nothing moves while the whole block is on screen;
    /// otherwise its start (the gap above it and its status line, when it
    /// has one) goes to the top edge of the pane, or as near as the history
    /// lets it, one step past the top of the history when its start shows
    /// only there (see [`Event::ScrollPastTheTop`]). A block taller than
    /// the pane starts at the top row.
    ScrollTo(BlockKey),
    /// Copy the output of the block: its text without its command lines
    /// (the prompt and the typed command).
    CopyContent(BlockKey),
    /// Copy the parsed command line without the prompt prefix.
    CopyCommand(BlockKey),
    /// Fold the block with the provided key, or unfold it when it is
    /// folded. Only a finished command block can be folded: a folded
    /// block shows its status line and its command lines, and hides its
    /// output.
    ToggleFold(BlockKey),
    /// Type the command of the block with the provided key at the prompt
    /// and press Enter. What is typed at the prompt is cleared first (Ctrl
    /// E, then Ctrl U). The command is pasted, as one paste when
    /// the shell asked for bracketed paste, so a command of several lines
    /// runs as one command. Nothing is written unless the shell waits at
    /// the prompt (its latest block is the prompt block): the text would go
    /// to the running program.
    RunAgain(BlockKey),
    /// Paste clipboard contents into the focused terminal.
    PasteClipboard,
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
    /// Show the history `offset` lines up from its bottom, or its top when
    /// it is shorter: where the scroll mark was dragged to, or where a jump
    /// to a block goes.
    ScrollToOffset {
        id: u64,
        offset: usize,
    },
    /// Show the top of the history and one step past it, where the first
    /// block starts at the top edge of the pane with the gap above it: the
    /// scroll mark dragged to the top of its way, or a jump to a block whose
    /// start shows only there. The terminal asks its program for the top of
    /// the history; the step shows once the view is there, when the gaps
    /// between the blocks cut the first block.
    ScrollPastTheTop {
        id: u64,
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
        /// Size of the area the text fills: the pane without the inset at
        /// its left and its right.
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
        block_key: BlockKey,
    },
    BlockCopied {
        id: u64,
        block_key: BlockKey,
    },
    BlockSelectionCleared {
        id: u64,
    },
    /// The pointer moved onto another block, or off every block
    /// (`block_key` is `None`). The terminal keeps it for its host, see
    /// [`Terminal::hovered_block_key`].
    BlockHovered {
        id: u64,
        block_key: Option<BlockKey>,
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
            ScrollToOffset {
                id,
                offset,
            } => f.write_fmt(format_args!("Event::ScrollToOffset id: {id}, offset: {offset}")),
            ScrollPastTheTop { id } => f.write_fmt(format_args!("Event::ScrollPastTheTop id: {id}")),
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
            BlockSelected { id, block_key } => {
                f.write_fmt(format_args!("Event::BlockSelected id: {id}, block_key: {block_key:?}"))
            }
            BlockCopied { id, block_key } => {
                f.write_fmt(format_args!("Event::BlockCopied id: {id}, block_key: {block_key:?}"))
            }
            BlockSelectionCleared { id } => {
                f.write_fmt(format_args!("Event::BlockSelectionCleared id: {id}"))
            }
            BlockHovered { id, block_key } => {
                f.write_fmt(format_args!("Event::BlockHovered id: {id}, block_key: {block_key:?}"))
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
            ScrollToOffset { id, .. } => id,
            ScrollPastTheTop { id } => id,
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
            BlockHovered { id, .. } => id,
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
    program_event_rx: Arc<Mutex<Receiver<TerminalEvent>>>,
    /// The block under the pointer, as the view last reported it.
    hovered_block_key: Option<BlockKey>,
}

impl<T> Terminal<T> {
    /// Start the program `settings` ask for and build its terminal.
    pub fn new(id: u64, settings: Settings) -> error::Result<Self> {
        let started = program::start_program(&settings.engine)?;

        Ok(Self::with_program(id, settings, started))
    }

    /// Build the terminal of a program that is already started.
    ///
    /// [`Terminal::new`] starts the program itself and ends here. A host
    /// that starts programs its own way, or a test that puts a mock in
    /// place of one, comes here directly. The session of `settings` is not
    /// started again; of its engine settings only the size is read.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_ui_term::settings::Settings;
    /// use otty_ui_term::{
    ///     SnapshotOwned, StartedProgram, Terminal, TerminalProgram,
    ///     TerminalRequest,
    /// };
    ///
    /// /// A program that ignores input and never has a new screen.
    /// struct Idle;
    ///
    /// impl TerminalProgram for Idle {
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
            engine: engine_settings,
            interaction,
        } = settings;
        let (program, program_events) = program.into_parts();

        Self {
            id,
            widget_id: iced::widget::Id::unique(),
            font: TermFont::new(font),
            theme: Theme::new(theme),
            bindings: BindingsLayout::default(),
            cache: Cache::default(),
            engine: engine::Engine::new(program, engine_settings.size),
            block_selection_mode: interaction.block_selection_mode(),
            program_event_rx: Arc::new(Mutex::new(program_events)),
            hovered_block_key: None,
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

    /// Return the key of the block under the pointer, as the view of the
    /// terminal last reported it with [`Event::BlockHovered`]; `None` while
    /// the pointer is over no block or away from the pane. A layer drawn
    /// over the terminal shows its controls for this block.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_ui_term::settings::Settings;
    /// use otty_ui_term::{
    ///     BlockKey, Event, SnapshotOwned, StartedProgram, Terminal,
    ///     TerminalProgram, TerminalRequest,
    /// };
    ///
    /// struct Idle;
    ///
    /// impl TerminalProgram for Idle {
    ///     fn send(&self, _request: TerminalRequest) {}
    ///
    ///     fn take_snapshot(&self) -> Option<SnapshotOwned> {
    ///         None
    ///     }
    /// }
    ///
    /// let (program, _events) = StartedProgram::new(Box::new(Idle));
    /// let mut terminal: Terminal =
    ///     Terminal::with_program(1, Settings::default(), program);
    ///
    /// terminal.handle(Event::BlockHovered {
    ///     id: 1,
    ///     block_key: Some(BlockKey::new(3)),
    /// });
    ///
    /// assert_eq!(terminal.hovered_block_key(), Some(BlockKey::new(3)));
    /// ```
    pub fn hovered_block_key(&self) -> Option<BlockKey> {
        self.hovered_block_key
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

    /// Return the blocks of the latest snapshot that have a line in the
    /// pane, as the pane shows them.
    ///
    /// The widget puts a gap above and below the lines of every block, and
    /// the bottom row stays at the bottom edge, so the space pushes the top
    /// rows over the top edge of the pane. Here a row pushed wholly over
    /// the edge is above the pane: a block loses it from its lines on
    /// screen. A row that still shows in part at the edge keeps its block
    /// in the pane, so a block counts as on screen while any part of one of
    /// its rows is; its gap alone does not. Every row over the edge, wholly
    /// or in part, counts among the lines above when it holds text. The
    /// status line stays on screen while at least half of its row is below
    /// the edge, so a few cut pixels bring no sticky header over it. One
    /// step past the top of the history (see
    /// [`Event::ScrollPastTheTop`]) the rows start at the top edge instead,
    /// and the space pushes the bottom rows below the line grid: those
    /// rows, even when cut in part, count as below the pane, and a block
    /// loses them from its lines on screen. A block with no line left in
    /// the pane is left out. A layer over the terminal decides with these
    /// blocks what is on screen, such as which statuses to show and whether
    /// the start of the top block is above the pane.
    ///
    /// # Examples
    ///
    /// ```
    /// use otty_ui_term::settings::Settings;
    /// use otty_ui_term::{
    ///     SnapshotOwned, StartedProgram, Terminal, TerminalProgram,
    ///     TerminalRequest,
    /// };
    ///
    /// struct Idle;
    ///
    /// impl TerminalProgram for Idle {
    ///     fn send(&self, _request: TerminalRequest) {}
    ///
    ///     fn take_snapshot(&self) -> Option<SnapshotOwned> {
    ///         None
    ///     }
    /// }
    ///
    /// let (program, _events) = StartedProgram::new(Box::new(Idle));
    /// let terminal: Terminal =
    ///     Terminal::with_program(1, Settings::default(), program);
    ///
    /// assert!(terminal.blocks_in_pane().is_empty());
    /// ```
    pub fn blocks_in_pane(&self) -> Vec<BlockSnapshot> {
        let rows = self.engine.row_layout();

        rows.blocks_in_pane(self.engine.snapshot().view().blocks())
    }

    /// Return where the blocks of the latest snapshot with a row on screen
    /// lie in a pane at `layout_pos` of `layout_size`, as the widget draws
    /// them. Every rectangle runs from the left edge of the pane to the
    /// right one and takes the rows of its block with the space the widget
    /// puts around them: the gap above the first row and, under the last
    /// row, the gap and the line that separates the block from the next
    /// one. The space pushes the top rows over the top edge, so a rectangle
    /// may start above it; one step past the top of the history (see
    /// [`Event::ScrollPastTheTop`]) the rows start at the top edge instead,
    /// and a rectangle may end below the bottom edge. A layer over the
    /// terminal puts the status lines of the blocks there
    /// ([`BlockRect::status_line`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use iced::{Point, Size};
    /// use otty_ui_term::settings::Settings;
    /// use otty_ui_term::{
    ///     SnapshotOwned, StartedProgram, Terminal, TerminalProgram,
    ///     TerminalRequest,
    /// };
    ///
    /// struct Idle;
    ///
    /// impl TerminalProgram for Idle {
    ///     fn send(&self, _request: TerminalRequest) {}
    ///
    ///     fn take_snapshot(&self) -> Option<SnapshotOwned> {
    ///         None
    ///     }
    /// }
    ///
    /// let (program, _events) = StartedProgram::new(Box::new(Idle));
    /// let terminal: Terminal =
    ///     Terminal::with_program(1, Settings::default(), program);
    ///
    /// let rects = terminal.block_rects(Point::ORIGIN, Size::new(800.0, 400.0));
    ///
    /// assert!(rects.is_empty());
    /// ```
    pub fn block_rects(
        &self,
        layout_pos: iced::Point,
        layout_size: Size<f32>,
    ) -> Vec<BlockRect> {
        let rows = self.engine.row_layout();

        block_layout::block_rects(
            rows,
            &self.engine.snapshot().view(),
            layout_pos,
            layout_size,
        )
    }

    pub fn subscription(&self) -> Subscription<Event<T>>
    where
        T: Send + 'static,
    {
        let data = TerminalSubscriptionData {
            id: self.id,
            event_receiver: self.program_event_rx.clone(),
        };

        Subscription::run_with(data, terminal_subscription_stream)
    }

    /// Handle `event` of the widget or of the program behind it.
    ///
    /// The step past the top of the history changes the rows at once,
    /// without a new snapshot of the program, so the screen is drawn again
    /// when an event takes or leaves it.
    pub fn handle(&mut self, event: Event<T>) {
        use Event::*;

        let was_past_the_top = self.engine.is_past_the_top();

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
            ScrollToOffset { offset, .. } => {
                self.engine.scroll_to_offset(offset)
            },
            ScrollPastTheTop { .. } => self.engine.scroll_past_the_top(),
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
            BlockHovered { block_key, .. } => {
                self.hovered_block_key = block_key
            },
            _ => {},
        }

        if self.engine.is_past_the_top() != was_past_the_top {
            self.cache.clear();
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

    /// Add key and mouse bindings of the host. A binding equal to one the
    /// terminal has (the same input, modifiers and modes) replaces it; any
    /// other one is matched before the built-in bindings. So a binding that
    /// holds only in some modes, such as the normal screen, takes the input
    /// there and leaves it to the built-in binding of the same keys in the
    /// other modes.
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
        Self::with_snapshots_for_test(id, vec![snapshot])
    }

    /// Build a terminal that already shows the first of `snapshots`: its
    /// program, a mock, takes any request and has the next one of
    /// `snapshots` ready on every [`Event::ContentSync`].
    pub(crate) fn with_snapshots_for_test(
        id: u64,
        snapshots: Vec<SnapshotOwned>,
    ) -> Self {
        let mut waiting = snapshots.into_iter();
        let mut program = crate::program::MockTerminalProgram::new();
        program.expect_send().return_const(());
        program
            .expect_take_snapshot()
            .returning(move || waiting.next());
        let (started, _no_events) = StartedProgram::new(Box::new(program));

        let mut terminal = Self::with_program(id, Settings::default(), started);
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
                                "iced_term stream {}: passing an event of the program to the widget failed",
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
    use otty_libterm::surface::{BlockKind, BlockMeta};

    use super::*;
    use crate::program::MockTerminalProgram;
    use crate::testing::{HistoryBlock, history_snapshot};

    fn snapshot_with_blocks() -> SnapshotOwned {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = vec![
            BlockSnapshot {
                key: BlockKey::new(1),
                meta: BlockMeta {
                    id: "block-1".into(),
                    kind: BlockKind::Command,
                    ..BlockMeta::default()
                },
                start_line: 0,
                line_count: 4,
                cached_text: None,
                has_status_line: false,
                is_status_line_visible: false,
                ..BlockSnapshot::default()
            },
            BlockSnapshot {
                key: BlockKey::new(2),
                meta: BlockMeta {
                    id: "block-2".into(),
                    kind: BlockKind::Prompt,
                    ..BlockMeta::default()
                },
                start_line: 4,
                line_count: 1,
                cached_text: None,
                has_status_line: false,
                is_status_line_visible: false,
                ..BlockSnapshot::default()
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

    /// A terminal over `program`, which reports no events.
    fn silent_terminal_over(program: MockTerminalProgram) -> Terminal {
        let (started, _no_events) = StartedProgram::new(Box::new(program));

        Terminal::with_program(7, Settings::default(), started)
    }

    /// Let `program` take the shutdown a terminal sends when it is dropped,
    /// after the requests of `requests`.
    fn expect_shutdown(
        program: &mut MockTerminalProgram,
        requests: &mut Sequence,
    ) {
        program
            .expect_send()
            .withf(|request| matches!(request, TerminalRequest::Shutdown))
            .times(1)
            .in_sequence(requests)
            .return_const(());
    }

    #[test]
    fn new_terminal_shows_an_empty_screen_and_asks_its_program_for_nothing() {
        let mut program = MockTerminalProgram::new();
        expect_shutdown(&mut program, &mut Sequence::new());

        // A take the mock does not expect would fail the test.
        let terminal = silent_terminal_over(program);

        assert!(shown_block_ids(&terminal).is_empty());
    }

    #[test]
    fn content_sync_takes_the_newest_snapshot_once_and_shows_it() {
        let mut program = MockTerminalProgram::new();
        program
            .expect_take_snapshot()
            .times(1)
            .return_once(|| Some(snapshot_with_blocks()));
        expect_shutdown(&mut program, &mut Sequence::new());
        let mut terminal = silent_terminal_over(program);

        terminal.handle(Event::ContentSync { id: 7 });

        assert_eq!(shown_block_ids(&terminal), ["block-1", "block-2"]);
        assert_eq!(terminal.blocks().len(), 2);
    }

    #[test]
    fn content_sync_without_a_new_snapshot_keeps_the_shown_one() {
        let mut waiting = Some(snapshot_with_blocks());
        let mut program = MockTerminalProgram::new();
        program
            .expect_take_snapshot()
            .times(2)
            .returning(move || waiting.take());
        expect_shutdown(&mut program, &mut Sequence::new());
        let mut terminal = silent_terminal_over(program);
        terminal.handle(Event::ContentSync { id: 7 });

        // A second notification for a snapshot that was already taken.
        terminal.handle(Event::ContentSync { id: 7 });

        assert_eq!(shown_block_ids(&terminal), ["block-1", "block-2"]);
    }

    #[test]
    fn write_event_passes_the_input_to_the_program() {
        let mut requests = Sequence::new();
        let mut program = MockTerminalProgram::new();
        program
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
        program
            .expect_send()
            .withf(|request| {
                matches!(request, TerminalRequest::ScrollDisplay(_))
            })
            .times(1)
            .in_sequence(&mut requests)
            .return_const(());
        expect_shutdown(&mut program, &mut requests);
        let mut terminal = silent_terminal_over(program);

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

    /// The events a started program hands over are the ones the widget
    /// subscribes to.
    #[tokio::test]
    async fn events_of_the_program_reach_the_subscription_of_its_terminal() {
        let mut program = MockTerminalProgram::new();
        expect_shutdown(&mut program, &mut Sequence::new());
        let (started, program_events) = StartedProgram::new(Box::new(program));
        let terminal: Terminal =
            Terminal::with_program(7, Settings::default(), started);
        let data = TerminalSubscriptionData {
            id: terminal.id,
            event_receiver: terminal.program_event_rx.clone(),
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
    fn the_terminal_keeps_the_block_its_view_reported_under_the_pointer() {
        let mut terminal = terminal_with_snapshot(snapshot_with_blocks());
        assert_eq!(terminal.hovered_block_key(), None);

        terminal.handle(Event::BlockHovered {
            id: 7,
            block_key: Some(BlockKey::new(1)),
        });
        let over_the_block = terminal.hovered_block_key();
        terminal.handle(Event::BlockHovered {
            id: 7,
            block_key: None,
        });

        assert_eq!(over_the_block, Some(BlockKey::new(1)));
        assert_eq!(terminal.hovered_block_key(), None);
    }

    /// A new terminal after `ls` and `pwd`, with rows of 16 px: 4 lines in
    /// a pane of 4 rows, so the history can not scroll, and the gaps push
    /// the status row of `ls` and most of its command row over the top
    /// edge.
    fn terminal_with_a_short_history_cut_at_the_top() -> Terminal {
        let snapshot = history_snapshot(
            4,
            &[
                HistoryBlock::command(BlockKey::new(1), 2),
                HistoryBlock::command(BlockKey::new(2), 1),
                HistoryBlock::plain(BlockKey::new(3), 1),
            ],
            0,
        );
        let mut terminal = terminal_with_snapshot(snapshot);
        terminal.engine.resize(None, Some(Size::new(8.0, 16.0)));

        terminal
    }

    /// The sticky header and the status lines of the app follow the step
    /// past the top: `ls` comes into the pane whole, with its status line
    /// on the gap above it.
    #[test]
    fn the_wheel_up_past_the_top_brings_the_first_block_into_the_pane_whole() {
        let mut terminal = terminal_with_a_short_history_cut_at_the_top();
        let first_before = terminal.blocks_in_pane()[0].clone();

        terminal.handle(Event::Scroll { id: 7, delta: 1 });

        // Before the step `ls` shows only a part of its command row: it is
        // in the pane, without its status line.
        assert_eq!(first_before.key, BlockKey::new(1));
        assert!(!first_before.is_status_line_visible);
        let blocks = terminal.blocks_in_pane();
        assert_eq!(blocks[0].key, BlockKey::new(1));
        assert!(blocks[0].is_status_line_visible);
        let rects =
            terminal.block_rects(iced::Point::ORIGIN, Size::new(320.0, 64.0));
        let status_line = rects[0].status_line().expect("a status line");
        assert_eq!((status_line.y, status_line.height), (0.0, 22.0));
    }

    #[test]
    fn a_scroll_past_the_top_brings_the_first_block_into_the_pane_whole() {
        let mut terminal = terminal_with_a_short_history_cut_at_the_top();

        terminal.handle(Event::ScrollPastTheTop { id: 7 });

        let blocks = terminal.blocks_in_pane();
        assert_eq!(blocks[0].key, BlockKey::new(1));
        assert!(blocks[0].is_status_line_visible);
    }
}
