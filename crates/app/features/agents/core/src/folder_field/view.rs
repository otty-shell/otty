//! The folder field with its suggestions floating under it, as both dialogs
//! draw it.

use iced::widget::Id;
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::text_field::TextField;
use otty_ui_kit::palette::UiPalette;

use super::event::FolderFieldIntent;
use crate::event::AgentsIntent;

/// The folder field `id` labelled `label`, with `text` typed in it and the
/// `suggestions` under it, `highlighted` the row the keys moved to. ↓ and
/// ↑ move through the suggestions, Enter and Tab pick the highlighted one,
/// and Esc closes them while they show; `send` wraps what the field sends
/// into the intent of its dialog. The field always takes suggestions, also
/// while there are none, so it keeps the keyboard while they come and go.
pub(crate) fn folder_field<'a>(
    id: Id,
    label: &str,
    text: &'a str,
    suggestions: Vec<String>,
    highlighted: Option<usize>,
    send: fn(FolderFieldIntent) -> AgentsIntent,
    palette: &UiPalette,
) -> TextField<'a, AgentsIntent> {
    TextField::new(label, text, palette)
        .id(id)
        .placeholder(i18n::t(Key::AgentsErrorFolderEmpty))
        .on_input(move |value| send(FolderFieldIntent::Changed(value)))
        .suggestions(suggestions, move |index| {
            send(FolderFieldIntent::SuggestionPicked(index))
        })
        .highlighted_suggestion(highlighted, move |row| {
            send(FolderFieldIntent::SuggestionHighlighted(row))
        })
        .on_suggestions_close(send(FolderFieldIntent::SuggestionsClosed))
}

#[cfg(test)]
mod tests {
    use iced::Element;
    use iced::keyboard::key::Named;
    use iced_test::simulator;
    use otty_ui_kit::palette::UiPalette;

    use super::*;
    use crate::new_session_dialog::NewSessionDialogIntent;

    const FIELD: Id = Id::new("folder_field_test");

    fn send(intent: FolderFieldIntent) -> AgentsIntent {
        AgentsIntent::NewSessionDialog(NewSessionDialogIntent::Folder(intent))
    }

    /// What `key` pressed in the field with two suggestions, the row
    /// `highlighted` highlighted, sends.
    fn tapped(highlighted: Option<usize>, key: Named) -> Vec<AgentsIntent> {
        let palette = UiPalette::otty_dark();
        let suggestions =
            vec![String::from("~/projects/"), String::from("~/promo/")];
        let field: Element<'_, AgentsIntent> = folder_field(
            FIELD,
            "Folder",
            "~/pro",
            suggestions,
            highlighted,
            send,
            &palette,
        )
        .into();
        let mut ui = simulator(field);

        ui.click(FIELD).expect("the field should show");
        ui.tap_key(key);

        ui.into_messages().collect()
    }

    /// Issue 39: ↓ and ↑ move the highlight, Enter and Tab pick the
    /// highlighted row, and Esc closes the list while it shows.
    #[test]
    fn the_keys_move_through_the_suggestions_and_pick_one() {
        let highlight =
            |row| vec![send(FolderFieldIntent::SuggestionHighlighted(row))];
        let pick = vec![send(FolderFieldIntent::SuggestionPicked(1))];

        assert_eq!(tapped(None, Named::ArrowDown), highlight(Some(0)));
        assert_eq!(tapped(Some(0), Named::ArrowUp), highlight(None));
        assert_eq!(tapped(Some(1), Named::ArrowUp), highlight(Some(0)));
        assert_eq!(tapped(Some(1), Named::Enter), pick);
        assert_eq!(tapped(Some(1), Named::Tab), pick);
        assert_eq!(
            tapped(None, Named::Escape),
            vec![send(FolderFieldIntent::SuggestionsClosed)]
        );
    }

    #[test]
    fn a_pressed_suggestion_is_picked_and_typing_sends_the_text() {
        let palette = UiPalette::otty_dark();
        let field = || -> Element<'_, AgentsIntent> {
            folder_field(
                FIELD,
                "Folder",
                "~/pro",
                vec![String::from("~/projects/"), String::from("~/promo/")],
                None,
                send,
                &palette,
            )
            .into()
        };
        let mut pressed = simulator(field());
        let mut typed = simulator(field());

        pressed
            .click("~/promo/")
            .expect("the suggestion should show");
        typed.click(FIELD).expect("the field should show");
        let _ = typed.typewrite("j");

        assert_eq!(
            pressed.into_messages().collect::<Vec<_>>(),
            vec![send(FolderFieldIntent::SuggestionPicked(1))]
        );
        assert_eq!(
            typed.into_messages().collect::<Vec<_>>(),
            vec![send(FolderFieldIntent::Changed(String::from("~/proj")))]
        );
    }
}
