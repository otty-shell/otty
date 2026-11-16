//! `BlockHeader`: the sticky header of a terminal command block, a bar at
//! the top of the pane while the start of the block is above it.

use iced::widget::{Row, Space, container, mouse_area};
use iced::{Alignment, Color, Element, Length, Padding};

use super::block_fit::{HeaderParts, HeaderWidths, command_cut, command_start};
use super::block_outcome::{
    BlockOutcome, label, label_width, outcome_icon, outcome_icon_width,
};
use super::ellipsis_text::{EllipsisText, text_width};
use super::glyph;
use super::icon_button::{IconButton, IconButtonSize};
use crate::palette::UiPalette;
use crate::{fonts, size, spacing, typography};

/// Height of the bar.
const HEADER_HEIGHT: f32 = 30.0;
/// Space between the parts of the bar.
const HEADER_GAP: f32 = 10.0;
/// Space between the right edge of the bar and the button.
const HEADER_PADDING_RIGHT: f32 = spacing::SPACE_3;
/// Side of the button that goes to the start of the block.
const BUTTON_SIZE: f32 = size::CONTROL_SM;

/// The 30 px sticky header of a command block, from edge to edge of the
/// pane on `surface-header` with the sticky shadow: the outcome icon, the
/// command in the terminal font and `412 lines above` at the left,
/// `text_inset` in like the text of the terminal; `exit 101`, the duration
/// and the 24 px arrow-up button at the right, 8 px from the edge.
///
/// Nothing in the bar wraps: when its `width` runs out, the command is cut
/// with "…" first, down to its first 8 characters; then `412 lines above`
/// is left out, then `exit 101` with the duration; then the command is cut
/// further and left out. The button goes last, when even the icon and the
/// button do not fit.
///
/// A press on the bar sends the message of [`BlockHeader::on_press`] and
/// does not reach what lies under it; the wheel goes through.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::block_header::BlockHeader;
/// use otty_ui_kit::components::block_outcome::BlockOutcome;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     Focus,
///     GoToStart,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let header: Element<'_, Message> = BlockHeader::new(
///     BlockOutcome::Failed,
///     "cargo build",
///     "48.2 s",
///     &palette,
/// )
/// .exit_code("exit 101")
/// .lines_above("412 lines above")
/// .go_to_start("Go to start", Message::GoToStart)
/// .on_press(Message::Focus)
/// .width(400.0)
/// .text_inset(16.0)
/// .into();
/// ```
pub struct BlockHeader<Message> {
    outcome: BlockOutcome,
    command: String,
    duration: String,
    palette: UiPalette,
    exit_code: Option<String>,
    lines_above: Option<String>,
    go_to_start: Option<(String, Message)>,
    on_press: Option<Message>,
    width: f32,
    text_inset: f32,
}

impl<Message> BlockHeader<Message> {
    /// Creates the header of the block whose `command`, on one line, ended
    /// as `outcome` after `duration`, already written (`48.2 s`). It has no
    /// limit on its width and starts its texts 16 px in, the side space of
    /// a block.
    pub fn new(
        outcome: BlockOutcome,
        command: impl Into<String>,
        duration: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            outcome,
            command: command.into(),
            duration: duration.into(),
            palette: palette.clone(),
            exit_code: None,
            lines_above: None,
            go_to_start: None,
            on_press: None,
            width: f32::INFINITY,
            text_inset: spacing::SPACE_5,
        }
    }

    /// Shows `exit_code`, such as `exit 101`, in `status-error` before the
    /// duration.
    pub fn exit_code(mut self, exit_code: impl Into<String>) -> Self {
        self.exit_code = Some(exit_code.into());
        self
    }

    /// Shows how much of the block is above the pane, such as
    /// `412 lines above`, after the command.
    pub fn lines_above(mut self, lines_above: impl Into<String>) -> Self {
        self.lines_above = Some(lines_above.into());
        self
    }

    /// Shows the arrow-up button, named `name` in its tooltip, that sends
    /// `on_press` to bring the start of the block into view.
    pub fn go_to_start(
        mut self,
        name: impl Into<String>,
        on_press: Message,
    ) -> Self {
        self.go_to_start = Some((name.into(), on_press));
        self
    }

    /// Sets the message sent on a press on the bar outside its button.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the width of the bar, the width of the pane, which decides
    /// which parts fit.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets how far in the texts start: where the text of the terminal
    /// starts.
    pub fn text_inset(mut self, text_inset: f32) -> Self {
        self.text_inset = text_inset;
        self
    }

    /// The widths of the parts of the bar, measured with the font and the
    /// size they are drawn with.
    fn widths(&self) -> HeaderWidths {
        let command = |content: &str| {
            text_width(content, typography::TERM.size(), fonts::MONO)
        };
        let duration = label_width(&self.duration);
        let outcome = match &self.exit_code {
            Some(exit_code) => label_width(exit_code) + HEADER_GAP + duration,
            None => duration,
        };

        HeaderWidths {
            icon: outcome_icon_width(&self.outcome),
            command_start: command(&command_start(&self.command)),
            command_cut: command(command_cut()),
            lines_above: self.lines_above.as_deref().map(label_width),
            outcome,
            button: self.go_to_start.as_ref().map(|_| BUTTON_SIZE),
            gap: HEADER_GAP,
        }
    }
}

impl<'a, Message: Clone + 'a> From<BlockHeader<Message>>
    for Element<'a, Message>
{
    fn from(header: BlockHeader<Message>) -> Self {
        let available = header.width - header.text_inset - HEADER_PADDING_RIGHT;
        let widths = header.widths();
        let parts = HeaderParts::fit(available, &widths);
        let BlockHeader {
            outcome,
            command,
            duration,
            palette,
            exit_code,
            lines_above,
            go_to_start,
            on_press,
            text_inset,
            ..
        } = header;

        let mut start: Vec<Element<'a, Message>> = Vec::new();
        if let Some(icon) = outcome_icon(&outcome, &palette) {
            start.push(icon);
        }
        if parts.shows_command() {
            // The command leaves the room of the lines above free and is
            // cut with "…" to what is left.
            let reserved = match widths.lines_above {
                Some(lines) if parts.shows_lines_above() => lines + HEADER_GAP,
                _ => 0.0,
            };
            let command = command_text(command, palette.text_strong());
            start.push(command.shrink(reserved).into());
        }
        if let Some(lines_above) =
            lines_above.filter(|_| parts.shows_lines_above())
        {
            let lines_above =
                lines_above_text(lines_above, palette.text_muted());
            start.push(lines_above.shrink(0.0).into());
        }

        let mut end: Vec<Element<'a, Message>> = Vec::new();
        if parts.shows_outcome() {
            if let Some(exit_code) = exit_code {
                end.push(label(exit_code, palette.status_error()));
            }
            end.push(label(duration, palette.text_muted()));
        }
        if let Some((name, on_press)) =
            go_to_start.filter(|_| parts.shows_button())
        {
            end.push(go_to_start_button(name, on_press, &palette));
        }

        // The start takes the room the end leaves, so the end sits at the
        // right edge.
        let has_start = !start.is_empty();
        let start = container(
            Row::with_children(start)
                .spacing(HEADER_GAP)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .clip(true);
        let mut content = Row::new().push(start).align_y(Alignment::Center);
        if !end.is_empty() {
            if has_start {
                content = content.push(Space::new().width(HEADER_GAP));
            }
            content = content.push(
                Row::with_children(end)
                    .spacing(HEADER_GAP)
                    .align_y(Alignment::Center),
            );
        }

        let background = palette.surface_header();
        let shadow = palette.shadow_sticky();
        let bar = container(content)
            .width(Length::Fill)
            .height(HEADER_HEIGHT)
            .align_y(Alignment::Center)
            .clip(true)
            .padding(Padding {
                left: text_inset,
                right: HEADER_PADDING_RIGHT,
                ..Padding::ZERO
            })
            .style(move |_| container::Style {
                background: Some(background.into()),
                shadow,
                ..container::Style::default()
            });

        match on_press {
            Some(on_press) => mouse_area(bar).on_press(on_press).into(),
            None => bar.into(),
        }
    }
}

/// `command` in the terminal font, in `color`. It ends with "…" when it
/// does not fit.
fn command_text(command: String, color: Color) -> EllipsisText {
    EllipsisText::new(command, typography::TERM.size(), fonts::MONO, color)
}

/// `lines_above` in `color`: as wide as a [`label`] of it.
fn lines_above_text(lines_above: String, color: Color) -> EllipsisText {
    EllipsisText::new(lines_above, typography::SMALL.size(), fonts::UI, color)
}

/// The 24 px arrow-up button that brings the start of the block into view.
fn go_to_start_button<'a, Message: Clone + 'a>(
    name: String,
    on_press: Message,
    palette: &UiPalette,
) -> Element<'a, Message> {
    IconButton::new(glyph::arrow(true), name, palette)
        .size(IconButtonSize::Small)
        .on_press(on_press)
        .into()
}

#[cfg(test)]
mod tests {
    use iced::widget::{Space, mouse_area, stack};
    use iced::{Event, Point, Size, mouse};
    use iced_test::Simulator;

    use super::*;

    const PANE: Size = Size::new(400.0, 200.0);
    const INSET: f32 = 16.0;
    /// A point over the bar, away from its texts and its button.
    const OVER_THE_BAR: Point = Point::new(300.0, 15.0);
    /// A point over the output under the bar.
    const UNDER_THE_BAR: Point = Point::new(300.0, 100.0);
    /// The texts of [`failed_cargo`].
    const CARGO_TEXTS: [&str; 4] =
        ["cargo build", "412 lines above", "exit 101", "48.2 s"];

    #[derive(Debug, Clone, PartialEq)]
    enum Reached {
        UnderPressed,
        UnderScrolled,
        BarPressed,
        GoToStart,
    }

    /// What a header shows in a pane: which of its texts are found,
    /// whether each found text stays on one line and whether a press at
    /// the right end reaches the button.
    struct Shown {
        texts: Vec<&'static str>,
        is_one_line: bool,
        has_button: bool,
    }

    /// `cargo build`, failed with 101 after 48.2 s, with 412 lines above
    /// the pane.
    fn failed_cargo(palette: &UiPalette) -> BlockHeader<Reached> {
        BlockHeader::new(BlockOutcome::Failed, "cargo build", "48.2 s", palette)
            .exit_code("exit 101")
            .lines_above("412 lines above")
            .go_to_start("Go to start", Reached::GoToStart)
            .on_press(Reached::BarPressed)
            .text_inset(INSET)
    }

    /// `header` in a pane `pane_width` wide over something that reports
    /// what reaches it.
    fn over_a_pane(
        header: BlockHeader<Reached>,
        pane_width: f32,
    ) -> Simulator<'static, Reached> {
        let under =
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_press(Reached::UnderPressed)
                .on_scroll(|_| Reached::UnderScrolled);
        let content: Element<'static, Reached> =
            stack![under, header.width(pane_width)]
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

        Simulator::with_size(
            iced::Settings::default(),
            Size::new(pane_width, PANE.height),
            content,
        )
    }

    fn press_and_scroll_at(ui: &mut Simulator<'_, Reached>, position: Point) {
        ui.point_at(position);
        let _ = ui.simulate([
            Event::Mouse(mouse::Event::CursorMoved { position }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 },
            }),
        ]);
    }

    /// A point over the button of a bar `pane_width` wide: 8 px from the
    /// edge, 24 px wide.
    fn over_the_button(pane_width: f32) -> Point {
        Point::new(pane_width - 8.0 - 12.0, 15.0)
    }

    fn shown_at(pane_width: f32) -> Shown {
        let palette = UiPalette::otty_dark();
        let mut ui = over_a_pane(failed_cargo(&palette), pane_width);

        let found: Vec<(&'static str, f32)> = CARGO_TEXTS
            .into_iter()
            .filter_map(|label| {
                let bounds = ui.find(label).ok()?.bounds();
                Some((label, bounds.height))
            })
            .collect();
        press_and_scroll_at(&mut ui, over_the_button(pane_width));
        let reached: Vec<Reached> = ui.into_messages().collect();

        Shown {
            texts: found.iter().map(|(label, _)| *label).collect(),
            is_one_line: found
                .iter()
                .all(|(_, height)| *height <= typography::TERM.line_height()),
            has_button: reached.contains(&Reached::GoToStart),
        }
    }

    /// The width of a pane whose bar has a room of `available` pixels for
    /// its parts: the room and the space at the sides.
    fn pane_for(available: f32) -> f32 {
        available + INSET + HEADER_PADDING_RIGHT
    }

    #[test]
    fn a_wide_header_shows_its_command_lines_above_exit_code_and_duration() {
        let palette = UiPalette::otty_dark();

        let mut ui = over_a_pane(failed_cargo(&palette), PANE.width);

        for text in CARGO_TEXTS {
            assert!(ui.find(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn a_press_on_the_button_goes_to_the_start_and_not_to_the_bar() {
        let palette = UiPalette::otty_dark();
        let mut ui = over_a_pane(failed_cargo(&palette), PANE.width);

        press_and_scroll_at(&mut ui, over_the_button(PANE.width));

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert!(reached.contains(&Reached::GoToStart), "got {reached:?}");
        assert!(!reached.contains(&Reached::UnderPressed), "got {reached:?}");
        assert!(!reached.contains(&Reached::BarPressed), "got {reached:?}");
    }

    #[test]
    fn a_press_on_the_bar_stays_with_it_and_the_wheel_goes_through() {
        let palette = UiPalette::otty_dark();
        let mut ui = over_a_pane(failed_cargo(&palette), PANE.width);

        press_and_scroll_at(&mut ui, OVER_THE_BAR);

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert_eq!(reached, [Reached::BarPressed, Reached::UnderScrolled]);
    }

    #[test]
    fn a_press_under_the_bar_reaches_what_is_under_it() {
        let palette = UiPalette::otty_dark();
        let mut ui = over_a_pane(failed_cargo(&palette), PANE.width);

        press_and_scroll_at(&mut ui, UNDER_THE_BAR);

        let reached: Vec<Reached> = ui.into_messages().collect();
        assert_eq!(reached, [Reached::UnderPressed, Reached::UnderScrolled]);
    }

    /// Owner, issue 11: at every width the header stays one line; the
    /// command is cut first, then `412 lines above` goes, then the exit
    /// code and the duration, then the command, and the button goes last.
    #[test]
    fn in_a_narrower_pane_the_parts_of_the_header_give_way_in_order() {
        let palette = UiPalette::otty_dark();
        let widths = failed_cargo(&palette).widths();
        let gap = widths.gap;
        let icon = widths.icon.unwrap_or_default();
        let lines = widths.lines_above.unwrap_or_default();
        let button = widths.button.unwrap_or_default();
        let start = icon + gap + widths.command_start;
        let end = widths.outcome + gap + button;
        let all = start + gap + lines + gap + end;
        let without_lines = start + gap + end;
        let with_cut_command = icon + gap + widths.command_cut + gap + button;
        let icon_and_button = icon + gap + button;

        let wide = shown_at(800.0);
        let middle = shown_at(pane_for(all - 1.0));
        let narrow = shown_at(pane_for(without_lines - 1.0));
        let very_narrow = shown_at(pane_for(with_cut_command - 1.0));
        let tiny = shown_at(pane_for(icon_and_button - 1.0));

        assert_eq!(wide.texts, CARGO_TEXTS);
        assert_eq!(middle.texts, ["cargo build", "exit 101", "48.2 s"]);
        assert_eq!(narrow.texts, ["cargo build"]);
        assert!(very_narrow.texts.is_empty());
        assert!(tiny.texts.is_empty());
        for shown in [&wide, &middle, &narrow, &very_narrow] {
            assert!(shown.is_one_line);
            assert!(shown.has_button);
        }
        assert!(!tiny.has_button);
    }
}
