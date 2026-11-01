//! The app window around a screen: title bar, panels, status bar.
//!
//! `App::new()` loads the user's settings and opens a terminal, so the
//! window is put together here from fixed data. The panels around the main
//! area come from the sidebar's own views and a real sidebar widget, the
//! same as in `view.rs`, so panel widths and focus mode match the app.

use std::collections::BTreeMap;

use iced::widget::{Space, column, container, row, text};
use iced::{Element, Length, Padding, Point, Size};
use otty_ui_kit::components::chip::Chip;
use otty_ui_kit::components::divider;
use otty_ui_kit::components::pane_header::PaneHeader;
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_kit::{fonts, typography};

use super::fixture::{Fixture, TAB_ZSH};
use crate::theme::ThemeProps;
use crate::widgets::chrome::model::{StatusBarInput, StatusBarViewModel};
use crate::widgets::chrome::view::{action_bar, status_bar};
use crate::widgets::explorer::model::ExplorerTreeViewModel;
use crate::widgets::explorer::view::sidebar_tree;
use crate::widgets::quick_launch::model::QuickLaunchTreeViewModel;
use crate::widgets::quick_launch::view::sidebar_panel;
use crate::widgets::sidebar::types::{HeaderMenu, SidebarPane};
use crate::widgets::sidebar::view::{focus_note, left_panel, panels};
use crate::widgets::sidebar::{SidebarIntent, SidebarViewModel, SidebarWidget};
use crate::widgets::tabs::model::TabsViewModel;
use crate::widgets::tabs::types::TabItem;
use crate::widgets::tabs::view::session_list;

/// Window size of every screenshot, as in the design.
pub(super) const WINDOW: Size = Size::new(1440.0, 900.0);

/// The message type of every screen: nothing is clicked.
pub(super) type Screen<'a> = Element<'a, ()>;

/// What the panels and the status bar show around the main area.
#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) active_tab: Option<u64>,
    pub(super) show_tabs: bool,
    pub(super) many_sessions: bool,
    pub(super) one_session: bool,
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
            right_open: true,
            focus_mode: false,
            menu_open: None,
            pane_count: Some(2),
        }
    }

    /// The sidebar widget with the panels of this frame, set with the same
    /// intents the app uses, so panel widths match the app.
    pub(super) fn sidebar(&self) -> SidebarWidget {
        let mut sidebar = SidebarWidget::new();

        if !self.right_open {
            let _ = sidebar.reduce(SidebarIntent::ToggleRight);
        }
        if self.focus_mode {
            let _ = sidebar.reduce(SidebarIntent::ToggleFocusMode);
        }
        if let Some(menu) = self.menu_open {
            let _ = sidebar.reduce(SidebarIntent::HeaderMenuOpen(menu));
        }

        sidebar
    }
}

/// Width of the left panel when the app opens.
pub(super) fn left_panel_width() -> f32 {
    WINDOW.width * SidebarWidget::new().left_ratio()
}

/// The window: title bar, the sidebar panels around the main area built by
/// `main`, status bar, and `overlays` on top of the panels and the main
/// area.
pub(super) fn window<'a>(
    fx: &'a Fixture,
    theme: ThemeProps<'a>,
    main: impl Fn() -> Screen<'a>,
    overlays: Vec<Screen<'a>>,
) -> Screen<'a> {
    let frame = fx.frame;
    let sidebar_vm = fx.sidebar.vm();
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
            left_open: sidebar_vm.is_left_open,
            right_open: sidebar_vm.is_right_open,
            focus_mode: sidebar_vm.is_focus_mode,
            is_title_bar_pressed: false,
        })
        .map(|_| ()),
        divider::horizontal(palette),
    ];

    let body = panels::view(panels::PanelsProps {
        panes: fx.sidebar.panes(),
        vm: sidebar_vm,
        theme,
        content: |slot| match slot {
            SidebarPane::Left => left_panel(fx, tabs, theme, sidebar_vm),
            SidebarPane::Main => main(),
            SidebarPane::Right => {
                sidebar_tree::view(sidebar_tree::SidebarTreeProps {
                    vm: ExplorerTreeViewModel::idle(Some("otty"), &fx.files),
                    theme,
                })
                .map(|_| ())
            },
        },
        on_intent: |_| (),
    });

    let mut layers = vec![body];
    if let Some(note) = focus_note::view(sidebar_vm.focus_note_opacity, theme) {
        layers.push(note);
    }
    layers.extend(overlays);
    let content = iced::widget::Stack::with_children(layers)
        .width(Length::Fill)
        .height(Length::Fill);

    let session_count = tabs.len();
    let folder = frame.pane_count.and_then(|_| fx.pane.folder_path());
    let status = StatusBarViewModel::new(StatusBarInput {
        machine_name: "fedora",
        machine_target: TargetKind::Local,
        folder,
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

/// Sessions over Launches, laid out by the sidebar's left panel view.
fn left_panel<'a>(
    fx: &'a Fixture,
    tabs: &'a BTreeMap<u64, TabItem>,
    theme: ThemeProps<'a>,
    sidebar_vm: SidebarViewModel,
) -> Screen<'a> {
    let active_tab = fx.frame.active_tab;
    let is_menu_open = move |menu| {
        sidebar_vm
            .open_header_menu
            .is_some_and(|open| open.menu() == menu)
    };

    left_panel::view(left_panel::LeftPanelProps {
        vm: sidebar_vm,
        theme,
        sections: move || {
            let sessions_vm =
                TabsViewModel::new(tabs, active_tab, Some("fedora"));
            let sessions_content_height =
                session_list::content_height(&sessions_vm);
            let sessions = session_list::view(session_list::SessionListProps {
                vm: sessions_vm,
                theme,
                is_open: sidebar_vm.is_sessions_open,
                is_menu_open: is_menu_open(HeaderMenu::Sessions),
                on_toggle: (),
                on_menu: (),
                on_intent: |_| (),
            });
            let launches_vm =
                QuickLaunchTreeViewModel::idle(&fx.launches, &fx.launching);
            let launches_content_height =
                sidebar_panel::content_height(&launches_vm);
            let launches =
                sidebar_panel::view(sidebar_panel::SidebarPanelProps {
                    vm: launches_vm,
                    theme,
                    is_open: sidebar_vm.is_launches_open,
                    is_menu_open: is_menu_open(HeaderMenu::Launches),
                    on_toggle: (),
                    on_menu: (),
                    on_intent: |_| (),
                });

            left_panel::LeftPanelSections {
                sessions,
                sessions_content_height,
                launches,
                launches_content_height,
            }
        },
    })
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
