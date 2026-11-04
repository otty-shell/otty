//! `IconButton`: square icon-only button with a tooltip.

use iced::widget::{button, container, svg, text, tooltip};
use iced::{Border, Color, Element, Length, Padding};

use super::faded;
use super::icon_hover::IconHover;
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Icon edge length inside the button. The design says 15px, but 16 is a
/// whole, even number of screen pixels at 100%, 125%, 150%, 175% and 200%
/// scale and sits on a whole pixel inside both button sizes. iced draws an
/// svg with nearest sampling and rounds the top and bottom of the drawn area
/// separately, so at 15px the icon lost or doubled a row at some heights
/// (for example the Launches plus under exactly one session at 125%).
const ICON_SIZE: f32 = 16.0;

/// Size of an [`IconButton`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconButtonSize {
    /// 28px, the default.
    Medium,
    /// 24px, for dense headers.
    Small,
}

/// Icon color of an [`IconButton`] while idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconTone {
    /// `text-muted`.
    Default,
    /// `accent`; only for the New session plus.
    Accent,
    /// `status-error`, on `surface-danger-hover` while hovered; for destructive
    /// actions such as Remove.
    Danger,
}

/// Square icon-only button. The label is shown as a tooltip. A button
/// without a message is disabled and draws its icon faded.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::icon_button::{IconButton, IconTone};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Close,
///     Remove,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let close: Element<'_, Message> =
///     IconButton::new(icon.clone(), "Close", &palette)
///         .on_press(Message::Close)
///         .into();
/// let can_remove = true;
/// let remove: Element<'_, Message> =
///     IconButton::new(icon, "Remove", &palette)
///         .tone(IconTone::Danger)
///         .on_press_maybe(can_remove.then_some(Message::Remove))
///         .into();
/// ```
pub struct IconButton<Message> {
    icon: svg::Handle,
    label: String,
    palette: UiPalette,
    size: IconButtonSize,
    tone: IconTone,
    has_hover_background: bool,
    on_press: Option<Message>,
}

impl<Message> IconButton<Message> {
    /// Creates a medium default-tone button with `icon` and a tooltip
    /// `label`.
    pub fn new(
        icon: svg::Handle,
        label: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            icon,
            label: label.into(),
            palette: palette.clone(),
            size: IconButtonSize::Medium,
            tone: IconTone::Default,
            has_hover_background: true,
            on_press: None,
        }
    }

    /// Sets the button size.
    pub fn size(mut self, size: IconButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Sets the idle icon tone.
    pub fn tone(mut self, tone: IconTone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets whether a background shows while the button is hovered or
    /// pressed. On by default. Without it only the icon turns brighter on
    /// hover, for a cross drawn over a row that has its own background.
    pub fn hover_background(mut self, has_hover_background: bool) -> Self {
        self.has_hover_background = has_hover_background;
        self
    }

    /// Sets the message sent on press.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the message sent on press, or none: a button without a message
    /// is disabled and draws its icon faded.
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }
}

impl<'a, Message: Clone + 'a> From<IconButton<Message>>
    for Element<'a, Message>
{
    fn from(value: IconButton<Message>) -> Self {
        let edge = match value.size {
            IconButtonSize::Medium => size::CONTROL_MD,
            IconButtonSize::Small => size::CONTROL_SM,
        };
        let tone = value.tone;
        let is_enabled = value.on_press.is_some();
        let idle = icon_color(&value.palette, tone, is_enabled, false);
        let hover = icon_color(&value.palette, tone, is_enabled, true);
        let has_hover_background = value.has_hover_background;
        let palette = value.palette.clone();

        let pointer = IconHover::new();
        let icon = svg(value.icon)
            .width(ICON_SIZE)
            .height(ICON_SIZE)
            .style(pointer.icon_style(idle, hover));

        let control = button(
            container(icon)
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill),
        )
        .width(edge)
        .height(edge)
        .padding(0)
        .on_press_maybe(value.on_press)
        .style(move |_, status| {
            button_style(&palette, tone, has_hover_background, status)
        });

        let hint = tooltip_hint(value.label, &value.palette);
        tooltip(pointer.watch(control), hint, tooltip::Position::Bottom)
            .gap(spacing::SPACE_1)
            .into()
    }
}

/// Builds the tooltip bubble shown over icon buttons.
pub(super) fn tooltip_hint<'a, Message: 'a>(
    label: String,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let style = typography::SMALL;
    let background = palette.surface_overlay();
    let color = palette.text_body();
    let shadow = palette.shadow_popover();

    container(text(label).font(fonts::UI).size(style.size()).color(color))
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

/// Returns the icon color of a button in `tone`, enabled or not, with the
/// pointer over the icon or not. A disabled icon keeps its idle color,
/// faded, and does not brighten.
fn icon_color(
    palette: &UiPalette,
    tone: IconTone,
    is_enabled: bool,
    is_hovered: bool,
) -> Color {
    let idle = match tone {
        IconTone::Default => palette.text_muted(),
        IconTone::Accent => palette.accent(),
        IconTone::Danger => palette.status_error(),
    };

    if !is_enabled {
        return faded(idle);
    }

    match (tone, is_hovered) {
        (_, false) => idle,
        (IconTone::Danger, true) => palette.text_danger(),
        _ => palette.text_strong(),
    }
}

/// Style of the button: a background while hovered or pressed, unless
/// `has_hover_background` is off: `surface-danger-hover` for the danger tone,
/// `surface-raised` otherwise.
fn button_style(
    palette: &UiPalette,
    tone: IconTone,
    has_hover_background: bool,
    status: button::Status,
) -> button::Style {
    let is_active =
        matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = (has_hover_background && is_active).then(|| match tone {
        IconTone::Danger => palette.surface_danger_hover().into(),
        _ => palette.surface_raised().into(),
    });

    button::Style {
        background,
        border: Border {
            radius: radius::RADIUS_SM.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::glyph;
    use crate::components::icon_hover::check::assert_icon_lights_up;
    use crate::test_support::TempDir;

    #[test]
    fn given_pointer_on_button_corner_then_icon_is_lit_like_over_the_icon() {
        for (name, tone, edge) in [
            ("default", IconTone::Default, IconButtonSize::Medium),
            ("accent", IconTone::Accent, IconButtonSize::Small),
            ("danger", IconTone::Danger, IconButtonSize::Medium),
        ] {
            let side = match edge {
                IconButtonSize::Medium => size::CONTROL_MD,
                IconButtonSize::Small => size::CONTROL_SM,
            };
            let view = move || {
                let palette = UiPalette::otty_dark();

                IconButton::new(glyph::plus(), "Add", &palette)
                    .tone(tone)
                    .size(edge)
                    .on_press(())
                    .into()
            };

            assert_icon_lights_up(
                &format!("icon-button-{name}"),
                view,
                iced::Rectangle::new(
                    iced::Point::ORIGIN,
                    iced::Size::new(side, side),
                ),
            );
        }
    }

    #[test]
    fn given_default_button_when_hovered_or_pressed_then_background_is_raised()
    {
        let palette = UiPalette::otty_dark();
        let raised = Some(palette.surface_raised().into());

        let style =
            |status| button_style(&palette, IconTone::Default, true, status);

        let hovered = style(button::Status::Hovered);
        let pressed = style(button::Status::Pressed);
        let idle = style(button::Status::Active);

        assert_eq!(hovered.background, raised);
        assert_eq!(pressed.background, raised);
        assert_eq!(idle.background, None);
    }

    #[test]
    fn given_button_without_hover_background_when_hovered_then_no_background_is_drawn()
     {
        let palette = UiPalette::otty_dark();

        for status in [
            button::Status::Active,
            button::Status::Hovered,
            button::Status::Pressed,
            button::Status::Disabled,
        ] {
            for tone in [IconTone::Default, IconTone::Accent, IconTone::Danger]
            {
                let style = button_style(&palette, tone, false, status);

                assert_eq!(style.background, None, "{tone:?} {status:?}");
            }
        }
    }

    #[test]
    fn given_danger_button_when_hovered_or_pressed_then_background_is_danger_hover()
     {
        let palette = UiPalette::otty_light();
        let danger = Some(palette.surface_danger_hover().into());
        let style =
            |status| button_style(&palette, IconTone::Danger, true, status);

        assert_eq!(style(button::Status::Hovered).background, danger);
        assert_eq!(style(button::Status::Pressed).background, danger);
        assert_eq!(style(button::Status::Active).background, None);
    }

    #[test]
    fn given_each_tone_then_icon_is_its_idle_color_and_brightens_on_hover() {
        let palette = UiPalette::otty_dark();

        let color =
            |tone, is_hovered| icon_color(&palette, tone, true, is_hovered);

        assert_eq!(color(IconTone::Default, false), palette.text_muted());
        assert_eq!(color(IconTone::Default, true), palette.text_strong());
        assert_eq!(color(IconTone::Accent, false), palette.accent());
        assert_eq!(color(IconTone::Accent, true), palette.text_strong());
        assert_eq!(color(IconTone::Danger, false), palette.status_error());
        assert_eq!(color(IconTone::Danger, true), palette.text_danger());
    }

    #[test]
    fn given_disabled_button_then_icon_is_faded_and_does_not_brighten() {
        let palette = UiPalette::otty_dark();

        for tone in [IconTone::Default, IconTone::Accent, IconTone::Danger] {
            let idle = icon_color(&palette, tone, true, false);

            for is_hovered in [false, true] {
                let disabled = icon_color(&palette, tone, false, is_hovered);

                assert_eq!(disabled, super::super::faded(idle), "{tone:?}");
            }
        }
    }

    /// The danger icon is a mark (3:1) on the surfaces an inspector and a
    /// table sit on, and on its own hover background; the hovered icon
    /// (`text-danger`) is text-grade (4.5:1) on that background.
    #[test]
    fn given_each_theme_when_danger_icon_drawn_then_it_reaches_3() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let idle = icon_color(&palette, IconTone::Danger, true, false);
            let hovered = icon_color(&palette, IconTone::Danger, true, true);
            let hover_background = palette.surface_danger_hover();

            for surface in [
                palette.surface_canvas(),
                palette.surface_panel(),
                palette.surface_raised(),
                hover_background,
            ] {
                let ratio = crate::contrast::contrast_ratio(idle, surface);

                assert!(ratio >= 3.0, "danger icon on {surface:?}: {ratio:.2}");
            }

            let ratio =
                crate::contrast::contrast_ratio(hovered, hover_background);
            assert!(ratio >= 4.5, "hovered danger icon: {ratio:.2}");
        }
    }

    /// The hovered background must stand out from the surfaces a danger
    /// button sits on, and by more than `surface-danger` does.
    #[test]
    fn given_each_theme_when_danger_hovered_then_background_stands_out() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let hover = palette.surface_danger_hover();

            for surface in [
                palette.surface_panel(),
                palette.surface_canvas(),
                palette.surface_raised(),
            ] {
                let ratio = crate::contrast::contrast_ratio(hover, surface);
                let old = crate::contrast::contrast_ratio(
                    palette.surface_danger(),
                    surface,
                );

                assert!(ratio >= 1.3, "hover on {surface:?}: {ratio:.2}");
                assert!(
                    ratio > old,
                    "hover {ratio:.2} vs surface-danger {old:.2}"
                );
            }
        }
    }

    /// Draws a button whose tooltip is `label`, with the pointer over it
    /// when `is_hovered`, and checks it against the picture saved in
    /// `reference`; the first call saves the picture. iced draws tooltips
    /// as overlays that widget queries do not reach, so the label is
    /// checked on the drawing.
    fn drawn_button_matches(
        label: &str,
        is_hovered: bool,
        reference: &std::path::Path,
    ) -> bool {
        let palette = UiPalette::otty_dark();
        let button =
            IconButton::new(glyph::plus(), label, &palette).on_press(());
        let mut ui = iced_test::simulator(
            container(button).padding(40).width(200).height(120),
        );
        let center = iced::Point::new(40.0 + size::CONTROL_MD / 2.0, 54.0);
        if is_hovered {
            ui.point_at(center);
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::CursorMoved { position: center },
            )]);
        }

        ui.snapshot(&iced::Theme::Dark)
            .expect("button should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn given_button_when_hovered_then_its_tooltip_text_shows() {
        let folder = TempDir::new("icon-button");
        let reference = folder.path().join("tooltip");

        assert!(drawn_button_matches("Prune unused", true, &reference));
        assert!(!drawn_button_matches("Remove", true, &reference));
        assert!(!drawn_button_matches("Prune unused", false, &reference));
        assert!(drawn_button_matches("Prune unused", true, &reference));
    }

    #[test]
    fn given_button_with_message_when_pressed_then_it_is_sent_and_without_none()
    {
        let palette = UiPalette::otty_dark();
        let center =
            iced::Point::new(size::CONTROL_MD / 2.0, size::CONTROL_MD / 2.0);

        for (message, expected) in [(Some(7), vec![7]), (None, vec![])] {
            let button = IconButton::new(glyph::plus(), "Remove", &palette)
                .tone(IconTone::Danger)
                .on_press_maybe(message);
            let mut ui = iced_test::simulator(button);

            ui.point_at(center);
            let _ = ui.simulate(iced_test::simulator::click());

            let messages: Vec<i32> = ui.into_messages().collect();
            assert_eq!(messages, expected);
        }
    }

    #[test]
    fn given_new_button_then_it_has_a_hover_background_until_turned_off() {
        let palette = UiPalette::otty_dark();

        let button = IconButton::<()>::new(glyph::cross(), "Close", &palette);
        let plain = IconButton::<()>::new(glyph::cross(), "Close", &palette)
            .hover_background(false);

        assert!(button.has_hover_background);
        assert!(!plain.has_hover_background);
    }

    #[test]
    fn given_common_display_scales_when_icon_drawn_anywhere_then_no_row_is_lost()
     {
        // iced rasterizes an svg to `ceil(edge * scale)` pixels and draws it
        // with nearest sampling in a quad whose top and bottom edges are
        // each rounded to the pixel grid. When the rounded quad is a row
        // shorter or taller than the texture, a row of the icon is dropped
        // or doubled. A header moves down by whole pixels as rows are added
        // above it, so every whole-pixel position of the button must keep
        // all rows.
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let texture_rows = (ICON_SIZE * scale).ceil();

            for button_top in 0..240 {
                for edge in [size::CONTROL_SM, size::CONTROL_MD] {
                    let top = button_top as f32 + (edge - ICON_SIZE) / 2.0;

                    let rows = drawn_rows(top, scale);

                    assert_eq!(
                        rows, texture_rows,
                        "scale {scale}, button top {button_top}, edge {edge}"
                    );
                }
            }
        }
    }

    /// Physical rows the wgpu image shader covers for an icon whose top is
    /// at `top` logical pixels: it rounds each edge on its own, ties to
    /// even (WGSL `round`).
    fn drawn_rows(top: f32, scale: f32) -> f32 {
        let bottom = top + ICON_SIZE;

        (bottom * scale).round_ties_even() - (top * scale).round_ties_even()
    }
}
