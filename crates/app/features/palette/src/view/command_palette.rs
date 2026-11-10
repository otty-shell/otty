use iced::widget::{Space, container, mouse_area, stack, svg};
use iced::{Element, Length, Padding, alignment, mouse};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons::SEARCH;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::command_palette::CommandPalette;

use crate::event::PaletteIntent;
use crate::model::PaletteViewModel;

/// Space between the top of the main area and the palette (`top: 14px`).
const PALETTE_TOP: f32 = 14.0;

/// Props for rendering the open command palette.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CommandPaletteProps<'a> {
    pub(crate) vm: PaletteViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the palette centered under the title bar over a transparent
/// layer: a click on that layer closes the palette, a click on the palette
/// background gives the input its focus back.
pub(crate) fn view(
    props: CommandPaletteProps<'_>,
) -> Element<'_, PaletteIntent> {
    let palette = props.theme.theme.ui();

    let command_palette = CommandPalette::new(
        props.vm.list,
        svg::Handle::from_memory(SEARCH),
        palette,
    )
    .placeholder(i18n::t(Key::PalettePlaceholder))
    .no_matches(i18n::t(Key::PaletteNoMatches))
    .hint("↑↓", i18n::t(Key::PaletteHintMove))
    .hint("Enter", i18n::t(Key::PaletteHintOpen))
    .on_action(PaletteIntent::Action);

    // A press the entries and the input leave alone lands here and must not
    // reach the dismiss layer under the palette.
    let command_palette = mouse_area(command_palette)
        .on_press(PaletteIntent::FocusInput)
        .interaction(mouse::Interaction::Idle);

    let positioned = container(command_palette)
        .padding(Padding {
            top: PALETTE_TOP,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Top);

    // Transparent layer over the main area: a press of any button on it
    // closes the palette, and the hidden widgets under it get no hover or
    // clicks.
    let dismiss_layer =
        mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(PaletteIntent::Dismiss)
            .on_right_press(PaletteIntent::Dismiss)
            .on_middle_press(PaletteIntent::Dismiss)
            .interaction(mouse::Interaction::Idle);

    stack![dismiss_layer, positioned]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::keyboard::key::Named;
    use iced_test::simulator;
    use otty_app_domain::commands::CommandCtx;
    use otty_app_testing::mouse::middle_click;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::command_palette::input_id;
    use otty_ui_kit::components::target_dot::TargetKind;

    use super::*;
    use crate::ctx::{PaletteCtx, PaletteSession};
    use crate::event::PaletteOutcome;
    use crate::reducer::reduce;
    use crate::state::PaletteState;

    fn ctx() -> PaletteCtx<'static> {
        let sessions = ["zsh", "htop", "htop logs"]
            .into_iter()
            .enumerate()
            .map(|(index, title)| PaletteSession {
                tab_id: index as u64,
                title,
                machine: "fedora",
                target: TargetKind::Local,
                is_active: index == 0,
            })
            .collect();

        PaletteCtx {
            sessions,
            launches: Vec::new(),
            pinned_modes: Vec::new(),
            commands: CommandCtx {
                is_terminal_mode: true,
                has_active_tab: true,
                has_focused_pane: true,
            },
        }
    }

    fn open_state() -> PaletteState {
        let mut state = PaletteState::default();
        let _ = reduce(&mut state, PaletteIntent::Open, &ctx());
        state
    }

    /// Render the open palette, run `drive` on it and return the intents
    /// it sent.
    fn drive_ui(
        state: &PaletteState,
        drive: impl FnOnce(&mut iced_test::Simulator<'_, PaletteIntent>),
    ) -> Vec<PaletteIntent> {
        let themes = ThemeManager::new();
        let vm = PaletteViewModel::from_state(state)
            .expect("palette should be open");
        let mut ui = simulator(view(CommandPaletteProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        drive(&mut ui);

        ui.into_messages().collect()
    }

    /// Reduce every intent and return the outcomes they reported.
    fn apply(
        state: &mut PaletteState,
        intents: Vec<PaletteIntent>,
    ) -> Vec<PaletteOutcome> {
        intents
            .into_iter()
            .flat_map(|intent| reduce(state, intent, &ctx()))
            .collect()
    }

    #[test]
    fn typing_then_arrow_then_enter_runs_the_chosen_session() {
        let mut state = open_state();

        let typed = drive_ui(&state, |ui| {
            ui.click(input_id()).expect("input should be visible");
            ui.typewrite("t");
        });
        assert!(apply(&mut state, typed).is_empty());

        let typed = drive_ui(&state, |ui| {
            ui.click(input_id()).expect("input should be visible");
            ui.typewrite("o");
        });
        assert!(apply(&mut state, typed).is_empty());
        assert_eq!(state.list().map(|list| list.query()), Some("to"));

        let moved = drive_ui(&state, |ui| {
            ui.click(input_id()).expect("input should be visible");
            ui.tap_key(Named::ArrowDown);
        });
        assert!(apply(&mut state, moved).is_empty());

        let entered = drive_ui(&state, |ui| {
            ui.click(input_id()).expect("input should be visible");
            ui.tap_key(Named::Enter);
        });

        assert_eq!(
            apply(&mut state, entered),
            vec![PaletteOutcome::SessionPicked { tab_id: 2 }]
        );
        assert!(!state.is_open());
    }

    #[test]
    fn esc_in_the_input_closes_the_palette() {
        let mut state = open_state();

        let intents = drive_ui(&state, |ui| {
            ui.click(input_id()).expect("input should be visible");
            ui.tap_key(Named::Escape);
        });

        assert_eq!(
            apply(&mut state, intents),
            vec![PaletteOutcome::KeyboardReleased]
        );
    }

    #[test]
    fn clicking_an_entry_runs_it() {
        let mut state = open_state();

        let intents = drive_ui(&state, |ui| {
            ui.click("htop logs").expect("entry should be visible");
        });

        assert_eq!(
            apply(&mut state, intents),
            vec![PaletteOutcome::SessionPicked { tab_id: 2 }]
        );
    }

    #[test]
    fn a_click_outside_the_palette_dismisses_it() {
        let state = open_state();

        let intents = drive_ui(&state, |ui| {
            ui.point_at(Point::new(20.0, 740.0));
            let _ = ui.simulate(iced_test::simulator::click());
        });

        assert!(matches!(intents.as_slice(), [PaletteIntent::Dismiss]));
    }

    /// A middle press away from the palette closes it, as a left press does.
    #[test]
    fn a_middle_press_outside_the_palette_dismisses_it() {
        let state = open_state();

        let intents = drive_ui(&state, |ui| {
            ui.point_at(Point::new(20.0, 740.0));
            let _ = ui.simulate(middle_click());
        });

        assert!(matches!(intents.as_slice(), [PaletteIntent::Dismiss]));
    }

    #[test]
    fn a_click_on_the_palette_background_focuses_the_input_again() {
        let state = open_state();

        // Inside the palette's top padding, left of the search icon.
        let intents = drive_ui(&state, |ui| {
            ui.point_at(Point::new(236.0, PALETTE_TOP + 2.0));
            let _ = ui.simulate(iced_test::simulator::click());
        });

        assert!(matches!(intents.as_slice(), [PaletteIntent::FocusInput]));
    }
}
