//! The app window around a screen: title bar, panels, status bar.
//!
//! The window is composed here the way `view.rs` composes it, because
//! `App::new()` loads the user's settings and opens a terminal.

use std::collections::BTreeMap;

use iced::widget::{Space, column, container, row, text};
use iced::{Element, Length, Padding, Point, Size};
use otty_ui_kit::components::chip::Chip;
use otty_ui_kit::components::divider;
use otty_ui_kit::components::pane_header::PaneHeader;
use otty_ui_kit::components::panel_height::SectionHeight;
use otty_ui_kit::components::surface::{Surface, surface};
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_kit::{fonts, typography};

use super::fixture::{Fixture, TAB_ZSH};
use crate::app::view::stack_left_panel;
use crate::theme::ThemeProps;
use crate::widgets::chrome::model::{StatusBarInput, StatusBarViewModel};
use crate::widgets::chrome::view::{action_bar, status_bar};
use crate::widgets::explorer::model::ExplorerTreeViewModel;
use crate::widgets::explorer::view::sidebar_tree;
use crate::widgets::quick_launch::model::QuickLaunchTreeViewModel;
use crate::widgets::quick_launch::view::sidebar_panel;
use crate::widgets::sidebar::types::HeaderMenu;
use crate::widgets::tabs::model::TabsViewModel;
use crate::widgets::tabs::types::TabItem;
use crate::widgets::tabs::view::session_list;

/// Window size of every screenshot, as in the design.
pub(super) const WINDOW: Size = Size::new(1440.0, 900.0);
/// Width of the left panel (a fifth of the window, the default).
pub(super) const LEFT_WIDTH: f32 = 288.0;
/// Width of the right panel when it is open.
const RIGHT_WIDTH: f32 = 280.0;

/// The message type of every screen: nothing is clicked.
pub(super) type Screen<'a> = Element<'a, ()>;

/// What the panels and the status bar show around the main area.
#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) active_tab: Option<u64>,
    pub(super) show_tabs: bool,
    pub(super) many_sessions: bool,
    pub(super) one_session: bool,
    pub(super) left_open: bool,
    pub(super) right_open: bool,
    pub(super) focus_mode: bool,
    pub(super) menu_open: Option<HeaderMenu>,
    pub(super) pane_count: Option<usize>,
}

impl Frame {
    /// The default workspace: a terminal tab, both panels, two panes.
    pub(super) fn terminal() -> Self {
        Self {
            active_tab: Some(TAB_ZSH),
            show_tabs: true,
            many_sessions: false,
            one_session: false,
            left_open: true,
            right_open: true,
            focus_mode: false,
            menu_open: None,
            pane_count: Some(2),
        }
    }
}

/// The window: title bar, panels around `main`, status bar and `overlays`
/// on top of the panels and the main area.
pub(super) fn window<'a>(
    fx: &'a Fixture,
    theme: ThemeProps<'a>,
    frame: Frame,
    main: Screen<'a>,
    overlays: Vec<Screen<'a>>,
) -> Screen<'a> {
    let palette = theme.theme.ui();
    let tabs = if !frame.show_tabs {
        &fx.no_tabs
    } else if frame.many_sessions {
        &fx.many_tabs
    } else if frame.one_session {
        &fx.one_tab
    } else {
        &fx.tabs
    };

    let title = tabs
        .get(&frame.active_tab.unwrap_or(u64::MAX))
        .map(TabItem::title);
    let header = column![
        action_bar::view(action_bar::ActionBarProps {
            title,
            theme,
            left_open: frame.left_open,
            right_open: frame.right_open,
            focus_mode: frame.focus_mode,
            is_title_bar_pressed: false,
        })
        .map(|_| ()),
        divider::horizontal(palette),
    ];

    let mut body = row![].height(Length::Fill);
    if frame.left_open {
        body = body
            .push(
                container(left_panel(fx, tabs, theme, frame)).width(LEFT_WIDTH),
            )
            .push(divider::vertical(palette));
    }
    body = body.push(container(main).width(Length::Fill));
    if frame.right_open {
        body = body.push(divider::vertical(palette)).push(
            container(surface(
                sidebar_tree::view(sidebar_tree::SidebarTreeProps {
                    vm: ExplorerTreeViewModel {
                        root_label: Some("otty"),
                        tree: &fx.files,
                        selected_path: None,
                        hovered_path: None,
                    },
                    theme,
                })
                .map(|_| ()),
                Surface::Panel,
                palette,
            ))
            .width(RIGHT_WIDTH),
        );
    }

    let mut layers = vec![body.into()];
    layers.extend(overlays);
    let content = iced::widget::Stack::with_children(layers)
        .width(Length::Fill)
        .height(Length::Fill);

    let session_count = tabs.len();
    let focused = (frame.pane_count.is_some()).then_some(&fx.pane);
    let status = StatusBarViewModel::new(StatusBarInput {
        machine_name: "fedora",
        machine_target: TargetKind::Local,
        focused_pane: focused,
        pane_count: frame.pane_count,
        session_count,
    });

    column![header, content, status_bar::view(&status, "0.1.0", theme)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Two panes with headers and a few lines of terminal text.
pub(super) fn terminal_main<'a>(
    fx: &'a Fixture,
    theme: ThemeProps<'a>,
) -> Screen<'a> {
    let palette = theme.theme.ui();

    let pane = |program: &str, folder: &str, is_focused: bool| {
        let chip = Chip::new("fedora", palette).target(TargetKind::Local);
        let header = PaneHeader::new(program, palette)
            .folder(folder)
            .focused(is_focused)
            .trailing(chip);

        let canvas = palette.surface_canvas();
        let strong = palette.text_strong();
        let muted = palette.text_muted();
        let lines = column![
            terminal_line("~/otty  main", "cargo test -p otty", strong, muted),
            terminal_line("", "test result: ok. 512 passed", muted, muted),
            terminal_line("~/otty  main", "", strong, muted),
        ]
        .spacing(4);

        column![
            Element::from(header),
            container(lines)
                .padding(12)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(canvas.into()),
                    ..container::Style::default()
                }),
        ]
        .width(Length::Fill)
    };

    let panes = row![
        pane(
            fx.pane.program(),
            fx.pane.folder_name().unwrap_or("~"),
            true
        ),
        divider::vertical(palette),
        pane("cargo", "app", false),
    ];

    panes.width(Length::Fill).height(Length::Fill).into()
}

/// Put `element` with its top left corner at `point` of a full-size layer.
pub(super) fn at<'a>(element: Screen<'a>, point: Point) -> Screen<'a> {
    container(row![Space::new().width(0), element])
        .padding(Padding {
            top: point.y,
            left: point.x,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Sessions over Launches on the panel surface, sharing the panel height
/// the way `view.rs` does.
fn left_panel<'a>(
    fx: &'a Fixture,
    tabs: &'a BTreeMap<u64, TabItem>,
    theme: ThemeProps<'a>,
    frame: Frame,
) -> Screen<'a> {
    let palette = theme.theme.ui();
    let launches_vm = || QuickLaunchTreeViewModel {
        data: &fx.launches,
        selected_path: None,
        hovered_path: None,
        inline_edit: None,
        launching: &fx.launching,
        drop_target: None,
    };
    let sessions_vm =
        move || TabsViewModel::new(tabs, frame.active_tab, Some("fedora"));

    let sessions_height = SectionHeight::panel_section(
        session_list::content_height(&sessions_vm()),
        true,
    );
    let launches_height = SectionHeight::panel_section(
        sidebar_panel::content_height(&launches_vm()),
        true,
    );

    let sections = stack_left_panel(
        sessions_height,
        launches_height,
        palette,
        move || {
            let sessions = session_list::view(session_list::SessionListProps {
                vm: sessions_vm(),
                theme,
                is_open: true,
                new_session_title: "zsh",
                is_menu_open: frame.menu_open == Some(HeaderMenu::Sessions),
                on_toggle: (),
                on_menu: (),
                on_intent: |_| (),
            });
            let launches =
                sidebar_panel::view(sidebar_panel::SidebarPanelProps {
                    vm: launches_vm(),
                    theme,
                    is_open: true,
                    is_menu_open: frame.menu_open == Some(HeaderMenu::Launches),
                    on_toggle: (),
                    on_menu: (),
                    on_intent: |_| (),
                });

            (sessions, launches)
        },
    );

    surface(sections, Surface::Panel, palette).into()
}

/// One line of terminal text: a prompt and a command.
fn terminal_line<'a>(
    prompt: &'a str,
    command: &'a str,
    command_color: iced::Color,
    prompt_color: iced::Color,
) -> Element<'a, ()> {
    let style = typography::BODY;

    row![
        text(prompt)
            .font(fonts::MONO)
            .size(style.size())
            .color(prompt_color),
        text(command)
            .font(fonts::MONO)
            .size(style.size())
            .color(command_color),
    ]
    .spacing(8)
    .into()
}
