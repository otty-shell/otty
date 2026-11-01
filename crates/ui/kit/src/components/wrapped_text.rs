//! Text that wraps at spaces, slashes, dashes, colons and dots first.
//!
//! The wrapping of `iced` text breaks at the places the Unicode line break
//! rules allow, which are rarely inside ports, paths, image names or IDs.
//! [`WrappedText`] breaks after a space, `/`, `-`, `:` or `.` when it can,
//! so `0.0.0.0:8080->80/tcp` splits after the `/` and not in the middle of
//! `tcp`. A piece that is wider than the line by itself is broken between
//! characters as a last resort, so a long ID never pushes its panel wider.

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

/// Characters after which a line may break.
const BREAK_AFTER: [char; 5] = [' ', '/', '-', ':', '.'];

/// Paragraph type of the renderer, used to measure and draw the text.
type Paragraph = <Renderer as core_text::Renderer>::Paragraph;

/// Text that fills its width and wraps at [`BREAK_AFTER`] characters
/// first. Widget queries such as `iced_test` selectors see the whole text.
pub(super) struct WrappedText {
    content: String,
    size: f32,
    font: Font,
    color: Color,
}

impl WrappedText {
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
        }
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

    /// Returns the layout format of the wrapped text: the lines are already
    /// split, so the text itself never wraps.
    fn format(&self) -> text_widget::Format<Font> {
        text_widget::Format {
            width: Length::Fill,
            height: Length::Shrink,
            size: Some(Pixels(self.size)),
            font: Some(self.font),
            line_height: LineHeight::default(),
            align_x: core_text::Alignment::Left,
            align_y: Vertical::Top,
            shaping: Shaping::default(),
            wrapping: Wrapping::None,
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for WrappedText {
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

        if !state.is_wrapped_for(self, max_width) {
            state.shown = wrap(&self.content, max_width, |candidate| {
                self.measure(candidate)
            });
            state.wrapped_for =
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

impl<'a, Message: 'a> From<WrappedText> for Element<'a, Message> {
    fn from(value: WrappedText) -> Self {
        Element::new(value)
    }
}

/// Layout state of a [`WrappedText`]: the text with its line breaks and
/// what it was wrapped for, so unchanged text is not measured again on
/// every layout.
#[derive(Default)]
struct State {
    paragraph: paragraph::Plain<Paragraph>,
    shown: String,
    wrapped_for: Option<(String, f32, f32, Font)>,
}

impl State {
    /// Returns whether the shown text was wrapped for this content, width,
    /// size and font.
    fn is_wrapped_for(&self, text: &WrappedText, max_width: f32) -> bool {
        self.wrapped_for
            .as_ref()
            .is_some_and(|(content, width, size, font)| {
                content == &text.content
                    && *width == max_width
                    && *size == text.size
                    && *font == text.font
            })
    }
}

/// Returns `content` with a newline at each line break, so that every line
/// is at most `max_width` wide by `measure`. Lines break after a character
/// in [`BREAK_AFTER`] when possible, otherwise between any two characters.
/// Spaces at the end of a line are dropped. Newlines in `content` stay.
fn wrap(
    content: &str,
    max_width: f32,
    mut measure: impl FnMut(&str) -> f32,
) -> String {
    if !max_width.is_finite() || measure(content) <= max_width {
        return content.to_owned();
    }

    content
        .split('\n')
        .map(|line| wrap_line(line, max_width, &mut measure))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Wraps `line`, which has no newline, as [`wrap`] does.
fn wrap_line(
    line: &str,
    max_width: f32,
    measure: &mut impl FnMut(&str) -> f32,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for piece in line.split_inclusive(BREAK_AFTER) {
        let joined = format!("{current}{piece}");
        if measure(joined.trim_end()) <= max_width {
            current = joined;
            continue;
        }

        if !current.is_empty() {
            lines.push(current.trim_end().to_owned());
            current.clear();
        }
        if measure(piece.trim_end()) <= max_width {
            current.push_str(piece);
            continue;
        }

        // The piece alone is wider than a line: break it anywhere.
        for character in piece.chars() {
            let joined = format!("{current}{character}");
            if current.is_empty() || measure(joined.trim_end()) <= max_width {
                current = joined;
            } else {
                lines.push(current.trim_end().to_owned());
                current = character.to_string();
            }
        }
    }
    lines.push(current.trim_end().to_owned());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::wrap;

    /// Measures one unit per character, so widths read as lengths.
    fn chars(text: &str) -> f32 {
        text.chars().count() as f32
    }

    #[test]
    fn given_text_that_fits_when_wrapped_then_it_is_unchanged() {
        assert_eq!(wrap("8080:80/tcp", 11.0, chars), "8080:80/tcp");
        assert_eq!(wrap("8080:80/tcp", 40.0, chars), "8080:80/tcp");
        assert_eq!(wrap("", 0.0, chars), "");
    }

    #[test]
    fn given_a_slash_when_wrapped_then_the_line_breaks_after_it() {
        assert_eq!(
            wrap("0.0.0.0:8080->80/tcp", 17.0, chars),
            "0.0.0.0:8080->80/\ntcp"
        );
        assert_eq!(
            wrap("usr/local/share/data", 12.0, chars),
            "usr/local/\nshare/data"
        );
    }

    #[test]
    fn given_spaces_when_wrapped_then_the_line_breaks_at_a_space_and_drops_it()
    {
        assert_eq!(
            wrap("hello brave world", 11.0, chars),
            "hello brave\nworld"
        );
        assert_eq!(
            wrap("hello brave world", 6.0, chars),
            "hello\nbrave\nworld"
        );
    }

    #[test]
    fn given_dashes_colons_and_dots_when_wrapped_then_the_line_breaks_after_them()
     {
        assert_eq!(wrap("otty-site-web-1", 10.0, chars), "otty-site-\nweb-1");
        assert_eq!(
            wrap("registry.example.com:5000", 17.0, chars),
            "registry.example.\ncom:5000"
        );
        assert_eq!(wrap("sha256:abcdef", 8.0, chars), "sha256:\nabcdef");
    }

    #[test]
    fn given_a_piece_wider_than_the_line_when_wrapped_then_it_breaks_between_characters()
     {
        assert_eq!(
            wrap("0123456789abcdef", 6.0, chars),
            "012345\n6789ab\ncdef"
        );
        assert_eq!(
            wrap("sha256:0123456789abcdef", 8.0, chars),
            "sha256:\n01234567\n89abcdef"
        );
    }

    #[test]
    fn given_a_line_that_ends_in_a_space_when_wrapped_then_the_space_does_not_count()
     {
        assert_eq!(wrap("abcde fgh", 5.0, chars), "abcde\nfgh");
    }

    #[test]
    fn given_new_lines_when_wrapped_then_each_line_wraps_on_its_own() {
        assert_eq!(wrap("a/b\nc/d", 3.0, chars), "a/b\nc/d");
        assert_eq!(wrap("ab/cd\nef", 3.0, chars), "ab/\ncd\nef");
    }

    #[test]
    fn given_multibyte_text_when_wrapped_then_it_breaks_between_characters() {
        // Non-ASCII on purpose: each "é" takes two bytes.
        assert_eq!(wrap("éééééé", 4.0, chars), "éééé\néé");
    }
}
