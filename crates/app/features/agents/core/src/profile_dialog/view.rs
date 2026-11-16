//! The profile dialog over the window: the agent, the name, the config
//! folder (found, typed or new) and, while a profile is edited, the program
//! path; Cancel and Add or Save.

use iced::widget::{Id, column, text};
use iced::{Color, Element};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::segmented::{segmented, segmented_with_disabled};
use otty_ui_kit::components::select::select_owned;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::event::{FolderChoice, ProfileDialogIntent};
use super::model::{FolderChoiceOption, ProfileDialogViewModel};
use crate::event::AgentsIntent;

/// The ways to give the folder of a new profile, in their order.
const FOLDER_CHOICES: [FolderChoiceOption; 3] = [
    FolderChoiceOption(FolderChoice::Found),
    FolderChoiceOption(FolderChoice::Typed),
    FolderChoiceOption(FolderChoice::Create),
];

/// Props for the profile dialog.
pub(crate) struct ProfileDialogProps<'a> {
    pub(crate) vm: ProfileDialogViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal profile dialog.
pub(crate) fn view(props: ProfileDialogProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let send = |intent| AgentsIntent::Dialog(intent);

    let mut body = column![].spacing(spacing::SPACE_4);
    if !vm.agents.is_empty() {
        let picked = vm.agent.clone().unwrap_or_else(|| vm.agents[0].clone());
        body = body.push(
            column![
                caption(i18n::t(Key::AgentsFieldAgent), palette),
                segmented(
                    &vm.agents,
                    &picked,
                    move |option| send(ProfileDialogIntent::AgentPicked(
                        option.id
                    )),
                    palette,
                ),
            ]
            .spacing(spacing::SPACE_2),
        );
    }

    let mut name =
        TextField::new(i18n::t(Key::AgentsFieldName), vm.name, palette)
            .id(field_id(Field::Name))
            .placeholder(i18n::t(Key::AgentsErrorEmptyName))
            .on_input(move |value| {
                send(ProfileDialogIntent::NameChanged(value))
            })
            .on_submit(send(ProfileDialogIntent::Submitted))
            .on_escape(send(ProfileDialogIntent::Cancelled));
    if let Some(error) = vm.name_error {
        name = name.error(error);
    }
    body = body.push(name);

    if let Some(choice) = vm.folder_choice {
        let disabled: &[FolderChoiceOption] = if vm.found.is_empty() {
            &FOLDER_CHOICES[..1]
        } else {
            &[]
        };
        body = body.push(
            column![
                caption(i18n::t(Key::AgentsFieldConfigDir), palette),
                segmented_with_disabled(
                    &FOLDER_CHOICES,
                    disabled,
                    &FolderChoiceOption(choice),
                    move |option| {
                        send(ProfileDialogIntent::FolderChoicePicked(option.0))
                    },
                    palette,
                ),
            ]
            .spacing(spacing::SPACE_2),
        );
    }

    if vm.folder_choice == Some(FolderChoice::Found) {
        body = body.push(select_owned(
            vm.found,
            vm.found_pick,
            move |folder| {
                send(ProfileDialogIntent::FoundFolderPicked(folder.0))
            },
            palette,
        ));
    } else {
        let label = match vm.folder_choice {
            Some(_) => "",
            None => i18n::t(Key::AgentsFieldConfigDir),
        };
        let mut path = TextField::new(label, vm.path, palette)
            .id(field_id(Field::Path))
            .placeholder(i18n::t(Key::AgentsErrorFolderEmpty))
            .on_input(move |value| {
                send(ProfileDialogIntent::PathChanged(value))
            })
            .on_submit(send(ProfileDialogIntent::Submitted))
            .on_escape(send(ProfileDialogIntent::Cancelled));
        if let Some(hint) = vm.path_hint {
            path = path.hint(hint);
        }
        if let Some(error) = vm.path_error {
            path = path.error(error);
        }
        body = body.push(path);
    }

    if let Some(program) = vm.program {
        body = body.push(
            TextField::new(i18n::t(Key::AgentsFieldProgram), program, palette)
                .id(field_id(Field::Program))
                .optional(i18n::t(Key::FieldOptional))
                .placeholder(i18n::t(Key::AgentsProgramPlaceholder))
                .on_input(move |value| {
                    send(ProfileDialogIntent::ProgramChanged(value))
                })
                .on_submit(send(ProfileDialogIntent::Submitted))
                .on_escape(send(ProfileDialogIntent::Cancelled)),
        );
    }

    if let Some(error) = vm.error {
        body = body.push(small_text(error, palette.status_error()));
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(send(ProfileDialogIntent::Cancelled));
    let mut submit =
        Button::new(vm.submit_label, palette).kind(ButtonKind::Primary);
    if vm.can_submit {
        submit = submit.on_press(send(ProfileDialogIntent::Submitted));
    }

    Dialog::new(vm.title, palette)
        .body(body)
        .action(cancel)
        .action(submit)
        .into()
}

/// A text field of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Field {
    Name,
    Path,
    Program,
}

/// Widget id of the text field `field`.
pub(crate) fn field_id(field: Field) -> Id {
    match field {
        Field::Name => Id::new("agents_profile_name"),
        Field::Path => Id::new("agents_profile_path"),
        Field::Program => Id::new("agents_profile_program"),
    }
}

/// The label above a row of choices.
fn caption<'a>(
    content: &'a str,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    small_text(content, palette.text_muted())
}

fn small_text<'a>(
    content: impl text::IntoFragment<'a>,
    color: Color,
) -> Element<'a, AgentsIntent> {
    let small = typography::SMALL;

    text(content)
        .size(small.size())
        .line_height(iced::Pixels(small.line_height()))
        .font(fonts::UI)
        .color(color)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_agents::AgentId;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::profile_dialog::model::{AgentOption, FoundFolder};

    fn add_dialog(can_submit: bool) -> ProfileDialogViewModel<'static> {
        let claude = AgentOption {
            id: AgentId::new("claude-code"),
            name: "Claude Code",
        };
        ProfileDialogViewModel {
            title: String::from("Add a profile"),
            agents: vec![
                claude.clone(),
                AgentOption {
                    id: AgentId::new("codex"),
                    name: "Codex",
                },
            ],
            agent: Some(claude),
            name: "",
            name_error: None,
            folder_choice: Some(FolderChoice::Create),
            found: Vec::new(),
            found_pick: None,
            path: "/home/user/.claude-work",
            path_error: None,
            path_hint: Some("OTTY creates this folder."),
            program: None,
            error: Some("disk full"),
            submit_label: i18n::t(Key::ButtonAdd),
            can_submit,
        }
    }

    fn sent(
        vm: ProfileDialogViewModel<'static>,
        act: impl FnOnce(&mut iced_test::Simulator<'_, AgentsIntent>),
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ProfileDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        act(&mut ui);

        ui.into_messages().collect()
    }

    fn dialog(intent: ProfileDialogIntent) -> AgentsIntent {
        AgentsIntent::Dialog(intent)
    }

    #[test]
    fn typing_a_name_sends_it() {
        let intents = sent(add_dialog(false), |ui| {
            ui.click(field_id(Field::Name)).expect("name should show");
            let _ = ui.typewrite("W");
        });

        assert_eq!(
            intents,
            vec![dialog(ProfileDialogIntent::NameChanged(String::from("W")))]
        );
    }

    #[test]
    fn esc_in_a_field_cancels_the_dialog() {
        let intents = sent(add_dialog(false), |ui| {
            ui.click(field_id(Field::Path)).expect("path should show");
            ui.tap_key(iced::keyboard::key::Named::Escape);
        });

        assert_eq!(intents, vec![dialog(ProfileDialogIntent::Cancelled)]);
    }

    #[test]
    fn the_agent_and_the_folder_choice_are_picked() {
        let agent = sent(add_dialog(false), |ui| {
            ui.click("Codex").expect("Codex should show");
        });
        let choice = sent(add_dialog(false), |ui| {
            ui.click(i18n::t(Key::AgentsFolderTyped))
                .expect("Path should show");
        });

        assert_eq!(
            agent,
            vec![dialog(ProfileDialogIntent::AgentPicked(AgentId::new(
                "codex"
            )))]
        );
        assert_eq!(
            choice,
            vec![dialog(ProfileDialogIntent::FolderChoicePicked(
                FolderChoice::Typed
            ))]
        );
    }

    #[test]
    fn add_sends_only_when_the_checks_pass_and_the_error_shows() {
        let blocked = sent(add_dialog(false), |ui| {
            ui.find("disk full").expect("the error should show");
            ui.click(i18n::t(Key::ButtonAdd)).expect("Add should show");
        });
        let added = sent(add_dialog(true), |ui| {
            ui.click(i18n::t(Key::ButtonAdd)).expect("Add should show");
        });

        assert!(blocked.is_empty());
        assert_eq!(added, vec![dialog(ProfileDialogIntent::Submitted)]);
    }

    #[test]
    fn the_edit_dialog_takes_the_program_path() {
        let vm = ProfileDialogViewModel {
            agents: Vec::new(),
            folder_choice: None,
            program: Some(""),
            path_hint: None,
            error: None,
            found: vec![FoundFolder(std::path::PathBuf::from("/x"))],
            ..add_dialog(true)
        };

        let intents = sent(vm, |ui| {
            ui.click(field_id(Field::Program))
                .expect("program should show");
            let _ = ui.typewrite("/");
        });

        assert_eq!(
            intents,
            vec![dialog(ProfileDialogIntent::ProgramChanged(String::from(
                "/"
            )))]
        );
    }
}
