use iced::font::{Family, Stretch, Weight};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{button, container, responsive, row, text};
use iced::Task;
use iced::{alignment, Font};
use iced::{window, Color, Element, Length, Size, Subscription};
use iced_term::TerminalView;
use std::collections::HashMap;

const TERM_FONT_JET_BRAINS_BYTES: &[u8] = include_bytes!(
    "../assets/fonts/JetBrains/JetBrainsMonoNerdFontMono-Bold.ttf"
);

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .antialiasing(false)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .subscription(App::subscription)
        .font(TERM_FONT_JET_BRAINS_BYTES)
        .run_with(App::new)
}

struct App {
    panes: pane_grid::State<Pane>,
    tabs: HashMap<u64, iced_term::Terminal>,
    term_settings: iced_term::settings::Settings,
    panes_created: usize,
    focus: Option<pane_grid::Pane>,
}

#[derive(Debug, Clone)]
enum Event {
    Split(pane_grid::Axis, pane_grid::Pane),
    Clicked(pane_grid::Pane),
    Resized(pane_grid::ResizeEvent),
    Close(pane_grid::Pane),
    Terminal(iced_term::Event),
}

impl App {
    fn new() -> (Self, Task<Event>) {
        let initial_pane_id = 0;
        let (panes, _) = pane_grid::State::new(Pane::new(initial_pane_id));
        let term_settings = iced_term::settings::Settings {
            font: iced_term::settings::FontSettings {
                size: 14.0,
                font_type: Font {
                    weight: Weight::Bold,
                    family: Family::Name("JetBrainsMono Nerd Font Mono"),
                    stretch: Stretch::Normal,
                    ..Default::default()
                },
                ..Default::default()
            },
            theme: iced_term::settings::ThemeSettings::default(),
            backend: iced_term::settings::BackendSettings {
                program: std::env::var("SHELL")
                    .expect("SHELL variable is not defined")
                    .to_string(),
                ..Default::default()
            },
        };

        let tab = iced_term::Terminal::new(
            initial_pane_id as u64,
            term_settings.clone(),
        )
        .expect("failed to create the new terminal instance");

        let mut tabs = HashMap::new();
        tabs.insert(initial_pane_id as u64, tab);

        (
            App {
                panes,
                panes_created: 1,
                tabs,
                term_settings,
                focus: None,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Terminal with split panes")
    }

    fn update(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::Split(axis, pane) => {
                let result =
                    self.panes.split(axis, pane, Pane::new(self.panes_created));

                let tab = iced_term::Terminal::new(
                    self.panes_created as u64,
                    self.term_settings.clone(),
                )
                .expect("failed to create the new terminal instance");

                let command = TerminalView::focus(tab.widget_id());
                self.tabs.insert(self.panes_created as u64, tab);

                if let Some((pane, _)) = result {
                    self.focus = Some(pane);
                }

                self.panes_created += 1;
                return command;
            },
            Event::Clicked(pane) => {
                let new_focused_pane = self.panes.get(pane).unwrap();
                let new_focused_tab =
                    self.tabs.get_mut(&(new_focused_pane.id as u64)).unwrap();

                self.focus = Some(pane);
                return TerminalView::focus(new_focused_tab.widget_id());
            },
            Event::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
            },
            Event::Close(pane) => {
                if let Some((closed_pane, sibling)) = self.panes.close(pane) {
                    let tab_id = closed_pane.id as u64;
                    self.tabs.remove(&tab_id);
                    self.focus = Some(sibling);

                    let new_focused_pane = self.panes.get(sibling).unwrap();
                    let new_focused_tab = self
                        .tabs
                        .get_mut(&(new_focused_pane.id as u64))
                        .unwrap();

                    return TerminalView::focus(new_focused_tab.widget_id());
                } else {
                    return window::get_latest().and_then(window::close);
                }
            },
            Event::Terminal(iced_term::Event::BackendCall(id, cmd)) => {
                if let Some(tab) = self.tabs.get_mut(&id) {
                    if tab.handle(iced_term::Command::ProxyToBackend(cmd))
                        == iced_term::actions::Action::Shutdown
                    {
                        if let Some(current_pane) = self.focus {
                            return self.update(Event::Close(current_pane));
                        }
                    }
                }
            },
        }

        Task::none()
    }

    fn view(&self) -> Element<Event> {
        let focus = self.focus;
        let total_panes = self.panes.len();

        let pane_grid = PaneGrid::new(&self.panes, |id, pane, _| {
            let is_focused = focus == Some(id);
            let title_color = if is_focused {
                PANE_ID_COLOR_FOCUSED
            } else {
                PANE_ID_COLOR_UNFOCUSED
            };

            let title =
                row!["Pane", text(pane.id.to_string()).color(title_color),]
                    .spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .controls(view_controls(id, total_panes, pane.is_pinned))
                .padding(10)
                .style(if is_focused {
                    style::title_bar_focused
                } else {
                    style::title_bar_active
                });

            let pane_id = pane.id as u64;
            pane_grid::Content::new(responsive(move |_| {
                view_content(pane_id, &self.tabs)
            }))
            .title_bar(title_bar)
            .style(if is_focused {
                style::pane_focused
            } else {
                style::pane_active
            })
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Event::Clicked)
        .on_resize(10, Event::Resized);

        container(pane_grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into()
    }

    fn subscription(&self) -> Subscription<Event> {
        let mut subscriptions = vec![];
        for id in self.tabs.keys() {
            let tab = self.tabs.get(id).unwrap();
            subscriptions
                .push(Subscription::run_with_id(tab.id, tab.subscription()));
        }

        Subscription::batch(subscriptions).map(Event::Terminal)
    }
}

const PANE_ID_COLOR_UNFOCUSED: Color = Color::from_rgb(
    0xFF as f32 / 255.0,
    0xC7 as f32 / 255.0,
    0xC7 as f32 / 255.0,
);
const PANE_ID_COLOR_FOCUSED: Color = Color::from_rgb(
    0xFF as f32 / 255.0,
    0x47 as f32 / 255.0,
    0x47 as f32 / 255.0,
);

#[derive(Clone, Copy)]
struct Pane {
    pub id: usize,
    pub is_pinned: bool,
}

impl Pane {
    fn new(id: usize) -> Self {
        Self {
            id,
            is_pinned: false,
        }
    }
}

fn view_content(
    pane_id: u64,
    tabs: &HashMap<u64, iced_term::Terminal>,
) -> Element<'_, Event> {
    let tab = tabs.get(&pane_id).expect("tab with target id not found");
    container(TerminalView::show(tab).map(Event::Terminal))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(5)
        .into()
}

fn view_controls<'a>(
    pane: pane_grid::Pane,
    total_panes: usize,
    is_pinned: bool,
) -> Element<'a, Event> {
    let mut row = row![].spacing(5);
    let mut close = button(text("Close").size(14))
        .style(button::danger)
        .padding(3);

    if total_panes > 1 && !is_pinned {
        close = close.on_press(Event::Close(pane));
    }

    let button = |label, event| {
        button(
            text(label)
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .size(16),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(event)
    };

    row = row.push(button(
        "Split horizontally",
        Event::Split(pane_grid::Axis::Horizontal, pane),
    ));

    row = row.push(button(
        "Split vertically",
        Event::Split(pane_grid::Axis::Vertical, pane),
    ));

    row.push(close).into()
}

mod style {
    use iced::widget::container;
    use iced::{Border, Theme};

    pub fn title_bar_active(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            text_color: Some(palette.background.strong.text),
            background: Some(palette.background.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn title_bar_focused(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            text_color: Some(palette.primary.strong.text),
            background: Some(palette.primary.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn pane_active(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.background.strong.color,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn pane_focused(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.background.strong.color,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
