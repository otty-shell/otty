//! The New session dialog over the window: the folder field with the
//! folders it may stand for floating under it, the profile, Cancel and
//! Start.

use std::fmt;

use iced::widget::{column, text};
use iced::{Color, Element};
use otty_agents::Profile;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::select::select_owned;
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::{fonts, spacing, typography};

use super::event::NewSessionDialogIntent;
use super::model::NewSessionDialogViewModel;
use crate::NEW_SESSION_FOLDER;
use crate::event::AgentsIntent;

/// Props for the New session dialog.
pub(crate) struct NewSessionDialogProps<'a> {
    pub(crate) vm: NewSessionDialogViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// A profile of the select, shown by its name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProfileOption {
    index: usize,
    name: String,
}

impl fmt::Display for ProfileOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// Render the modal New session dialog. Esc in the folder field closes the
/// suggestions while they show, else cancels the dialog; Enter starts.
pub(crate) fn view(
    props: NewSessionDialogProps<'_>,
) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let send = AgentsIntent::NewSessionDialog;

    let escape = if vm.suggestions.is_empty() {
        NewSessionDialogIntent::Cancelled
    } else {
        NewSessionDialogIntent::SuggestionsClosed
    };
    let mut folder =
        TextField::new(i18n::t(Key::AgentsFieldFolder), vm.folder, palette)
            .id(NEW_SESSION_FOLDER)
            .placeholder(i18n::t(Key::AgentsErrorFolderEmpty))
            .on_input(move |value| {
                send(NewSessionDialogIntent::FolderChanged(value))
            })
            .on_submit(send(NewSessionDialogIntent::Submitted))
            .on_escape(send(escape))
            .suggestions(vm.suggestions, move |index| {
                send(NewSessionDialogIntent::SuggestionPicked(index))
            })
            .on_suggestions_close(send(
                NewSessionDialogIntent::SuggestionsClosed,
            ));
    if let Some(error) = vm.folder_error {
        folder = folder.error(error);
    }

    let options: Vec<ProfileOption> = vm
        .profiles
        .iter()
        .enumerate()
        .map(|(index, profile)| ProfileOption {
            index,
            name: profile.name().to_string(),
        })
        .collect();
    let picked = vm.profile.as_ref().and_then(|profile| {
        let index = vm.profiles.iter().position(|option| option == profile)?;
        options.get(index).cloned()
    });
    let profiles: Vec<Profile> = vm.profiles;
    let mut profile = column![
        small_text(i18n::t(Key::AgentsFieldProfile), palette.text_muted()),
        select_owned(
            options,
            picked,
            move |option: ProfileOption| {
                send(NewSessionDialogIntent::ProfilePicked(
                    profiles[option.index].clone(),
                ))
            },
            palette,
        ),
    ]
    .spacing(spacing::SPACE_2);
    if let Some(note) = vm.profile_note {
        profile = profile.push(small_text(note, palette.text_muted()));
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(send(NewSessionDialogIntent::Cancelled));
    let mut start = Button::new(i18n::t(Key::AgentsStart), palette)
        .kind(ButtonKind::Primary);
    if vm.can_start {
        start = start.on_press(send(NewSessionDialogIntent::Submitted));
    }

    Dialog::new(i18n::t(Key::AgentsNewSession), palette)
        .body(column![folder, profile].spacing(spacing::SPACE_4))
        .action(cancel)
        .action(start)
        .into()
}

/// A line of small text in `color`.
fn small_text<'a>(content: &'a str, color: Color) -> Element<'a, AgentsIntent> {
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
    use crate::testing::{claude_work, codex};

    fn vm() -> NewSessionDialogViewModel<'static> {
        NewSessionDialogViewModel {
            folder: "~/pro",
            folder_error: None,
            suggestions: vec![
                String::from("~/projects/"),
                String::from("~/promo/"),
            ],
            profiles: vec![claude_work(), codex()],
            profile: Some(claude_work()),
            profile_note: None,
            can_start: true,
        }
    }

    /// What a press on `label` in the dialog of `vm` sends.
    fn pressed(
        vm: NewSessionDialogViewModel<'_>,
        label: &str,
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(NewSessionDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(label).expect("the label should show");

        ui.into_messages().collect()
    }

    fn sent(intent: NewSessionDialogIntent) -> Vec<AgentsIntent> {
        vec![AgentsIntent::NewSessionDialog(intent)]
    }

    #[test]
    fn the_dialog_shows_the_folder_the_profile_and_its_buttons() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(NewSessionDialogProps {
            vm: vm(),
            theme: ThemeProps::new(themes.current()),
        }));

        for label in [
            i18n::t(Key::AgentsNewSession),
            i18n::t(Key::AgentsFieldFolder),
            i18n::t(Key::AgentsFieldProfile),
            "~/projects/",
            i18n::t(Key::ButtonCancel),
            i18n::t(Key::AgentsStart),
        ] {
            ui.find(label).expect("the dialog part should show");
        }
    }

    /// A press on Start or Cancel while the suggestions show closes them
    /// first, as any press away from them does.
    #[test]
    fn start_cancel_and_a_suggestion_send_their_intents() {
        let closed = || NewSessionDialogViewModel {
            suggestions: Vec::new(),
            ..vm()
        };

        assert_eq!(
            pressed(closed(), i18n::t(Key::AgentsStart)),
            sent(NewSessionDialogIntent::Submitted)
        );
        assert_eq!(
            pressed(closed(), i18n::t(Key::ButtonCancel)),
            sent(NewSessionDialogIntent::Cancelled)
        );
        assert_eq!(
            pressed(vm(), i18n::t(Key::AgentsStart)),
            vec![
                AgentsIntent::NewSessionDialog(
                    NewSessionDialogIntent::SuggestionsClosed
                ),
                AgentsIntent::NewSessionDialog(
                    NewSessionDialogIntent::Submitted
                ),
            ]
        );
        assert_eq!(
            pressed(vm(), "~/promo/"),
            sent(NewSessionDialogIntent::SuggestionPicked(1))
        );
    }

    #[test]
    fn start_is_off_while_it_can_not_start_and_the_field_says_why() {
        let off = NewSessionDialogViewModel {
            folder_error: Some(i18n::t(Key::AgentsErrorFolderNotFound)),
            suggestions: Vec::new(),
            can_start: false,
            ..vm()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(NewSessionDialogProps {
            vm: off.clone(),
            theme: ThemeProps::new(themes.current()),
        }));
        ui.find(i18n::t(Key::AgentsErrorFolderNotFound))
            .expect("the error should show");

        let started = pressed(off, i18n::t(Key::AgentsStart));

        assert!(started.is_empty());
    }

    #[test]
    fn the_note_says_when_no_profile_can_start() {
        let none = NewSessionDialogViewModel {
            profiles: Vec::new(),
            profile: None,
            profile_note: Some(i18n::t(Key::AgentsNoProfileToStart)),
            can_start: false,
            ..vm()
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(NewSessionDialogProps {
            vm: none,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find(i18n::t(Key::AgentsNoProfileToStart))
            .expect("the note should show");
    }
}
