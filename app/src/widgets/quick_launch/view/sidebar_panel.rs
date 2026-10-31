use iced::widget::{Space, column, row, svg, text};
use iced::{Element, Length, Padding, Theme};
use otty_ui_kit::components::icon_button::{IconButton, IconButtonSize};
use otty_ui_kit::components::surface::{Surface, surface};
use otty_ui_kit::{fonts, size, spacing, typography};

use super::super::event::QuickLaunchIntent;
use super::super::model::QuickLaunchTreeViewModel;
use super::sidebar_tree;
use crate::i18n::{self, Key};
use crate::icons;
use crate::theme::ThemeProps;

/// Props for the quick launch sidebar panel.
pub(crate) struct SidebarPanelProps<'a> {
    pub(crate) vm: QuickLaunchTreeViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch sidebar panel with header and tree.
pub(crate) fn view(
    props: SidebarPanelProps<'_>,
) -> Element<'_, QuickLaunchIntent, Theme, iced::Renderer> {
    let palette = props.theme.theme.ui();
    let title_style = typography::HEADING;

    let add_button = IconButton::new(
        svg::Handle::from_memory(icons::ADD_TAB_HEADER),
        i18n::t(Key::CreateQuickLaunch),
        palette,
    )
    .size(IconButtonSize::Medium)
    .on_press(QuickLaunchIntent::HeaderAddButtonPressed);

    // The header is as tall as the tab strip so both bottom edges line up.
    let header = surface(
        row![
            text(i18n::t(Key::QuickLaunchPanelTitle))
                .size(title_style.size())
                .font(fonts::UI_SEMIBOLD)
                .color(palette.text_strong()),
            Space::new().width(Length::Fill),
            add_button,
        ]
        .align_y(iced::alignment::Vertical::Center)
        .width(Length::Fill),
        Surface::Panel,
        palette,
    )
    .height(size::TAB)
    .padding(Padding::from([0.0, spacing::SPACE_4]))
    .align_y(iced::alignment::Vertical::Center);

    let tree = sidebar_tree::view(sidebar_tree::SidebarTreeProps {
        data: props.vm.data,
        selected_path: props.vm.selected_path,
        hovered_path: props.vm.hovered_path,
        inline_edit: props.vm.inline_edit,
        launching: props.vm.launching,
        drop_target: props.vm.drop_target,
        theme: props.theme,
    });

    column![header, tree]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(0)
        .into()
}
