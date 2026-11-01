//! The Docker mode right panel: the inspector header of the selected row.
//! Its buttons and tabs come with tickets 11 to 13.

use iced::widget::{Space, container};
use iced::{Element, Length, Padding};
use otty_ui_kit::components::inspector_header::InspectorHeader;
use otty_ui_kit::components::status_badge::StatusBadge;

use super::super::event::DockerIntent;
use super::super::model::InspectorViewModel;
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
/// nothing selected the panel stays empty.
pub(crate) fn view(props: InspectorProps<'_>) -> Element<'_, DockerIntent> {
    let Some(vm) = props.vm else {
        return Space::new().width(Length::Fill).height(Length::Fill).into();
    };

    let palette = props.theme.theme.ui();
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
    fn nothing_selected_shows_nothing() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(InspectorProps {
            vm: None,
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find("otty-fx-web-1").is_err());
    }
}
