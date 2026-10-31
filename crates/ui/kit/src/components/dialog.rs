//! `Dialog`: a modal card centered over a dimmed window.

use iced::widget::{Space, column, container, opaque, row, text};
use iced::{Element, Length, alignment};

use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Modal dialog: a title, an optional body and a row of actions, `space-5`
/// apart, on a `surface_overlay` card `size::DIALOG` wide. The card sits in
/// the middle of a full-window `scrim`. Clicks never reach the UI behind it.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::dialog::Dialog;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let dialog: Element<'_, ()> = Dialog::new("Delete session", &palette)
///     .body(text("This cannot be undone."))
///     .action(text("Cancel"))
///     .action(text("Delete"))
///     .into();
/// ```
pub struct Dialog<'a, Message> {
    title: String,
    palette: UiPalette,
    body: Option<Element<'a, Message>>,
    actions: Vec<Element<'a, Message>>,
}

impl<'a, Message> Dialog<'a, Message> {
    /// Creates a dialog with a title and no body or actions.
    pub fn new(title: impl Into<String>, palette: &UiPalette) -> Self {
        Self {
            title: title.into(),
            palette: palette.clone(),
            body: None,
            actions: Vec::new(),
        }
    }

    /// Sets the body, shown under the title.
    pub fn body(mut self, body: impl Into<Element<'a, Message>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Appends an action. Actions sit right-aligned in one row, in the order
    /// added.
    pub fn action(mut self, action: impl Into<Element<'a, Message>>) -> Self {
        self.actions.push(action.into());
        self
    }
}

impl<'a, Message: 'a> From<Dialog<'a, Message>> for Element<'a, Message> {
    fn from(value: Dialog<'a, Message>) -> Self {
        let palette = value.palette;

        let title = text(value.title)
            .size(typography::HEADING.size())
            .font(fonts::UI)
            .color(palette.text_strong());

        let mut content =
            column![title].spacing(spacing::SPACE_5).width(Length::Fill);
        if let Some(body) = value.body {
            content = content.push(body);
        }
        if !value.actions.is_empty() {
            let actions = row![Space::new().width(Length::Fill)]
                .extend(value.actions)
                .spacing(spacing::SPACE_3)
                .width(Length::Fill);
            content = content.push(actions);
        }

        let card_background = palette.surface_overlay();
        let card_shadow = palette.shadow_popover();
        let card = container(content)
            .width(size::DIALOG)
            .padding(spacing::SPACE_5)
            .style(move |_| container::Style {
                background: Some(card_background.into()),
                border: iced::Border {
                    radius: radius::RADIUS_LG.into(),
                    ..Default::default()
                },
                shadow: card_shadow,
                ..Default::default()
            });

        let scrim = palette.scrim();

        // `opaque` keeps clicks from reaching the UI behind the dialog.
        opaque(
            container(card)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(move |_| container::Style {
                    background: Some(scrim.into()),
                    ..Default::default()
                }),
        )
    }
}
