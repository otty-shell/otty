//! `SessionGroup`: label row that starts a group of sessions on one target.

use iced::widget::text::Wrapping;
use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length, Padding};

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

        let mut content = row![
            target_dot(value.target, &palette),
            container(
                text(value.name)
                    .size(NAME_SIZE)
                    .font(fonts::UI)
                    .color(palette.text_muted())
                    .wrapping(Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true),
        ]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center);
        if let Some(note) = value.note {
            content = content.push(
                text(note)
                    .size(caption.size())
                    .font(fonts::UI)
                    .color(palette.text_muted()),
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
