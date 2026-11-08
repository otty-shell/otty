use std::fmt;

use iced::widget::{column, container, row, text};
use iced::{Element, Length};
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::form_page::FormPage;
use otty_ui_kit::components::form_section::FormSection;
use otty_ui_kit::components::key_value_field::KeyValueField;
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::components::setting_row::SettingRow;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::components::token_field::TokenField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::QuickLaunchIntent;
use super::super::model::WizardFormViewModel;
use super::super::types::QuickLaunchType;
use crate::domain::exit_behavior::ExitBehavior;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Width of the SSH port field: room for five digits. No size token.
const PORT_WIDTH: f32 = 96.0;

type WizardElement<'a> = Element<'a, QuickLaunchIntent>;

/// One option of a segmented control: the value and its translated label.
#[derive(Clone, PartialEq)]
struct Choice<T> {
    value: T,
    label: &'static str,
}

impl<T> fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

/// Props for the quick launch wizard form.
pub(crate) struct WizardFormProps<'a> {
    pub(crate) tab_id: u64,
    pub(crate) vm: WizardFormViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch wizard form.
pub(crate) fn view(props: WizardFormProps<'_>) -> WizardElement<'_> {
    let tab_id = props.tab_id;
    let vm = props.vm;
    let palette = props.theme.theme.ui();

    let command_section = match vm.command_type {
        QuickLaunchType::Custom => custom_section(tab_id, vm, palette),
        QuickLaunchType::Ssh => ssh_section(tab_id, vm, palette),
    };
    let Some(command_section) = command_section else {
        return container(text(i18n::t(Key::InvalidCustomEditorState)))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    };

    let mut page = FormPage::new(palette)
        .section(heading(vm, palette))
        .section(general_section(tab_id, vm, palette))
        .section(command_section)
        .section(exit_section(tab_id, vm, palette));

    if let Some(error) = unassigned_error(vm) {
        page = page.section(
            text(error)
                .size(typography::SMALL.size())
                .font(fonts::UI)
                .color(palette.status_error()),
        );
    }

    page.footer(footer(tab_id, palette)).into()
}

fn heading<'a>(
    vm: WizardFormViewModel<'_>,
    palette: &UiPalette,
) -> WizardElement<'a> {
    let title = if vm.is_create_mode {
        i18n::t(Key::WizardTitleNew)
    } else {
        i18n::t(Key::WizardTitleEdit)
    };

    column![
        text(title)
            .size(typography::TITLE.size())
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong()),
        text(i18n::t(Key::WizardSubtitle))
            .size(typography::SMALL.size())
            .font(fonts::UI)
            .color(palette.text_muted()),
    ]
    .spacing(spacing::SPACE_1)
    .into()
}

fn general_section<'a>(
    tab_id: u64,
    vm: WizardFormViewModel<'a>,
    palette: &UiPalette,
) -> WizardElement<'a> {
    let title = TextField::new(i18n::t(Key::FieldTitle), vm.title, palette)
        .placeholder("codex: review")
        .on_input(move |value| QuickLaunchIntent::WizardUpdateTitle {
            tab_id,
            value,
        });

    let command_type: WizardElement<'a> = if vm.is_create_mode {
        let options = [
            Choice {
                value: QuickLaunchType::Custom,
                label: i18n::t(Key::TypeCustom),
            },
            Choice {
                value: QuickLaunchType::Ssh,
                label: i18n::t(Key::TypeSsh),
            },
        ];
        let selected = Choice {
            value: vm.command_type,
            label: type_label(vm.command_type),
        };

        segmented(
            &options,
            &selected,
            move |choice| QuickLaunchIntent::WizardSelectCommandType {
                tab_id,
                command_type: choice.value,
            },
            palette,
        )
    } else {
        text(type_label(vm.command_type))
            .size(typography::BODY.size())
            .font(fonts::UI)
            .color(palette.text_body())
            .into()
    };

    let title = match field_error(vm, Key::ErrTitleRequired) {
        Some(error) => title.error(error),
        None => title,
    };

    column![
        title,
        SettingRow::new(i18n::t(Key::FieldType), palette).control(command_type),
    ]
    .spacing(spacing::SPACE_4)
    .into()
}

fn custom_section<'a>(
    tab_id: u64,
    vm: WizardFormViewModel<'a>,
    palette: &UiPalette,
) -> Option<WizardElement<'a>> {
    let custom = vm.custom?;

    let program =
        TextField::new(i18n::t(Key::FieldProgram), custom.program(), palette)
            .placeholder("/usr/bin/bash")
            .on_input(move |value| QuickLaunchIntent::WizardUpdateProgram {
                tab_id,
                value,
            });
    let program = match field_error(vm, Key::ErrProgramRequired) {
        Some(error) => program.error(error),
        None => program,
    };

    let args = TokenField::new(
        i18n::t(Key::FieldArguments),
        custom.args_list(),
        palette,
    )
    .placeholder("--flag")
    .optional(i18n::t(Key::FieldOptional))
    .remove_label(i18n::t(Key::ButtonRemove))
    .on_input(move |value| QuickLaunchIntent::WizardArgInput { tab_id, value })
    .on_submit(QuickLaunchIntent::WizardArgSubmit { tab_id })
    .on_backspace_empty(QuickLaunchIntent::WizardArgBackspace { tab_id })
    .on_remove(move |index| QuickLaunchIntent::WizardRemoveArg {
        tab_id,
        index,
    });

    let env = KeyValueField::new(
        i18n::t(Key::FieldEnvironment),
        custom.env(),
        palette,
    )
    .optional(i18n::t(Key::FieldOptional))
    .placeholders(
        i18n::t(Key::EnvKeyPlaceholder),
        i18n::t(Key::EnvValuePlaceholder),
    )
    .add_label(i18n::t(Key::ButtonAddEnv))
    .remove_label(i18n::t(Key::ButtonRemoveEnv))
    .on_name(move |index, value| QuickLaunchIntent::WizardUpdateEnvKey {
        tab_id,
        index,
        value,
    })
    .on_value(
        move |index, value| QuickLaunchIntent::WizardUpdateEnvValue {
            tab_id,
            index,
            value,
        },
    )
    .on_remove(move |index| QuickLaunchIntent::WizardRemoveEnv {
        tab_id,
        index,
    })
    .on_add(QuickLaunchIntent::WizardAddEnv { tab_id });

    let workdir = TextField::new(
        i18n::t(Key::FieldWorkdir),
        custom.working_directory(),
        palette,
    )
    .placeholder("/path/to/project")
    .optional(i18n::t(Key::FieldOptional))
    .on_input(move |value| {
        QuickLaunchIntent::WizardUpdateWorkingDirectory { tab_id, value }
    });

    Some(
        FormSection::new(i18n::t(Key::SectionCustomCommand), palette)
            .field(program)
            .field(args)
            .field(env)
            .field(workdir)
            .into(),
    )
}

fn ssh_section<'a>(
    tab_id: u64,
    vm: WizardFormViewModel<'a>,
    palette: &UiPalette,
) -> Option<WizardElement<'a>> {
    let ssh = vm.ssh?;

    let host = TextField::new(i18n::t(Key::FieldHost), ssh.host(), palette)
        .placeholder("example.com")
        .on_input(move |value| QuickLaunchIntent::WizardUpdateHost {
            tab_id,
            value,
        });
    let host = match field_error(vm, Key::ErrHostRequired) {
        Some(error) => host.error(error),
        None => host,
    };

    let port = TextField::new(i18n::t(Key::FieldPort), ssh.port(), palette)
        .placeholder("22")
        .width(Length::Fixed(PORT_WIDTH))
        .on_input(move |value| QuickLaunchIntent::WizardUpdatePort {
            tab_id,
            value,
        });
    let port = match field_error(vm, Key::ErrInvalidPort) {
        Some(error) => port.error(error),
        None => port,
    };

    let host_and_port = row![host, port]
        .spacing(spacing::SPACE_4)
        .width(Length::Fill);

    let user = TextField::new(i18n::t(Key::FieldUser), ssh.user(), palette)
        .placeholder("ubuntu")
        .optional(i18n::t(Key::FieldOptional))
        .on_input(move |value| QuickLaunchIntent::WizardUpdateUser {
            tab_id,
            value,
        });

    // No file picker exists in the app yet, so there is no Browse action.
    let identity_file = TextField::new(
        i18n::t(Key::FieldIdentityFile),
        ssh.identity_file(),
        palette,
    )
    .placeholder("~/.ssh/id_ed25519")
    .optional(i18n::t(Key::FieldOptional))
    .on_input(move |value| QuickLaunchIntent::WizardUpdateIdentityFile {
        tab_id,
        value,
    });

    let extra_args = TokenField::new(
        i18n::t(Key::FieldExtraArgs),
        ssh.extra_args_list(),
        palette,
    )
    .placeholder("-A")
    .optional(i18n::t(Key::FieldOptional))
    .remove_label(i18n::t(Key::ButtonRemove))
    .on_input(move |value| QuickLaunchIntent::WizardExtraArgInput {
        tab_id,
        value,
    })
    .on_submit(QuickLaunchIntent::WizardExtraArgSubmit { tab_id })
    .on_backspace_empty(QuickLaunchIntent::WizardExtraArgBackspace { tab_id })
    .on_remove(move |index| QuickLaunchIntent::WizardRemoveExtraArg {
        tab_id,
        index,
    });

    Some(
        FormSection::new(i18n::t(Key::SectionSshConnection), palette)
            .field(host_and_port)
            .field(user)
            .field(identity_file)
            .field(extra_args)
            .into(),
    )
}

fn exit_section<'a>(
    tab_id: u64,
    vm: WizardFormViewModel<'_>,
    palette: &UiPalette,
) -> WizardElement<'a> {
    let options = [
        exit_choice(ExitBehavior::Close),
        exit_choice(ExitBehavior::CloseOnSuccess),
        exit_choice(ExitBehavior::KeepOpen),
    ];
    let selected = exit_choice(vm.exit_behavior);

    let control = segmented(
        &options,
        &selected,
        move |choice| QuickLaunchIntent::WizardSelectExitBehavior {
            tab_id,
            exit_behavior: choice.value,
        },
        palette,
    );
    let hint = text(i18n::t(Key::ExitBehaviorHint))
        .size(typography::SMALL.size())
        .font(fonts::UI)
        .color(palette.text_muted());

    FormSection::new(i18n::t(Key::FieldAfterExit), palette)
        .field(control)
        .field(hint)
        .into()
}

fn footer<'a>(tab_id: u64, palette: &UiPalette) -> WizardElement<'a> {
    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(QuickLaunchIntent::WizardCancel { tab_id });
    let save = Button::new(i18n::t(Key::ButtonSave), palette)
        .kind(ButtonKind::Primary)
        .on_press(QuickLaunchIntent::WizardSave { tab_id });

    row![cancel, save].spacing(spacing::SPACE_3).into()
}

fn exit_choice(value: ExitBehavior) -> Choice<ExitBehavior> {
    let label = match value {
        ExitBehavior::Close => i18n::t(Key::ExitBehaviorClose),
        ExitBehavior::CloseOnSuccess => {
            i18n::t(Key::ExitBehaviorCloseOnSuccess)
        },
        ExitBehavior::KeepOpen => i18n::t(Key::ExitBehaviorKeepOpen),
    };

    Choice { value, label }
}

fn type_label(command_type: QuickLaunchType) -> &'static str {
    match command_type {
        QuickLaunchType::Ssh => i18n::t(Key::TypeSsh),
        _ => i18n::t(Key::TypeCustom),
    }
}

/// Validation errors that belong to a field and are drawn under it.
const FIELD_ERRORS: [Key; 4] = [
    Key::ErrTitleRequired,
    Key::ErrProgramRequired,
    Key::ErrHostRequired,
    Key::ErrInvalidPort,
];

/// Returns the form error when it is the validation message `key`.
fn field_error(vm: WizardFormViewModel<'_>, key: Key) -> Option<&str> {
    vm.error.filter(|error| *error == i18n::t(key))
}

/// Returns the form error when no field shows it.
fn unassigned_error(vm: WizardFormViewModel<'_>) -> Option<&str> {
    vm.error
        .filter(|error| !FIELD_ERRORS.iter().any(|key| *error == i18n::t(*key)))
}

#[cfg(test)]
mod tests {
    use iced::{Event, Point, mouse};
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};
    use otty_ui_kit::components::screen_area::ScreenArea;
    use otty_ui_kit::{size, spacing};

    use super::{WizardElement, WizardFormProps, view};
    use crate::i18n::{self, Key};
    use crate::testing::alone;
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::quick_launch::event::QuickLaunchIntent;
    use crate::widgets::quick_launch::model::WizardFormViewModel;
    use crate::widgets::quick_launch::state::QuickLaunchState;
    use crate::widgets::quick_launch::types::QuickLaunchType;

    const TAB_ID: u64 = 7;
    /// Height of the kit's footer bar (`.form-foot`).
    const BAR_HEIGHT: f32 = 60.0;

    /// The form in the area it has in the app.
    fn page(props: WizardFormProps<'_>) -> WizardElement<'_> {
        let palette = props.theme.theme.ui();

        ScreenArea::new(0, view(props), palette).into()
    }

    fn messages_after_click(
        state: &QuickLaunchState,
        label: &str,
    ) -> Vec<QuickLaunchIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(page(WizardFormProps {
            tab_id: TAB_ID,
            vm: vm(state),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(label).expect("label should be visible");

        ui.into_messages().collect()
    }

    /// Draws the editor of `state` in a window of the given size.
    fn in_window<'a>(
        state: &'a QuickLaunchState,
        themes: &'a ThemeManager,
        width: f32,
        height: f32,
    ) -> Simulator<'a, QuickLaunchIntent> {
        Simulator::with_size(
            iced::Settings::default(),
            (width, height),
            page(WizardFormProps {
                tab_id: TAB_ID,
                vm: vm(state),
                theme: ThemeProps::new(themes.current()),
            }),
        )
    }

    /// Scrolls the form as far as `x` and `y` go.
    fn wheel(ui: &mut Simulator<'_, QuickLaunchIntent>, x: f32, y: f32) {
        ui.point_at(Point::new(100.0, 100.0));
        let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x, y },
        })]);
    }

    /// How many scrollers the page has.
    fn scroller_count(ui: &mut Simulator<'_, QuickLaunchIntent>) -> usize {
        let mut count = 0;

        let _ = ui.find(|candidate: Candidate<'_>| {
            if matches!(candidate, Candidate::Scrollable { .. }) {
                count += 1;
            }
            None::<()>
        });

        count
    }

    /// The state with the editor of a new launch of the given type in tab
    /// `TAB_ID`.
    fn state_of(command_type: QuickLaunchType) -> QuickLaunchState {
        let mut state = QuickLaunchState::default();
        state.wizard_mut().initialize_create(TAB_ID, Vec::new());
        state
            .wizard_mut()
            .editor_mut(TAB_ID)
            .expect("the editor was just created")
            .set_command_type(command_type);

        state
    }

    /// The form of the editor in tab `TAB_ID` of `state`.
    fn vm(state: &QuickLaunchState) -> WizardFormViewModel<'_> {
        WizardFormViewModel::from_state(state, TAB_ID)
            .expect("the tab should have an editor")
    }

    #[test]
    fn given_either_launch_type_in_a_small_window_then_the_page_has_one_scroller()
     {
        for command_type in [QuickLaunchType::Custom, QuickLaunchType::Ssh] {
            let state = state_of(command_type);
            let themes = ThemeManager::new();
            let mut ui = in_window(&state, &themes, 300.0, 300.0);

            assert_eq!(scroller_count(&mut ui), 1);
        }
    }

    #[test]
    fn given_either_launch_type_in_a_tall_window_then_cancel_and_save_sit_at_the_bottom_right()
     {
        let (width, height) = (1024.0, 1400.0);
        for command_type in [QuickLaunchType::Custom, QuickLaunchType::Ssh] {
            let state = state_of(command_type);
            let themes = ThemeManager::new();
            let mut ui = in_window(&state, &themes, width, height);

            let save =
                ui.find(i18n::t(Key::ButtonSave)).expect("Save").bounds();
            let cancel = ui
                .find(i18n::t(Key::ButtonCancel))
                .expect("Cancel")
                .bounds();

            let form_right = (width + size::FORM) / 2.0;
            assert!(save.y > height - BAR_HEIGHT && save.y < height);
            assert_eq!(cancel.y, save.y);
            assert!(cancel.x + cancel.width < save.x, "Cancel, then Save");
            assert!(save.x + save.width <= form_right);
            assert!(save.x + save.width > form_right - 20.0);
        }
    }

    #[test]
    fn given_a_low_window_then_save_comes_after_the_fields_and_is_reached_by_scrolling()
     {
        let state = state_of(QuickLaunchType::Custom);
        let themes = ThemeManager::new();
        let mut ui = in_window(&state, &themes, 1024.0, 300.0);
        let save = i18n::t(Key::ButtonSave);

        let hidden = ui.find(save).expect("Save should exist");
        wheel(&mut ui, 0.0, -10_000.0);
        ui.click(save).expect("Save should be in reach");
        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();

        assert_eq!(hidden.visible_bounds(), None);
        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::WizardSave { tab_id: TAB_ID }]
        ));
    }

    #[test]
    fn given_a_small_window_then_the_form_keeps_its_width_and_save_is_in_reach()
    {
        let state = state_of(QuickLaunchType::Custom);
        let themes = ThemeManager::new();
        let mut ui = in_window(&state, &themes, 300.0, 300.0);
        let save = i18n::t(Key::ButtonSave);

        let found = ui.find(save).expect("Save should exist");
        wheel(&mut ui, -10_000.0, -10_000.0);
        ui.click(save).expect("Save should be in reach");
        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();

        let edge = found.bounds().x + found.bounds().width;
        let form_right = size::SCREEN_MIN - spacing::SPACE_7;
        assert!(
            edge <= form_right && edge > form_right - 20.0,
            "Save ends at {edge}"
        );
        assert_eq!(found.visible_bounds(), None);
        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::WizardSave { tab_id: TAB_ID }]
        ));
    }

    #[test]
    fn given_a_normal_window_then_the_form_is_as_wide_as_before() {
        let state = state_of(QuickLaunchType::Custom);
        let themes = ThemeManager::new();
        let mut ui = in_window(&state, &themes, 1024.0, 1400.0);

        let title = ui.find(i18n::t(Key::WizardTitleNew)).expect("title");

        assert_eq!(title.bounds().x, (1024.0 - size::FORM) / 2.0);
    }

    #[test]
    fn given_a_small_window_when_a_field_is_scrolled_to_then_typing_reaches_it()
    {
        let state = state_of(QuickLaunchType::Custom);
        let themes = ThemeManager::new();
        let mut ui = in_window(&state, &themes, 300.0, 300.0);

        wheel(&mut ui, -20.0, -20.0);
        let label = ui
            .find(i18n::t(Key::FieldTitle))
            .expect("the title label should exist")
            .visible_bounds()
            .expect("the title label should be in view");
        ui.point_at(Point::new(label.x + 40.0, label.y + label.height + 16.0));
        let _ = ui.simulate(iced_test::simulator::click());
        let _ = ui.typewrite("c");

        let messages: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(
            matches!(
                messages.as_slice(),
                [QuickLaunchIntent::WizardUpdateTitle { tab_id: TAB_ID, value }]
                    if value == "c"
            ),
            "{messages:?}"
        );
    }

    #[test]
    fn given_a_small_window_then_a_field_error_pushes_the_next_field_down() {
        let mut state = state_of(QuickLaunchType::Custom);
        state
            .wizard_mut()
            .editor_mut(TAB_ID)
            .expect("the tab should have an editor")
            .set_error(i18n::t(Key::ErrTitleRequired).to_string());
        let themes = ThemeManager::new();
        let mut ui = in_window(&state, &themes, 300.0, 1400.0);

        let error = ui
            .find(i18n::t(Key::ErrTitleRequired))
            .expect("the error should show")
            .bounds();
        let next = ui
            .find(i18n::t(Key::FieldType))
            .expect("the next field should show")
            .bounds();

        assert!(error.y + error.height <= next.y, "{error:?} over {next:?}");
    }

    #[test]
    fn given_form_when_save_pressed_then_save_intent_is_sent() {
        let state = state_of(QuickLaunchType::Custom);

        let messages = messages_after_click(&state, i18n::t(Key::ButtonSave));

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::WizardSave { tab_id: TAB_ID }]
        ));
    }

    #[test]
    fn given_form_when_cancel_pressed_then_cancel_intent_is_sent() {
        let state = state_of(QuickLaunchType::Custom);

        let messages = messages_after_click(&state, i18n::t(Key::ButtonCancel));

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::WizardCancel { tab_id: TAB_ID }]
        ));
    }

    #[test]
    fn given_new_launch_when_ssh_pressed_then_type_intent_is_sent() {
        let state = state_of(QuickLaunchType::Custom);

        let messages = messages_after_click(&state, i18n::t(Key::TypeSsh));

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::WizardSelectCommandType {
                tab_id: TAB_ID,
                command_type: QuickLaunchType::Ssh,
            }]
        ));
    }

    #[test]
    fn given_any_language_when_drawn_then_the_optional_marker_is_in_that_language()
     {
        if !alone::is_alone(
            "given_any_language_when_drawn_then_the_optional_marker",
        ) {
            return;
        }

        let themes = ThemeManager::new();
        for locale in i18n::Locale::ALL {
            i18n::set_locale(locale);
            let marker = i18n::t(Key::FieldOptional);
            for command_type in [QuickLaunchType::Custom, QuickLaunchType::Ssh]
            {
                let state = state_of(command_type);
                let mut ui = in_window(&state, &themes, 1024.0, 1400.0);

                assert!(ui.find(marker).is_ok(), "{locale:?}: {marker}");
                assert_eq!(
                    ui.find("optional").is_ok(),
                    marker == "optional",
                    "{locale:?}"
                );
            }
        }
    }

    #[test]
    fn given_new_launch_when_shown_then_title_and_subtitle_are_visible() {
        let state = state_of(QuickLaunchType::Custom);
        let themes = ThemeManager::new();
        let mut ui = simulator(page(WizardFormProps {
            tab_id: TAB_ID,
            vm: vm(&state),
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find(i18n::t(Key::WizardTitleNew)).is_ok());
        assert!(ui.find(i18n::t(Key::WizardSubtitle)).is_ok());
        assert!(ui.find(i18n::t(Key::WizardTitleEdit)).is_err());
    }
}
