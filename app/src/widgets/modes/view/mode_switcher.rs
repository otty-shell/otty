//! The mode switcher row at the top of the left panel.

use iced::widget::{row, svg, text};
use iced::{Alignment, Element, Length, Padding};
use otty_ui_kit::components::mode_switcher::{ModeSwitcher, ModeTab};
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::ModesIntent;
use super::super::model::ModesViewModel;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Space around the switcher row (`padding: 8px 8px 4px`).
const ROW_PADDING: Padding = Padding {
    top: spacing::SPACE_3,
    right: spacing::SPACE_3,
    bottom: spacing::SPACE_1,
    left: spacing::SPACE_3,
};

/// Props for the mode switcher row.
pub(crate) struct ModeSwitcherProps<'a> {
    pub(crate) vm: ModesViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the kit `ModeSwitcher` (Terminal, the pinned modes, the temporary
/// tab, "⋯") with the name of the active mode next to it (spec Decision
/// 2). A tab selects its mode; "⋯" opens or closes the All modes menu.
pub(crate) fn view<'a>(
    props: ModeSwitcherProps<'a>,
) -> Element<'a, ModesIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let mut switcher = ModeSwitcher::new(palette);
    for tab in &vm.tabs {
        switcher = switcher.tab(
            ModeTab::new(
                svg::Handle::from_memory(tab.mode.icon()),
                i18n::t(tab.mode.name_key()),
            )
            .active(tab.is_active)
            .temporary(tab.is_temporary)
            .on_press(ModesIntent::Select(tab.mode)),
        );
    }
    let switcher = switcher.more(
        i18n::t(Key::ModesAllModes),
        vm.is_menu_open,
        ModesIntent::ToggleMenu,
    );

    let name = text(i18n::t(vm.active.name_key()))
        .size(typography::BODY.size())
        .font(fonts::UI_SEMIBOLD)
        .color(palette.text_strong())
        .width(Length::Fill);

    row![Element::from(switcher), name]
        .spacing(spacing::SPACE_3)
        .padding(ROW_PADDING)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::{Simulator, simulator};
    use otty_ui_kit::size;

    use super::*;
    use crate::theme::ThemeManager;
    use crate::widgets::modes::state::ModesState;
    use crate::widgets::modes::types::Mode;

    /// Padding of the kit switcher group around its tabs.
    const GROUP_PADDING: f32 = 3.0;
    /// Gap between two kit switcher tabs.
    const TAB_GAP: f32 = 2.0;

    fn tab_center(index: usize) -> Point {
        Point::new(
            ROW_PADDING.left
                + GROUP_PADDING
                + size::MODE_TAB_WIDTH / 2.0
                + index as f32 * (size::MODE_TAB_WIDTH + TAB_GAP),
            ROW_PADDING.top + GROUP_PADDING + size::MODE_TAB_HEIGHT / 2.0,
        )
    }

    fn click_at(ui: &mut Simulator<'_, ModesIntent>, point: Point) {
        ui.point_at(point);
        let _ = ui.simulate(iced_test::simulator::click());
    }

    fn clicked(state: &ModesState, point: Point) -> Vec<ModesIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ModeSwitcherProps {
            vm: ModesViewModel::from_state(state),
            theme: ThemeProps::new(themes.current()),
        }));

        click_at(&mut ui, point);

        ui.into_messages().collect()
    }

    #[test]
    fn clicking_a_tab_selects_its_mode() {
        let intents = clicked(&ModesState::default(), tab_center(1));

        assert!(matches!(
            intents.as_slice(),
            [ModesIntent::Select(Mode::Docker)]
        ));
    }

    #[test]
    fn clicking_the_terminal_tab_selects_terminal_mode() {
        let mut state = ModesState::default();
        state.select(Mode::Docker);

        let intents = clicked(&state, tab_center(0));

        assert!(matches!(
            intents.as_slice(),
            [ModesIntent::Select(Mode::Terminal)]
        ));
    }

    #[test]
    fn clicking_more_opens_or_closes_the_all_modes_menu() {
        let intents = clicked(&ModesState::default(), tab_center(2));

        assert!(matches!(intents.as_slice(), [ModesIntent::ToggleMenu]));
    }

    #[test]
    fn a_temporary_tab_comes_before_more() {
        let mut state = ModesState::with_pinned(&[]);
        state.select(Mode::Docker);

        let temporary = clicked(&state, tab_center(1));
        let more = clicked(&state, tab_center(2));

        assert!(matches!(
            temporary.as_slice(),
            [ModesIntent::Select(Mode::Docker)]
        ));
        assert!(matches!(more.as_slice(), [ModesIntent::ToggleMenu]));
    }

    #[test]
    fn the_name_of_the_active_mode_is_shown_next_to_the_tabs() {
        let themes = ThemeManager::new();
        let mut state = ModesState::default();
        state.select(Mode::Docker);
        let mut ui = simulator(view(ModeSwitcherProps {
            vm: ModesViewModel::from_state(&state),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::ModeDocker))
            .expect("active mode name should be visible");
        assert!(ui.find(i18n::t(Key::ModeTerminal)).is_err());
    }
}
