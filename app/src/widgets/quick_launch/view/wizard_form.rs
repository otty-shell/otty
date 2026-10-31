use std::fmt;

use iced::widget::{Space, container, row, text};
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
        .section(general_section(tab_id, editor, palette))
        .section(command_section);

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

    FormSection::new(i18n::t(Key::WizardHeader), palette)
        .field(title)
        .field(
            SettingRow::new(i18n::t(Key::FieldType), palette)
                .control(command_type),
        )
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
            .field(exit_behavior_row(tab_id, editor, palette))
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
            .field(exit_behavior_row(tab_id, editor, palette))
            .into(),
    )
}

fn exit_behavior_row<'a>(
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

    SettingRow::new(i18n::t(Key::FieldAfterExit), palette)
        .control(control)
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
