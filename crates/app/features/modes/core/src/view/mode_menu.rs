//! The All modes menu, opened by the "⋯" button of the mode switcher.

use iced::widget::{Space, container, mouse_area, opaque, stack, svg};
use iced::{Element, Length, Padding, alignment, mouse};
use otty_app_domain::shortcuts::AppShortcut;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons::mode_icon;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::mode_menu::{
    ModeMenu, ModeMenuSection, ModePin, ModeRow,
};

use super::super::event::ModesIntent;
use super::super::model::{ModeMenuRowViewModel, ModesViewModel, PinButton};
use super::name_key;

/// Distance from the top of the panels to the menu: under the switcher
/// row, as on the `DockerMode` board (`top: 42px`).
const MENU_TOP: f32 = 42.0;
/// Distance from the left edge of the window to the menu (`left: 8px`),
/// lined up with the switcher.
const MENU_LEFT: f32 = 8.0;

/// Props for the All modes menu.
pub(crate) struct ModeMenuProps<'a> {
    pub(crate) vm: ModesViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the kit `ModeMenu` under the switcher over a transparent layer
/// (spec Decision 3). A row selects its mode, the pin toggle pins or
/// unpins it, and a press of any button outside the menu closes it.
pub(crate) fn view<'a>(props: ModeMenuProps<'a>) -> Element<'a, ModesIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;

    let mut pinned = ModeMenuSection::new(i18n::t(Key::ModesPinnedSection))
        .note(i18n::modes_pinned_note(vm.pinned_count, vm.pin_limit));
    // A pinned row's place in the list is its switcher position, so it
    // carries that position's shortcut.
    for (position, menu_row) in vm.pinned_rows.iter().enumerate() {
        let mut pinned_row = mode_row(menu_row);
        if let Some(shortcut) = AppShortcut::for_mode_position(position) {
            pinned_row = pinned_row.shortcut(shortcut.label());
        }

        pinned = pinned.row(pinned_row);
    }

    let mut others = ModeMenuSection::new(i18n::t(Key::ModesOtherSection));
    for menu_row in &vm.other_rows {
        others = others.row(mode_row(menu_row));
    }
    if vm.other_rows.is_empty() {
        others = others.hint(i18n::t(Key::ModesNoOtherModes));
    }

    let menu: Element<'a, ModesIntent> = ModeMenu::new(palette)
        .section(pinned)
        .section(others)
        .into();

    // Presses on the menu's own padding must not reach the layer below.
    let positioned = container(opaque(menu))
        .padding(Padding {
            top: MENU_TOP,
            left: MENU_LEFT,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Left)
        .align_y(alignment::Vertical::Top);

    let dismiss_layer =
        mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(ModesIntent::CloseMenu)
            .on_right_press(ModesIntent::CloseMenu)
            .on_middle_press(ModesIntent::CloseMenu)
            .interaction(mouse::Interaction::Idle);

    stack![dismiss_layer, positioned]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// One menu row: pressing the name selects the mode.
fn mode_row(menu_row: &ModeMenuRowViewModel) -> ModeRow<ModesIntent> {
    let mode = menu_row.mode;

    ModeRow::new(
        svg::Handle::from_memory(mode_icon(mode)),
        i18n::t(name_key(mode)),
    )
    .pin(pin_toggle(menu_row))
    .on_press(ModesIntent::Select(mode))
}

/// The pin toggle of a row and the intent it sends.
fn pin_toggle(menu_row: &ModeMenuRowViewModel) -> ModePin<ModesIntent> {
    let mode = menu_row.mode;

    match menu_row.pin {
        PinButton::Unpin => ModePin::Pinned {
            label: i18n::t(Key::ModesUnpin).to_owned(),
            on_press: ModesIntent::Unpin(mode),
        },
        PinButton::Pin => ModePin::Unpinned {
            label: i18n::t(Key::ModesPin).to_owned(),
            on_press: ModesIntent::Pin(mode),
        },
        PinButton::PinDisabled => ModePin::Disabled {
            hint: i18n::t(Key::ModesUnpinAnotherFirst).to_owned(),
        },
        _ => ModePin::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::{Simulator, simulator};
    use otty_app_domain::mode::Mode;
    use otty_app_testing::mouse::middle_click;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::state::ModesState;

    fn drive(
        state: &ModesState,
        steps: impl FnOnce(&mut Simulator<'_, ModesIntent>),
    ) -> Vec<ModesIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ModeMenuProps {
            vm: ModesViewModel::from_state(state),
            theme: ThemeProps::new(themes.current()),
        }));

        steps(&mut ui);

        ui.into_messages().collect()
    }

    fn row(mode: Mode, pin: PinButton) -> ModeMenuRowViewModel {
        ModeMenuRowViewModel {
            mode,
            is_active: false,
            pin,
        }
    }

    #[test]
    fn the_menu_lists_pinned_with_its_note_and_other_modes() {
        let _ = drive(&ModesState::default(), |ui| {
            ui.find(i18n::t(Key::ModesPinnedSection))
                .expect("pinned section should be visible");
            ui.find(i18n::modes_pinned_note(1, 4).as_str())
                .expect("pinned note should be visible");
            ui.find(i18n::t(Key::ModesOtherSection))
                .expect("other modes section should be visible");
            ui.find(i18n::t(Key::ModesNoOtherModes))
                .expect("empty other modes hint should be visible");
        });
    }

    #[test]
    fn pinned_rows_show_their_mode_shortcuts() {
        let _ = drive(&ModesState::default(), |ui| {
            ui.find(AppShortcut::TerminalMode.label().as_str())
                .expect("terminal shortcut should be visible");
            ui.find(AppShortcut::PinnedMode1.label().as_str())
                .expect("containers shortcut should be visible");
        });
    }

    #[test]
    fn other_modes_have_no_shortcut() {
        let _ = drive(&ModesState::with_pinned(&[]), |ui| {
            assert!(
                ui.find(AppShortcut::PinnedMode1.label().as_str()).is_err()
            );
        });
    }

    #[test]
    fn clicking_a_mode_name_selects_it() {
        let intents = drive(&ModesState::default(), |ui| {
            ui.click(i18n::t(Key::ModeContainers))
                .expect("containers row should be visible");
        });

        assert!(matches!(
            intents.as_slice(),
            [ModesIntent::Select(Mode::Containers)]
        ));
    }

    #[test]
    fn an_unpinned_mode_is_listed_under_other_modes_without_the_hint() {
        let _ = drive(&ModesState::with_pinned(&[]), |ui| {
            ui.find(i18n::t(Key::ModeContainers))
                .expect("containers row should be visible");
            assert!(ui.find(i18n::t(Key::ModesNoOtherModes)).is_err());
        });
    }

    #[test]
    fn a_click_outside_the_menu_closes_it() {
        let intents = drive(&ModesState::default(), |ui| {
            ui.point_at(Point::new(700.0, 600.0));
            let _ = ui.simulate(iced_test::simulator::click());
        });

        assert!(matches!(intents.as_slice(), [ModesIntent::CloseMenu]));
    }

    /// A middle press away from the menu closes it, as a left press does.
    #[test]
    fn a_middle_press_outside_the_menu_closes_it() {
        let intents = drive(&ModesState::default(), |ui| {
            ui.point_at(Point::new(700.0, 600.0));
            let _ = ui.simulate(middle_click());
        });

        assert!(matches!(intents.as_slice(), [ModesIntent::CloseMenu]));
    }

    #[test]
    fn a_click_on_the_menu_padding_does_not_close_it() {
        let intents = drive(&ModesState::default(), |ui| {
            ui.point_at(Point::new(MENU_LEFT + 2.0, MENU_TOP + 2.0));
            let _ = ui.simulate(iced_test::simulator::click());
        });

        assert!(intents.is_empty());
    }

    #[test]
    fn a_pinned_mode_toggle_unpins_it() {
        let pin = pin_toggle(&row(Mode::Containers, PinButton::Unpin));

        assert!(matches!(
            pin,
            ModePin::Pinned {
                on_press: ModesIntent::Unpin(Mode::Containers),
                ..
            }
        ));
    }

    #[test]
    fn an_unpinned_mode_toggle_pins_it() {
        let pin = pin_toggle(&row(Mode::Containers, PinButton::Pin));

        assert!(matches!(
            pin,
            ModePin::Unpinned {
                on_press: ModesIntent::Pin(Mode::Containers),
                ..
            }
        ));
    }

    #[test]
    fn with_every_slot_taken_the_toggle_says_why_it_is_off() {
        let pin = pin_toggle(&row(Mode::Containers, PinButton::PinDisabled));

        assert!(matches!(
            pin,
            ModePin::Disabled { hint }
                if hint == i18n::t(Key::ModesUnpinAnotherFirst)
        ));
    }

    #[test]
    fn terminal_has_no_pin_toggle() {
        let pin = pin_toggle(&row(Mode::Terminal, PinButton::Hidden));

        assert!(matches!(pin, ModePin::Hidden));
    }
}
