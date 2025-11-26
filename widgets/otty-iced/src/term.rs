use crate::{engine, error};
use crate::bindings::{Binding, BindingAction, BindingsLayout, InputKind};
use crate::font::TermFont;
use crate::settings::{FontSettings, Settings, ThemeSettings};
use crate::theme::{ColorPalette, Theme};

use iced::futures::{SinkExt, Stream};
use iced::widget::canvas::Cache;
use otty_libterm::TerminalEvent;
use otty_libterm::surface::SnapshotOwned;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum Event {
    // BackendCall(u64, engine::Request),
    Redraw { id: u64, frame: Arc<SnapshotOwned> },
    Shutdown { id: u64 },
    Ignore
}

#[derive(Clone)]
pub enum Request {
    ChangeTheme(Box<ColorPalette>),
    ChangeFont(FontSettings),
    AddBindings(Vec<(Binding<InputKind>, BindingAction)>),
    Redraw(Arc<SnapshotOwned>)
    // ProxyToBackend(engine::Command),
}

pub struct Terminal {
    pub id: u64,
    pub(crate) font: TermFont,
    pub(crate) theme: Theme,
    pub(crate) cache: Cache,
    pub(crate) bindings: BindingsLayout,
    pub(crate) backend: engine::Engine,
    backend_event_rx: Arc<Mutex<Receiver<TerminalEvent>>>,
}

impl Terminal {
    pub fn new(id: u64, settings: Settings) -> error::Result<Self> {
        let (backend_event_tx, backend_event_rx) = mpsc::channel(100);
        let theme = Theme::new(settings.theme);
        let font = TermFont::new(settings.font);

        Ok(Self {
            id,
            font,
            theme,
            bindings: BindingsLayout::default(),
            cache: Cache::default(),
            backend: engine::Engine::new(
                id,
                backend_event_tx,
                settings.backend,
            )?,
            backend_event_rx: Arc::new(Mutex::new(backend_event_rx)),
        })
    }

    pub fn widget_id(&self) -> iced::widget::text_input::Id {
        iced::widget::text_input::Id::new(self.id.to_string())
    }

    pub fn subscription(&self) -> impl Stream<Item = Event> {
        let id = self.id;
        let event_receiver = self.backend_event_rx.clone();
        iced::stream::channel(100, move |mut output| async move {
            let mut shutdown = false;
            loop {
                let mut event_receiver = event_receiver.lock().await;
                match event_receiver.recv().await {
                    Some(event) => {
                        let event = match event {
                            TerminalEvent::ChildExit { status } => {
                                shutdown = true;
                                Event::Shutdown { id }
                            },
                            TerminalEvent::Frame { frame } => {
                                Event::Redraw { id, frame }
                            },
                            // TerminalEvent::Bell => {

                            // },
                            // TerminalEvent::TitleChanged { title } => {

                            // },
                            // TerminalEvent::ResetTitle => {

                            // }
                            _ => Event::Ignore
                        };

                        output
                            .send(event)
                            .await
                            .unwrap_or_else(|_| {
                                panic!("iced_term stream {}: sending BackendEventReceived event is failed", id)
                            });
                    },
                    None => {
                        if !shutdown {
                            panic!("iced_term stream {}: terminal event channel closed unexpected", id);
                        }
                    },
                }
            }
        })
    }

    pub fn handle(&mut self, cmd: Request) {
        // let mut action = Action::default();

        match cmd {
            Request::ChangeTheme(color_pallete) => {
                self.theme = Theme::new(ThemeSettings::new(color_pallete));
            },
            Request::ChangeFont(font_settings) => {
                self.font = TermFont::new(font_settings);
            },
            Request::AddBindings(bindings) => {
                self.bindings.add_bindings(bindings);
            },
            Request::Redraw(frame) => {
                self.backend.handle(engine::Request::SyncSurfaceContent(frame));
                self.cache.clear();
            }
        };
    }

    // fn sync_and_redraw(&mut self) {
    //     self.sync_font();
    //     self.backend.sync();
    //     self.redraw();
    // }

    // fn sync_font(&mut self) {
    //     self.font.sync();
    //     self.backend
    //         .handle(engine::Command::Resize(None, Some(self.font.measure)));
    // }

    fn redraw(&mut self) {
        self.cache.clear();
    }
}
