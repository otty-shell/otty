//! The Docker mode right panel: the inspector header of the selected row.
//! Its buttons and tabs come with tickets 11 to 13.

use iced::widget::{container, text};
use iced::{Element, Length, Padding};
use otty_ui_kit::components::inspector_header::InspectorHeader;
use otty_ui_kit::components::status_badge::StatusBadge;
use otty_ui_kit::{fonts, typography};

use super::super::event::DockerIntent;
use super::super::model::InspectorViewModel;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Padding of the inspector (`padding: 14px 16px`).
const PADDING: Padding = Padding {
    top: 14.0,
    right: 16.0,
    bottom: 14.0,
    left: 16.0,
};

/// Props for the inspector.
pub(crate) struct InspectorProps<'a> {
    pub(crate) vm: Option<InspectorViewModel>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the header of the selected row: its name and status line. With
/// nothing selected the panel says there is nothing to inspect.
pub(crate) fn view(props: InspectorProps<'_>) -> Element<'_, DockerIntent> {
    let palette = props.theme.theme.ui();

    let Some(vm) = props.vm else {
        let hint = typography::SMALL;

        return container(
            text(i18n::t(Key::DockerNothingToInspect))
                .size(hint.size())
                .line_height(iced::Pixels(hint.line_height()))
                .font(fonts::UI)
                .color(palette.text_muted()),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(PADDING)
        .into();
    };

    let status = StatusBadge::new(vm.status, vm.tone, palette);
    let header = InspectorHeader::new(vm.title, palette).status(status);

    container(header)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(PADDING)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_ui_kit::components::status_badge::StatusTone;

    use super::*;
    use crate::theme::ThemeManager;

    #[test]
    fn the_header_shows_the_name_and_the_status() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(InspectorProps {
            vm: Some(InspectorViewModel {
                title: String::from("otty-fx-web-1"),
                status: String::from("Up 2 hours"),
                tone: StatusTone::Ok,
            }),
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
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find("otty-fx-web-1").is_err());
        ui.find(i18n::t(Key::DockerNothingToInspect))
            .expect("the hint should show");
    }
}
