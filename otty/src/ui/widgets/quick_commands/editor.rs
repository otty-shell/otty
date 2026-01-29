use iced::alignment;
use iced::widget::button::Status as ButtonStatus;
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, row, scrollable, text, text_input,
};
use iced::{Element, Length};

use crate::features::quick_commands::editor::{
    QuickCommandEditorEvent, QuickCommandEditorMode, QuickCommandEditorState,
    QuickCommandType,
};
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
        &props.editor.title,
        QuickCommandEditorEvent::UpdateTitle,
    ));

    content = match props.editor.mode {
        QuickCommandEditorMode::Create { .. } => {
            content.push(command_type_selector(props.editor, props.theme))
        },
        QuickCommandEditorMode::Edit { .. } => {
            let command_type = props.editor.command_type.label();
            let label = format!("Type: {command_type}");
            content.push(text(label).size(LABEL_SIZE))
        },
    };

    match props.editor.command_type {
        QuickCommandType::Custom => {
            content =
                content.push(section_header("Custom command", props.theme));
            content = content.push(text_input_row(
                "Program",
                &props.editor.program,
                QuickCommandEditorEvent::UpdateProgram,
            ));
            content = content.push(list_editor(
                "Arguments",
                &props.editor.args,
                QuickCommandEditorEvent::AddArg,
                QuickCommandEditorEvent::RemoveArg,
                update_arg,
                props.theme,
            ));
            content = content.push(env_editor(&props.editor.env, props.theme));
            content = content.push(text_input_row(
                "Working directory",
                &props.editor.working_directory,
                QuickCommandEditorEvent::UpdateWorkingDirectory,
            ));
        },
        QuickCommandType::Ssh => {
            content =
                content.push(section_header("SSH connection", props.theme));
            content = content.push(text_input_row(
                "Host",
                &props.editor.host,
                QuickCommandEditorEvent::UpdateHost,
            ));
            let port_row = text_input_row(
                "Port",
                &props.editor.port,
                QuickCommandEditorEvent::UpdatePort,
            );
            content = content.push(port_row);
            content = content.push(text_input_row(
                "User",
                &props.editor.user,
                QuickCommandEditorEvent::UpdateUser,
            ));
            content = content.push(text_input_row(
                "Identity file",
                &props.editor.identity_file,
                QuickCommandEditorEvent::UpdateIdentityFile,
            ));
            content = content.push(list_editor(
                "Extra args",
                &props.editor.extra_args,
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

    let scrollable = scrollable::Scrollable::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(6)
                .margin(4)
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
    value: &'a str,
    on_input: fn(String) -> QuickCommandEditorEvent,
) -> Element<'a, QuickCommandEditorEvent> {
    let label = field_label(label);
    let input = text_input("", value)
        .on_input(on_input)
        .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
        .size(INPUT_SIZE)
        .width(Length::Fill);

    row![label, input]
        .spacing(FIELD_SPACING)
        .align_y(alignment::Vertical::Center)
        .width(Length::Fill)
        .into()
}

fn command_type_selector<'a>(
    editor: &'a QuickCommandEditorState,
    theme: ThemeProps<'a>,
) -> Element<'a, QuickCommandEditorEvent> {
    let palette = theme.theme.iced_palette();
    let selected_style = iced::widget::button::Style {
        background: Some(palette.blue.into()),
        text_color: palette.background,
        ..Default::default()
    };
    let base_style = iced::widget::button::Style {
        background: Some(palette.dim_black.into()),
        text_color: palette.foreground,
        ..Default::default()
    };

    let selected_style_custom = selected_style;
    let base_style_custom = base_style;
    let custom = button(text("Custom").size(LABEL_SIZE))
        .on_press(QuickCommandEditorEvent::SelectCommandType(
            QuickCommandType::Custom,
        ))
        .padding([0.0, BUTTON_PADDING_X])
        .height(Length::Fixed(BUTTON_HEIGHT))
        .style(move |_, _| {
            if editor.command_type == QuickCommandType::Custom {
                selected_style_custom
            } else {
                base_style_custom
            }
        });

    let selected_style_ssh = selected_style;
    let base_style_ssh = base_style;
    let ssh = button(text("SSH").size(LABEL_SIZE))
        .on_press(QuickCommandEditorEvent::SelectCommandType(
            QuickCommandType::Ssh,
        ))
        .padding([0.0, BUTTON_PADDING_X])
        .height(Length::Fixed(BUTTON_HEIGHT))
        .style(move |_, _| {
            if editor.command_type == QuickCommandType::Ssh {
                selected_style_ssh
            } else {
                base_style_ssh
            }
        });

    row![field_label("Type"), custom, ssh]
        .spacing(FIELD_SPACING)
        .align_y(alignment::Vertical::Center)
        .into()
}

fn list_editor<'a>(
    label: &'a str,
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
        let input = text_input("", value)
            .on_input(move |val| on_update(index, val))
            .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
            .size(INPUT_SIZE)
            .width(Length::Fill);

        let remove = editor_button("Remove", on_remove(index), theme);

        column = column.push(row![input, remove].spacing(FIELD_SPACING));
    }

    column.push(editor_button("Add", on_add, theme)).into()
}

fn env_editor<'a>(
    env: &'a [(String, String)],
    theme: ThemeProps<'a>,
) -> Element<'a, QuickCommandEditorEvent> {
    let mut column = column![text("Environment").size(LABEL_SIZE)]
        .spacing(FIELD_SPACING)
        .width(Length::Fill);

    for (index, (key, value)) in env.iter().enumerate() {
        let key_input = text_input("", key)
            .on_input(move |val| QuickCommandEditorEvent::UpdateEnvKey {
                index,
                value: val,
            })
            .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
            .size(INPUT_SIZE)
            .width(Length::Fill);

        let value_input = text_input("", value)
            .on_input(move |val| QuickCommandEditorEvent::UpdateEnvValue {
                index,
                value: val,
            })
            .padding([INPUT_PADDING_Y, INPUT_PADDING_X])
            .size(INPUT_SIZE)
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

impl QuickCommandType {
    fn label(&self) -> &'static str {
        match self {
            QuickCommandType::Custom => "Custom",
            QuickCommandType::Ssh => "SSH",
        }
    }
}

fn field_label<'a>(label: &'a str) -> Element<'a, QuickCommandEditorEvent> {
    container(
        text(label)
            .size(LABEL_SIZE)
            .width(Length::Fixed(LABEL_WIDTH))
            .align_x(alignment::Horizontal::Left)
            .wrapping(Wrapping::None),
    )
    .height(Length::Fill)
    .align_y(alignment::Vertical::Center)
    .into()
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
    // .width(Length::Fill)
    // .height(Length::Fill)
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
