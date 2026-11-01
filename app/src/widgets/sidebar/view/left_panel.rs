use iced::widget::{column, container, responsive};
use iced::{Element, Length};
use otty_ui_kit::components::divider;
use otty_ui_kit::components::panel_height::{
    SectionHeight, share_panel_height,
};
use otty_ui_kit::size;

use super::super::model::SidebarViewModel;
use crate::theme::ThemeProps;

/// Props for the left panel: the Sessions section over the Launches section.
///
/// The sidebar owns which sections are open and how they share the panel
/// height; the sections themselves come from the tabs and quick launch
/// widgets through `sections`.
pub(crate) struct LeftPanelProps<'a, Sections> {
    pub(crate) vm: SidebarViewModel,
    pub(crate) theme: ThemeProps<'a>,
    /// Height the Sessions rows need to be shown without scrolling.
    pub(crate) sessions_content_height: f32,
    /// Height the Launches rows need to be shown without scrolling.
    pub(crate) launches_content_height: f32,
    /// Builds the Sessions and Launches sections. It runs again whenever
    /// the panel size changes.
    pub(crate) sections: Sections,
}

/// Stack the Sessions and Launches sections with a line between them and
/// share the panel height with [`share_panel_height`]: each open section
/// takes its content height while everything fits, and scrolls inside its
/// share when it does not.
///
/// The panel height is known only during layout, so this uses
/// `iced::widget::responsive`. Launches is the last section and takes all
/// height left under Sessions, so its empty area still takes drops and
/// right clicks.
pub(crate) fn view<'a, Message, Sections>(
    props: LeftPanelProps<'a, Sections>,
) -> Element<'a, Message>
where
    Message: 'a,
    Sections: Fn() -> (Element<'a, Message>, Element<'a, Message>) + 'a,
{
    let sessions_height = SectionHeight::panel_section(
        props.sessions_content_height,
        props.vm.is_sessions_open,
    );
    let launches_height = SectionHeight::panel_section(
        props.launches_content_height,
        props.vm.is_launches_open,
    );
    let palette = props.theme.theme.ui();
    let sections = props.sections;

    responsive(move |panel| {
        let available = (panel.height - size::LINE).max(0.0);
        let heights =
            share_panel_height(available, &[sessions_height, launches_height]);
        let sessions_share = heights.first().copied().unwrap_or(0.0);

        let (sessions, launches) = sections();

        column![
            container(sessions)
                .width(Length::Fill)
                .height(sessions_share)
                .clip(true),
            divider::horizontal(palette),
            container(launches).width(Length::Fill).height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::text;
    use iced_test::simulator;

    use super::*;
    use crate::theme::ThemeManager;
    use crate::widgets::sidebar::SidebarWidget;

    #[test]
    fn both_sections_are_shown_in_the_panel() {
        let themes = ThemeManager::new();
        let sidebar = SidebarWidget::new();

        let panel: Element<'_, ()> = view(LeftPanelProps {
            vm: sidebar.vm(),
            theme: ThemeProps::new(themes.current()),
            sessions_content_height: 40.0,
            launches_content_height: 40.0,
            sections: || {
                (text("Sessions rows").into(), text("Launches rows").into())
            },
        });
        let mut ui = simulator(panel);

        ui.find("Sessions rows")
            .expect("sessions should be visible");
        ui.find("Launches rows")
            .expect("launches should be visible");
    }
}
