use std::collections::VecDeque;

use iced::alignment::Vertical;
use iced::widget::canvas::{
    self, Canvas, Frame, Geometry, Path, Stroke, Text as CanvasText,
};
use iced::widget::{column, container, stack, text};
use iced::{
    Alignment, Element, Length, Point, Rectangle, Size, Subscription, Task,
    Theme,
};
use otty_libterm::surface::BlockKind;
use otty_ui_term::settings::{
    BackendSettings, LocalSessionOptions, SessionKind, Settings,
};
use otty_ui_term::{
    BlockCommand, BlockRect, BlockUiMode, Terminal, TerminalView, block_rects,
    compute_action_button_geometry,
};

const MAX_LOG_ENTRIES: usize = 5;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .antialiasing(false)
        .window_size(Size {
            width: 1200.0,
            height: 720.0,
        })
        .title(App::title)
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Terminal(otty_ui_term::Event),
    OverlayCopy(String),
}

struct App {
    terminal: Terminal,
    log: VecDeque<BlockLogEntry>,
    selected_block: Option<String>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let settings = terminal_settings();
        let terminal = Terminal::new(0, settings.clone())
            .expect("failed to create terminal")
            .with_block_ui_mode(BlockUiMode::ExternalOverlay);

        (
            App {
                terminal,
                log: VecDeque::new(),
                selected_block: None,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        String::from("OTTY block overlay example")
    }

    fn subscription(&self) -> Subscription<Message> {
        self.terminal.subscription().map(Message::Terminal)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Terminal(event) => self.handle_terminal_event(event),
            Message::OverlayCopy(block_id) => TerminalView::command(
                self.terminal.widget_id().clone(),
                BlockCommand::Copy(block_id),
            ),
        }
    }

    fn view(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let terminal_view =
            TerminalView::show(&self.terminal).map(Message::Terminal);
        let overlay = BlockOverlay::new(
            self.terminal.snapshot_arc(),
            self.selected_block.clone(),
        );
        let overlay_layer: Element<'_, Message, Theme, iced::Renderer> =
            container(
                Canvas::new(overlay)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let stacked = stack![terminal_view, overlay_layer]
            .width(Length::Fill)
            .height(Length::Fill);

        let log_view = self.view_log();

        column![stacked, log_view].spacing(16).padding(16).into()
    }

    fn view_log(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let header = text("Recent block events").size(18);
        let log_entries = self
            .log
            .iter()
            .rev()
            .fold(column![header].spacing(6), |col, entry| {
                col.push(text(entry.to_string()).size(16))
            });

        container(log_entries)
            .width(Length::Fill)
            .style(|theme: &Theme| iced::widget::container::Style {
                background: Some(
                    theme.extended_palette().background.weak.color.into(),
                ),
                ..Default::default()
            })
            .padding(12)
            .into()
    }

    fn handle_terminal_event(
        &mut self,
        event: otty_ui_term::Event,
    ) -> Task<Message> {
        match event {
            otty_ui_term::Event::BlockSelected { block_id, .. } => {
                self.selected_block = Some(block_id.clone());
                self.push_log(BlockEventKind::Selected, block_id);
                Task::none()
            },
            otty_ui_term::Event::BlockCopied { block_id, .. } => {
                self.push_log(BlockEventKind::Copied, block_id);
                Task::none()
            },
            other => {
                self.terminal.handle(other);
                Task::none()
            },
        }
    }

    fn push_log(&mut self, kind: BlockEventKind, block_id: String) {
        if self.log.len() == MAX_LOG_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(BlockLogEntry { kind, block_id });
    }
}

fn terminal_settings() -> Settings {
    let shell =
        std::env::var("SHELL").unwrap_or_else(|_| String::from("/bin/sh"));
    let session_options = LocalSessionOptions::default().with_program(&shell);
    let session = SessionKind::from_local_options(session_options);
    Settings {
        backend: BackendSettings::default().with_session(session),
        ..Default::default()
    }
}

#[derive(Clone, Debug)]
struct BlockLogEntry {
    kind: BlockEventKind,
    block_id: String,
}

impl std::fmt::Display for BlockLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            BlockEventKind::Selected => {
                write!(f, "selected  - {}", self.block_id)
            },
            BlockEventKind::Copied => {
                write!(f, "copied    - {}", self.block_id)
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum BlockEventKind {
    Selected,
    Copied,
}

struct BlockOverlay {
    snapshot: otty_ui_term::SnapshotArc,
    selected_block: Option<String>,
}

impl BlockOverlay {
    fn new(
        snapshot: otty_ui_term::SnapshotArc,
        selected_block: Option<String>,
    ) -> Self {
        Self {
            snapshot,
            selected_block,
        }
    }

    fn block_rects_for_bounds(
        &self,
        bounds: Rectangle,
    ) -> (Vec<BlockRect>, f32) {
        let view = self.snapshot.view();
        let rows = view.size.screen_lines.max(1) as f32;
        let cell_height = bounds.height / rows;
        let rects = block_rects(
            &view,
            Point::new(0.0, 0.0),
            Size::new(bounds.width, bounds.height),
            cell_height,
        );
        (rects, cell_height)
    }
}

impl canvas::Program<Message> for BlockOverlay {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (rects, cell_height) = self.block_rects_for_bounds(bounds);
        let mut button_geometries = Vec::new();
        for rect in &rects {
            if rect.kind == BlockKind::Prompt {
                continue;
            }
            if let Some(button) =
                compute_action_button_geometry(rect, cell_height)
            {
                button_geometries.push(button);
            }
            let block_path = Path::rectangle(
                Point::new(rect.rect.x, rect.rect.y),
                Size::new(rect.rect.width, rect.rect.height),
            );
            if self.selected_block.as_deref() == Some(rect.block_id.as_str()) {
                let mut fill = theme.extended_palette().primary.strong.color;
                fill.a = 0.08;
                frame.fill(&block_path, fill);
            }
            let mut stroke_color =
                theme.extended_palette().primary.strong.color;
            stroke_color.a = 0.5;
            frame.stroke(
                &block_path,
                Stroke::default().with_color(stroke_color).with_width(1.0),
            );
        }

        for button in button_geometries {
            let button_path = Path::rectangle(
                Point::new(button.rect.x, button.rect.y),
                Size::new(button.rect.width, button.rect.height),
            );
            let mut bg = theme.extended_palette().primary.strong.color;
            bg.a = 0.15;
            frame.fill(&button_path, bg);

            frame.fill_text(CanvasText {
                content: "C".to_string(),
                position: Point::new(
                    button.rect.x + (button.rect.width / 2.0),
                    button.rect.y + (button.rect.height / 2.0),
                ),
                align_x: Alignment::Center.into(),
                align_y: Vertical::Center,
                color: theme.palette().text,
                ..Default::default()
            });
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            )) => {
                let position = cursor.position_in(bounds)?;
                let (rects, cell_height) = self.block_rects_for_bounds(bounds);
                for rect in &rects {
                    if rect.kind == BlockKind::Prompt {
                        continue;
                    }
                    if let Some(button) =
                        compute_action_button_geometry(rect, cell_height)
                    {
                        if button.rect.contains(position) {
                            return Some(
                                canvas::Action::publish(Message::OverlayCopy(
                                    button.block_id.clone(),
                                ))
                                .and_capture(),
                            );
                        }
                    }
                }
                None
            },
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        if let Some(position) = cursor.position_in(bounds) {
            let (rects, cell_height) = self.block_rects_for_bounds(bounds);
            for rect in &rects {
                if rect.kind == BlockKind::Prompt {
                    continue;
                }
                if let Some(button) =
                    compute_action_button_geometry(rect, cell_height)
                {
                    if button.rect.contains(position) {
                        return iced::mouse::Interaction::Pointer;
                    }
                }
            }
        }
        iced::mouse::Interaction::Idle
    }
}
