//! `SessionGroup`: label row that starts a group of sessions on one target.

use iced::widget::text::Alignment as TextAlignment;
use iced::widget::{container, row};
use iced::{Alignment, Element, Length, Padding};

use super::ellipsis_text::EllipsisText;
use super::target_dot::{TargetKind, target_dot};
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Name size (`.ot-group { font-size: 12.5px }`).
const NAME_SIZE: f32 = 12.5;

/// Group label: target dot, machine name and a muted kind such as
/// "this computer" or "ssh".
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::session_group::SessionGroup;
/// use otty_ui_kit::components::target_dot::TargetKind;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let group: Element<'_, ()> =
///     SessionGroup::new("fedora", TargetKind::Local, &palette)
///         .note("this computer")
///         .into();
/// ```
pub struct SessionGroup {
    name: String,
    target: TargetKind,
    palette: UiPalette,
    note: Option<String>,
}

impl SessionGroup {
    /// Creates a group label for the machine `name`.
    pub fn new(
        name: impl Into<String>,
        target: TargetKind,
        palette: &UiPalette,
    ) -> Self {
        Self {
            name: name.into(),
            target,
            palette: palette.clone(),
            note: None,
        }
    }

    /// Shows a muted kind after the name.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

impl<'a, Message: 'a> From<SessionGroup> for Element<'a, Message> {
    fn from(value: SessionGroup) -> Self {
        let palette = value.palette;
        let caption = typography::CAPTION;

        // The name takes the room it needs first; the note on the right takes
        // what is left and ends with "…" when that is too little.
        let mut content = row![
            target_dot(value.target, &palette),
            EllipsisText::new(
                value.name,
                NAME_SIZE,
                fonts::UI,
                palette.text_muted()
            )
            .shrink(0.0),
        ]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center);
        if let Some(note) = value.note {
            content = content.push(
                EllipsisText::new(
                    note,
                    caption.size(),
                    fonts::UI,
                    palette.text_muted(),
                )
                .align_x(TextAlignment::Right),
            );
        }

        container(content)
            .width(Length::Fill)
            .height(size::GROUP)
            .padding(Padding::from([0.0, spacing::SPACE_3]))
            .align_y(Alignment::Center)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::Rectangle;
    use iced_test::simulator;

    use super::*;

    /// Bounds of the name and of the note of a "fedora" group placed in a
    /// space `width` wide.
    fn name_and_note_bounds(width: f32) -> (Rectangle, Rectangle) {
        let palette = UiPalette::otty_dark();
        let group: Element<'_, ()> =
            SessionGroup::new("fedora", TargetKind::Local, &palette)
                .note("this computer")
                .into();
        let mut ui = simulator(container(group).width(width));
        let name = ui.find("fedora").expect("name should show").bounds();
        let note = ui.find("this computer").expect("note should show").bounds();

        (name, note)
    }

    #[test]
    fn given_narrow_group_when_laid_out_then_the_note_gives_way_on_one_line() {
        let (full_name, full_note) = name_and_note_bounds(600.0);

        let (name, note) = name_and_note_bounds(170.0);

        assert!(
            (name.width - full_name.width).abs() < 0.5,
            "the name should keep its width: {name:?} against {full_name:?}"
        );
        assert!(
            note.width < full_note.width,
            "the note should shrink: {note:?} against {full_note:?}"
        );
        assert!(
            (note.height - full_note.height).abs() < 0.5,
            "the note should stay one line: {note:?}"
        );
        assert!(note.x >= name.x + name.width, "the note should not overlap");
        assert!(note.x + note.width <= 170.5, "the note stays inside");
    }

    #[test]
    fn given_minimum_panel_width_when_laid_out_then_nothing_wraps_or_overlaps()
    {
        let (full_name, full_note) = name_and_note_bounds(600.0);

        let (name, note) = name_and_note_bounds(128.0);

        assert!(
            (name.height - full_name.height).abs() < 0.5,
            "the name should stay one line: {name:?}"
        );
        assert!(
            (note.height - full_note.height).abs() < 0.5,
            "the note should stay one line: {note:?}"
        );
        assert!(note.x >= name.x + name.width, "the note should not overlap");
        assert!(note.x + note.width <= 128.5, "the note stays inside");
    }

    #[test]
    fn given_very_narrow_group_when_laid_out_then_the_name_ends_with_an_ellipsis()
     {
        let (full_name, _) = name_and_note_bounds(600.0);

        let (name, note) = name_and_note_bounds(60.0);

        assert!(
            name.width < full_name.width,
            "the name should be cut: {name:?} against {full_name:?}"
        );
        assert!(
            note.width < 20.0,
            "the note should be nearly gone: {note:?}"
        );
        assert!(name.x + name.width <= 60.5, "the name stays inside");
    }
}
