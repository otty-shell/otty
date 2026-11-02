//! Text that wraps at spaces, slashes, dashes, colons and dots first.
//!
//! The wrapping of `iced` text breaks at the places the Unicode line break
//! rules allow, which are rarely inside ports, paths, image names or IDs.
//! [`WrappedText`] breaks after a space, `/`, `-`, `:` or `.` when it can,
//! so `0.0.0.0:8080->80/tcp` splits after the `/` and not in the middle of
//! `tcp`. A piece that is wider than the line by itself is broken between
//! characters as a last resort, so a long ID never pushes its panel wider.
//!
//! Parts of the text can be links: drawn in the link color, underlined
//! while the pointer is over them, and sending their message when pressed.
//! The text wraps as one, so a link can break onto the next line too.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::text::{self as core_text, Paragraph as _, Span};
use iced::advanced::widget::{
    Operation, Tree, Widget, text as text_widget, tree,
};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Vertical;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::{
    Color, Element, Event, Font, Length, Pixels, Rectangle, Renderer, Size,
    Theme, Vector, mouse,
};

/// Characters after which a line may break.
const BREAK_AFTER: [char; 5] = [' ', '/', '-', ':', '.'];
/// Distance from the bottom of a link's line box up to its underline, as
/// in the table's link cells.
const UNDERLINE_RISE: f32 = 2.0;
/// Thickness of a link's underline.
const UNDERLINE_WIDTH: f32 = 1.0;

/// Paragraph type of the renderer, used to measure and draw the text.
type Paragraph = <Renderer as core_text::Renderer>::Paragraph;

/// Text that fills its width and wraps at [`BREAK_AFTER`] characters
/// first; parts with a message are links. Widget queries such as
/// `iced_test` selectors see each link's text where it is drawn, then the
/// whole text.
pub(super) struct WrappedText<Message> {
    /// The text in order; a part with a message is a link.
    parts: Vec<(String, Option<Message>)>,
    /// All parts joined.
    content: String,
    size: f32,
    font: Font,
    color: Color,
    link_color: Color,
}

impl<Message> WrappedText<Message> {
    /// Creates left-aligned text from `parts`: plain text in `color`, and
    /// links (parts with a message) in `link_color`.
    pub(super) fn new(
        parts: Vec<(String, Option<Message>)>,
        size: f32,
        font: Font,
        color: Color,
        link_color: Color,
    ) -> Self {
        let content = parts.iter().map(|(text, _)| text.as_str()).collect();

        Self {
            parts,
            content,
            size,
            font,
            color,
            link_color,
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

    /// Returns the parts as text with the index of each link.
    fn part_links(&self) -> Vec<(&str, Option<usize>)> {
        self.parts
            .iter()
            .enumerate()
            .map(|(index, (text, message))| {
                (text.as_str(), message.as_ref().map(|_| index))
            })
            .collect()
    }

    /// Returns the shown text as paragraph spans: links in the link color.
    fn paragraph_spans<'a>(
        &self,
        shown: &'a [ShownSpan],
    ) -> Vec<Span<'a, (), Font>> {
        shown
            .iter()
            .map(|span| {
                let color = match span.link {
                    Some(_) => self.link_color,
                    None => self.color,
                };
                Span::new(span.text.as_str()).color(color)
            })
            .collect()
    }

    /// Returns the index of the link part under `cursor`.
    fn link_at(
        &self,
        state: &State,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) -> Option<usize> {
        let position = cursor.position_in(layout.bounds())?;
        let span = state.paragraph.hit_span(position)?;

        state.shown.get(span)?.link
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for WrappedText<Message> {
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
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let max_width = limits.max().width;

        if !state.is_laid_out_for(self, max_width) {
            let wrapped = wrap(&self.content, max_width, |candidate| {
                self.measure(candidate)
            });
            state.shown = split_wrapped(&self.part_links(), &wrapped);
            state.paragraph = Paragraph::with_spans(core_text::Text {
                content: self.paragraph_spans(&state.shown).as_slice(),
                bounds: Size::new(max_width, f32::INFINITY),
                size: Pixels(self.size),
                line_height: LineHeight::default(),
                font: self.font,
                align_x: core_text::Alignment::Left,
                align_y: Vertical::Top,
                shaping: Shaping::default(),
                wrapping: Wrapping::None,
            });
            state.laid_out_for = Some(LaidOutFor::of(self, max_width));
        }

        layout::Node::new(limits.resolve(
            Length::Fill,
            Length::Shrink,
            state.paragraph.min_bounds(),
        ))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        text_widget::draw(
            renderer,
            style,
            bounds,
            &state.paragraph,
            text_widget::Style {
                color: Some(self.color),
            },
            viewport,
        );

        let Some(hovered) = self.link_at(state, layout, cursor) else {
            return;
        };
        let origin = Vector::new(bounds.x, bounds.y);
        for (index, span) in state.shown.iter().enumerate() {
            if span.link != Some(hovered) {
                continue;
            }

            for region in state.paragraph.span_bounds(index) {
                let underline = Rectangle {
                    y: region.y + region.height - UNDERLINE_RISE,
                    height: UNDERLINE_WIDTH,
                    ..region
                };
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: underline + origin,
                        snap: true,
                        ..renderer::Quad::default()
                    },
                    self.link_color,
                );
            }
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let link =
            self.link_at(tree.state.downcast_ref::<State>(), layout, cursor);
        let state = tree.state.downcast_mut::<State>();

        if link != state.hovered {
            state.hovered = link;
            shell.request_redraw();
        }

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if link.is_some() =>
            {
                state.pressed = link;
                shell.capture_event();
            },
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let pressed = state.pressed.take();
                let message = pressed
                    .filter(|index| link == Some(*index))
                    .and_then(|index| self.parts.get(index))
                    .and_then(|(_, message)| message.clone());
                if let Some(message) = message {
                    shell.publish(message);
                    shell.capture_event();
                }
            },
            _ => {},
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        match self.link_at(state, layout, cursor) {
            Some(_) => mouse::Interaction::Pointer,
            None => mouse::Interaction::None,
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let origin = Vector::new(bounds.x, bounds.y);

        // Links first, at the start of their text, so a query for a link
        // finds the link and not a whole value with the same text.
        for (index, (text, message)) in self.parts.iter().enumerate() {
            if message.is_none() {
                continue;
            }
            let first_region = state
                .shown
                .iter()
                .position(|span| span.link == Some(index))
                .and_then(|span| {
                    state.paragraph.span_bounds(span).first().copied()
                });
            if let Some(region) = first_region {
                operation.text(None, region + origin, text);
            }
        }
        operation.text(None, bounds, &self.content);
    }
}

impl<'a, Message: Clone + 'a> From<WrappedText<Message>>
    for Element<'a, Message>
{
    fn from(value: WrappedText<Message>) -> Self {
        Element::new(value)
    }
}

/// Layout state of a [`WrappedText`]: the shown spans, their paragraph,
/// what they were laid out for (so unchanged text is not measured again on
/// every layout) and the link under the pointer and pressed.
#[derive(Default)]
struct State {
    paragraph: Paragraph,
    shown: Vec<ShownSpan>,
    laid_out_for: Option<LaidOutFor>,
    hovered: Option<usize>,
    pressed: Option<usize>,
}

impl State {
    /// Returns whether the paragraph was laid out for these parts, width,
    /// size, font and colors.
    fn is_laid_out_for<Message>(
        &self,
        text: &WrappedText<Message>,
        max_width: f32,
    ) -> bool {
        self.laid_out_for
            .as_ref()
            .is_some_and(|laid_out| laid_out.matches(text, max_width))
    }
}

/// What the paragraph of a [`WrappedText`] was built for.
struct LaidOutFor {
    /// Each part's text and whether it is a link.
    parts: Vec<(String, bool)>,
    max_width: f32,
    size: f32,
    font: Font,
    color: Color,
    link_color: Color,
}

impl LaidOutFor {
    /// Records `text` laid out in `max_width`.
    fn of<Message>(text: &WrappedText<Message>, max_width: f32) -> Self {
        Self {
            parts: text
                .parts
                .iter()
                .map(|(part, message)| (part.clone(), message.is_some()))
                .collect(),
            max_width,
            size: text.size,
            font: text.font,
            color: text.color,
            link_color: text.link_color,
        }
    }

    /// Returns whether `text` in `max_width` is what was laid out.
    fn matches<Message>(
        &self,
        text: &WrappedText<Message>,
        max_width: f32,
    ) -> bool {
        let same_parts = self.parts.len() == text.parts.len()
            && self.parts.iter().zip(&text.parts).all(
                |((laid_out, is_link), (part, message))| {
                    laid_out == part && *is_link == message.is_some()
                },
            );

        same_parts
            && self.max_width == max_width
            && self.size == text.size
            && self.font == text.font
            && self.color == text.color
            && self.link_color == text.link_color
    }
}

/// One run of the shown text: its characters and, for a link, the index of
/// its part.
#[derive(Debug, Clone, PartialEq)]
struct ShownSpan {
    text: String,
    link: Option<usize>,
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

/// Splits `wrapped`, the joined text of `parts` after [`wrap`], back into
/// the parts (text and link index). Each line break that [`wrap`] added is
/// a span of its own outside any link, so a link broken over two lines is
/// two spans with the same link. Spaces [`wrap`] dropped at a line end are
/// left out, and so are empty spans.
fn split_wrapped(
    parts: &[(&str, Option<usize>)],
    wrapped: &str,
) -> Vec<ShownSpan> {
    let mut spans = Vec::new();
    let mut shown = wrapped.chars().peekable();

    for &(text, link) in parts {
        let mut current = String::new();

        for character in text.chars() {
            if character != '\n' && shown.peek() == Some(&'\n') {
                push_span(&mut spans, std::mem::take(&mut current), link);
                push_span(&mut spans, String::from("\n"), None);
                shown.next();
            }

            if shown.peek() == Some(&character) {
                current.push(character);
                shown.next();
            }
        }

        push_span(&mut spans, current, link);
    }

    spans
}

/// Adds a span of `text` to `spans` unless it is empty.
fn push_span(spans: &mut Vec<ShownSpan>, text: String, link: Option<usize>) {
    if !text.is_empty() {
        spans.push(ShownSpan { text, link });
    }
}

#[cfg(test)]
mod tests {
    use super::{ShownSpan, split_wrapped, wrap};

    /// Measures one unit per character, so widths read as lengths.
    fn chars(text: &str) -> f32 {
        text.chars().count() as f32
    }

    fn span(text: &str, link: Option<usize>) -> ShownSpan {
        ShownSpan {
            text: text.to_owned(),
            link,
        }
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

    #[test]
    fn given_unwrapped_parts_when_split_then_each_part_is_one_span() {
        let parts = [("image ", None), ("busybox:1.37", Some(1))];

        let spans = split_wrapped(&parts, "image busybox:1.37");

        assert_eq!(
            spans,
            vec![span("image ", None), span("busybox:1.37", Some(1))]
        );
    }

    #[test]
    fn given_a_break_inside_a_link_when_split_then_both_halves_keep_the_link() {
        let parts = [("otty-fx-data", Some(0)), (" → /data", None)];

        let spans = split_wrapped(&parts, "otty-fx-\ndata → /data");

        assert_eq!(
            spans,
            vec![
                span("otty-fx-", Some(0)),
                span("\n", None),
                span("data", Some(0)),
                span(" → /data", None),
            ]
        );
    }

    #[test]
    fn given_a_break_at_a_dropped_space_when_split_then_the_space_is_gone() {
        let parts = [
            ("8080:80/tcp", Some(0)),
            (", ", None),
            ("8443:443/tcp", Some(2)),
        ];

        let spans = split_wrapped(&parts, "8080:80/tcp,\n8443:443/tcp");

        assert_eq!(
            spans,
            vec![
                span("8080:80/tcp", Some(0)),
                span(",", None),
                span("\n", None),
                span("8443:443/tcp", Some(2)),
            ]
        );
    }

    #[test]
    fn given_a_break_between_parts_when_split_then_it_stands_alone() {
        let parts = [("abc/", None), ("def", Some(1))];

        let spans = split_wrapped(&parts, "abc/\ndef");

        assert_eq!(
            spans,
            vec![span("abc/", None), span("\n", None), span("def", Some(1))]
        );
    }

    #[test]
    fn given_empty_parts_and_kept_new_lines_when_split_then_no_empty_span_is_left()
     {
        let parts = [("", Some(0)), ("a\nb", None), ("", None)];

        let spans = split_wrapped(&parts, "a\nb");

        assert_eq!(spans, vec![span("a\nb", None)]);
    }
}
