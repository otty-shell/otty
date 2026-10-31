use iced::widget::container;
use iced::{Element, Length, alignment};
use otty_ui_kit::components::floating_note::FloatingNote;

use crate::i18n::{self, Key};
use crate::shortcuts::AppShortcut;
use crate::theme::ThemeProps;

/// Distance between the top of the main area and the note.
const NOTE_TOP_GAP: f32 = 12.0;

/// Render the focus mode note at the top center of the main area, or `None`
/// outside focus mode.
///
/// The returned layer is full size but has no interactive parts, so mouse
/// input outside the note still reaches the layers below.
pub(crate) fn view<'a, Message: 'a>(
    focus_mode: bool,
    theme: ThemeProps<'a>,
) -> Option<Element<'a, Message>> {
    if !focus_mode {
        return None;
    }

    let note: Element<'a, Message> = FloatingNote::new(theme.theme.ui())
        .text(i18n::t(Key::WindowFocusMode))
        .muted(i18n::t(Key::FocusNoteHidden))
        .kbd(AppShortcut::FocusMode.label())
        .muted(i18n::t(Key::FocusNoteShow))
        .into();

    Some(
        container(note)
            .padding(iced::Padding {
                top: NOTE_TOP_GAP,
                ..iced::Padding::ZERO
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Top)
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::theme::ThemeManager;

    #[test]
    fn note_is_hidden_outside_focus_mode() {
        let themes = ThemeManager::new();

        assert!(view::<()>(false, ThemeProps::new(themes.current())).is_none());
    }

    #[test]
    fn note_names_the_mode_and_the_shortcut_in_focus_mode() {
        let themes = ThemeManager::new();
        let note = view::<()>(true, ThemeProps::new(themes.current()))
            .expect("focus mode should show the note");
        let mut ui = simulator(note);

        ui.find(i18n::t(Key::WindowFocusMode))
            .expect("mode name should be visible");
        ui.find(i18n::t(Key::FocusNoteHidden))
            .expect("hidden hint should be visible");
        ui.find(i18n::t(Key::FocusNoteShow))
            .expect("show hint should be visible");
        ui.find(AppShortcut::FocusMode.label())
            .expect("shortcut should be visible");
    }
}
