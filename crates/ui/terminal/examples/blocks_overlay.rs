//! An app layer over the terminal. The terminal paints what lies under
//! its text: the backgrounds and the lines of the blocks. This example
//! puts a canvas above it and draws a frame and a copy button on every
//! command block, at the places the terminal reports with
//! `Terminal::block_rects`. The button copies the output of its block.

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
    EngineSettings, LocalSessionOptions, SessionKind, Settings,
};
use otty_ui_term::{BlockCommand, BlockKey, BlockRect, Terminal, TerminalView};

const MAX_LOG_ENTRIES: usize = 5;
/// Side of the square copy button, in pixels.
const COPY_BUTTON_SIZE: f32 = 16.0;
/// Space between the copy button and the top right corner of its block.
const COPY_BUTTON_MARGIN: f32 = 6.0;

#[derive(Debug, Clone)]
enum Message {
    Terminal(otty_ui_term::Event),
    OverlayCopy(BlockKey),
}

struct App {
    terminal: Terminal,
    log: VecDeque<BlockLogEntry>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let settings = terminal_settings();
        let terminal = Terminal::new(0, settings.clone())
            .expect("failed to create terminal");

        (
            App {
                terminal,
                log: VecDeque::new(),
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
            Message::OverlayCopy(block_key) => TerminalView::command(
                self.terminal.widget_id().clone(),
                BlockCommand::CopyContent(block_key),
            ),
        }
    }

    fn view(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let terminal_view =
            TerminalView::show(&self.terminal).map(Message::Terminal);
        let overlay = BlockOverlay::new(&self.terminal);
        // The terminal fills its pane, so the layer above it does too.
        let overlay_layer: Element<'_, Message, Theme, iced::Renderer> =
            Canvas::new(overlay)
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
            otty_ui_term::Event::BlockSelected { block_key, .. } => {
                self.push_log(BlockEventKind::Selected, block_key);
                Task::none()
            },
            otty_ui_term::Event::BlockCopied { block_key, .. } => {
                self.push_log(BlockEventKind::Copied, block_key);
                Task::none()
            },
            other => {
                self.terminal.handle(other);
                Task::none()
            },
        }
    }

    fn push_log(&mut self, kind: BlockEventKind, block_key: BlockKey) {
        if self.log.len() == MAX_LOG_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(BlockLogEntry { kind, block_key });
    }
}

#[derive(Clone, Debug)]
struct BlockLogEntry {
    kind: BlockEventKind,
    block_key: BlockKey,
}

impl std::fmt::Display for BlockLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            BlockEventKind::Selected => {
                write!(f, "selected  - {:?}", self.block_key)
            },
            BlockEventKind::Copied => {
                write!(f, "copied    - {:?}", self.block_key)
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum BlockEventKind {
    Selected,
    Copied,
}

struct BlockOverlay<'a> {
    terminal: &'a Terminal,
}

impl<'a> BlockOverlay<'a> {
    fn new(terminal: &'a Terminal) -> Self {
        Self { terminal }
    }

    /// The places of the command blocks in a layer of `bounds`, in the
    /// coordinates of the layer. Every block runs from its left edge to
    /// its right one.
    fn command_block_rects(&self, bounds: Rectangle) -> Vec<BlockRect> {
        self.terminal
            .block_rects(Point::ORIGIN, bounds.size())
            .into_iter()
            .filter(|rect| rect.kind == BlockKind::Command)
            .collect()
    }
}

impl canvas::Program<Message> for BlockOverlay<'_> {
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

        for rect in self.command_block_rects(bounds) {
            let block_path = Path::rectangle(
                Point::new(rect.rect.x, rect.rect.y),
                Size::new(rect.rect.width, rect.rect.height),
            );
            let mut stroke_color =
                theme.extended_palette().primary.strong.color;
            stroke_color.a = 0.5;
            frame.stroke(
                &block_path,
                Stroke::default().with_color(stroke_color).with_width(1.0),
            );

            let button = copy_button_rect(&rect);
            let button_path = Path::rectangle(
                Point::new(button.x, button.y),
                Size::new(button.width, button.height),
            );
            let mut bg = theme.extended_palette().primary.strong.color;
            bg.a = 0.15;
            frame.fill(&button_path, bg);
            frame.fill_text(CanvasText {
                content: "C".to_string(),
                position: Point::new(
                    button.x + (button.width / 2.0),
                    button.y + (button.height / 2.0),
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
                let pressed = self
                    .command_block_rects(bounds)
                    .into_iter()
                    .find(|rect| copy_button_rect(rect).contains(position))?;

                Some(
                    canvas::Action::publish(Message::OverlayCopy(
                        pressed.block_key,
                    ))
                    .and_capture(),
                )
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
        let is_over_a_button =
            cursor.position_in(bounds).is_some_and(|position| {
                self.command_block_rects(bounds)
                    .iter()
                    .any(|rect| copy_button_rect(rect).contains(position))
            });

        if is_over_a_button {
            iced::mouse::Interaction::Pointer
        } else {
            iced::mouse::Interaction::Idle
        }
    }
}

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

fn terminal_settings() -> Settings {
    let shell =
        std::env::var("SHELL").unwrap_or_else(|_| String::from("/bin/sh"));
    let session_options = LocalSessionOptions::default().with_program(&shell);
    let session = SessionKind::from_local_options(session_options);
    Settings {
        engine: EngineSettings::default().with_session(session),
        ..Default::default()
    }
}

/// The place of the copy button of a block: a square in its top right
/// corner, made smaller when the block is lower than the button.
fn copy_button_rect(block: &BlockRect) -> Rectangle {
    let size = COPY_BUTTON_SIZE.min(block.rect.height);
    let margin = COPY_BUTTON_MARGIN.min((block.rect.height - size) / 2.0);

    Rectangle {
        x: block.rect.x + block.rect.width - size - COPY_BUTTON_MARGIN,
        y: block.rect.y + margin,
        width: size,
        height: size,
    }
}
