//! The profile dialog over the window: the agent as a drop-down with the
//! provider of each, the name, the way to give the config folder and the
//! folder with its hint and, while an API key profile is edited, its admin
//! key and its monthly budget; Cancel and Add or Save.

use iced::widget::{Id, column, text};
use iced::{Color, Element};
use otty_agents::AgentId;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::note_select::{NoteSelect, NoteSelectOption};
use otty_ui_kit::components::segmented::segmented;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::event::{AdminKeyChoice, FolderChoice, ProfileDialogIntent};
use super::model::{
    AdminKeyChoiceOption, FolderChoiceOption, ProfileDialogViewModel,
};
use crate::event::AgentsIntent;

/// The ways to give the folder of a new profile, in their order.
const FOLDER_CHOICES: [FolderChoiceOption; 2] = [
    FolderChoiceOption(FolderChoice::Typed),
    FolderChoiceOption(FolderChoice::Create),
];

/// The places of the admin key, in their order.
const ADMIN_KEY_CHOICES: [AdminKeyChoiceOption; 2] = [
    AdminKeyChoiceOption(AdminKeyChoice::File),
    AdminKeyChoiceOption(AdminKeyChoice::Variable),
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
        let picked = vm.agent.as_ref().and_then(|agent| {
            vm.agents.iter().position(|option| option == agent)
        });
        let options = vm
            .agents
            .iter()
            .map(|agent| {
                NoteSelectOption::new(agent.name).with_note(agent.provider)
            })
            .collect();
        let ids: Vec<AgentId> =
            vm.agents.iter().map(|agent| agent.id).collect();
        body = body.push(
            column![
                caption(i18n::t(Key::AgentsFieldAgent), palette),
                NoteSelect::new(options, picked, palette)
                    .open(vm.is_agent_list_open)
                    .on_toggle(send(ProfileDialogIntent::AgentListToggled))
                    .on_pick(move |index| {
                        send(ProfileDialogIntent::AgentPicked(ids[index]))
                    }),
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
        body = body.push(segmented(
            &FOLDER_CHOICES,
            &FolderChoiceOption(choice),
            move |option| {
                send(ProfileDialogIntent::FolderChoicePicked(option.0))
            },
            palette,
        ));
    }

    let mut path =
        TextField::new(i18n::t(Key::AgentsFieldConfigDir), vm.path, palette)
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

    if let Some(choice) = vm.admin_key_choice {
        let (label, placeholder) = match choice {
            AdminKeyChoice::File => {
                (Key::AgentsAdminKeyFile, Key::AgentsAdminKeyFilePlaceholder)
            },
            AdminKeyChoice::Variable => (
                Key::AgentsAdminKeyVariable,
                Key::AgentsAdminKeyVariablePlaceholder,
            ),
        };
        let mut key = TextField::new(i18n::t(label), vm.admin_key, palette)
            .id(field_id(Field::AdminKey))
            .optional(i18n::t(Key::FieldOptional))
            .placeholder(i18n::t(placeholder))
            .hint(i18n::t(Key::AgentsAdminKeyHint))
            .on_input(move |value| {
                send(ProfileDialogIntent::AdminKeyChanged(value))
            })
            .on_submit(send(ProfileDialogIntent::Submitted))
            .on_escape(send(ProfileDialogIntent::Cancelled));
        if let Some(error) = vm.admin_key_error {
            key = key.error(error);
        }
        body = body.push(
            column![
                caption(i18n::t(Key::AgentsFieldAdminKey), palette),
                segmented(
                    &ADMIN_KEY_CHOICES,
                    &AdminKeyChoiceOption(choice),
                    move |option| {
                        send(ProfileDialogIntent::AdminKeyChoicePicked(
                            option.0,
                        ))
                    },
                    palette,
                ),
            ]
            .spacing(spacing::SPACE_2),
        );
        body = body.push(key);
    }

    if let Some(budget) = vm.budget {
        let mut field = TextField::new(
            i18n::t(Key::AgentsFieldMonthlyBudget),
            budget,
            palette,
        )
        .id(field_id(Field::Budget))
        .optional(i18n::t(Key::FieldOptional))
        .placeholder(i18n::t(Key::AgentsBudgetPlaceholder))
        .on_input(move |value| send(ProfileDialogIntent::BudgetChanged(value)))
        .on_submit(send(ProfileDialogIntent::Submitted))
        .on_escape(send(ProfileDialogIntent::Cancelled));
        if let Some(error) = vm.budget_error {
            field = field.error(error);
        }
        body = body.push(field);
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
    AdminKey,
    Budget,
}

/// Widget id of the text field `field`.
pub(crate) fn field_id(field: Field) -> Id {
    match field {
        Field::Name => Id::new("agents_profile_name"),
        Field::Path => Id::new("agents_profile_path"),
        Field::AdminKey => Id::new("agents_profile_admin_key"),
        Field::Budget => Id::new("agents_profile_budget"),
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
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::profile_dialog::model::AgentOption;

    fn add_dialog(can_submit: bool) -> ProfileDialogViewModel<'static> {
        let claude = AgentOption {
            id: AgentId::new("claude-code"),
            name: "Claude Code",
            provider: "Anthropic",
        };
        ProfileDialogViewModel {
            title: "Add profile",
            agents: vec![
                claude.clone(),
                AgentOption {
                    id: AgentId::new("codex"),
                    name: "Codex",
                    provider: "OpenAI",
                },
            ],
            is_agent_list_open: false,
            agent: Some(claude),
            name: "",
            name_error: None,
            folder_choice: Some(FolderChoice::Create),
            path: "/home/user/.claude-work",
            path_error: None,
            path_hint: Some("OTTY creates this folder."),
            admin_key_choice: None,
            admin_key: "",
            admin_key_error: None,
            budget: None,
            budget_error: None,
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
    fn the_agent_field_opens_the_list_where_an_agent_is_picked() {
        let toggled = sent(add_dialog(false), |ui| {
            ui.find("Anthropic")
                .expect("the provider of the agent shows");
            ui.click("Claude Code")
                .expect("the agent field should show");
        });
        let open = ProfileDialogViewModel {
            is_agent_list_open: true,
            ..add_dialog(false)
        };
        let picked = sent(open, |ui| {
            ui.find("OpenAI").expect("each provider shows in the list");
            ui.click("Codex").expect("Codex should show in the list");
        });

        assert_eq!(
            toggled,
            vec![dialog(ProfileDialogIntent::AgentListToggled)]
        );
        assert_eq!(
            picked,
            vec![dialog(ProfileDialogIntent::AgentPicked(AgentId::new(
                "codex"
            )))]
        );
    }

    /// Owner, issue 26: the folder of a new profile is a typed path or a
    /// new folder; "Found on disk" is gone.
    #[test]
    fn the_folder_ways_are_a_typed_path_and_a_new_folder() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ProfileDialogProps {
            vm: add_dialog(false),
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find(i18n::t(Key::AgentsFolderTyped)).is_ok());
        assert!(ui.find(i18n::t(Key::AgentsFolderCreate)).is_ok());
        assert!(ui.find("Found").is_err(), "no found folder way");
    }

    /// Coordinator, issue 27: the path field is "Config folder" under the
    /// way, and the way's word is only on the choice.
    #[test]
    fn the_path_field_is_labelled_config_folder_under_the_way() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ProfileDialogProps {
            vm: add_dialog(false),
            theme: ThemeProps::new(themes.current()),
        }));

        let way = ui
            .find(i18n::t(Key::AgentsFolderCreate))
            .expect("the way should show")
            .bounds();
        let label = ui
            .find(i18n::t(Key::AgentsFieldConfigDir))
            .expect("the field label should show")
            .bounds();
        let field = ui
            .find(field_id(Field::Path))
            .expect("the path should show")
            .bounds();

        assert!(way.y < label.y, "the label is under the way: {label:?}");
        assert!(label.y < field.y, "the label is over the field: {field:?}");
    }

    #[test]
    fn the_folder_choice_is_picked() {
        let choice = sent(add_dialog(false), |ui| {
            ui.click(i18n::t(Key::AgentsFolderTyped))
                .expect("Path should show");
        });

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
    fn the_edit_dialog_has_no_program_field() {
        let vm = ProfileDialogViewModel {
            agents: Vec::new(),
            folder_choice: None,
            path_hint: None,
            error: None,
            ..add_dialog(true)
        };

        let themes = ThemeManager::new();
        let mut ui = simulator(view(ProfileDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find("Program").is_err(), "the dialog has no program");
        assert!(ui.find(i18n::t(Key::AgentsFieldName)).is_ok());
    }

    fn api_key_dialog() -> ProfileDialogViewModel<'static> {
        ProfileDialogViewModel {
            agents: Vec::new(),
            folder_choice: None,
            path_hint: None,
            error: None,
            admin_key_choice: Some(AdminKeyChoice::File),
            admin_key: "admin-key",
            admin_key_error: Some("Type a full path"),
            budget: Some(""),
            budget_error: Some("Type an amount"),
            ..add_dialog(true)
        }
    }

    #[test]
    fn the_admin_key_place_is_picked_and_its_path_typed() {
        let picked = sent(api_key_dialog(), |ui| {
            ui.find(i18n::t(Key::AgentsFieldAdminKey))
                .expect("the admin key field should show");
            ui.find("Type a full path").expect("its error should show");
            ui.click(i18n::t(Key::AgentsAdminKeyVariable))
                .expect("the variable choice should show");
        });
        let typed = sent(api_key_dialog(), |ui| {
            ui.click(field_id(Field::AdminKey))
                .expect("the key path should show");
            let _ = ui.typewrite("~");
        });

        assert_eq!(
            picked,
            vec![dialog(ProfileDialogIntent::AdminKeyChoicePicked(
                AdminKeyChoice::Variable
            ))]
        );
        assert_eq!(
            typed,
            vec![dialog(ProfileDialogIntent::AdminKeyChanged(String::from(
                "admin-key~"
            )))]
        );
    }

    #[test]
    fn the_monthly_budget_is_typed() {
        let intents = sent(api_key_dialog(), |ui| {
            ui.find("Type an amount").expect("its error should show");
            ui.click(field_id(Field::Budget))
                .expect("the budget should show");
            let _ = ui.typewrite("3");
        });

        assert_eq!(
            intents,
            vec![dialog(ProfileDialogIntent::BudgetChanged(String::from(
                "3"
            )))]
        );
    }

    /// In a window smaller than the dialog it keeps its width and scrolls
    /// both ways, so Add can still be reached and pressed.
    #[test]
    fn in_a_window_smaller_than_the_dialog_it_scrolls_to_its_buttons() {
        let themes = ThemeManager::new();
        let content = view(ProfileDialogProps {
            vm: add_dialog(true),
            theme: ThemeProps::new(themes.current()),
        });
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(360.0, 240.0),
            content,
        );
        let name = ui
            .find(field_id(Field::Name))
            .expect("the name field should show")
            .bounds();

        ui.point_at(iced::Point::new(180.0, 120.0));
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Pixels {
                    x: -10_000.0,
                    y: -10_000.0,
                },
            },
        )]);
        ui.click(i18n::t(Key::ButtonAdd))
            .expect("Add should be in reach");

        assert!(name.width > 360.0, "the dialog keeps its width: {name:?}");
        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![dialog(ProfileDialogIntent::Submitted)]);
    }
}
