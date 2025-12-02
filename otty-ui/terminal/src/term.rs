use std::fmt::Debug;
use std::process::ExitStatus;
use std::sync::Arc;

use iced::Size;
use iced::futures::{SinkExt, Stream};
use iced::widget::canvas::Cache;
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
    pub(crate) font: TermFont,
    pub(crate) theme: Theme,
    pub(crate) cache: Cache,
    pub(crate) bindings: BindingsLayout,
    pub(crate) engine: engine::Engine,
    backend_event_rx: Arc<Mutex<Receiver<TerminalEvent>>>,
}

impl Terminal {
    pub fn new(id: u64, settings: Settings) -> error::Result<Self> {
        let (backend_event_tx, backend_event_rx) = mpsc::channel(100);
        let theme = Theme::new(settings.theme);
        let font = TermFont::new(settings.font);
        let engine = engine::Engine::new(backend_event_tx, settings.backend)?;

        Ok(Self {
            id,
            font,
            theme,
            bindings: BindingsLayout::default(),
            cache: Cache::default(),
            engine,
            backend_event_rx: Arc::new(Mutex::new(backend_event_rx)),
        })
    }

    pub fn widget_id(&self) -> iced::widget::text_input::Id {
        iced::widget::text_input::Id::new(self.id.to_string())
    }

    /// Borrow the latest render snapshot shared by the terminal engine.
    ///
    /// The returned [`SnapshotArc`] reflects the most recent `ContentSync`
    /// event handled by this terminal instance.
    pub fn snapshot_arc(&self) -> SnapshotArc {
        self.engine.snapshot()
    }

    /// Return a copy of block metadata captured in the latest snapshot.
    pub fn blocks(&self) -> Vec<BlockSnapshot> {
        let snapshot = self.engine.snapshot();
        snapshot.view().blocks().to_vec()
    }

    pub fn subscription(&self) -> impl Stream<Item = Event> + Send + 'static {
        let id = self.id;
        let event_receiver = self.backend_event_rx.clone();
        iced::stream::channel(1000, move |mut output| async move {
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
                                panic!("iced_term stream {}: sending BackendEventReceived event is failed", id)
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
    }

    pub fn handle(&mut self, event: Event) {
        use Event::*;

        match event {
            Redraw { .. } => self.cache.clear(),
            ContentSync { frame, .. } => self.content_sync(frame),
            Write { data, .. } => self.engine.write(data),
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

    fn content_sync(&mut self, frame: Arc<SnapshotOwned>) {
        self.engine.sync_snapshot(frame);
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use otty_libterm::surface::{BlockKind, BlockMetaPublic};

    fn snapshot_with_blocks() -> Arc<SnapshotOwned> {
        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks = vec![
            BlockSnapshot {
                id: "block-1".into(),
                meta: BlockMetaPublic {
                    id: "block-1".into(),
                    kind: BlockKind::Command,
                    ..BlockMetaPublic::default()
                },
                start_line: 0,
                line_count: 4,
                is_alt_screen: false,
            },
            BlockSnapshot {
                id: "block-2".into(),
                meta: BlockMetaPublic {
                    id: "block-2".into(),
                    kind: BlockKind::Prompt,
                    ..BlockMetaPublic::default()
                },
                start_line: 4,
                line_count: 1,
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
            font,
            theme,
            cache: Cache::default(),
            bindings: BindingsLayout::default(),
            engine: engine::Engine::with_snapshot_for_test(
                Arc::clone(&snapshot),
                settings.backend.size,
            ),
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
        assert_eq!(blocks[0].id, "block-1");
        assert_eq!(blocks[1].meta.kind, BlockKind::Prompt);
    }
}
