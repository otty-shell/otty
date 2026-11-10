//! The Containers mode right panel: the inspector header of the selected row
//! with its action buttons, its tabs (Overview, Logs, Inspect) and the open
//! tab, whose body the inspector module draws.

use iced::widget::text::IntoFragment;
use iced::widget::{column, container, svg, text};
use iced::{Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::icon_button::{IconButton, IconTone};
use otty_ui_kit::components::inspector_header::InspectorHeader;
use otty_ui_kit::components::inspector_tabs::inspector_tabs;
use otty_ui_kit::components::status_badge::StatusBadge;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, typography};

use super::super::actions::ActionsViewModel;
use super::super::actions::event::ActionsIntent;
use super::super::actions::types::RowAction;
use super::super::event::ContainersIntent;
use super::super::inspector::InspectorTabsViewModel;
use super::super::inspector::view::tab_body::{self, TabBodyProps};
use super::super::model::InspectorViewModel;

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
pub(crate) fn view(props: InspectorProps<'_>) -> Element<'_, ContainersIntent> {
    let palette = props.theme.theme.ui();

    let Some(vm) = props.vm else {
        return container(hint(
            i18n::t(Key::ContainersNothingToInspect),
            palette,
        ))
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
            .on_press_maybe(button.is_enabled.then_some(
                ContainersIntent::Actions(ActionsIntent::Act(button.action)),
            ));
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
        let body = tab_body::view(TabBodyProps {
            content: tabs.content,
            theme: props.theme,
        })
        .map(ContainersIntent::Inspector);

        panel = panel
            .push(inspector_tabs(
                tabs.tabs,
                &tabs.selected,
                ContainersIntent::SelectInspectorTab,
                palette,
            ))
            .push(body);
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

/// A muted small line of text.
fn hint<'a>(
    message: impl IntoFragment<'a>,
    palette: &UiPalette,
) -> Element<'a, ContainersIntent> {
    let style = typography::SMALL;

    text(message)
        .size(style.size())
        .line_height(iced::Pixels(style.line_height()))
        .font(fonts::UI)
        .color(palette.text_muted())
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_testing::temp_dir::TempDir;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::status_badge::StatusTone;
    use otty_ui_kit::spacing;

    use super::*;
    use crate::actions::ActionButton;
    use crate::inspector::TabContent;
    use crate::overview::{Field, Piece};
    use crate::types::{InspectorTab, ResourceKind};

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
        ui.find(i18n::t(Key::ContainersNothingToInspect))
            .expect("the hint should show");
    }

    /// An Overview field with a plain text value.
    fn text_field(key: Key, value: &str) -> Field {
        (i18n::t(key), vec![Piece::Text(String::from(value))])
    }

    #[test]
    fn the_tabs_show_under_the_header_and_a_press_selects_one() {
        let themes = ThemeManager::new();
        let fields =
            vec![text_field(Key::ContainersColumnImage, "busybox:1.37")];
        let mut ui = simulator(view(InspectorProps {
            vm: header(),
            actions: None,
            tabs: container_tabs(
                InspectorTab::Overview,
                TabContent::Overview(fields),
            ),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::ContainersTabOverview))
            .expect("overview tab");
        ui.find(i18n::t(Key::ContainersTabInspect))
            .expect("inspect tab");
        ui.click(i18n::t(Key::ContainersTabLogs)).expect("logs tab");

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![ContainersIntent::SelectInspectorTab(InspectorTab::Logs)]
        );
    }

    /// Width of the narrow panel in the layout tests.
    const NARROW: f32 = 260.0;

    fn narrow(
        element: Element<'_, ContainersIntent>,
    ) -> Element<'_, ContainersIntent> {
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
        let fields =
            vec![text_field(Key::ContainersColumnImage, "busybox:1.37")];
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
                ui.find(i18n::t(Key::ContainersTabOverview))
                    .expect("tab")
                    .bounds()
                    .x
                    - 2.0,
            ),
            (
                "body",
                ui.find(i18n::t(Key::ContainersColumnImage))
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
    ) -> Element<'_, ContainersIntent> {
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
        ui: &mut iced_test::Simulator<'_, ContainersIntent>,
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
        ui: &mut iced_test::Simulator<'_, ContainersIntent>,
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

        let intents: Vec<ContainersIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                ContainersIntent::Actions(ActionsIntent::Act(RowAction::Stop)),
                ContainersIntent::Actions(ActionsIntent::Act(
                    RowAction::Remove
                ))
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
                button(RowAction::Unpause, true),
                button(RowAction::PullAgain, true),
            ],
            ThemeProps::new(themes.current()),
        ));

        assert!(ui.find(i18n::t(Key::ContainersActionStop)).is_err());
        assert!(ui.find(i18n::t(Key::ContainersActionRemove)).is_err());
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

        let intents: Vec<ContainersIntent> = ui.into_messages().collect();
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
        let folder = TempDir::new("containers-inspector");
        let reference = folder.path().join("tooltip");

        assert!(hovered_button_matches("Stop", &reference));
        assert!(!hovered_button_matches("Stop now", &reference));
        assert!(hovered_button_matches("Stop", &reference));
    }
}
