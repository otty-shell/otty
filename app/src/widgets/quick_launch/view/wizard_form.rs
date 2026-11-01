use std::fmt;

use iced::widget::{Space, column, container, row, text};
use iced::{Element, Length, Theme};
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
use super::super::state::WizardEditorState;
use super::super::types::QuickLaunchType;
use crate::domain::exit_behavior::ExitBehavior;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Width of the SSH port field: room for five digits. No size token.
const PORT_WIDTH: f32 = 96.0;

type WizardElement<'a> = Element<'a, QuickLaunchIntent, Theme, iced::Renderer>;

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
    pub(crate) editor: &'a WizardEditorState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch wizard form.
pub(crate) fn view(props: WizardFormProps<'_>) -> WizardElement<'_> {
    let tab_id = props.tab_id;
    let editor = props.editor;
    let palette = props.theme.theme.ui();

    let command_section = match editor.command_type() {
        QuickLaunchType::Custom => custom_section(tab_id, editor, palette),
        QuickLaunchType::Ssh => ssh_section(tab_id, editor, palette),
    };
    let Some(command_section) = command_section else {
        return container(text(i18n::t(Key::InvalidCustomEditorState)))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    };

    let mut page = FormPage::new(palette)
        .section(heading(editor, palette))
        .section(general_section(tab_id, editor, palette))
        .section(command_section)
        .section(exit_section(tab_id, editor, palette));

    if let Some(error) = unassigned_error(editor) {
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
    editor: &WizardEditorState,
    palette: &UiPalette,
) -> WizardElement<'a> {
    let title = if editor.is_create_mode() {
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
    editor: &'a WizardEditorState,
    palette: &UiPalette,
) -> WizardElement<'a> {
    let title =
        TextField::new(i18n::t(Key::FieldTitle), editor.title(), palette)
            .placeholder("codex: review")
            .on_input(move |value| QuickLaunchIntent::WizardUpdateTitle {
                tab_id,
                value,
            });

    let command_type: WizardElement<'a> = if editor.is_create_mode() {
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
            value: editor.command_type(),
            label: type_label(editor.command_type()),
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
        text(type_label(editor.command_type()))
            .size(typography::BODY.size())
            .font(fonts::UI)
            .color(palette.text_body())
            .into()
    };

    let title = match field_error(editor, Key::ErrTitleRequired) {
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
    editor: &'a WizardEditorState,
    palette: &UiPalette,
) -> Option<WizardElement<'a>> {
    let custom = editor.custom()?;

    let program =
        TextField::new(i18n::t(Key::FieldProgram), custom.program(), palette)
            .placeholder("/usr/bin/bash")
            .on_input(move |value| QuickLaunchIntent::WizardUpdateProgram {
                tab_id,
                value,
            });
    let program = match field_error(editor, Key::ErrProgramRequired) {
        Some(error) => program.error(error),
        None => program,
    };

    let args = TokenField::new(
        i18n::t(Key::FieldArguments),
        custom.args_list(),
        palette,
    )
    .placeholder("--flag")
    .optional(true)
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
    .optional(true)
    .add_label(i18n::t(Key::ButtonAddEnv))
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
    .optional(true)
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
    editor: &'a WizardEditorState,
    palette: &UiPalette,
) -> Option<WizardElement<'a>> {
    let ssh = editor.ssh()?;

    let host = TextField::new(i18n::t(Key::FieldHost), ssh.host(), palette)
        .placeholder("example.com")
        .on_input(move |value| QuickLaunchIntent::WizardUpdateHost {
            tab_id,
            value,
        });
    let host = match field_error(editor, Key::ErrHostRequired) {
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
    let port = match field_error(editor, Key::ErrInvalidPort) {
        Some(error) => port.error(error),
        None => port,
    };

    let host_and_port = row![host, port]
        .spacing(spacing::SPACE_4)
        .width(Length::Fill);

    let user = TextField::new(i18n::t(Key::FieldUser), ssh.user(), palette)
        .placeholder("ubuntu")
        .optional(true)
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
    .optional(true)
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
    .optional(true)
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
    editor: &WizardEditorState,
    palette: &UiPalette,
) -> WizardElement<'a> {
    let options = [
        exit_choice(ExitBehavior::Close),
        exit_choice(ExitBehavior::CloseOnSuccess),
        exit_choice(ExitBehavior::KeepOpen),
    ];
    let selected = exit_choice(editor.exit_behavior());

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

    row![Space::new().width(Length::Fill), cancel, save]
        .spacing(spacing::SPACE_3)
        .width(Length::Fill)
        .into()
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

/// Returns the editor error when it is the validation message `key`.
fn field_error(editor: &WizardEditorState, key: Key) -> Option<&str> {
    editor.error().filter(|error| *error == i18n::t(key))
}

/// Returns the editor error when no field shows it.
fn unassigned_error(editor: &WizardEditorState) -> Option<&str> {
    editor
        .error()
        .filter(|error| !FIELD_ERRORS.iter().any(|key| *error == i18n::t(*key)))
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::{WizardFormProps, view};
    use crate::i18n::{self, Key};
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::quick_launch::event::QuickLaunchIntent;
    use crate::widgets::quick_launch::state::WizardEditorState;
    use crate::widgets::quick_launch::types::QuickLaunchType;

    const TAB_ID: u64 = 7;

    fn messages_after_click(
        editor: &WizardEditorState,
        label: &str,
    ) -> Vec<QuickLaunchIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(WizardFormProps {
            tab_id: TAB_ID,
            editor,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(label).expect("label should be visible");

        ui.into_messages().collect()
    }

    #[test]
    fn given_form_when_save_pressed_then_save_intent_is_sent() {
        let editor = WizardEditorState::new(Vec::new());

        let messages = messages_after_click(&editor, i18n::t(Key::ButtonSave));

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::WizardSave { tab_id: TAB_ID }]
        ));
    }

    #[test]
    fn given_form_when_cancel_pressed_then_cancel_intent_is_sent() {
        let editor = WizardEditorState::new(Vec::new());

        let messages =
            messages_after_click(&editor, i18n::t(Key::ButtonCancel));

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::WizardCancel { tab_id: TAB_ID }]
        ));
    }

    #[test]
    fn given_new_launch_when_ssh_pressed_then_type_intent_is_sent() {
        let editor = WizardEditorState::new(Vec::new());

        let messages = messages_after_click(&editor, i18n::t(Key::TypeSsh));

        assert!(matches!(
            messages.as_slice(),
            [QuickLaunchIntent::WizardSelectCommandType {
                tab_id: TAB_ID,
                command_type: QuickLaunchType::Ssh,
            }]
        ));
    }

    #[test]
    fn given_new_launch_when_shown_then_title_and_subtitle_are_visible() {
        let editor = WizardEditorState::new(Vec::new());
        let themes = ThemeManager::new();
        let mut ui = simulator(view(WizardFormProps {
            tab_id: TAB_ID,
            editor: &editor,
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find(i18n::t(Key::WizardTitleNew)).is_ok());
        assert!(ui.find(i18n::t(Key::WizardSubtitle)).is_ok());
        assert!(ui.find(i18n::t(Key::WizardTitleEdit)).is_err());
    }
}
