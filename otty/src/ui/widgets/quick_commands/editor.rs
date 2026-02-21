use iced::alignment;
use iced::widget::button::Status as ButtonStatus;
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Element, Length, Padding};

use crate::features::quick_commands::editor::{
    QuickCommandEditorEvent, QuickCommandEditorMode, QuickCommandEditorState,
};
use crate::features::quick_commands::model::QuickCommandType;
use crate::theme::{IcedColorPalette, ThemeProps};

const SECTION_SPACING: f32 = 16.0;
const FIELD_SPACING: f32 = 8.0;
const LABEL_SIZE: f32 = 13.0;
const LABEL_WIDTH: f32 = 160.0;
const INPUT_SIZE: f32 = 13.0;
const INPUT_PADDING_X: f32 = 8.0;
const INPUT_PADDING_Y: f32 = 6.0;
const BUTTON_HEIGHT: f32 = 28.0;
const BUTTON_PADDING_X: f32 = 12.0;
const HEADER_PADDING: f32 = 16.0;
const CONTENT_PADDING_RIGHT: f32 = 8.0;

/// Props for rendering a quick command editor tab.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Props<'a> {
    pub(crate) editor: &'a QuickCommandEditorState,
    pub(crate) theme: ThemeProps<'a>,
}

pub(crate) fn view<'a>(
    props: Props<'a>,
) -> Element<'a, QuickCommandEditorEvent> {
    let mut content = column![].spacing(SECTION_SPACING).width(Length::Fill);

    content = content.push(section_header("Quick command", props.theme));
    content = content.push(text_input_row(
        "Title",
        "codex: review",
        &props.editor.title,
        QuickCommandEditorEvent::UpdateTitle,
        props.theme,
    ));

    let command_type = props.editor.command_type();
    content = match props.editor.mode {
        QuickCommandEditorMode::Create { .. } => {
            content.push(command_type_selector(props.editor))
        },
        QuickCommandEditorMode::Edit { .. } => {
            let label = format!("Type: {command_type}");
            content.push(text(label).size(LABEL_SIZE))
        },
    };

    match command_type {
        QuickCommandType::Custom => {
            let Some(custom) = props.editor.custom() else {
                return container(text("Invalid custom editor state"))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            };
            content =
                content.push(section_header("Custom command", props.theme));
            content = content.push(text_input_row(
                "Program",
                "/usr/bin/bash",
                &custom.program,
                QuickCommandEditorEvent::UpdateProgram,
                props.theme,
            ));
            content = content.push(list_editor(
                "Arguments",
                "--flag",
                &custom.args,
                QuickCommandEditorEvent::AddArg,
                QuickCommandEditorEvent::RemoveArg,
                update_arg,
                props.theme,
            ));
            content = content.push(env_editor(
                &custom.env,
                "KEY",
                "value",
                props.theme,
            ));
            content = content.push(text_input_row(
                "Workdir (cwd)",
                "/path/to/project",
                &custom.working_directory,
                QuickCommandEditorEvent::UpdateWorkingDirectory,
                props.theme,
            ));
        },
        QuickCommandType::Ssh => {
            let Some(ssh) = props.editor.ssh() else {
                return container(text("Invalid SSH editor state"))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            };
            content =
                content.push(section_header("SSH connection", props.theme));
            content = content.push(text_input_row(
                "Host",
                "example.com",
                &ssh.host,
                QuickCommandEditorEvent::UpdateHost,
                props.theme,
            ));
            let port_row = text_input_row(
                "Port",
                "22",
                &ssh.port,
                QuickCommandEditorEvent::UpdatePort,
                props.theme,
            );
            content = content.push(port_row);
            content = content.push(text_input_row(
                "User",
                "ubuntu",
                &ssh.user,
                QuickCommandEditorEvent::UpdateUser,
                props.theme,
            ));
            content = content.push(text_input_row(
                "Identity file",
                "~/.ssh/id_ed25519",
                &ssh.identity_file,
                QuickCommandEditorEvent::UpdateIdentityFile,
                props.theme,
            ));
            content = content.push(list_editor(
                "Extra args",
                "-A",
                &ssh.extra_args,
                QuickCommandEditorEvent::AddExtraArg,
                QuickCommandEditorEvent::RemoveExtraArg,
                update_extra_arg,
                props.theme,
            ));
        },
    }

    if let Some(error) = &props.editor.error {
        let error_color = iced::Color::from_rgb(0.9, 0.4, 0.4);
        content = content.push(text(error).size(LABEL_SIZE).style(move |_| {
            iced::widget::text::Style {
                color: Some(error_color),
            }
        }));
    }

    let action_row = row![
        editor_button("Save", QuickCommandEditorEvent::Save, props.theme),
        editor_button("Cancel", QuickCommandEditorEvent::Cancel, props.theme)
    ]
    .spacing(8);

    content = content.push(action_row);

    let content = container(content).padding(Padding {
        top: 0.0,
        right: CONTENT_PADDING_RIGHT,
        bottom: 0.0,
        left: 0.0,
    });

    let scrollable = scrollable::Scrollable::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(6)
                .margin(0)
                .scroller_width(6),
        ));

    container(scrollable)
        .padding(HEADER_PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn section_header<'a>(
    label: &'a str,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickCommandEditorEvent> {
    let palette = theme.theme.iced_palette();
    container(text(label).size(LABEL_SIZE).style(move |_| {
        iced::widget::text::Style {
            color: Some(palette.bright_foreground),
        }
    }))
    .width(Length::Fill)
    .into()
}

fn text_input_row<'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: fn(String) -> QuickCommandEditorEvent,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickCommandEditorEvent> {
    let label = field_label(label);
    let input = text_input(placeholder, value)
        .on_input(on_input)
        .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
        .size(INPUT_SIZE)
        .style(text_input_style(theme))
        .width(Length::Fill);

    row![label, input]
        .spacing(FIELD_SPACING)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .into()
}

fn command_type_selector<'a>(
    editor: &'a QuickCommandEditorState,
) -> Element<'a, QuickCommandEditorEvent> {
    let options = [QuickCommandType::Custom, QuickCommandType::Ssh];
    let selector = pick_list(
        options,
        Some(editor.command_type()),
        QuickCommandEditorEvent::SelectCommandType,
    )
    .placeholder("Select type")
    .text_size(LABEL_SIZE)
    .width(Length::Fill);

    row![field_label("Type"), selector]
        .spacing(FIELD_SPACING)
        .align_y(alignment::Vertical::Center)
        .into()
}

fn list_editor<'a>(
    label: &'a str,
    placeholder: &'a str,
    values: &'a [String],
    on_add: QuickCommandEditorEvent,
    on_remove: fn(usize) -> QuickCommandEditorEvent,
    on_update: fn(usize, String) -> QuickCommandEditorEvent,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickCommandEditorEvent> {
    let mut column = column![text(label).size(LABEL_SIZE)]
        .spacing(FIELD_SPACING)
        .width(Length::Fill);

    for (index, value) in values.iter().enumerate() {
        let input = text_input(placeholder, value)
            .on_input(move |val| on_update(index, val))
            .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
            .size(INPUT_SIZE)
            .style(text_input_style(theme))
            .width(Length::Fill);

        let remove = editor_button("Remove", on_remove(index), theme);

        column = column.push(row![input, remove].spacing(FIELD_SPACING));
    }

    column.push(editor_button("Add", on_add, theme)).into()
}

fn env_editor<'a>(
    env: &'a [(String, String)],
    key_placeholder: &'a str,
    value_placeholder: &'a str,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickCommandEditorEvent> {
    let mut column = column![text("Environment").size(LABEL_SIZE)]
        .spacing(FIELD_SPACING)
        .width(Length::Fill);

    for (index, (key, value)) in env.iter().enumerate() {
        let key_input = text_input(key_placeholder, key)
            .on_input(move |val| QuickCommandEditorEvent::UpdateEnvKey {
                index,
                value: val,
            })
            .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
            .size(INPUT_SIZE)
            .style(text_input_style(theme))
            .width(Length::Fill);

        let value_input = text_input(value_placeholder, value)
            .on_input(move |val| QuickCommandEditorEvent::UpdateEnvValue {
                index,
                value: val,
            })
            .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
            .size(INPUT_SIZE)
            .style(text_input_style(theme))
            .width(Length::Fill);

        let remove = editor_button(
            "Remove",
            QuickCommandEditorEvent::RemoveEnv(index),
            theme,
        );

        column = column.push(row![key_input, value_input, remove].spacing(6));
    }

    column
        .push(editor_button(
            "Add env",
            QuickCommandEditorEvent::AddEnv,
            theme,
        ))
        .into()
}

fn update_arg(index: usize, value: String) -> QuickCommandEditorEvent {
    QuickCommandEditorEvent::UpdateArg { index, value }
}

fn update_extra_arg(index: usize, value: String) -> QuickCommandEditorEvent {
    QuickCommandEditorEvent::UpdateExtraArg { index, value }
}

fn field_label<'a>(label: &'a str) -> Element<'a, QuickCommandEditorEvent> {
    text(label)
        .size(LABEL_SIZE)
        .width(Length::Fixed(LABEL_WIDTH))
        .align_x(alignment::Horizontal::Left)
        .wrapping(Wrapping::None)
        .into()
}

fn text_input_style(
    theme: ThemeProps<'_>,
) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style + 'static {
    let palette = theme.theme.iced_palette().clone();
    move |base: &iced::Theme, status| {
        let mut style = iced::widget::text_input::default(base, status);
        style.selection = palette.blue;
        style
    }
}

fn editor_button<'a>(
    label: &'a str,
    on_press: QuickCommandEditorEvent,
    theme: ThemeProps<'a>,
) -> iced::widget::Button<'a, QuickCommandEditorEvent> {
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
        border: iced::Border {
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    }
}
