//! The Docker mode right panel: the inspector header of the selected row
//! with its action buttons, its tabs (Overview, Logs, Inspect) and the open
//! tab.

use std::collections::BTreeSet;

use iced::widget::text::IntoFragment;
use iced::widget::{Row, Space, column, container, row, scrollable, svg, text};
use iced::{Alignment, Element, Length, Padding};
use otty_ui_kit::components::data_tree::{DataRow, DataTree};
use otty_ui_kit::components::icon_button::{
    IconButton, IconButtonSize, IconTone,
};
use otty_ui_kit::components::inspector_header::InspectorHeader;
use otty_ui_kit::components::inspector_tabs::inspector_tabs;
use otty_ui_kit::components::key_value_list::{KeyValueList, ValuePart};
use otty_ui_kit::components::log_view::{LogView, LogViewState};
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::status_badge::StatusBadge;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::action_model::ActionsViewModel;
use super::super::actions::RowAction;
use super::super::event::DockerIntent;
use super::super::inspect_tree::InspectTree;
use super::super::inspector_tabs::{
    InspectContent, InspectLoadedViewModel, InspectorTabsViewModel, TabContent,
};
use super::super::links::LinkTarget;
use super::super::model::InspectorViewModel;
use super::super::overview::Piece;
use crate::i18n::{self, Key};
use crate::icons;
use crate::theme::ThemeProps;

/// Padding of the inspector (`padding: 14px 16px`).
const PADDING: Padding = Padding {
    top: 14.0,
    right: 16.0,
    bottom: 14.0,
    left: 16.0,
};
/// Space between the header, the tabs and the tab body (`gap: 14px`).
const SECTION_GAP: f32 = 14.0;

/// Props for the inspector.
pub(crate) struct InspectorProps<'a> {
    pub(crate) vm: Option<InspectorViewModel>,
    /// The action buttons of the selected row.
    pub(crate) actions: Option<ActionsViewModel>,
    /// The tabs under the header and the open tab's body.
    pub(crate) tabs: Option<InspectorTabsViewModel<'a>>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the header of the selected row (its name, status line, action
/// buttons and the hint under them), the tabs and the open tab. With
/// nothing selected the panel says there is nothing to inspect.
pub(crate) fn view(props: InspectorProps<'_>) -> Element<'_, DockerIntent> {
    let palette = props.theme.theme.ui();

    let Some(vm) = props.vm else {
        return container(hint(i18n::t(Key::DockerNothingToInspect), palette))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(PADDING)
            .into();
    };

    let status = StatusBadge::new(vm.status, vm.tone, palette).wrapping(true);
    let mut header = InspectorHeader::new(vm.title, palette).status(status);
    if let Some(actions) = props.actions {
        for button in actions.buttons {
            let tone = if button.is_danger {
                IconTone::Danger
            } else {
                IconTone::Default
            };
            let action = IconButton::new(
                svg::Handle::from_memory(action_icon(button.action)),
                button.label,
                palette,
            )
            .tone(tone)
            .on_press_maybe(
                button
                    .is_enabled
                    .then_some(DockerIntent::Act(button.action)),
            );
            header = header.action(action);
        }
        if let Some(hint) = actions.hint {
            header = header.hint(hint);
        }
    }

    let mut panel = column![header]
        .spacing(SECTION_GAP)
        .width(Length::Fill)
        .height(Length::Fill);
    if let Some(tabs) = props.tabs {
        panel = panel
            .push(inspector_tabs(
                tabs.tabs,
                &tabs.selected,
                DockerIntent::SelectInspectorTab,
                palette,
            ))
            .push(tab_body(tabs.content, palette));
    }

    container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(PADDING)
        .into()
}

/// The icon of an action button.
fn action_icon(action: RowAction) -> &'static [u8] {
    match action {
        RowAction::Shell => icons::SHELL,
        RowAction::Start | RowAction::Run => icons::START,
        RowAction::Stop => icons::STOP,
        RowAction::Restart => icons::RESTART,
        RowAction::Pause => icons::PAUSE,
        RowAction::Unpause => icons::RESUME,
        RowAction::Remove => icons::TRASH,
        RowAction::PullAgain => icons::PULL,
    }
}

/// The body of the open tab.
fn tab_body<'a>(
    content: TabContent<'a>,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    match content {
        TabContent::Overview(fields) => {
            let mut list = KeyValueList::new(palette);
            for (label, value) in fields {
                list =
                    list.entry_parts(label, value.into_iter().map(value_part));
            }
            // The thin bar draws over the content: the padding keeps the
            // values clear of it, as in the Inspect tree.
            let rows = container(list).padding(Padding {
                right: spacing::SPACE_3,
                ..Padding::ZERO
            });

            scrollable(rows)
                .direction(scrollbar::vertical())
                .style(scrollbar::style(palette))
                .height(Length::Fill)
                .into()
        },
        TabContent::Logs { lines, error } => {
            // The hint has its own line: beside the Follow switch it wraps
            // to one word per line in the narrow panel.
            let mut body = column![hint(i18n::t(Key::DockerLogsHint), palette)]
                .spacing(spacing::SPACE_3)
                .height(Length::Fill);
            if let Some(lines) = lines {
                body = body.push(log_follow(lines, palette));
            }
            if let Some(error) = error {
                body = body.push(error_notice(
                    i18n::docker_logs_failed(error),
                    palette,
                ));
            }
            if let Some(lines) = lines {
                body = body.push(
                    LogView::new(lines, palette)
                        .on_action(DockerIntent::LogAction),
                );
            }
            body.into()
        },
        TabContent::Inspect(InspectContent::Loading) => {
            hint(i18n::t(Key::DockerLoading), palette)
        },
        TabContent::Inspect(InspectContent::Failed(error)) => {
            error_notice(i18n::docker_inspect_failed(error), palette)
        },
        TabContent::Inspect(InspectContent::Loaded(loaded)) => {
            inspect_body(loaded, palette)
        },
    }
}

/// The loaded Inspect tab: its switches and the Copy JSON icon above the
/// tree, or above the raw JSON when there is no tree to show. The icon stays
/// at the end of the first row of switches; the switches wrap when narrow.
fn inspect_body<'a>(
    loaded: InspectLoadedViewModel<'a>,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let mut switches = row![inspect_switch(
        Switch::new(loaded.tree.is_none(), palette)
            .label(i18n::t(Key::DockerInspectRawJson))
            .on_toggle(DockerIntent::InspectRawJsonToggled)
    )]
    .spacing(spacing::SPACE_4);
    if loaded.tree.is_some() {
        switches = switches.push(inspect_switch(
            Switch::new(loaded.shows_empty, palette)
                .label(i18n::t(Key::DockerInspectShowEmpty))
                .on_toggle(DockerIntent::InspectShowEmptyToggled),
        ));
    }

    let controls = row![
        container(switches.wrap().vertical_spacing(spacing::SPACE_2))
            .width(Length::Fill),
        copy_json_button(palette),
    ]
    .spacing(spacing::SPACE_3);

    let body: Element<'a, DockerIntent> = match loaded.tree {
        Some(tree) => {
            // The thin bar draws over the content: the padding keeps the
            // values clear of it.
            let rows =
                container(data_tree(tree, loaded.open_sections, palette))
                    .padding(Padding {
                        right: spacing::SPACE_3,
                        ..Padding::ZERO
                    });

            scrollable(rows)
                .direction(scrollbar::vertical())
                .style(scrollbar::style(palette))
                .height(Length::Fill)
                .into()
        },
        None => LogView::new(loaded.json, palette)
            .on_action(DockerIntent::InspectAction)
            .into(),
    };

    column![controls, body]
        .spacing(spacing::SPACE_3)
        .height(Length::Fill)
        .into()
}

/// One switch of the Inspect controls, as tall as the Copy JSON icon, so
/// they line up when the switches wrap too.
fn inspect_switch<'a>(
    switch: Switch<'a, DockerIntent>,
) -> Element<'a, DockerIntent> {
    container(switch)
        .height(size::CONTROL_SM)
        .align_y(Alignment::Center)
        .into()
}

/// The Copy JSON icon button; its label shows as a tooltip.
fn copy_json_button<'a>(palette: &UiPalette) -> Element<'a, DockerIntent> {
    IconButton::new(
        svg::Handle::from_memory(icons::COPY),
        i18n::t(Key::DockerInspectCopyJson),
        palette,
    )
    .size(IconButtonSize::Small)
    .on_press(DockerIntent::CopyInspectJson)
    .into()
}

/// The inspect rows as a kit tree: the General group (when it has rows),
/// then the sections with their engine-given titles.
fn data_tree(
    tree: InspectTree,
    open_sections: &BTreeSet<String>,
    palette: &UiPalette,
) -> DataTree<DockerIntent> {
    let mut data_tree = DataTree::new(palette);
    if !tree.general.is_empty() {
        data_tree = data_tree
            .group(i18n::t(Key::DockerInspectGeneral), tree_rows(tree.general));
    }

    for section in tree.sections {
        let is_open = open_sections.contains(&section.title);
        let on_toggle =
            DockerIntent::InspectSectionToggled(section.title.clone());
        data_tree = data_tree.section(
            section.title,
            is_open,
            tree_rows(section.rows),
            on_toggle,
        );
    }

    data_tree
}

/// Inspect rows for the kit tree: a link follows its target when pressed.
fn tree_rows(rows: Vec<DataRow<LinkTarget>>) -> Vec<DataRow<DockerIntent>> {
    rows.into_iter()
        .map(|row| row.map(DockerIntent::FollowLink))
        .collect()
}

/// A piece of an Overview value as a kit value part: a link follows its
/// target when pressed.
fn value_part(piece: Piece) -> ValuePart<DockerIntent> {
    match piece {
        Piece::Link(text, target) => {
            ValuePart::link(text, DockerIntent::FollowLink(target))
        },
        Piece::Text(text) => ValuePart::text(text),
    }
}

/// Add to the logs header how many lines wait (while Follow is off and some
/// do) and the Follow switch.
fn log_follow<'a>(
    lines: &LogViewState,
    palette: &UiPalette,
) -> Row<'a, DockerIntent> {
    let waiting = lines.pending_count();
    let count: Element<'a, DockerIntent> =
        if !lines.is_following() && waiting > 0 {
            hint(i18n::docker_logs_new_lines(waiting), palette)
        } else {
            Space::new().into()
        };

    row![
        container(count).width(Length::Fill),
        Switch::new(lines.is_following(), palette)
            .label(i18n::t(Key::DockerLogsFollow))
            .on_toggle(DockerIntent::LogFollowToggled),
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center)
}

/// A muted small line of text.
fn hint<'a>(
    message: impl IntoFragment<'a>,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let style = typography::SMALL;

    text(message)
        .size(style.size())
        .line_height(iced::Pixels(style.line_height()))
        .font(fonts::UI)
        .color(palette.text_muted())
        .into()
}

/// The engine's message in an error notice.
fn error_notice<'a>(
    message: String,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    Notice::new(
        message,
        svg::Handle::from_memory(icons::NOTICE_ERROR),
        palette,
    )
    .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_ui_kit::components::log_view::LogViewState;
    use otty_ui_kit::components::status_badge::StatusTone;

    use super::*;
    use crate::theme::{BuiltinTheme, ThemeManager};
    use crate::widgets::docker::action_model::{
        ActionButton, ActionsViewModel,
    };
    use crate::widgets::docker::actions::RowAction;
    use crate::widgets::docker::inspect_tree::InspectSection;
    use crate::widgets::docker::inspector::InspectorTab;
    use crate::widgets::docker::overview::Field;
    use crate::widgets::docker::types::ResourceKind;

    fn header() -> Option<InspectorViewModel> {
        Some(InspectorViewModel {
            title: String::from("otty-fx-web-1"),
            status: String::from("Up 2 hours"),
            tone: StatusTone::Ok,
        })
    }

    fn container_tabs(
        selected: InspectorTab,
        content: TabContent<'_>,
    ) -> Option<InspectorTabsViewModel<'_>> {
        Some(InspectorTabsViewModel {
            tabs: InspectorTab::of(ResourceKind::Containers),
            selected,
            content,
        })
    }

    #[test]
    fn the_header_shows_the_name_and_the_status() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(InspectorProps {
            vm: Some(InspectorViewModel {
                title: String::from("otty-fx-web-1"),
                status: String::from("Up 2 hours"),
                tone: StatusTone::Ok,
            }),
            actions: None,
            tabs: None,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("otty-fx-web-1").expect("title should show");
        ui.find("Up 2 hours").expect("status should show");
    }

    #[test]
    fn nothing_selected_says_there_is_nothing_to_inspect() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(InspectorProps {
            vm: None,
            actions: None,
            tabs: None,
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find("otty-fx-web-1").is_err());
        ui.find(i18n::t(Key::DockerNothingToInspect))
            .expect("the hint should show");
    }

    /// An Overview field with a plain text value.
    fn text_field(key: Key, value: &str) -> Field {
        (i18n::t(key), vec![Piece::Text(String::from(value))])
    }

    #[test]
    fn the_tabs_show_under_the_header_and_a_press_selects_one() {
        let themes = ThemeManager::new();
        let fields = vec![text_field(Key::DockerColumnImage, "busybox:1.37")];
        let mut ui = simulator(view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Overview,
                TabContent::Overview(fields),
            ),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::DockerTabOverview))
            .expect("overview tab");
        ui.find(i18n::t(Key::DockerTabInspect))
            .expect("inspect tab");
        ui.click(i18n::t(Key::DockerTabLogs)).expect("logs tab");

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![DockerIntent::SelectInspectorTab(InspectorTab::Logs)]
        );
    }

    #[test]
    fn the_overview_lists_its_fields() {
        let themes = ThemeManager::new();
        let fields = vec![
            text_field(Key::DockerColumnImage, "busybox:1.37"),
            text_field(Key::DockerFieldCommand, "sleep 3600"),
        ];
        let mut ui = simulator(view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Overview,
                TabContent::Overview(fields),
            ),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::DockerColumnImage)).expect("label");
        ui.find("busybox:1.37").expect("value");
        ui.find("sleep 3600").expect("value");
    }

    /// Overview fields with an image link, two port links (the UDP port is
    /// text) and a network link, as a container shows them.
    fn linked_fields() -> Vec<Field> {
        let port =
            |port: u16| LinkTarget::Url(format!("http://localhost:{port}"));

        vec![
            (
                i18n::t(Key::DockerColumnImage),
                vec![Piece::Link(
                    String::from("busybox:1.37"),
                    LinkTarget::Item(
                        ResourceKind::Images,
                        String::from("sha256:a7db"),
                    ),
                )],
            ),
            (
                i18n::t(Key::DockerColumnPorts),
                vec![
                    Piece::Link(String::from("18080:80/tcp"), port(18080)),
                    Piece::Text(String::from(", ")),
                    Piece::Link(String::from("18443:443/tcp"), port(18443)),
                    Piece::Text(String::from(", 53/udp")),
                ],
            ),
            (
                i18n::t(Key::DockerNetworks),
                vec![Piece::Link(
                    String::from("otty-fx-net"),
                    LinkTarget::Item(
                        ResourceKind::Networks,
                        String::from("9b55"),
                    ),
                )],
            ),
        ]
    }

    #[test]
    fn pressing_an_overview_link_follows_it_in_both_themes() {
        let mut themes = ThemeManager::new();

        for theme in BuiltinTheme::ALL {
            themes.set_theme(theme);
            let mut ui = simulator(narrow(view(InspectorProps {
                vm: header(),
                actions: None,
                tabs: container_tabs(
                    InspectorTab::Overview,
                    TabContent::Overview(linked_fields()),
                ),
                theme: ThemeProps::new(themes.current()),
            })));

            ui.click("18443:443/tcp").expect("second port link");
            ui.click("busybox:1.37").expect("image link");
            ui.click("otty-fx-net").expect("network link");
            ui.click(i18n::t(Key::DockerColumnPorts)).expect("label");

            assert_eq!(
                ui.into_messages().collect::<Vec<_>>(),
                vec![
                    DockerIntent::FollowLink(LinkTarget::Url(String::from(
                        "http://localhost:18443"
                    ))),
                    DockerIntent::FollowLink(LinkTarget::Item(
                        ResourceKind::Images,
                        String::from("sha256:a7db")
                    )),
                    DockerIntent::FollowLink(LinkTarget::Item(
                        ResourceKind::Networks,
                        String::from("9b55")
                    )),
                ]
            );
        }
    }

    #[test]
    fn the_logs_tab_shows_its_hint_and_the_error() {
        let themes = ThemeManager::new();
        let mut lines = LogViewState::new();
        lines.push_lines(["ready"]);
        let mut ui = simulator(view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Logs,
                TabContent::Logs {
                    lines: Some(&lines),
                    error: Some("stream closed"),
                },
            ),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::DockerLogsHint)).expect("hint");
        ui.find(i18n::docker_logs_failed("stream closed"))
            .expect("error notice");
    }

    #[test]
    fn pressing_follow_in_the_logs_tab_asks_to_turn_it_off() {
        let themes = ThemeManager::new();
        let mut lines = LogViewState::new();
        lines.push_lines(["ready"]);
        let mut ui = simulator(view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Logs,
                TabContent::Logs {
                    lines: Some(&lines),
                    error: None,
                },
            ),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::DockerLogsFollow))
            .expect("follow should show");

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![DockerIntent::LogFollowToggled(false)]
        );
    }

    #[test]
    fn with_follow_off_the_logs_tab_counts_the_waiting_lines() {
        let mut themes = ThemeManager::new();
        let mut lines = LogViewState::new();
        lines.push_lines(["ready"]);
        lines.set_following(false);
        let waiting: Vec<String> =
            (0..12).map(|n| format!("line {n}")).collect();
        lines.push_lines(waiting);

        for theme in BuiltinTheme::ALL {
            themes.set_theme(theme);
            let mut ui = simulator(view(InspectorProps {
                vm: header(),
                actions: None,
                tabs: container_tabs(
                    InspectorTab::Logs,
                    TabContent::Logs {
                        lines: Some(&lines),
                        error: None,
                    },
                ),
                theme: ThemeProps::new(themes.current()),
            }));

            ui.find(i18n::docker_logs_new_lines(12))
                .expect("the waiting counter should show");
            ui.click(i18n::t(Key::DockerLogsFollow))
                .expect("follow should show");
            assert_eq!(
                ui.into_messages().collect::<Vec<_>>(),
                vec![DockerIntent::LogFollowToggled(true)]
            );
        }
    }

    #[test]
    fn while_following_the_logs_tab_shows_no_waiting_counter() {
        let themes = ThemeManager::new();
        let mut lines = LogViewState::new();
        lines.push_lines(["ready"]);
        let mut ui = simulator(view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Logs,
                TabContent::Logs {
                    lines: Some(&lines),
                    error: None,
                },
            ),
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find(i18n::docker_logs_new_lines(0)).is_err());
        assert!(ui.find(i18n::docker_logs_new_lines(1)).is_err());
    }

    #[test]
    fn the_inspect_tab_says_it_loads_or_why_it_failed() {
        let themes = ThemeManager::new();
        let mut loading = simulator(view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Inspect,
                TabContent::Inspect(InspectContent::Loading),
            ),
            theme: ThemeProps::new(themes.current()),
        }));
        let mut failed = simulator(view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Inspect,
                TabContent::Inspect(InspectContent::Failed("no such")),
            ),
            theme: ThemeProps::new(themes.current()),
        }));

        loading.find(i18n::t(Key::DockerLoading)).expect("loading");
        failed
            .find(i18n::docker_inspect_failed("no such"))
            .expect("error notice");
    }

    /// `lines` as the view of the inspect JSON.
    fn json_lines(lines: &[&str]) -> LogViewState {
        let mut json = LogViewState::new();
        json.push_lines(lines.iter().copied());
        json
    }

    /// A General row, an open State section and a closed Config section.
    fn fixture_tree() -> InspectTree {
        InspectTree {
            general: vec![DataRow::new(0, "Id", "850b")],
            sections: vec![
                InspectSection {
                    title: String::from("State"),
                    rows: vec![DataRow::new(0, "Status", "running")],
                },
                InspectSection {
                    title: String::from("Config"),
                    rows: vec![DataRow::new(0, "Image", "busybox:1.37")],
                },
            ],
        }
    }

    /// The inspector on the Inspect tab with `tree` (`None`: raw JSON).
    fn inspect_panel<'a>(
        json: &'a LogViewState,
        tree: Option<InspectTree>,
        open_sections: &'a BTreeSet<String>,
        theme: ThemeProps<'a>,
    ) -> Element<'a, DockerIntent> {
        view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Inspect,
                TabContent::Inspect(InspectContent::Loaded(
                    InspectLoadedViewModel {
                        json,
                        tree,
                        open_sections,
                        shows_empty: false,
                    },
                )),
            ),
            theme,
        })
    }

    /// [`inspect_panel`] in a narrow panel; only State is open.
    fn inspect_view<'a>(
        json: &'a LogViewState,
        tree: Option<InspectTree>,
        open_sections: &'a BTreeSet<String>,
        theme: ThemeProps<'a>,
    ) -> Element<'a, DockerIntent> {
        narrow(inspect_panel(json, tree, open_sections, theme))
    }

    fn only_state_open() -> BTreeSet<String> {
        BTreeSet::from([String::from("State")])
    }

    #[test]
    fn the_inspect_tree_shows_general_and_the_open_section_in_both_themes() {
        let mut themes = ThemeManager::new();
        let json = json_lines(&["{}"]);
        let open = only_state_open();

        for theme in BuiltinTheme::ALL {
            themes.set_theme(theme);
            let mut ui = simulator(inspect_view(
                &json,
                Some(fixture_tree()),
                &open,
                ThemeProps::new(themes.current()),
            ));

            ui.find(i18n::t(Key::DockerInspectGeneral)).expect("group");
            ui.find("850b").expect("general value");
            ui.find("running").expect("value of the open section");
            ui.find("Config").expect("title of the closed section");
            assert!(ui.find("busybox:1.37").is_err());
        }
    }

    #[test]
    fn an_inspect_tree_without_general_values_shows_no_general_title() {
        let themes = ThemeManager::new();
        let json = json_lines(&["{}"]);
        let open = only_state_open();
        let tree = InspectTree {
            general: Vec::new(),
            ..fixture_tree()
        };
        let mut ui = simulator(inspect_view(
            &json,
            Some(tree),
            &open,
            ThemeProps::new(themes.current()),
        ));

        ui.find("State").expect("the section stays");
        assert!(ui.find(i18n::t(Key::DockerInspectGeneral)).is_err());
    }

    #[test]
    fn pressing_a_section_title_asks_to_toggle_that_section() {
        let themes = ThemeManager::new();
        let json = json_lines(&["{}"]);
        let open = only_state_open();
        let mut ui = simulator(inspect_view(
            &json,
            Some(fixture_tree()),
            &open,
            ThemeProps::new(themes.current()),
        ));

        ui.click("Config").expect("closed section");
        ui.click("State").expect("open section");

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![
                DockerIntent::InspectSectionToggled(String::from("Config")),
                DockerIntent::InspectSectionToggled(String::from("State")),
            ]
        );
    }

    /// Center of the Copy JSON icon: at the end of the first row of controls,
    /// beside the Raw JSON switch, in a panel `width` wide.
    fn copy_center(
        ui: &mut iced_test::Simulator<'_, DockerIntent>,
        width: f32,
    ) -> iced::Point {
        let raw = ui
            .find(i18n::t(Key::DockerInspectRawJson))
            .expect("raw json switch")
            .bounds();

        iced::Point::new(
            width - PADDING.right - otty_ui_kit::size::CONTROL_SM / 2.0,
            raw.center_y(),
        )
    }

    #[test]
    fn the_inspect_controls_send_their_intents() {
        let themes = ThemeManager::new();
        let json = json_lines(&["{}"]);
        let open = only_state_open();
        let mut ui = simulator(inspect_view(
            &json,
            Some(fixture_tree()),
            &open,
            ThemeProps::new(themes.current()),
        ));

        ui.click(i18n::t(Key::DockerInspectRawJson))
            .expect("raw json switch");
        ui.click(i18n::t(Key::DockerInspectShowEmpty))
            .expect("show empty switch");
        let copy = copy_center(&mut ui, NARROW);
        click_at(&mut ui, copy);

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![
                DockerIntent::InspectRawJsonToggled(true),
                DockerIntent::InspectShowEmptyToggled(true),
                DockerIntent::CopyInspectJson,
            ]
        );
    }

    #[test]
    fn raw_json_replaces_the_tree_and_keeps_its_switch_and_copy() {
        let themes = ThemeManager::new();
        let json = json_lines(&["{", "  \"Id\": \"850b\"", "}"]);
        let open = only_state_open();
        let mut ui = simulator(inspect_view(
            &json,
            None,
            &open,
            ThemeProps::new(themes.current()),
        ));

        assert!(ui.find("State").is_err());
        assert!(ui.find(i18n::t(Key::DockerInspectGeneral)).is_err());
        assert!(ui.find(i18n::t(Key::DockerInspectShowEmpty)).is_err());
        ui.click(i18n::t(Key::DockerInspectRawJson))
            .expect("raw json switch");
        let copy = copy_center(&mut ui, NARROW);
        click_at(&mut ui, copy);

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![
                DockerIntent::InspectRawJsonToggled(false),
                DockerIntent::CopyInspectJson,
            ]
        );
    }

    /// Width of a panel where all controls fit on one row.
    const WIDE: f32 = 420.0;

    #[test]
    fn the_copy_icon_sits_on_the_row_of_the_switches_in_both_themes() {
        let mut themes = ThemeManager::new();
        let json = json_lines(&["{}"]);
        let open = only_state_open();

        for theme in BuiltinTheme::ALL {
            themes.set_theme(theme);
            let panel = inspect_panel(
                &json,
                Some(fixture_tree()),
                &open,
                ThemeProps::new(themes.current()),
            );
            let mut ui = simulator(container(panel).width(WIDE).height(600));

            let raw = ui
                .find(i18n::t(Key::DockerInspectRawJson))
                .expect("raw json switch")
                .bounds();
            let empty = ui
                .find(i18n::t(Key::DockerInspectShowEmpty))
                .expect("show empty switch")
                .bounds();
            let copy = copy_center(&mut ui, WIDE);
            click_at(&mut ui, copy);

            assert!(
                (raw.center_y() - empty.center_y()).abs() < 0.5,
                "{raw:?} and {empty:?} on one row"
            );
            assert!(copy.x > empty.x + empty.width, "{copy:?} after {empty:?}");
            assert_eq!(
                ui.into_messages().collect::<Vec<_>>(),
                vec![DockerIntent::CopyInspectJson]
            );
        }
    }

    #[test]
    fn the_copy_json_text_shows_only_as_a_tooltip() {
        let themes = ThemeManager::new();
        let json = json_lines(&["{}"]);
        let open = only_state_open();
        let mut ui = simulator(inspect_view(
            &json,
            Some(fixture_tree()),
            &open,
            ThemeProps::new(themes.current()),
        ));

        assert!(ui.find(i18n::t(Key::DockerInspectCopyJson)).is_err());
    }

    /// Draws `button` with the pointer over it and checks it against the
    /// picture saved in `reference`; the first call saves the picture.
    /// Tooltips are overlays that widget queries do not reach.
    fn hovered_copy_button_matches(
        button: Element<'_, DockerIntent>,
        reference: &std::path::Path,
    ) -> bool {
        const MARGIN: f32 = 80.0;
        let mut ui =
            simulator(container(button).padding(MARGIN).width(320).height(200));
        let center = iced::Point::new(
            MARGIN + otty_ui_kit::size::CONTROL_SM / 2.0,
            MARGIN + otty_ui_kit::size::CONTROL_SM / 2.0,
        );
        ui.point_at(center);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved { position: center },
        )]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("button should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn a_hovered_copy_icon_shows_the_copy_json_text_as_its_tooltip() {
        let reference = std::env::temp_dir()
            .join(format!("otty-docker-copy-json-{}", std::process::id()))
            .join("tooltip");
        let palette = UiPalette::otty_dark();
        let icon_button = |label: &'static str| -> Element<'_, DockerIntent> {
            IconButton::new(
                svg::Handle::from_memory(icons::COPY),
                label,
                &palette,
            )
            .size(IconButtonSize::Small)
            .on_press(DockerIntent::CopyInspectJson)
            .into()
        };

        assert!(hovered_copy_button_matches(
            copy_json_button(&palette),
            &reference
        ));
        assert!(hovered_copy_button_matches(
            icon_button(i18n::t(Key::DockerInspectCopyJson)),
            &reference
        ));
        assert!(!hovered_copy_button_matches(
            icon_button("Another text"),
            &reference
        ));
    }

    /// A tree with an image link in General, a port link and a text value in
    /// the open State section, a network heading link under it, and a link
    /// in the closed Config section.
    fn linked_tree() -> InspectTree {
        let image =
            LinkTarget::Item(ResourceKind::Images, String::from("sha256:a7db"));
        let network =
            LinkTarget::Item(ResourceKind::Networks, String::from("9b55"));
        let port = LinkTarget::Url(String::from("http://localhost:18081"));
        let project = LinkTarget::ComposeProject(String::from("otty-fx"));

        InspectTree {
            general: vec![DataRow::with_parts(
                0,
                "Image",
                [ValuePart::link("sha256:a7db", image)],
            )],
            sections: vec![
                InspectSection {
                    title: String::from("State"),
                    rows: vec![
                        DataRow::with_parts(
                            0,
                            "HostPort",
                            [ValuePart::link("18081", port)],
                        ),
                        DataRow::new(0, "HostIp", "0.0.0.0"),
                        DataRow::heading_link(0, "otty-fx-net", network),
                    ],
                },
                InspectSection {
                    title: String::from("Config"),
                    rows: vec![DataRow::with_parts(
                        0,
                        "Project",
                        [ValuePart::link("otty-fx", project)],
                    )],
                },
            ],
        }
    }

    #[test]
    fn pressing_a_link_of_the_inspect_tree_follows_it_in_both_themes() {
        let mut themes = ThemeManager::new();
        let json = json_lines(&["{}"]);
        let open = only_state_open();

        for theme in BuiltinTheme::ALL {
            themes.set_theme(theme);
            let mut ui = simulator(inspect_view(
                &json,
                Some(linked_tree()),
                &open,
                ThemeProps::new(themes.current()),
            ));

            ui.click("sha256:a7db").expect("image link");
            ui.click("18081").expect("port link");
            ui.click("otty-fx-net").expect("network heading link");
            ui.click("0.0.0.0").expect("plain value");
            ui.click("HostPort").expect("key");
            assert!(ui.find("otty-fx").is_err(), "the section is closed");

            assert_eq!(
                ui.into_messages().collect::<Vec<_>>(),
                vec![
                    DockerIntent::FollowLink(LinkTarget::Item(
                        ResourceKind::Images,
                        String::from("sha256:a7db")
                    )),
                    DockerIntent::FollowLink(LinkTarget::Url(String::from(
                        "http://localhost:18081"
                    ))),
                    DockerIntent::FollowLink(LinkTarget::Item(
                        ResourceKind::Networks,
                        String::from("9b55")
                    )),
                ]
            );
        }
    }

    /// Draws raw JSON of `lines` and checks it against the picture saved in
    /// `reference`; the first call saves it. A text editor is not reached by
    /// widget queries.
    fn raw_json_matches(lines: &[&str], reference: &std::path::Path) -> bool {
        let themes = ThemeManager::new();
        let json = json_lines(lines);
        let open = BTreeSet::new();
        let mut ui = simulator(inspect_view(
            &json,
            None,
            &open,
            ThemeProps::new(themes.current()),
        ));

        ui.snapshot(&iced::Theme::Dark)
            .expect("inspector should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn raw_json_draws_the_json_text() {
        let reference = std::env::temp_dir()
            .join(format!("otty-docker-raw-json-{}", std::process::id()))
            .join("raw");

        assert!(raw_json_matches(&["{\"Id\": \"850b\"}"], &reference));
        assert!(!raw_json_matches(&["{\"Id\": \"9c21\"}"], &reference));
    }

    #[test]
    fn the_inspect_tree_and_its_controls_stay_inside_the_panel_padding() {
        let themes = ThemeManager::new();
        let json = json_lines(&["{}"]);
        let open = only_state_open();
        let mut ui = simulator(inspect_view(
            &json,
            Some(fixture_tree()),
            &open,
            ThemeProps::new(themes.current()),
        ));

        let general = ui
            .find(i18n::t(Key::DockerInspectGeneral))
            .expect("group")
            .bounds();
        let empty = ui
            .find(i18n::t(Key::DockerInspectShowEmpty))
            .expect("show empty switch")
            .bounds();

        let value = ui.find("850b").expect("general value").bounds();

        assert!(
            (general.x - PADDING.left).abs() < 0.5,
            "{general:?} starts at the side padding"
        );
        assert!(
            value.x + value.width
                <= NARROW - PADDING.right - spacing::SPACE_3 + 0.5,
            "{value:?} stays clear of the scrollbar"
        );
        assert!(
            empty.x + empty.width <= NARROW - PADDING.right + 0.5,
            "{empty:?} stays inside the padding"
        );
    }

    #[test]
    fn the_overview_values_stay_clear_of_the_thin_scrollbar() {
        let themes = ThemeManager::new();
        let fields = vec![text_field(
            Key::DockerFieldCommand,
            "sleep 3600 and a long tail of words that has to wrap",
        )];
        let mut ui = simulator(narrow(view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Overview,
                TabContent::Overview(fields),
            ),
            theme: ThemeProps::new(themes.current()),
        })));

        let value = ui
            .find("sleep 3600 and a long tail of words that has to wrap")
            .expect("value")
            .bounds();

        assert!(
            value.x + value.width
                <= NARROW - PADDING.right - spacing::SPACE_3 + 0.5,
            "{value:?} stays clear of the scrollbar"
        );
    }

    /// Width of the narrow panel in the layout tests.
    const NARROW: f32 = 260.0;

    fn narrow(element: Element<'_, DockerIntent>) -> Element<'_, DockerIntent> {
        container(element).width(NARROW).height(600).into()
    }

    #[test]
    fn a_long_status_line_wraps_inside_the_panel() {
        let themes = ThemeManager::new();
        let status = "Exited (137) 3 hours ago, stopped by the out of memory \
                      killer";
        let mut ui = simulator(narrow(view(InspectorProps {
            vm: Some(InspectorViewModel {
                title: String::from("otty-fx-web-1"),
                status: String::from(status),
                tone: StatusTone::Error,
            }),
            actions: None,
            tabs: None,
            theme: ThemeProps::new(themes.current()),
        })));

        let line = ui.find(status).expect("status should show").bounds();

        assert!(
            line.x + line.width <= NARROW - PADDING.right + 0.5,
            "status {line:?} should stay inside the padding"
        );
        assert!(line.height > 2.0 * 12.5, "status {line:?} should wrap");
    }

    #[test]
    fn a_long_hint_under_the_buttons_wraps_inside_the_panel() {
        let themes = ThemeManager::new();
        let hint = "Used by otty-fx-web-1, otty-fx-web-2 and otty-fx-worker-1";
        let mut ui = simulator(narrow(view(InspectorProps {
            vm: header(),
            actions: Some(ActionsViewModel {
                buttons: vec![button(RowAction::Stop, true)],
                hint: Some(String::from(hint)),
            }),
            tabs: None,
            theme: ThemeProps::new(themes.current()),
        })));

        let line = ui.find(hint).expect("hint should show").bounds();

        assert!(
            line.x + line.width <= NARROW - PADDING.right + 0.5,
            "hint {line:?} should stay inside the padding"
        );
        assert!(line.height > 2.0 * 12.0, "hint {line:?} should wrap");
    }

    #[test]
    fn the_header_the_buttons_the_hint_the_tabs_and_the_body_share_one_side_padding()
     {
        let themes = ThemeManager::new();
        let fields = vec![text_field(Key::DockerColumnImage, "busybox:1.37")];
        let mut ui = simulator(narrow(view(InspectorProps {
            vm: header(),
            actions: Some(ActionsViewModel {
                buttons: vec![button(RowAction::Stop, true)],
                hint: Some(String::from(HINT)),
            }),
            tabs: container_tabs(
                InspectorTab::Overview,
                TabContent::Overview(fields),
            ),
            theme: ThemeProps::new(themes.current()),
        })));

        let left_edges = [
            ("title", ui.find("otty-fx-web-1").expect("title").bounds().x),
            ("hint", ui.find(HINT).expect("hint").bounds().x),
            (
                "tabs",
                ui.find(i18n::t(Key::DockerTabOverview))
                    .expect("tab")
                    .bounds()
                    .x
                    - 2.0,
            ),
            (
                "body",
                ui.find(i18n::t(Key::DockerColumnImage))
                    .expect("label")
                    .bounds()
                    .x,
            ),
        ];
        for (part, x) in left_edges {
            assert!(
                (x - PADDING.left).abs() < 0.5,
                "{part} starts at {x}, not at the side padding"
            );
        }
    }

    fn button(action: RowAction, is_enabled: bool) -> ActionButton {
        ActionButton {
            action,
            label: i18n::t(action.label_key()),
            is_danger: action == RowAction::Remove,
            is_enabled,
        }
    }

    /// Edge of an action icon button.
    const BUTTON: f32 = otty_ui_kit::size::CONTROL_MD;
    /// Hint under the action buttons in the tests.
    const HINT: &str = "Used by otty-fx-web-1";

    /// Inspector with the header, `buttons` and the hint under them.
    fn with_buttons(
        buttons: Vec<ActionButton>,
        theme: ThemeProps<'_>,
    ) -> Element<'_, DockerIntent> {
        view(InspectorProps {
            vm: header(),
            actions: Some(ActionsViewModel {
                buttons,
                hint: Some(String::from(HINT)),
            }),
            tabs: None,
            theme,
        })
    }

    /// Center of the action button at `index`: the buttons sit in one row
    /// from the left padding, right above the hint.
    fn button_center(
        ui: &mut iced_test::Simulator<'_, DockerIntent>,
        index: usize,
    ) -> iced::Point {
        let hint = ui.find(HINT).expect("hint should show").bounds();

        iced::Point::new(
            PADDING.left
                + BUTTON / 2.0
                + index as f32 * (BUTTON + spacing::SPACE_2),
            hint.y - spacing::SPACE_1 - BUTTON / 2.0,
        )
    }

    fn click_at(
        ui: &mut iced_test::Simulator<'_, DockerIntent>,
        point: iced::Point,
    ) {
        ui.point_at(point);
        let _ = ui.simulate(iced_test::simulator::click());
    }

    #[test]
    fn a_press_on_an_action_icon_button_sends_its_action() {
        let themes = ThemeManager::new();
        let mut ui = simulator(with_buttons(
            vec![
                button(RowAction::Stop, true),
                button(RowAction::Remove, true),
            ],
            ThemeProps::new(themes.current()),
        ));

        let stop = button_center(&mut ui, 0);
        let remove = button_center(&mut ui, 1);
        click_at(&mut ui, stop);
        click_at(&mut ui, remove);

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                DockerIntent::Act(RowAction::Stop),
                DockerIntent::Act(RowAction::Remove)
            ]
        );
    }

    #[test]
    fn the_action_buttons_are_icons_whose_labels_are_not_shown_as_text() {
        let themes = ThemeManager::new();
        let mut ui = simulator(with_buttons(
            vec![
                button(RowAction::Stop, true),
                button(RowAction::Remove, true),
            ],
            ThemeProps::new(themes.current()),
        ));

        assert!(ui.find(i18n::t(Key::DockerActionStop)).is_err());
        assert!(ui.find(i18n::t(Key::DockerActionRemove)).is_err());
    }

    #[test]
    fn a_disabled_button_and_its_hint_show_but_send_nothing() {
        let themes = ThemeManager::new();
        let mut ui = simulator(with_buttons(
            vec![button(RowAction::Remove, false)],
            ThemeProps::new(themes.current()),
        ));

        let remove = button_center(&mut ui, 0);
        click_at(&mut ui, remove);
        ui.find(HINT).expect("hint should show");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert!(intents.is_empty());
    }

    /// Draws the inspector whose first action is Stop labeled `label`, with
    /// the pointer over that button, and checks it against the picture saved
    /// in `reference`; the first call saves the picture. Tooltips are
    /// overlays that widget queries do not reach.
    fn hovered_button_matches(
        label: &'static str,
        reference: &std::path::Path,
    ) -> bool {
        let themes = ThemeManager::new();
        let stop = ActionButton {
            label,
            ..button(RowAction::Stop, true)
        };
        let mut ui = simulator(with_buttons(
            vec![stop, button(RowAction::Remove, true)],
            ThemeProps::new(themes.current()),
        ));
        let center = button_center(&mut ui, 0);
        ui.point_at(center);
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::CursorMoved { position: center },
        )]);

        ui.snapshot(&iced::Theme::Dark)
            .expect("inspector should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn a_hovered_action_button_shows_its_label_as_a_tooltip() {
        let reference = std::env::temp_dir()
            .join(format!("otty-docker-inspector-{}", std::process::id()))
            .join("tooltip");

        assert!(hovered_button_matches("Stop", &reference));
        assert!(!hovered_button_matches("Stop now", &reference));
        assert!(hovered_button_matches("Stop", &reference));
    }
}
