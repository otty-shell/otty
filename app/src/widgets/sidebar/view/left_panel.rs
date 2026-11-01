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
    /// Builds both sections with the heights their rows need. It runs
    /// during layout, once per panel size, so each section's data is built
    /// once and used for both its height and its rows.
    pub(crate) sections: Sections,
}

/// The two sections of the left panel and the heights their rows need to
/// be shown without scrolling.
pub(crate) struct LeftPanelSections<'a, Message> {
    pub(crate) sessions: Element<'a, Message>,
    pub(crate) sessions_content_height: f32,
    pub(crate) launches: Element<'a, Message>,
    pub(crate) launches_content_height: f32,
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
    Sections: Fn() -> LeftPanelSections<'a, Message> + 'a,
{
    let vm = props.vm;
    let palette = props.theme.theme.ui();
    let sections = props.sections;

    responsive(move |panel| {
        let LeftPanelSections {
            sessions,
            sessions_content_height,
            launches,
            launches_content_height,
        } = sections();

        let sessions_height = SectionHeight::panel_section(
            sessions_content_height,
            vm.is_sessions_open,
        );
        let launches_height = SectionHeight::panel_section(
            launches_content_height,
            vm.is_launches_open,
        );
        let available = (panel.height - size::LINE).max(0.0);
        let heights =
            share_panel_height(available, &[sessions_height, launches_height]);
        let sessions_share = heights.first().copied().unwrap_or(0.0);

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
            sections: || LeftPanelSections {
                sessions: text("Sessions rows").into(),
                sessions_content_height: 40.0,
                launches: text("Launches rows").into(),
                launches_content_height: 40.0,
            },
        });
        let mut ui = simulator(panel);

        ui.find("Sessions rows")
            .expect("sessions should be visible");
        ui.find("Launches rows")
            .expect("launches should be visible");
    }
}
