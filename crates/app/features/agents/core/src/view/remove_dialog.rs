//! The dialog that asks before an added profile is removed: its config
//! folder and its sessions stay on disk.

use iced::Element;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::small_text::small_text;

use super::super::event::AgentsIntent;
use super::super::model::RemoveDialogViewModel;

/// Props for the remove dialog.
pub(crate) struct RemoveDialogProps<'a> {
    pub(crate) vm: RemoveDialogViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal remove dialog.
pub(crate) fn view(props: RemoveDialogProps<'_>) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let body = small_text(vm.text, palette.text_muted());
    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(AgentsIntent::RemoveCancelled);
    let confirm = Button::new(vm.confirm_label, palette)
        .kind(ButtonKind::Danger)
        .on_press(AgentsIntent::RemoveConfirmed);

    Dialog::new(vm.title, palette)
        .body(body)
        .action(cancel)
        .action(confirm)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    #[test]
    fn the_dialog_says_the_folder_stays_and_sends_its_answers() {
        let vm = RemoveDialogViewModel {
            title: String::from("Remove Claude Code · Work?"),
            text: String::from("The folder stays on disk."),
            confirm_label: i18n::t(Key::ButtonRemove),
        };
        let themes = ThemeManager::new();
        let mut ui = simulator(view(RemoveDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Remove Claude Code · Work?")
            .expect("title should show");
        ui.find("The folder stays on disk.")
            .expect("text should show");
        ui.click(i18n::t(Key::ButtonCancel))
            .expect("cancel should show");
        ui.click(i18n::t(Key::ButtonRemove))
            .expect("remove should show");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![AgentsIntent::RemoveCancelled, AgentsIntent::RemoveConfirmed]
        );
    }

    /// In a window smaller than the dialog it keeps its width and scrolls
    /// both ways, so Remove can still be reached and pressed.
    #[test]
    fn in_a_window_smaller_than_the_dialog_it_scrolls_to_its_buttons() {
        let vm = RemoveDialogViewModel {
            title: String::from("Remove Claude Code · Work?"),
            text: String::from("The folder stays on disk."),
            confirm_label: i18n::t(Key::ButtonRemove),
        };
        let themes = ThemeManager::new();
        let dialog = view(RemoveDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        });
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(240.0, 120.0),
            dialog,
        );

        ui.point_at(iced::Point::new(120.0, 60.0));
        let _ = ui.simulate([iced::Event::Mouse(
            iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Pixels {
                    x: -10_000.0,
                    y: -10_000.0,
                },
            },
        )]);
        ui.click(i18n::t(Key::ButtonRemove))
            .expect("Remove should be in reach");

        let intents: Vec<AgentsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![AgentsIntent::RemoveConfirmed]);
    }
}
