//! One line of text that ends with "…" when it is wider than its space.
//!
//! `iced` 0.14 text has no ellipsis: a long value is just cut at the edge.
//! [`DataTable`] cells must show that a value goes on, so they draw their
//! text with [`EllipsisText`], which measures the text and drops characters
//! from the end until the rest and "…" fit.
//!
//! [`DataTable`]: super::data_table::DataTable

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::text::{self as core_text, Paragraph as _, paragraph};
use iced::advanced::widget::{
    Operation, Tree, Widget, text as text_widget, tree,
};
use iced::alignment::Vertical;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::{
    Color, Element, Font, Length, Pixels, Rectangle, Renderer, Size, Theme,
    mouse,
};

/// Mark added at the end of a shortened text.
const ELLIPSIS: &str = "…";

/// Paragraph type of the renderer, used to measure and draw the text.
type Paragraph = <Renderer as core_text::Renderer>::Paragraph;

/// Single-line text that fills its width and ends with "…" when it is too
/// long. Widget queries such as `iced_test` selectors see the full text.
pub(super) struct EllipsisText {
    content: String,
    size: f32,
    font: Font,
    color: Color,
    align_x: core_text::Alignment,
}

impl EllipsisText {
    /// Creates left-aligned text.
    pub(super) fn new(
        content: impl Into<String>,
        size: f32,
        font: Font,
        color: Color,
    ) -> Self {
        Self {
            content: content.into(),
            size,
            font,
            color,
            align_x: core_text::Alignment::Left,
        }
    }

    /// Sets the horizontal alignment inside the available width.
    pub(super) fn align_x(mut self, align_x: core_text::Alignment) -> Self {
        self.align_x = align_x;
        self
    }

    /// Returns the width of `candidate` on one line in this text's font.
    fn measure(&self, candidate: &str) -> f32 {
        let paragraph = Paragraph::with_text(core_text::Text {
            content: candidate,
            bounds: Size::new(f32::INFINITY, f32::INFINITY),
            size: Pixels(self.size),
            line_height: LineHeight::default(),
            font: self.font,
            align_x: core_text::Alignment::Default,
            align_y: Vertical::Top,
            shaping: Shaping::default(),
            wrapping: Wrapping::None,
        });

        paragraph.min_bounds().width
    }

    /// Returns the layout format of the shown text.
    fn format(&self) -> text_widget::Format<Font> {
        text_widget::Format {
            width: Length::Fill,
            height: Length::Shrink,
            size: Some(Pixels(self.size)),
            font: Some(self.font),
            line_height: LineHeight::default(),
            align_x: self.align_x,
            align_y: Vertical::Top,
            shaping: Shaping::default(),
            wrapping: Wrapping::None,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for EllipsisText {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Shrink,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let max_width = limits.max().width;

        if !state.is_fitted_for(self, max_width) {
            state.shown = fit(&self.content, max_width, |candidate| {
                self.measure(candidate)
            });
            state.fitted_for =
                Some((self.content.clone(), max_width, self.size, self.font));
        }

        text_widget::layout(
            &mut state.paragraph,
            renderer,
            limits,
            &state.shown,
            self.format(),
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();

        text_widget::draw(
            renderer,
            style,
            layout.bounds(),
            state.paragraph.raw(),
            text_widget::Style {
                color: Some(self.color),
            },
            viewport,
        );
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.text(None, layout.bounds(), &self.content);
    }
}

impl<'a, Message: 'a> From<EllipsisText> for Element<'a, Message> {
    fn from(value: EllipsisText) -> Self {
        Element::new(value)
    }
}

/// Layout state of an [`EllipsisText`]: the shown text and what it was
/// fitted for, so an unchanged cell is not measured again on every layout.
#[derive(Default)]
struct State {
    paragraph: paragraph::Plain<Paragraph>,
    shown: String,
    fitted_for: Option<(String, f32, f32, Font)>,
}

impl State {
    /// Returns whether the shown text was fitted for this content, width,
    /// size and font.
    fn is_fitted_for(&self, text: &EllipsisText, max_width: f32) -> bool {
        self.fitted_for
            .as_ref()
            .is_some_and(|(content, width, size, font)| {
                content == &text.content
                    && *width == max_width
                    && *size == text.size
                    && *font == text.font
            })
    }
}

/// Returns `content` when `measure` says it fits in `max_width`. Otherwise
/// returns the longest start of `content`, without trailing spaces,
/// followed by "…" that fits, or "…" alone when no character fits.
fn fit(
    content: &str,
    max_width: f32,
    mut measure: impl FnMut(&str) -> f32,
) -> String {
    if measure(content) <= max_width {
        return content.to_owned();
    }

    // Byte offsets where a shorter start of `content` can end: after its
    // first character, after the second, and so on up to the last one.
    let cuts: Vec<usize> = content
        .char_indices()
        .skip(1)
        .map(|(index, _)| index)
        .collect();
    let shortened =
        |end: usize| format!("{}{ELLIPSIS}", content[..end].trim_end());

    // `kept` is the largest number of cuts known to fit; `limit` the
    // largest not yet ruled out.
    let mut kept = 0;
    let mut limit = cuts.len();
    while kept < limit {
        let middle = (kept + limit).div_ceil(2);
        if measure(&shortened(cuts[middle - 1])) <= max_width {
            kept = middle;
        } else {
            limit = middle - 1;
        }
    }

    match kept {
        0 => ELLIPSIS.to_owned(),
        _ => shortened(cuts[kept - 1]),
    }
}

#[cfg(test)]
mod tests {
    use super::fit;

    /// Measures one unit per character, so widths read as lengths.
    fn chars(text: &str) -> f32 {
        text.chars().count() as f32
    }

    #[test]
    fn given_text_that_fits_when_fitted_then_it_is_unchanged() {
        assert_eq!(fit("redis", 5.0, chars), "redis");
        assert_eq!(fit("redis", 40.0, chars), "redis");
        assert_eq!(fit("", 0.0, chars), "");
    }

    #[test]
    fn given_long_text_when_fitted_then_it_ends_with_ellipsis_in_the_width() {
        assert_eq!(fit("otty-site-web-1", 8.0, chars), "otty-si…");
        assert_eq!(fit("otty-site-web-1", 14.0, chars), "otty-site-web…");
    }

    #[test]
    fn given_cut_after_a_space_when_fitted_then_the_space_is_dropped() {
        assert_eq!(fit("hello world", 7.0, chars), "hello…");
    }

    #[test]
    fn given_width_for_no_character_when_fitted_then_only_ellipsis_is_left() {
        assert_eq!(fit("redis", 1.0, chars), "…");
        assert_eq!(fit("redis", 0.0, chars), "…");
    }

    #[test]
    fn given_multibyte_text_when_fitted_then_it_is_cut_between_characters() {
        // Non-ASCII on purpose: each "é" takes two bytes.
        assert_eq!(fit("éééééé", 3.0, chars), "éé…");
    }
}
