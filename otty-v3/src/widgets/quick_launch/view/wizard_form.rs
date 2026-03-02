use iced::widget::button::Status as ButtonStatus;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length, Theme, alignment};

use crate::theme::{IcedColorPalette, ThemeProps};
use crate::widgets::quick_launch::event::QuickLaunchIntent;
use crate::widgets::quick_launch::model::QuickLaunchType;
use crate::widgets::quick_launch::state::WizardEditorState;

const SECTION_SPACING: f32 = 16.0;
const FIELD_SPACING: f32 = 8.0;
const LABEL_SIZE: f32 = 13.0;
const LABEL_WIDTH: f32 = 160.0;
const INPUT_SIZE: f32 = 13.0;
const INPUT_PADDING_X: f32 = 8.0;
const INPUT_PADDING_Y: f32 = 6.0;
const BUTTON_HEIGHT: f32 = 28.0;
const BUTTON_PADDING_X: f32 = 12.0;
const FORM_PADDING: f32 = 16.0;
const FORM_RIGHT_PADDING: f32 = 8.0;

/// Props for the quick launch wizard form.
pub(crate) struct WizardFormProps<'a> {
    pub(crate) tab_id: u64,
    pub(crate) editor: &'a WizardEditorState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch wizard form.
pub(crate) fn view(
    props: WizardFormProps<'_>,
) -> Element<'_, QuickLaunchIntent, Theme, iced::Renderer> {
    let tab_id = props.tab_id;
    let editor = props.editor;
    let theme = props.theme;

    let mut content = column![].spacing(SECTION_SPACING).width(Length::Fill);

    content = content.push(section_header("Quick launch", theme));
    content = content.push(text_input_row(
        "Title",
        "codex: review",
        editor.title(),
        move |value| QuickLaunchIntent::WizardUpdateTitle { tab_id, value },
        theme,
    ));

    if editor.is_create_mode() {
        content = content.push(command_type_selector(
            tab_id,
            editor.command_type(),
            theme,
        ));
    } else {
        content = content.push(row![
            field_label("Type"),
            text(quick_launch_type_label(editor.command_type()))
                .size(LABEL_SIZE),
        ]);
    }

    match editor.command_type() {
        QuickLaunchType::Custom => {
            let Some(custom) = editor.custom() else {
                return container(text("Invalid custom editor state"))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            };

            content = content.push(section_header("Custom command", theme));
            content = content.push(text_input_row(
                "Program",
                "/usr/bin/bash",
                custom.program(),
                move |value| QuickLaunchIntent::WizardUpdateProgram {
                    tab_id,
                    value,
                },
                theme,
            ));
            content = content.push(list_editor(
                "Arguments",
                "--flag",
                custom.args(),
                tab_id,
                wizard_add_arg,
                wizard_remove_arg,
                wizard_update_arg,
                theme,
            ));
            content = content.push(env_editor(
                custom.env(),
                "KEY",
                "value",
                tab_id,
                theme,
            ));
            content = content.push(text_input_row(
                "Workdir (cwd)",
                "/path/to/project",
                custom.working_directory(),
                move |value| QuickLaunchIntent::WizardUpdateWorkingDirectory {
                    tab_id,
                    value,
                },
                theme,
            ));
        },
        QuickLaunchType::Ssh => {
            let Some(ssh) = editor.ssh() else {
                return container(text("Invalid SSH editor state"))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            };

            content = content.push(section_header("SSH connection", theme));
            content = content.push(text_input_row(
                "Host",
                "example.com",
                ssh.host(),
                move |value| QuickLaunchIntent::WizardUpdateHost {
                    tab_id,
                    value,
                },
                theme,
            ));
            content = content.push(text_input_row(
                "Port",
                "22",
                ssh.port(),
                move |value| QuickLaunchIntent::WizardUpdatePort {
                    tab_id,
                    value,
                },
                theme,
            ));
            content = content.push(text_input_row(
                "User",
                "ubuntu",
                ssh.user(),
                move |value| QuickLaunchIntent::WizardUpdateUser {
                    tab_id,
                    value,
                },
                theme,
            ));
            content = content.push(text_input_row(
                "Identity file",
                "~/.ssh/id_ed25519",
                ssh.identity_file(),
                move |value| QuickLaunchIntent::WizardUpdateIdentityFile {
                    tab_id,
                    value,
                },
                theme,
            ));
            content = content.push(list_editor(
                "Extra args",
                "-A",
                ssh.extra_args(),
                tab_id,
                wizard_add_extra_arg,
                wizard_remove_extra_arg,
                wizard_update_extra_arg,
                theme,
            ));
        },
    }

    if let Some(error) = editor.error() {
        let error_color = theme.theme.iced_palette().red;
        content = content.push(text(error).size(LABEL_SIZE).style(move |_| {
            iced::widget::text::Style {
                color: Some(error_color),
            }
        }));
    }

    let action_row = row![
        editor_button("Save", QuickLaunchIntent::WizardSave { tab_id }, theme,),
        editor_button(
            "Cancel",
            QuickLaunchIntent::WizardCancel { tab_id },
            theme,
        ),
    ]
    .spacing(FIELD_SPACING);
    content = content.push(action_row);

    let content = container(content).padding(iced::Padding {
        top: 0.0,
        right: FORM_RIGHT_PADDING,
        bottom: 0.0,
        left: 0.0,
    });

    let scrollable = iced::widget::scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill);

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(FORM_PADDING)
        .into()
}

fn quick_launch_type_label(command_type: QuickLaunchType) -> &'static str {
    match command_type {
        QuickLaunchType::Custom => "Custom",
        QuickLaunchType::Ssh => "SSH",
    }
}

fn section_header<'a>(
    label: &'a str,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let palette = theme.theme.iced_palette();
    container(text(label).size(LABEL_SIZE).style(move |_| {
        iced::widget::text::Style {
            color: Some(palette.bright_foreground),
        }
    }))
    .width(Length::Fill)
    .into()
}

fn text_input_row<'a, F>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: F,
    _theme: ThemeProps<'a>,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer>
where
    F: 'a + Fn(String) -> QuickLaunchIntent,
{
    let input = text_input(placeholder, value)
        .on_input(on_input)
        .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
        .size(INPUT_SIZE)
        .width(Length::Fill);

    row![field_label(label), input]
        .spacing(FIELD_SPACING)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .into()
}

fn command_type_selector<'a>(
    tab_id: u64,
    selected: QuickLaunchType,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let custom = toggle_button(
        "Custom",
        selected == QuickLaunchType::Custom,
        QuickLaunchIntent::WizardSelectCommandType {
            tab_id,
            command_type: QuickLaunchType::Custom,
        },
        theme,
    );
    let ssh = toggle_button(
        "SSH",
        selected == QuickLaunchType::Ssh,
        QuickLaunchIntent::WizardSelectCommandType {
            tab_id,
            command_type: QuickLaunchType::Ssh,
        },
        theme,
    );

    row![
        field_label("Type"),
        row![custom, ssh]
            .spacing(FIELD_SPACING)
            .align_y(alignment::Vertical::Center),
    ]
    .spacing(FIELD_SPACING)
    .align_y(alignment::Vertical::Center)
    .width(Length::Fill)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn list_editor<'a>(
    label: &'a str,
    placeholder: &'a str,
    values: &'a [String],
    tab_id: u64,
    on_add: fn(u64) -> QuickLaunchIntent,
    on_remove: fn(u64, usize) -> QuickLaunchIntent,
    on_update: fn(u64, usize, String) -> QuickLaunchIntent,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let mut col = column![text(label).size(LABEL_SIZE)]
        .spacing(FIELD_SPACING)
        .width(Length::Fill);

    for (index, value) in values.iter().enumerate() {
        let input = text_input(placeholder, value)
            .on_input(move |next| on_update(tab_id, index, next))
            .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
            .size(INPUT_SIZE)
            .width(Length::Fill);
        let remove = editor_button("Remove", on_remove(tab_id, index), theme);

        col = col.push(row![input, remove].spacing(FIELD_SPACING));
    }

    col.push(editor_button("Add", on_add(tab_id), theme)).into()
}

fn env_editor<'a>(
    env: &'a [(String, String)],
    key_placeholder: &'a str,
    value_placeholder: &'a str,
    tab_id: u64,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let mut col = column![text("Environment").size(LABEL_SIZE)]
        .spacing(FIELD_SPACING)
        .width(Length::Fill);

    for (index, (key, value)) in env.iter().enumerate() {
        let key_input = text_input(key_placeholder, key)
            .on_input(move |next| QuickLaunchIntent::WizardUpdateEnvKey {
                tab_id,
                index,
                value: next,
            })
            .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
            .size(INPUT_SIZE)
            .width(Length::Fill);
        let value_input = text_input(value_placeholder, value)
            .on_input(move |next| QuickLaunchIntent::WizardUpdateEnvValue {
                tab_id,
                index,
                value: next,
            })
            .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
            .size(INPUT_SIZE)
            .width(Length::Fill);
        let remove = editor_button(
            "Remove",
            QuickLaunchIntent::WizardRemoveEnv { tab_id, index },
            theme,
        );

        col = col
            .push(row![key_input, value_input, remove].spacing(FIELD_SPACING));
    }

    col.push(editor_button(
        "Add env",
        QuickLaunchIntent::WizardAddEnv { tab_id },
        theme,
    ))
    .into()
}

fn field_label<'a>(
    label: &'a str,
) -> Element<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    text(label)
        .size(LABEL_SIZE)
        .width(Length::Fixed(LABEL_WIDTH))
        .align_x(alignment::Horizontal::Left)
        .into()
}

fn editor_button<'a>(
    label: &'a str,
    on_press: QuickLaunchIntent,
    theme: ThemeProps<'a>,
) -> iced::widget::Button<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let palette = theme.theme.iced_palette().clone();
    let content = container(
        text(label)
            .size(LABEL_SIZE)
            .align_x(alignment::Horizontal::Center),
    )
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center);

    button(content)
        .on_press(on_press)
        .padding([0.0, BUTTON_PADDING_X])
        .height(Length::Fixed(BUTTON_HEIGHT))
        .style(move |_, status| button_style(&palette, status))
}

fn toggle_button<'a>(
    label: &'a str,
    selected: bool,
    on_press: QuickLaunchIntent,
    theme: ThemeProps<'a>,
) -> iced::widget::Button<'a, QuickLaunchIntent, Theme, iced::Renderer> {
    let palette = theme.theme.iced_palette().clone();
    let content = container(text(label).size(LABEL_SIZE))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center);

    button(content)
        .on_press(on_press)
        .padding([0.0, BUTTON_PADDING_X])
        .height(Length::Fixed(BUTTON_HEIGHT))
        .style(move |_, status| {
            let mut style = button_style(&palette, status);
            if selected {
                style.background = Some(palette.dim_blue.into());
                style.text_color = palette.dim_black;
            }
            style
        })
}

fn button_style(
    palette: &IcedColorPalette,
    status: ButtonStatus,
) -> button::Style {
    let background = match status {
        ButtonStatus::Hovered | ButtonStatus::Pressed => {
            Some(palette.dim_blue.into())
        },
        _ => Some(palette.overlay.into()),
    };

    let text_color = match status {
        ButtonStatus::Hovered | ButtonStatus::Pressed => palette.dim_black,
        _ => palette.foreground,
    };

    button::Style {
        background,
        text_color,
        border: iced::Border::default(),
        ..Default::default()
    }
}

fn wizard_add_arg(tab_id: u64) -> QuickLaunchIntent {
    QuickLaunchIntent::WizardAddArg { tab_id }
}

fn wizard_remove_arg(tab_id: u64, index: usize) -> QuickLaunchIntent {
    QuickLaunchIntent::WizardRemoveArg { tab_id, index }
}

fn wizard_update_arg(
    tab_id: u64,
    index: usize,
    value: String,
) -> QuickLaunchIntent {
    QuickLaunchIntent::WizardUpdateArg {
        tab_id,
        index,
        value,
    }
}

fn wizard_add_extra_arg(tab_id: u64) -> QuickLaunchIntent {
    QuickLaunchIntent::WizardAddExtraArg { tab_id }
}

fn wizard_remove_extra_arg(tab_id: u64, index: usize) -> QuickLaunchIntent {
    QuickLaunchIntent::WizardRemoveExtraArg { tab_id, index }
}

fn wizard_update_extra_arg(
    tab_id: u64,
    index: usize,
    value: String,
) -> QuickLaunchIntent {
    QuickLaunchIntent::WizardUpdateExtraArg {
        tab_id,
        index,
        value,
    }
}
