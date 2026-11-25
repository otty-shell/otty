//! `Notice`: an error, or the result of a finished action, with a dismiss
//! button, inline or floating over content.

use iced::widget::{button, container, row, svg, text};
use iced::{Alignment, Border, Color, Element, Length, Padding, Shadow};

use super::glyph;
use super::icon_hover::IconHover;
use super::tooltip::tooltip;
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing};

/// Gap between icon, text and dismiss button (`.ot-banner { gap: 10px }`).
const GAP: f32 = 10.0;
/// Padding of the notice (`.ot-banner { padding: 9px 12px }`).
const PADDING: Padding = Padding {
    top: 9.0,
    right: spacing::SPACE_4,
    bottom: 9.0,
    left: spacing::SPACE_4,
};
/// Text size (`.ot-banner { font-size: 12.5px }`).
const TEXT_SIZE: f32 = 12.5;
/// How much of `status-warn` the wash under a warning holds
/// (`color-mix(in srgb, var(--status-warn) 12%, transparent)`).
const WARN_WASH: f32 = 0.12;
/// Edge of the leading icon. The design says 15px; 16 keeps the icon on
/// whole pixels at every scale, as in [`IconButton`].
///
/// [`IconButton`]: super::icon_button::IconButton
const ICON_SIZE: f32 = 16.0;
/// Edge of the dismiss button (`.clear-x { width: 16px; height: 16px }`).
const DISMISS_SIZE: f32 = 16.0;
/// Edge of the cross in the dismiss button. The design says 11px; 12 is an
/// even number of pixels and keeps the cross sharp.
const CROSS_SIZE: f32 = 12.0;

/// A message with its icon, the text and a dismiss cross. Long text wraps.
/// It sits inline in a column, or floats over content with
/// [`Notice::floating`], as a toast does.
///
/// It is an error on `surface-danger` by default, what a failed action
/// shows; [`NoticeTone::Success`] makes it the neutral notice of a finished
/// action, [`NoticeTone::Warn`] a warning. Components never bundle icons,
/// so the caller passes the icon of the tone: the error mark, the warning
/// sign or the check.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::notice::{Notice, NoticeTone};
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Dismiss,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let notice: Element<'_, Message> =
///     Notice::new("Container is already stopped.", icon.clone(), &palette)
///         .on_dismiss(Message::Dismiss, "Dismiss")
///         .into();
/// let toast: Element<'_, Message> =
///     Notice::new("Could not save the pinned modes.", icon.clone(), &palette)
///         .floating()
///         .on_dismiss(Message::Dismiss, "Dismiss")
///         .into();
/// let done: Element<'_, Message> =
///     Notice::new("Plugin OpenCode reloaded", icon, &palette)
///         .tone(NoticeTone::Success)
///         .floating()
///         .into();
/// ```
pub struct Notice<Message> {
    text: String,
    icon: svg::Handle,
    palette: UiPalette,
    tone: NoticeTone,
    is_floating: bool,
    on_dismiss: Option<(Message, String)>,
}

impl<Message> Notice<Message> {
    /// Creates an error notice with `text` and the error `icon`, without a
    /// dismiss button.
    pub fn new(
        text: impl Into<String>,
        icon: svg::Handle,
        palette: &UiPalette,
    ) -> Self {
        Self {
            text: text.into(),
            icon,
            palette: palette.clone(),
            tone: NoticeTone::Error,
            is_floating: false,
            on_dismiss: None,
        }
    }

    /// Sets what the notice tells: an error (the default), a warning or
    /// that an action finished.
    pub fn tone(mut self, tone: NoticeTone) -> Self {
        self.tone = tone;
        self
    }

    /// Floats the notice over content, such as a toast at the top of a
    /// window: it casts `shadow-popover`, so it stands out from what lies
    /// under it.
    pub fn floating(mut self) -> Self {
        self.is_floating = true;
        self
    }

    /// Shows a dismiss cross that sends `message`. `label` is the cross's
    /// tooltip, such as "Dismiss", in the app's language.
    pub fn on_dismiss(
        mut self,
        message: Message,
        label: impl Into<String>,
    ) -> Self {
        self.on_dismiss = Some((message, label.into()));
        self
    }
}

impl<'a, Message: Clone + 'a> From<Notice<Message>> for Element<'a, Message> {
    fn from(value: Notice<Message>) -> Self {
        let palette = value.palette;
        let tone = value.tone;
        let is_floating = value.is_floating;
        let color = tone.text_color(&palette);
        let icon_color = tone.icon_color(&palette);

        let mut content = row![
            svg(value.icon).width(ICON_SIZE).height(ICON_SIZE).style(
                move |_, _| svg::Style {
                    color: Some(icon_color)
                }
            ),
            text(value.text)
                .size(TEXT_SIZE)
                .font(fonts::UI)
                .color(color)
                .width(Length::Fill),
        ]
        .spacing(GAP)
        .align_y(Alignment::Center);
        if let Some((message, label)) = value.on_dismiss {
            content =
                content.push(dismiss_button(message, label, tone, &palette));
        }

        container(content)
            .width(Length::Fill)
            .padding(PADDING)
            .style(move |_| notice_style(&palette, tone, is_floating))
            .into()
    }
}

/// What a [`Notice`] tells: an error, a warning, or that an action
/// finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeTone {
    /// A failed action: `text-danger` on `surface-danger`, the icon in
    /// `status-error`.
    Error,
    /// Something to weigh before going on, such as a plugin reviewed by
    /// its author only: `status-warn` text and icon on a faint wash of
    /// `status-warn`.
    Warn,
    /// A finished action, such as "Plugin OpenCode reloaded": neutral,
    /// `text-strong` on `surface-overlay` with a `line` border, the icon
    /// (a check) in `status-ok`.
    Success,
}

impl NoticeTone {
    /// Returns the color of the text in `palette`.
    pub const fn text_color(self, palette: &UiPalette) -> Color {
        match self {
            Self::Error => palette.text_danger(),
            Self::Warn => palette.status_warn(),
            Self::Success => palette.text_strong(),
        }
    }

    /// Returns the color of the icon in `palette`.
    pub const fn icon_color(self, palette: &UiPalette) -> Color {
        match self {
            Self::Error => palette.status_error(),
            Self::Warn => palette.status_warn(),
            Self::Success => palette.status_ok(),
        }
    }

    /// Returns the background of the notice in `palette`.
    const fn background(self, palette: &UiPalette) -> Color {
        match self {
            Self::Error => palette.surface_danger(),
            Self::Warn => Color {
                a: WARN_WASH,
                ..palette.status_warn()
            },
            Self::Success => palette.surface_overlay(),
        }
    }

    /// Returns the color of the dismiss cross at rest in `palette`.
    const fn cross_color(self, palette: &UiPalette) -> Color {
        match self {
            Self::Error => palette.text_danger(),
            Self::Warn => palette.status_warn(),
            Self::Success => palette.text_muted(),
        }
    }
}

/// Style of the notice: the background of `tone` with rounded corners, a
/// `line` border for a success, and `shadow-popover` while it floats over
/// content.
fn notice_style(
    palette: &UiPalette,
    tone: NoticeTone,
    is_floating: bool,
) -> container::Style {
    let shadow = if is_floating {
        palette.shadow_popover()
    } else {
        Shadow::default()
    };
    let (border_width, border_color) = match tone {
        NoticeTone::Success => (size::LINE, palette.line()),
        NoticeTone::Error | NoticeTone::Warn => (0.0, Color::TRANSPARENT),
    };

    container::Style {
        background: Some(tone.background(palette).into()),
        border: Border {
            radius: radius::RADIUS_MD.into(),
            width: border_width,
            color: border_color,
        },
        shadow,
        ..container::Style::default()
    }
}

/// Draws the small dismiss cross: `text-danger` on an error, `text-muted`
/// on a success, turning `text-strong` on a `surface-hover` square while
/// hovered (`.clear-x`).
fn dismiss_button<'a, Message: Clone + 'a>(
    message: Message,
    label: String,
    tone: NoticeTone,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let idle = tone.cross_color(palette);
    let hover = palette.text_strong();
    let hover_background = palette.surface_hover();

    let pointer = IconHover::new();
    let cross = svg(glyph::cross())
        .width(CROSS_SIZE)
        .height(CROSS_SIZE)
        .style(pointer.icon_style(idle, hover));

    let control = button(container(cross).center(Length::Fill))
        .width(DISMISS_SIZE)
        .height(DISMISS_SIZE)
        .padding(0)
        .on_press(message)
        .style(move |_, status| {
            let is_active = matches!(
                status,
                button::Status::Hovered | button::Status::Pressed
            );

            button::Style {
                background: is_active.then(|| hover_background.into()),
                border: Border {
                    radius: radius::RADIUS_XS.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
        });

    tooltip(pointer.watch(control), label, palette)
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced_test::simulator;

    use super::*;
    use crate::contrast::contrast_ratio;
    use crate::testing::pictures::assert_icon_lights_up;

    #[test]
    fn given_pointer_on_dismiss_corner_then_cross_is_lit_like_over_the_icon() {
        let view = || {
            dismiss_button(
                (),
                "Dismiss".to_owned(),
                NoticeTone::Error,
                &UiPalette::otty_dark(),
            )
        };

        assert_icon_lights_up(
            "notice-dismiss",
            view,
            iced::Rectangle::new(
                Point::ORIGIN,
                iced::Size::new(DISMISS_SIZE, DISMISS_SIZE),
            ),
        );
    }

    /// WCAG 2.1 AA minimum for normal text.
    const TEXT_MIN: f32 = 4.5;
    /// WCAG 2.1 AA minimum for non-text marks.
    const NON_TEXT_MIN: f32 = 3.0;
    /// Width of the notice in the tests.
    const WIDTH: f32 = 400.0;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Dismiss,
    }

    fn notice_view(text: &str) -> Element<'_, Message> {
        let palette = UiPalette::otty_dark();

        container(
            Notice::new(text, glyph::plus(), &palette)
                .on_dismiss(Message::Dismiss, "Dismiss"),
        )
        .width(WIDTH)
        .into()
    }

    #[test]
    fn given_notice_when_dismiss_pressed_then_dismiss_is_sent() {
        let mut ui = simulator(notice_view("Port 8080 is already in use."));

        // The cross sits in the last 16px before the 12px right padding,
        // centered in a one-line notice: 9 + 16 / 2 from the top.
        ui.point_at(Point::new(WIDTH - 12.0 - 8.0, 9.0 + 8.0));
        let _ = ui.simulate(iced_test::simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Dismiss]);
    }

    #[test]
    fn given_notice_when_text_pressed_then_nothing_is_sent() {
        let mut ui = simulator(notice_view("Port 8080 is already in use."));

        ui.click("Port 8080 is already in use.")
            .expect("text should be visible");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn given_new_notice_then_it_does_not_float_until_asked() {
        let palette = UiPalette::otty_dark();

        let inline = Notice::<Message>::new("Busy", glyph::plus(), &palette);
        let floating =
            Notice::<Message>::new("Busy", glyph::plus(), &palette).floating();

        assert!(!inline.is_floating);
        assert!(floating.is_floating);
    }

    /// A notice that floats over content, such as a toast, casts the
    /// popover shadow so it stands out from what lies under it. An inline
    /// notice sits flat in its column.
    #[test]
    fn given_floating_notice_then_it_casts_the_popover_shadow() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let floating = notice_style(&palette, NoticeTone::Error, true);
            let inline = notice_style(&palette, NoticeTone::Error, false);

            assert_eq!(floating.shadow, palette.shadow_popover());
            assert_eq!(inline.shadow, Shadow::default());
            assert_eq!(
                floating.background,
                Some(palette.surface_danger().into())
            );
            assert_eq!(floating.border, inline.border);
        }
    }

    /// Issue 35: a finished action is told by a neutral notice with a
    /// green check, not by the red one of an error.
    #[test]
    fn given_success_notice_then_it_is_neutral_with_a_border() {
        let palette = UiPalette::otty_dark();

        let success = notice_style(&palette, NoticeTone::Success, true);
        let error = notice_style(&palette, NoticeTone::Error, true);

        assert_eq!(success.background, Some(palette.surface_overlay().into()));
        assert_eq!(success.border.width, size::LINE);
        assert_eq!(success.border.color, palette.line());
        assert_eq!(success.shadow, palette.shadow_popover());
        assert_eq!(error.border.width, 0.0);
        assert_eq!(
            NoticeTone::Success.icon_color(&palette),
            palette.status_ok()
        );
        assert_eq!(
            NoticeTone::Success.text_color(&palette),
            palette.text_strong()
        );
    }

    #[test]
    fn given_new_notice_then_it_is_an_error_until_told_otherwise() {
        let palette = UiPalette::otty_dark();

        let error = Notice::<Message>::new("Busy", glyph::plus(), &palette);
        let success = Notice::<Message>::new("Done", glyph::plus(), &palette)
            .tone(NoticeTone::Success);

        assert_eq!(error.tone, NoticeTone::Error);
        assert_eq!(success.tone, NoticeTone::Success);
    }

    #[test]
    fn given_each_theme_when_success_notice_drawn_then_it_is_readable() {
        for (theme, palette) in [
            ("OTTY Dark", UiPalette::otty_dark()),
            ("OTTY Light", UiPalette::otty_light()),
        ] {
            let background = palette.surface_overlay();
            let text = contrast_ratio(palette.text_strong(), background);
            let icon = contrast_ratio(palette.status_ok(), background);

            assert!(text >= TEXT_MIN, "{theme}: text is {text:.2}:1");
            assert!(icon >= NON_TEXT_MIN, "{theme}: icon is {icon:.2}:1");
        }
    }

    /// Owner, 2026-11-22: a warning, such as the note of a community
    /// plugin in its install dialog, is drawn in the warn tone on a faint
    /// warn wash, readable over the dialog card in both themes.
    #[test]
    fn given_warn_notice_then_it_is_warn_on_a_warn_wash_and_readable() {
        for (theme, palette) in [
            ("OTTY Dark", UiPalette::otty_dark()),
            ("OTTY Light", UiPalette::otty_light()),
        ] {
            let style = notice_style(&palette, NoticeTone::Warn, false);
            let wash = Color {
                a: WARN_WASH,
                ..palette.status_warn()
            };
            let card = palette.surface_overlay();
            let shown = Color {
                r: card.r + (wash.r - card.r) * WARN_WASH,
                g: card.g + (wash.g - card.g) * WARN_WASH,
                b: card.b + (wash.b - card.b) * WARN_WASH,
                a: 1.0,
            };

            let text =
                contrast_ratio(NoticeTone::Warn.text_color(&palette), shown);
            let icon =
                contrast_ratio(NoticeTone::Warn.icon_color(&palette), shown);

            assert_eq!(style.background, Some(wash.into()));
            assert_eq!(style.border.width, 0.0);
            assert_eq!(
                NoticeTone::Warn.text_color(&palette),
                palette.status_warn()
            );
            assert!(text >= TEXT_MIN, "{theme}: text is {text:.2}:1");
            assert!(icon >= NON_TEXT_MIN, "{theme}: icon is {icon:.2}:1");
        }
    }

    #[test]
    fn given_each_theme_when_notice_drawn_then_text_and_icon_are_readable() {
        for (theme, palette) in [
            ("OTTY Dark", UiPalette::otty_dark()),
            ("OTTY Light", UiPalette::otty_light()),
        ] {
            let text =
                contrast_ratio(palette.text_danger(), palette.surface_danger());
            let icon = contrast_ratio(
                palette.status_error(),
                palette.surface_danger(),
            );

            assert!(text >= TEXT_MIN, "{theme}: text is {text:.2}:1");
            assert!(icon >= NON_TEXT_MIN, "{theme}: icon is {icon:.2}:1");
        }
    }
}
