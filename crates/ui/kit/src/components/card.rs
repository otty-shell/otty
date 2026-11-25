//! `Card`: a part of a page on a raised surface with a thin border, its
//! title inside.

use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Border, Color, Element, Length, Padding};

use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing, typography};

/// Space between the edge of a card and what it holds
/// (`padding: 14px 16px 16px`, on the kit spacing).
const PADDING: Padding = Padding {
    top: spacing::SPACE_4,
    right: spacing::SPACE_5,
    bottom: spacing::SPACE_5,
    left: spacing::SPACE_5,
};
/// Gap between the title row and the body (`gap: 14px`).
const HEAD_GAP: f32 = spacing::SPACE_4;
/// Gap between the title and the muted line under it.
const LINE_GAP: f32 = spacing::SPACE_1;
/// Gap between the title and the trailing element.
const TRAILING_GAP: f32 = spacing::SPACE_4;
/// Edge of the dot before the title of a warning card (`width: 7px`).
const DOT_SIZE: f32 = 7.0;
/// Gap between the dot and the title (`gap: 8px`).
const DOT_GAP: f32 = spacing::SPACE_3;
/// How far the surface of a card goes from `surface-canvas` toward
/// `surface-overlay`: in OTTY Dark that is about `surface-header`, in OTTY
/// Light nearly white.
const RAISE: f32 = 0.4;
/// How much of `text-muted` the border and the rules of a card hold
/// (`color-mix(in srgb, var(--text-muted) 24%, transparent)`).
const BORDER_ALPHA: f32 = 0.24;
/// How much of `status-warn` the tint of a warning card holds
/// (`color-mix(in srgb, var(--status-warn) 8%, ...)`).
const WARN_WASH: f32 = 0.08;
/// How much of `status-warn` the border of a warning card holds.
const WARN_BORDER_ALPHA: f32 = 0.45;
/// How much of `status-warn` the rules inside a warning card hold.
const WARN_RULE_ALPHA: f32 = 0.28;

/// What a [`Card`] tells by its look.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CardTone {
    /// A part of a page: the raised surface with a thin muted border.
    #[default]
    Default,
    /// Something the user has to do, such as "Needs from you": tinted in
    /// `status-warn`, with a warning border and a dot before the title.
    Warning,
}

/// A part of a page: the title at the top inside, with an optional muted
/// line under it and an optional element at the right end of its row,
/// such as a period picker or a button, then the body. Stack cards in a
/// column `spacing::SPACE_5` apart. Split rows inside the body with
/// [`rule`].
///
/// It fills the width it gets.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::text;
/// use otty_ui_kit::components::card::{Card, CardTone};
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let money: Element<'_, ()> = Card::new(text("$42.10"), &palette)
///     .title("Money")
///     .line(text("Updated 2 min ago"))
///     .into();
/// let needs: Element<'_, ()> = Card::new(text("Log in"), &palette)
///     .title("Needs from you")
///     .tone(CardTone::Warning)
///     .into();
/// ```
pub struct Card<'a, Message> {
    body: Element<'a, Message>,
    palette: UiPalette,
    title: Option<String>,
    line: Option<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    tone: CardTone,
}

impl<'a, Message> Card<'a, Message> {
    /// Creates a card in the default tone that holds `body`, with no
    /// title.
    pub fn new(
        body: impl Into<Element<'a, Message>>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            body: body.into(),
            palette: palette.clone(),
            title: None,
            line: None,
            trailing: None,
            tone: CardTone::Default,
        }
    }

    /// Sets the title at the top of the card.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets a line under the title, usually muted small text such as when
    /// the numbers were read.
    pub fn line(mut self, line: impl Into<Element<'a, Message>>) -> Self {
        self.line = Some(line.into());
        self
    }

    /// Sets an element at the right end of the title row, such as a
    /// period picker or a button.
    pub fn trailing(
        mut self,
        trailing: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// Sets what the card tells by its look.
    pub fn tone(mut self, tone: CardTone) -> Self {
        self.tone = tone;
        self
    }
}

impl<'a, Message: 'a> From<Card<'a, Message>> for Element<'a, Message> {
    fn from(value: Card<'a, Message>) -> Self {
        let palette = value.palette;
        let tone = value.tone;
        let has_head = value.title.is_some()
            || value.line.is_some()
            || value.trailing.is_some();

        let mut content = column![].spacing(HEAD_GAP).width(Length::Fill);
        if has_head {
            content = content.push(head(
                value.title,
                value.line,
                value.trailing,
                tone,
                &palette,
            ));
        }
        let content = content.push(value.body);

        let style = style(tone, &palette);
        container(content)
            .width(Length::Fill)
            .padding(PADDING)
            .style(move |_| style)
            .into()
    }
}

/// Returns a thin rule to split the rows inside the body of a card of
/// `tone`, as wide as the body: muted in a default card, tinted in a
/// warning card.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::{column, text};
/// use otty_ui_kit::components::card::{Card, CardTone, rule};
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let rows = column![
///     text("5 hours"),
///     rule(CardTone::Default, &palette),
///     text("Weekly"),
/// ];
/// let card: Element<'_, ()> =
///     Card::new(rows, &palette).title("Limits").into();
/// ```
pub fn rule<'a, Message: 'a>(
    tone: CardTone,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let color = rule_color(tone, palette);

    container(Space::new())
        .width(Length::Fill)
        .height(size::LINE)
        .style(move |_| container::Style {
            background: Some(color.into()),
            ..container::Style::default()
        })
        .into()
}

/// The title row: the title, the dot before it in a warning card, the
/// line under it, and the trailing element at the right end.
fn head<'a, Message: 'a>(
    title: Option<String>,
    line: Option<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    tone: CardTone,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let heading = typography::HEADING;

    let mut lines = column![].spacing(LINE_GAP);
    if let Some(title) = title {
        let title = text(title)
            .size(heading.size())
            .line_height(iced::Pixels(heading.line_height()))
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong());
        lines = match tone {
            CardTone::Warning => lines.push(
                row![dot(palette), title]
                    .spacing(DOT_GAP)
                    .align_y(Alignment::Center),
            ),
            _ => lines.push(title),
        };
    }
    if let Some(line) = line {
        lines = lines.push(line);
    }

    // The trailing element sits at the top, beside the title, even when a
    // line under the title makes the head taller (`align-items: start`).
    let mut head = row![container(lines).width(Length::Fill)]
        .spacing(TRAILING_GAP)
        .align_y(Alignment::Start);
    if let Some(trailing) = trailing {
        head = head.push(trailing);
    }

    head.into()
}

/// The round `status-warn` dot before the title of a warning card.
fn dot<'a, Message: 'a>(palette: &UiPalette) -> Element<'a, Message> {
    let color = palette.status_warn();

    container(Space::new())
        .width(DOT_SIZE)
        .height(DOT_SIZE)
        .style(move |_| container::Style {
            background: Some(color.into()),
            border: Border {
                radius: (DOT_SIZE / 2.0).into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// The look of a card of `tone`: its surface, its border and its text.
fn style(tone: CardTone, palette: &UiPalette) -> container::Style {
    let (background, border) = match tone {
        CardTone::Warning => (
            palette.status_warn().scale_alpha(WARN_WASH),
            palette.status_warn().scale_alpha(WARN_BORDER_ALPHA),
        ),
        _ => (
            raised(palette),
            palette.text_muted().scale_alpha(BORDER_ALPHA),
        ),
    };

    container::Style {
        background: Some(background.into()),
        text_color: Some(palette.text_body()),
        border: Border {
            color: border,
            width: size::LINE,
            radius: radius::RADIUS_LG.into(),
        },
        ..container::Style::default()
    }
}

/// The color of the rules inside a card of `tone`.
fn rule_color(tone: CardTone, palette: &UiPalette) -> Color {
    match tone {
        CardTone::Warning => palette.status_warn().scale_alpha(WARN_RULE_ALPHA),
        _ => palette.text_muted().scale_alpha(BORDER_ALPHA),
    }
}

/// The surface of a default card: `surface-canvas` raised [`RAISE`] of
/// the way toward `surface-overlay`.
pub(crate) fn raised(palette: &UiPalette) -> Color {
    let canvas = palette.surface_canvas();
    let overlay = palette.surface_overlay();
    let mix = |from: f32, to: f32| from + (to - from) * RAISE;

    Color::from_rgb(
        mix(canvas.r, overlay.r),
        mix(canvas.g, overlay.g),
        mix(canvas.b, overlay.b),
    )
}

#[cfg(test)]
mod tests {
    use iced::widget::{column, container, text};
    use iced::{Background, Color, Element, Length, Rectangle};
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::palette::UiPalette;
    use crate::{radius, size};

    /// Width of the card in the tests.
    const WIDTH: f32 = 400.0;

    fn drawn(card: Card<'static, ()>) -> Simulator<'static, ()> {
        let page: Element<'static, ()> =
            column![container(card).width(WIDTH), text("after")].into();

        simulator(page)
    }

    fn at(ui: &mut Simulator<'_, ()>, label: &str) -> Rectangle {
        ui.find(label).expect("the text should show").bounds()
    }

    fn dark() -> UiPalette {
        UiPalette::otty_dark()
    }

    #[test]
    fn the_title_and_the_body_lie_inside_the_padding() {
        let palette = dark();
        let mut ui = drawn(Card::new(text("body"), &palette).title("Money"));

        let title = at(&mut ui, "Money");
        let body = at(&mut ui, "body");
        let after = at(&mut ui, "after");

        assert_eq!(title.x, PADDING.left);
        assert_eq!(title.y, PADDING.top);
        assert_eq!(body.x, PADDING.left);
        assert!(
            body.y >= title.y + title.height + HEAD_GAP - 0.5,
            "{body:?} under {title:?}"
        );
        assert!(
            (after.y - (body.y + body.height + PADDING.bottom)).abs() < 0.5,
            "the card ends {} under its body: {after:?}",
            PADDING.bottom
        );
    }

    #[test]
    fn the_body_takes_the_width_inside_the_padding() {
        let palette = dark();
        let body = text("whole").width(Length::Fill);
        let mut ui = drawn(Card::new(body, &palette).title("Limits"));

        let whole = at(&mut ui, "whole");

        assert_eq!(whole.x, PADDING.left);
        assert_eq!(whole.width, WIDTH - PADDING.left - PADDING.right);
    }

    #[test]
    fn the_trailing_element_ends_the_title_row() {
        let palette = dark();
        let card = Card::new(text("body"), &palette)
            .title("Usage")
            .trailing(text("7 days"));
        let mut ui = drawn(card);

        let title = at(&mut ui, "Usage");
        let trailing = at(&mut ui, "7 days");
        let body = at(&mut ui, "body");

        assert!(
            (trailing.x + trailing.width - (WIDTH - PADDING.right)).abs() < 0.5,
            "{trailing:?} ends the row"
        );
        assert!((trailing.center_y() - title.center_y()).abs() < 1.0);
        assert!(body.y >= trailing.y + trailing.height, "{body:?}");
    }

    #[test]
    fn the_line_shows_under_the_title() {
        let palette = dark();
        let card = Card::new(text("body"), &palette)
            .title("Money")
            .line(text("Updated 2 min ago"));
        let mut ui = drawn(card);

        let title = at(&mut ui, "Money");
        let line = at(&mut ui, "Updated 2 min ago");
        let body = at(&mut ui, "body");

        assert_eq!(line.x, title.x);
        assert!(line.y >= title.y + title.height, "{line:?} under {title:?}");
        assert!(body.y >= line.y + line.height + HEAD_GAP - 0.5, "{body:?}");
    }

    #[test]
    fn a_card_without_a_title_starts_with_its_body() {
        let palette = dark();
        let mut ui = drawn(Card::new(text("body"), &palette));

        let body = at(&mut ui, "body");

        assert_eq!(body.x, PADDING.left);
        assert_eq!(body.y, PADDING.top);
    }

    /// Owner, issue 24 of spec 2026-11-23: "Needs from you" is a warning
    /// card, with a dot before its title.
    #[test]
    fn a_warning_card_has_a_dot_before_its_title() {
        let palette = dark();
        let card = Card::new(text("body"), &palette)
            .title("Needs from you")
            .tone(CardTone::Warning);
        let mut ui = drawn(card);

        let title = at(&mut ui, "Needs from you");
        let body = at(&mut ui, "body");

        assert_eq!(title.x, PADDING.left + DOT_SIZE + DOT_GAP);
        assert_eq!(body.x, PADDING.left, "the body is not moved by the dot");
    }

    /// Whether every channel of `color` lies between those of `a` and `b`.
    fn between(color: Color, a: Color, b: Color) -> bool {
        let inside = |x: f32, a: f32, b: f32| x >= a.min(b) && x <= a.max(b);

        inside(color.r, a.r, b.r)
            && inside(color.g, a.g, b.g)
            && inside(color.b, a.b, b.b)
    }

    #[test]
    fn a_default_card_is_raised_over_the_canvas_with_a_thin_muted_border() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let style = style(CardTone::Default, &palette);

            let Some(Background::Color(background)) = style.background else {
                panic!("a card has a plain background: {style:?}");
            };
            assert!(
                between(
                    background,
                    palette.surface_canvas(),
                    palette.surface_overlay()
                ),
                "raised toward the overlay: {background:?}"
            );
            assert_ne!(background, palette.surface_canvas());
            assert_eq!(background.a, 1.0, "the card hides what is under it");
            assert_eq!(style.border.width, size::LINE);
            assert_eq!(style.border.radius, radius::RADIUS_LG.into());
            assert_eq!(
                style.border.color,
                palette.text_muted().scale_alpha(BORDER_ALPHA)
            );
            assert_eq!(style.text_color, Some(palette.text_body()));
        }
    }

    #[test]
    fn a_warning_card_is_tinted_with_a_warning_border() {
        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            let style = style(CardTone::Warning, &palette);

            assert_eq!(
                style.background,
                Some(Background::Color(
                    palette.status_warn().scale_alpha(WARN_WASH)
                ))
            );
            assert_eq!(style.border.width, size::LINE);
            assert_eq!(style.border.radius, radius::RADIUS_LG.into());
            assert_eq!(
                style.border.color,
                palette.status_warn().scale_alpha(WARN_BORDER_ALPHA)
            );
        }
    }

    #[test]
    fn the_rule_between_rows_takes_the_tone_of_its_card() {
        let palette = dark();

        assert_eq!(
            rule_color(CardTone::Default, &palette),
            palette.text_muted().scale_alpha(BORDER_ALPHA)
        );
        assert_eq!(
            rule_color(CardTone::Warning, &palette),
            palette.status_warn().scale_alpha(WARN_RULE_ALPHA)
        );
    }
}
