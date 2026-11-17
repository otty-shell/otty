//! `tooltip`: the kit's tooltip bubble over any content, such as a short
//! text whose whole value is too long to show.

use iced::Element;
use iced::widget::tooltip as iced_tooltip;

use super::icon_button::tooltip_hint;
use crate::palette::UiPalette;
use crate::spacing;

/// Shows `label` in the kit's tooltip bubble under `content` while the
/// pointer is over it, as the icon buttons show theirs.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::tooltip::tooltip;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let path: Element<'_, ()> = tooltip(
///     text("~/…/ottyrun/otty"),
///     "/home/user/projects/main/ottyrun/otty",
///     &palette,
/// );
/// ```
pub fn tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    label: impl Into<String>,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let hint = tooltip_hint(label.into(), palette);

    iced_tooltip(content, hint, iced_tooltip::Position::Bottom)
        .gap(spacing::SPACE_1)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::text;

    use super::*;
    use crate::testing::{TempDir, draws};

    /// A word as wide as the hover point below needs.
    fn hinted(label: &'static str) -> impl Fn() -> Element<'static, ()> {
        move || tooltip(text("Path"), label, &UiPalette::otty_dark())
    }

    #[test]
    fn given_pointer_over_the_content_then_its_label_shows_under_it() {
        let folder = TempDir::new("tooltip");
        let idle = folder.path().join("idle");
        let hovered = folder.path().join("hovered");
        let over = iced::Point::new(45.0, 48.0);

        assert!(draws(&hinted("/home/user/otty"), None, &idle), "save");
        assert!(draws(&hinted("/srv/otty"), None, &idle), "no label away");
        assert!(draws(&hinted("/home/user/otty"), Some(over), &hovered));
        assert!(
            !draws(&hinted("/srv/otty"), Some(over), &hovered),
            "the label shows under the pointer"
        );
        assert!(
            !draws(&hinted("/home/user/otty"), Some(over), &idle),
            "the bubble shows under the pointer"
        );
    }
}
