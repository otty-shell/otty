//! The dialog a destructive action asks first with: what goes, an optional
//! switch ("Stop it first", "Force"), Cancel and the action.

use iced::Element;
use iced::widget::{column, text};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::ActionsIntent;
use super::super::model::ConfirmDialogViewModel;

/// Props for the confirmation dialog.
pub(crate) struct ConfirmDialogProps<'a> {
    pub(crate) vm: ConfirmDialogViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal confirmation dialog.
pub(crate) fn view(
    props: ConfirmDialogProps<'_>,
) -> Element<'_, ActionsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let hint = typography::SMALL;

    let mut body = column![
        text(vm.text)
            .size(hint.size())
            .line_height(iced::Pixels(hint.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted())
    ]
    .spacing(spacing::SPACE_4);
    if let Some((label, is_on)) = vm.switch {
        body = body.push(
            Switch::new(is_on, palette)
                .label(label)
                .on_toggle(ActionsIntent::ConfirmSwitchToggled),
        );
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(ActionsIntent::ConfirmCancelled);
    let confirm = Button::new(vm.confirm_label, palette)
        .kind(ButtonKind::Danger)
        .on_press(ActionsIntent::ConfirmAccepted);

    Dialog::new(vm.title, palette)
        .body(body)
        .action(cancel)
        .action(confirm)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_i18n::Locale;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::palette::UiPalette;
    use otty_ui_kit::size;

    use super::*;

    fn remove_dialog(switch: Option<bool>) -> ConfirmDialogViewModel {
        ConfirmDialogViewModel {
            title: String::from("Remove container web-1?"),
            text: String::from("The container is deleted."),
            switch: switch.map(|on| (i18n::t(Key::ContainersStopItFirst), on)),
            confirm_label: i18n::t(Key::ContainersActionRemove),
        }
    }

    #[test]
    fn the_dialog_shows_its_title_text_and_switch() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ConfirmDialogProps {
            vm: remove_dialog(Some(true)),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Remove container web-1?")
            .expect("title should show");
        ui.find("The container is deleted.")
            .expect("text should show");
        ui.find(i18n::t(Key::ContainersStopItFirst))
            .expect("switch label should show");
    }

    #[test]
    fn cancel_and_the_action_send_their_intents() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ConfirmDialogProps {
            vm: remove_dialog(None),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ButtonCancel))
            .expect("cancel should show");
        ui.click(i18n::t(Key::ContainersActionRemove))
            .expect("remove should show");

        let intents: Vec<ActionsIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![
                ActionsIntent::ConfirmCancelled,
                ActionsIntent::ConfirmAccepted
            ]
        );
    }

    #[test]
    fn a_dialog_without_a_switch_shows_none() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ConfirmDialogProps {
            vm: remove_dialog(None),
            theme: ThemeProps::new(themes.current()),
        }));

        assert!(ui.find(i18n::t(Key::ContainersStopItFirst)).is_err());
    }

    #[test]
    fn the_switch_sends_its_new_state() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(ConfirmDialogProps {
            vm: remove_dialog(Some(true)),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ContainersStopItFirst))
            .expect("switch should show");

        let intents: Vec<ActionsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![ActionsIntent::ConfirmSwitchToggled(false)]);
    }

    #[test]
    fn the_named_volumes_switch_sends_its_new_state() {
        let themes = ThemeManager::new();
        let mut vm = remove_dialog(None);
        vm.switch =
            Some((i18n::t(Key::ContainersAlsoRemoveNamedVolumes), false));
        let mut ui = simulator(view(ConfirmDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(i18n::t(Key::ContainersAlsoRemoveNamedVolumes))
            .expect("switch should show");

        let intents: Vec<ActionsIntent> = ui.into_messages().collect();
        assert_eq!(intents, vec![ActionsIntent::ConfirmSwitchToggled(true)]);
    }

    /// A dialog switch stays on one line, so its label has to fit whole: it
    /// says what a removal will force. The Chinese, Japanese and Korean
    /// labels are short, and a test machine may have no font to measure them
    /// with. Copied from the old i18n tests: it measures a kit `Switch`, so
    /// it needs iced and lives with the confirm dialogs.
    #[test]
    fn given_a_confirm_dialog_switch_then_its_label_fits_the_dialog() {
        let palette = UiPalette::otty_dark();
        let room = size::DIALOG - 2.0 * spacing::SPACE_5;

        for locale in [
            Locale::En,
            Locale::Fr,
            Locale::De,
            Locale::Es,
            Locale::PtBr,
            Locale::Ru,
        ] {
            for key in
                [Key::ContainersStopItFirst, Key::ContainersForceRemoveImage]
            {
                let label = i18n::text_in(locale, key);
                let switch: Switch<'_, ()> =
                    Switch::new(true, &palette).label(label);

                assert!(
                    switch.width() <= room,
                    "{label} in {} is {} wide, the dialog has {room}",
                    locale.tag(),
                    switch.width(),
                );
            }
        }
    }
}
