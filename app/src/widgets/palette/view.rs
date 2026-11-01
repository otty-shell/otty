use iced::widget::{Space, container, mouse_area, stack, svg};
use iced::{Element, Length, Padding, alignment, mouse};
use otty_ui_kit::components::command_palette::CommandPalette;

use super::event::PaletteIntent;
use super::model::PaletteViewModel;
use crate::i18n::{self, Key};
use crate::icons::SEARCH;
use crate::theme::ThemeProps;

/// Space between the top of the main area and the palette (`top: 14px`).
const PALETTE_TOP: f32 = 14.0;

/// Props for rendering the open command palette.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PaletteProps<'a> {
    pub(crate) vm: PaletteViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the palette centered under the title bar over a transparent
/// layer: a click on that layer closes the palette, a click on the palette
/// background gives the input its focus back.
pub(crate) fn view(props: PaletteProps<'_>) -> Element<'_, PaletteIntent> {
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

    // Transparent layer over the main area: a click on it closes the
    // palette, and the hidden widgets under it get no hover or clicks.
    let dismiss_layer =
        mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
            .on_press(PaletteIntent::Dismiss)
            .on_right_press(PaletteIntent::Dismiss)
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
    use otty_ui_kit::components::command_palette::input_id;
    use otty_ui_kit::components::target_dot::TargetKind;

    use super::*;
    use crate::task_outputs::collect_outputs;
    use crate::theme::ThemeManager;
    use crate::widgets::palette::PaletteEffect;
    use crate::widgets::palette::event::PaletteEvent;
    use crate::widgets::palette::reducer::{PaletteCtx, reduce};
    use crate::widgets::palette::state::PaletteState;
    use crate::widgets::palette::types::PaletteTarget;
    use crate::widgets::tabs::model::{
        SessionGroupViewModel, SessionViewModel, TabsViewModel,
    };

    fn ctx() -> PaletteCtx<'static> {
        let sessions = ["zsh", "htop", "htop logs"]
            .into_iter()
            .enumerate()
            .map(|(index, title)| SessionViewModel {
                tab_id: index as u64,
                title,
                state: None,
                is_active: index == 0,
            })
            .collect::<Vec<_>>();

        PaletteCtx {
            sessions: TabsViewModel {
                groups: vec![SessionGroupViewModel {
                    name: "fedora",
                    target: TargetKind::Local,
                    sessions,
                }],
                session_count: 3,
            },
            launches: Vec::new(),
            can_split_pane: true,
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
        let list = state.list().expect("palette should be open");
        let mut ui = simulator(view(PaletteProps {
            vm: PaletteViewModel { list },
            theme: ThemeProps::new(themes.current()),
        }));

        drive(&mut ui);

        ui.into_messages().collect()
    }

    /// Reduce every intent and return the effects they produced.
    fn apply(
        state: &mut PaletteState,
        intents: Vec<PaletteIntent>,
    ) -> Vec<PaletteEffect> {
        intents
            .into_iter()
            .flat_map(|intent| collect_outputs(reduce(state, intent, &ctx())))
            .filter_map(|event| match event {
                PaletteEvent::Effect(effect) => Some(effect),
                PaletteEvent::Intent(_) => None,
            })
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
            vec![PaletteEffect::Run(PaletteTarget::Session { tab_id: 2 })]
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

        assert_eq!(apply(&mut state, intents), vec![PaletteEffect::Dismissed]);
    }

    #[test]
    fn clicking_an_entry_runs_it() {
        let mut state = open_state();

        let intents = drive_ui(&state, |ui| {
            ui.click("htop logs").expect("entry should be visible");
        });

        assert_eq!(
            apply(&mut state, intents),
            vec![PaletteEffect::Run(PaletteTarget::Session { tab_id: 2 })]
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
