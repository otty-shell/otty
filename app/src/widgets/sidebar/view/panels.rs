use iced::widget::Space;
use iced::widget::pane_grid::{self, Content, PaneGrid};
use iced::{Element, Length};
use otty_ui_kit::components::split;
use otty_ui_kit::components::surface::{Surface, surface};
use otty_ui_kit::size;

use super::super::event::SidebarIntent;
use super::super::model::SidebarViewModel;
use super::super::types::SidebarPane;
use crate::theme::ThemeProps;

/// Props for the `left panel | main | right panel` split around the main
/// area.
///
/// The sidebar owns the split, the panel widths and focus mode; what goes
/// into each slot comes from other widgets through `content`.
pub(crate) struct PanelsProps<'a, Message, Slot> {
    pub(crate) panes: &'a pane_grid::State<SidebarPane>,
    pub(crate) vm: SidebarViewModel,
    pub(crate) theme: ThemeProps<'a>,
    /// Builds the content of a slot. It is called only for the main area
    /// and the panels that are shown.
    pub(crate) content: Slot,
    /// Turns a sidebar intent into the caller's message.
    pub(crate) on_intent: fn(SidebarIntent) -> Message,
}

/// Render `left panel | main | right panel` with a pane grid. A closed panel
/// is an empty zero-width pane, so its edge can still be dragged open.
///
/// Focus mode keeps the same pane grid with both panels at zero width, so
/// the main area keeps its widget state (terminal focus, scroll); only the
/// drag handles are removed.
pub(crate) fn view<'a, Message, Slot>(
    props: PanelsProps<'a, Message, Slot>,
) -> Element<'a, Message>
where
    Message: 'a,
    Slot: Fn(SidebarPane) -> Element<'a, Message>,
{
    let vm = props.vm;
    let palette = props.theme.theme.ui();
    let on_intent = props.on_intent;
    let content = props.content;

    let panels = PaneGrid::new(props.panes, |_pane, slot, _| {
        let element = match *slot {
            SidebarPane::Left if vm.is_left_open => {
                surface(content(SidebarPane::Left), Surface::Panel, palette)
                    .into()
            },
            SidebarPane::Right if vm.is_right_open => {
                surface(content(SidebarPane::Right), Surface::Panel, palette)
                    .into()
            },
            SidebarPane::Main => content(SidebarPane::Main),
            _ => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        Content::new(element)
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(size::LINE)
    .min_size(0)
    .style(split::style(palette));

    if vm.is_focus_mode {
        return panels.into();
    }

    panels
        .on_resize(size::SPLIT_GRAB, move |event| {
            on_intent(SidebarIntent::Resized(event))
        })
        .into()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use iced::widget::text;

    use super::*;
    use crate::theme::ThemeManager;
    use crate::widgets::sidebar::SidebarWidget;

    /// Render the panels of `sidebar` and return the slots whose content
    /// was built, in pane grid order.
    fn built_slots(sidebar: &SidebarWidget) -> Vec<SidebarPane> {
        let themes = ThemeManager::new();
        let built = RefCell::new(Vec::new());

        let _panels: Element<'_, ()> = view(PanelsProps {
            panes: sidebar.panes(),
            vm: sidebar.vm(),
            theme: ThemeProps::new(themes.current()),
            content: |slot| {
                built.borrow_mut().push(slot);
                text("slot").into()
            },
            on_intent: |_| (),
        });

        let mut slots = built.into_inner();
        slots.sort_by_key(|slot| *slot as u8);
        slots
    }

    #[test]
    fn open_panels_and_the_main_area_get_their_content() {
        let sidebar = SidebarWidget::new();

        let slots = built_slots(&sidebar);

        assert_eq!(
            slots,
            vec![SidebarPane::Left, SidebarPane::Main, SidebarPane::Right]
        );
    }

    #[test]
    fn closed_panel_content_is_not_built() {
        let mut sidebar = SidebarWidget::new();
        let _ = sidebar.reduce(SidebarIntent::ToggleRight);

        let slots = built_slots(&sidebar);

        assert_eq!(slots, vec![SidebarPane::Left, SidebarPane::Main]);
    }

    #[test]
    fn focus_mode_builds_only_the_main_area() {
        let mut sidebar = SidebarWidget::new();
        let _ = sidebar.reduce(SidebarIntent::ToggleFocusMode);

        let slots = built_slots(&sidebar);

        assert_eq!(slots, vec![SidebarPane::Main]);
    }
}
