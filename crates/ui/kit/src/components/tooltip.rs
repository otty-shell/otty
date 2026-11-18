//! `tooltip`: the kit's tooltip bubble over any content, such as a short
//! text whose whole value is too long to show, an icon button or a bar of
//! a chart. Every control of the kit that shows a tooltip shows this one.

use iced::widget::{container, text, tooltip as iced_tooltip};
use iced::{Border, Element, Padding};

use crate::palette::UiPalette;
use crate::{fonts, radius, spacing, typography};

/// Widest the bubble grows; a longer label wraps onto more lines.
const TOOLTIP_MAX_WIDTH: f32 = 360.0;

/// Shows `label` in the kit's tooltip bubble under `content` while the
/// pointer is over it, as the icon buttons show theirs. A long label wraps.
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
    placed(
        content,
        label.into(),
        iced_tooltip::Position::Bottom,
        palette,
    )
}

/// Shows `label` in the kit's tooltip bubble over `content`, for content
/// near the bottom of its place, such as the bars of a chart.
pub(super) fn tooltip_above<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    label: String,
    palette: &UiPalette,
) -> Element<'a, Message> {
    placed(content, label, iced_tooltip::Position::Top, palette)
}

/// Shows the bubble of `label` at `position` of `content`, a small gap
/// away from it.
fn placed<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    label: String,
    position: iced_tooltip::Position,
    palette: &UiPalette,
) -> Element<'a, Message> {
    iced_tooltip(content, bubble(label, palette), position)
        .gap(spacing::SPACE_1)
        .into()
}

/// The bubble: `label` in small text on the overlay surface, no wider than
/// [`TOOLTIP_MAX_WIDTH`].
fn bubble<'a, Message: 'a>(
    label: String,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let style = typography::SMALL;
    let background = palette.surface_overlay();
    let color = palette.text_body();
    let shadow = palette.shadow_popover();

    container(text(label).font(fonts::UI).size(style.size()).color(color))
        .max_width(TOOLTIP_MAX_WIDTH)
        .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_3]))
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_XS.into(),
                ..Border::default()
            },
            shadow,
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::text;
    use iced_test::simulator;

    use super::*;
    use crate::testing::{SMALL_WINDOW, TempDir, draws};

    /// A word as wide as the hover point below needs.
    fn hinted(label: &'static str) -> impl Fn() -> Element<'static, ()> {
        move || tooltip(text("Path"), label, &UiPalette::otty_dark())
    }

    #[test]
    fn given_pointer_over_the_content_then_its_label_shows_under_it() {
        let folder = TempDir::new("tooltip");
        let idle = folder.path().join("idle");
        let hovered = folder.path().join("hovered");
        let over = iced::Point::new(5.0, 8.0);

        assert!(
            draws(&hinted("/home/user/otty"), SMALL_WINDOW, None, &idle),
            "save"
        );
        assert!(
            draws(&hinted("/srv/otty"), SMALL_WINDOW, None, &idle),
            "no label away"
        );
        assert!(draws(
            &hinted("/home/user/otty"),
            SMALL_WINDOW,
            Some(over),
            &hovered
        ));
        assert!(
            !draws(&hinted("/srv/otty"), SMALL_WINDOW, Some(over), &hovered),
            "the label shows under the pointer"
        );
        assert!(
            !draws(&hinted("/home/user/otty"), SMALL_WINDOW, Some(over), &idle),
            "the bubble shows under the pointer"
        );
    }

    /// A long label wraps inside the widest bubble instead of running
    /// across the window.
    #[test]
    fn a_long_label_wraps_inside_the_widest_bubble() {
        let label = "The container stopped because its main process ended \
                     with exit code 137 after the engine sent it a kill \
                     signal";
        let hint: Element<'_, ()> =
            bubble(label.to_owned(), &UiPalette::otty_dark());
        let mut ui = simulator(hint);

        let words = ui.find(label).expect("the label should show").bounds();

        assert!(words.width <= TOOLTIP_MAX_WIDTH, "{words:?}");
        assert!(words.height > 2.0 * typography::SMALL.size(), "{words:?}");
    }
}
