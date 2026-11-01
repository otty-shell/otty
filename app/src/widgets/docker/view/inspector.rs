//! The Docker mode right panel: the inspector header of the selected row,
//! its tabs (Overview, Logs, Inspect) and the open tab. Its buttons come
//! with tickets 11 and 12.

use iced::widget::{column, container, scrollable, svg, text};
use iced::{Element, Length, Padding};
use otty_ui_kit::components::inspector_header::InspectorHeader;
use otty_ui_kit::components::inspector_tabs::inspector_tabs;
use otty_ui_kit::components::key_value_list::KeyValueList;
use otty_ui_kit::components::log_view::LogView;
use otty_ui_kit::components::notice::Notice;
use otty_ui_kit::components::status_badge::StatusBadge;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::DockerIntent;
use super::super::inspector_tabs::{
    InspectContent, InspectorTabsViewModel, TabContent,
};
use super::super::model::InspectorViewModel;
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
    /// The tabs under the header and the open tab's body.
    pub(crate) tabs: Option<InspectorTabsViewModel<'a>>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the header of the selected row (its name and status line), the
/// tabs and the open tab. With nothing selected the panel says there is
/// nothing to inspect.
pub(crate) fn view(props: InspectorProps<'_>) -> Element<'_, DockerIntent> {
    let palette = props.theme.theme.ui();

    let Some(vm) = props.vm else {
        return container(hint(i18n::t(Key::DockerNothingToInspect), palette))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(PADDING)
            .into();
    };

    let status = StatusBadge::new(vm.status, vm.tone, palette);
    let header = InspectorHeader::new(vm.title, palette).status(status);

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

/// The body of the open tab.
fn tab_body<'a>(
    content: TabContent<'a>,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    match content {
        TabContent::Overview(fields) => {
            let mut list = KeyValueList::new(palette);
            for (label, value) in fields {
                list = list.entry(label, value);
            }
            scrollable(list).height(Length::Fill).into()
        },
        TabContent::Logs { lines, error } => {
            let mut body = column![hint(i18n::t(Key::DockerLogsHint), palette)]
                .spacing(spacing::SPACE_3)
                .height(Length::Fill);
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
        TabContent::Inspect(InspectContent::Loaded(json)) => {
            LogView::new(json, palette)
                .on_action(DockerIntent::InspectAction)
                .into()
        },
    }
}

/// A muted small line of text.
fn hint<'a>(
    message: &'a str,
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
    use crate::widgets::docker::inspector::InspectorTab;
    use crate::widgets::docker::inspector_tabs::{InspectContent, TabContent};
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
            tabs: None,
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find("otty-fx-web-1").is_err());
        ui.find(i18n::t(Key::DockerNothingToInspect))
            .expect("the hint should show");
    }

    #[test]
    fn the_tabs_show_under_the_header_and_a_press_selects_one() {
        let themes = ThemeManager::new();
        let fields = vec![(
            i18n::t(Key::DockerColumnImage),
            String::from("busybox:1.37"),
        )];
        let mut ui = simulator(view(InspectorProps {
            vm: header(),
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
            (
                i18n::t(Key::DockerColumnImage),
                String::from("busybox:1.37"),
            ),
            (i18n::t(Key::DockerFieldCommand), String::from("sleep 3600")),
        ];
        let mut ui = simulator(view(InspectorProps {
            vm: header(),
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

    #[test]
    fn the_logs_tab_shows_its_hint_and_the_error() {
        let themes = ThemeManager::new();
        let mut lines = LogViewState::new();
        lines.push_lines(["ready"]);
        let mut ui = simulator(view(InspectorProps {
            vm: header(),
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
    fn the_inspect_tab_says_it_loads_or_why_it_failed() {
        let themes = ThemeManager::new();
        let mut loading = simulator(view(InspectorProps {
            vm: header(),
            tabs: container_tabs(
                InspectorTab::Inspect,
                TabContent::Inspect(InspectContent::Loading),
            ),
            theme: ThemeProps::new(themes.current()),
        }));
        let mut failed = simulator(view(InspectorProps {
            vm: header(),
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

    #[test]
    fn the_loaded_json_renders_in_both_themes() {
        let mut themes = ThemeManager::new();
        let mut json = LogViewState::new();
        json.push_lines(["{", "  \"Id\": \"850b\"", "}"]);

        for theme in BuiltinTheme::ALL {
            themes.set_theme(theme);
            let mut ui = simulator(view(InspectorProps {
                vm: header(),
                tabs: container_tabs(
                    InspectorTab::Inspect,
                    TabContent::Inspect(InspectContent::Loaded(&json)),
                ),
                theme: ThemeProps::new(themes.current()),
            }));

            ui.find("otty-fx-web-1").expect("the header stays");
            assert!(ui.find(i18n::t(Key::DockerLoading)).is_err());
        }
    }
}
