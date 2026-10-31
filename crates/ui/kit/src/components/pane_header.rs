//! `PaneHeader`: thin bar above a terminal pane.

use iced::widget::{Space, container, row, text};
use iced::{Alignment, Element, Length, Padding};

use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Bar above a pane: program name, muted folder, a gap and an optional
/// trailing element such as a [`Chip`] or a state word.
///
/// The focused pane sits on `surface-header` with a strong program name;
/// the other panes sit on `surface-canvas` with muted text.
///
/// [`Chip`]: super::chip::Chip
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::chip::Chip;
/// use otty_ui_kit::components::pane_header::PaneHeader;
/// use otty_ui_kit::components::target_dot::TargetKind;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let header: Element<'_, ()> = PaneHeader::new("zsh", &palette)
///     .folder("~/projects/otty")
///     .focused(true)
///     .trailing(Chip::new("fedora", &palette).target(TargetKind::Local))
///     .into();
/// ```
pub struct PaneHeader<'a, Message> {
    program: String,
    palette: UiPalette,
    folder: Option<String>,
    is_focused: bool,
    trailing: Option<Element<'a, Message>>,
}

impl<'a, Message> PaneHeader<'a, Message> {
    /// Creates a header with the `program` name.
    pub fn new(program: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            program: program.into(),
            palette: palette.clone(),
            folder: None,
            is_focused: false,
            trailing: None,
        }
    }

    /// Shows the working folder after the program name, in the mono font.
    pub fn folder(mut self, folder: impl Into<String>) -> Self {
        self.folder = Some(folder.into());
        self
    }

    /// Marks the header as the one of the focused pane.
    pub fn focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    /// Shows an element at the right edge, such as a `Chip`.
    pub fn trailing(
        mut self,
        element: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.trailing = Some(element.into());
        self
    }
}

impl<'a, Message: 'a> From<PaneHeader<'a, Message>> for Element<'a, Message> {
    fn from(value: PaneHeader<'a, Message>) -> Self {
        let palette = value.palette;
        let style = typography::SMALL;
        let is_focused = value.is_focused;
        let program_color = if is_focused {
            palette.text_strong()
        } else {
            palette.text_muted()
        };
        let background = if is_focused {
            palette.surface_header()
        } else {
            palette.surface_canvas()
        };

        let mut content = row![
            text(value.program)
                .size(style.size())
                .line_height(iced::Pixels(style.line_height()))
                .font(fonts::UI)
                .color(program_color)
        ]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center);
        if let Some(folder) = value.folder {
            content = content.push(
                text(folder)
                    .size(style.size())
                    .line_height(iced::Pixels(style.line_height()))
                    .font(fonts::MONO)
                    .color(palette.text_muted()),
            );
        }
        content = content.push(Space::new().width(Length::Fill));
        if let Some(trailing) = value.trailing {
            content = content.push(trailing);
        }

        container(content)
            .width(Length::Fill)
            .height(size::PANE_HEADER)
            .align_y(Alignment::Center)
            .padding(Padding::from([0.0, spacing::SPACE_4]))
            .style(move |_| container::Style {
                background: Some(background.into()),
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::components::chip::Chip;

    #[test]
    fn given_program_folder_and_chip_when_rendered_then_all_are_shown() {
        let palette = UiPalette::otty_dark();
        let header = PaneHeader::new("zsh", &palette)
            .folder("~/projects/otty")
            .focused(true)
            .trailing(Chip::new("fedora", &palette));
        let header: Element<'_, ()> = header.into();
        let mut ui = simulator(header);

        assert!(ui.find("zsh").is_ok());
        assert!(ui.find("~/projects/otty").is_ok());
        assert!(ui.find("fedora").is_ok());
    }

    #[test]
    fn given_header_without_folder_when_rendered_then_program_is_shown() {
        let palette = UiPalette::otty_light();
        let header: Element<'_, ()> =
            PaneHeader::new("claude", &palette).into();
        let mut ui = simulator(header);

        assert!(ui.find("claude").is_ok());
    }
}
