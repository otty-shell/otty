use iced::widget::{Space, column, container, row, text};
use iced::{Element, Length, Theme};

use crate::shared::ui::theme::ThemeProps;
use crate::widgets::quick_launch::event::QuickLaunchEvent;
use crate::widgets::quick_launch::model::QuickLaunchType;
use crate::widgets::quick_launch::state::WizardEditorState;

const SECTION_SPACING: f32 = 16.0;
const FIELD_SPACING: f32 = 8.0;
const LABEL_SIZE: f32 = 13.0;
const INPUT_SIZE: f32 = 13.0;
const FORM_PADDING: f32 = 24.0;

/// Props for the quick launch wizard form.
pub(crate) struct WizardFormProps<'a> {
    pub(crate) tab_id: u64,
    pub(crate) editor: &'a WizardEditorState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the quick launch wizard form.
pub(crate) fn view(
    props: WizardFormProps<'_>,
) -> Element<'_, QuickLaunchEvent, Theme, iced::Renderer> {
    let palette = props.theme.theme.iced_palette();
    let tab_id = props.tab_id;
    let editor = props.editor;

    let mut sections: Vec<
        Element<'_, QuickLaunchEvent, Theme, iced::Renderer>,
    > = Vec::new();

    // Title
    let title_mode = if editor.is_create_mode() {
        "Create Quick Launch"
    } else {
        "Edit Quick Launch"
    };
    sections.push(text(title_mode).size(LABEL_SIZE + 4.0).into());

    // Title field
    sections.push(field_row(
        "Title",
        iced::widget::text_input("Command title", editor.title())
            .on_input(move |v| QuickLaunchEvent::WizardUpdateTitle {
                tab_id,
                value: v,
            })
            .size(INPUT_SIZE)
            .into(),
    ));

    // Command type options
    match editor.command_type() {
        QuickLaunchType::Custom => {
            if let Some(custom) = editor.custom() {
                sections.push(field_row(
                    "Program",
                    iced::widget::text_input("Program path", custom.program())
                        .on_input(move |v| {
                            QuickLaunchEvent::WizardUpdateProgram {
                                tab_id,
                                value: v,
                            }
                        })
                        .size(INPUT_SIZE)
                        .into(),
                ));

                sections.push(field_row(
                    "Working Dir",
                    iced::widget::text_input(
                        "Working directory",
                        custom.working_directory(),
                    )
                    .on_input(move |v| {
                        QuickLaunchEvent::WizardUpdateWorkingDirectory {
                            tab_id,
                            value: v,
                        }
                    })
                    .size(INPUT_SIZE)
                    .into(),
                ));
            }
        },
        QuickLaunchType::Ssh => {
            if let Some(ssh) = editor.ssh() {
                sections.push(field_row(
                    "Host",
                    iced::widget::text_input("SSH host", ssh.host())
                        .on_input(move |v| QuickLaunchEvent::WizardUpdateHost {
                            tab_id,
                            value: v,
                        })
                        .size(INPUT_SIZE)
                        .into(),
                ));

                sections.push(field_row(
                    "Port",
                    iced::widget::text_input("Port", ssh.port())
                        .on_input(move |v| QuickLaunchEvent::WizardUpdatePort {
                            tab_id,
                            value: v,
                        })
                        .size(INPUT_SIZE)
                        .into(),
                ));

                sections.push(field_row(
                    "User",
                    iced::widget::text_input("User", ssh.user())
                        .on_input(move |v| QuickLaunchEvent::WizardUpdateUser {
                            tab_id,
                            value: v,
                        })
                        .size(INPUT_SIZE)
                        .into(),
                ));

                sections.push(field_row(
                    "Identity File",
                    iced::widget::text_input(
                        "Path to key",
                        ssh.identity_file(),
                    )
                    .on_input(move |v| {
                        QuickLaunchEvent::WizardUpdateIdentityFile {
                            tab_id,
                            value: v,
                        }
                    })
                    .size(INPUT_SIZE)
                    .into(),
                ));
            }
        },
    }

    // Error message
    if let Some(error) = editor.error() {
        let error_color = palette.red;
        sections.push(
            container(text(error).size(LABEL_SIZE))
                .style(move |_| iced::widget::container::Style {
                    text_color: Some(error_color),
                    ..Default::default()
                })
                .into(),
        );
    }

    // Action buttons
    let save_button = iced::widget::button(text("Save").size(LABEL_SIZE))
        .on_press(QuickLaunchEvent::WizardSave { tab_id });

    let cancel_button = iced::widget::button(text("Cancel").size(LABEL_SIZE))
        .on_press(QuickLaunchEvent::WizardCancel { tab_id });

    sections.push(
        row![
            save_button,
            Space::new().width(Length::Fixed(8.0)),
            cancel_button
        ]
        .into(),
    );

    let content = column(sections).spacing(FIELD_SPACING).width(Length::Fill);

    container(
        iced::widget::scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(FORM_PADDING)
    .into()
}

fn field_row<'a>(
    label: &'a str,
    input: Element<'a, QuickLaunchEvent, Theme, iced::Renderer>,
) -> Element<'a, QuickLaunchEvent, Theme, iced::Renderer> {
    row![
        container(text(label).size(LABEL_SIZE))
            .width(Length::Fixed(120.0))
            .align_y(iced::alignment::Vertical::Center),
        input,
    ]
    .spacing(FIELD_SPACING)
    .align_y(iced::alignment::Vertical::Center)
    .width(Length::Fill)
    .into()
}
