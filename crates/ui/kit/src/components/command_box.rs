//! `CommandBox`: a shell command in a mono font with a copy icon.

use iced::widget::text::Wrapping;
use iced::widget::{container, row, scrollable, svg, text};
use iced::{Alignment, Border, Element, Length, Padding};

use super::icon_button::{IconButton, IconButtonSize};
use super::scrollbar;
use crate::palette::UiPalette;
use crate::{fonts, radius, size, spacing};

/// Size of the command text (`font-size: 12.5px`).
const COMMAND_SIZE: f32 = 12.5;
/// Left padding of the box (`padding: 6px 6px 6px 12px`).
const PADDING_LEFT: f32 = 12.0;
/// Other paddings of the box.
const PADDING_OTHER: f32 = 6.0;

/// A command the user can read and copy. The box is one line high whatever
/// the command or the width: a command that does not fit scrolls sideways,
/// with a thin bar floating over the line, and is never wrapped or shortened.
/// An icon button stays on its right, with the label as a tooltip, and sends
/// the message set with [`CommandBox::on_copy`]. The caller writes the command
/// to the clipboard.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use iced::widget::svg;
/// use otty_ui_kit::components::command_box::CommandBox;
/// use otty_ui_kit::palette::UiPalette;
///
/// #[derive(Clone)]
/// enum Message {
///     Copy,
/// }
///
/// let palette = UiPalette::otty_dark();
/// let copy_icon = svg::Handle::from_memory(&b"<svg/>"[..]);
/// let command: Element<'_, Message> = CommandBox::new(
///     "sudo systemctl start docker",
///     copy_icon,
///     "Copy",
///     &palette,
/// )
/// .on_copy(Message::Copy)
/// .into();
/// ```
pub struct CommandBox<Message> {
    command: String,
    copy_icon: svg::Handle,
    copy_label: String,
    palette: UiPalette,
    on_copy: Option<Message>,
}

impl<Message> CommandBox<Message> {
    /// Creates a box for `command` with a `copy_icon` button whose tooltip
    /// is `copy_label`.
    pub fn new(
        command: impl Into<String>,
        copy_icon: svg::Handle,
        copy_label: impl Into<String>,
        palette: &UiPalette,
    ) -> Self {
        Self {
            command: command.into(),
            copy_icon,
            copy_label: copy_label.into(),
            palette: palette.clone(),
            on_copy: None,
        }
    }

    /// Sets the message sent when the copy button is pressed.
    pub fn on_copy(mut self, message: Message) -> Self {
        self.on_copy = Some(message);
        self
    }
}

impl<'a, Message: Clone + 'a> From<CommandBox<Message>>
    for Element<'a, Message>
{
    fn from(value: CommandBox<Message>) -> Self {
        let palette = value.palette;
        let background = palette.surface_chrome();

        let command = text(value.command)
            .size(COMMAND_SIZE)
            .font(fonts::MONO)
            .color(palette.text_strong())
            .wrapping(Wrapping::None);
        // The line is as high as the icon. The bar floats over the room left
        // under the text, so it takes no height and moves nothing when it
        // shows.
        let line = scrollable(
            container(command)
                .height(size::CONTROL_SM)
                .align_y(Alignment::Center),
        )
        .direction(scrollbar::horizontal())
        .style(scrollbar::style(&palette))
        .width(Length::Fill);
        let copy = IconButton::new(value.copy_icon, value.copy_label, &palette)
            .size(IconButtonSize::Small)
            .on_press_maybe(value.on_copy);

        container(
            row![line, copy]
                .spacing(spacing::SPACE_2)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(Padding {
            top: PADDING_OTHER,
            right: PADDING_OTHER,
            bottom: PADDING_OTHER,
            left: PADDING_LEFT,
        })
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_SM.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::Modifiers;
    use iced::mouse::ScrollDelta;
    use iced::widget::{column, row};
    use iced::{Point, Size, Theme, event, mouse};
    use iced_test::selector::Candidate;
    use iced_test::{Simulator, simulator};

    use super::*;
    use crate::components::glyph;
    use crate::components::icon_button::{IconButton, IconButtonSize};
    use crate::size;
    use crate::test_support::TempDir;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        Copy,
    }

    /// The longest command of the no-engine cards.
    const LONG: &str = "systemctl --user enable --now podman.socket";
    const SHORT: &str = "ls";
    /// What shows under the box, to see how much height the box takes.
    const BELOW: &str = "text under the box";
    /// A window where [`LONG`] does not fit its box.
    const NARROW: Size = Size::new(200.0, 120.0);
    /// The height of the box: one line, whatever the command or the width.
    const BOX_HEIGHT: f32 = 36.0;
    /// Height of one line of the command.
    const LINE_HEIGHT: f32 = 16.25;
    /// A place on the text of the command.
    const OVER_TEXT: Point = Point::new(40.0, 18.0);

    fn command_box(command: &str) -> CommandBox<Message> {
        let palette = UiPalette::otty_dark();

        CommandBox::new(command, glyph::plus(), "Copy", &palette)
            .on_copy(Message::Copy)
    }

    /// The box in a window of `window`, above a line of text.
    fn box_in_window(
        command: &str,
        window: Size,
    ) -> Simulator<'static, Message> {
        Simulator::with_size(
            iced::Settings::default(),
            window,
            column![command_box(command), text(BELOW)],
        )
    }

    /// Where the middle of the copy icon is in a box `width` wide.
    fn copy_icon_at(width: f32) -> Point {
        Point::new(width - PADDING_OTHER - size::CONTROL_SM / 2.0, 18.0)
    }

    /// The scroller of the box: how far it is scrolled sideways, how wide
    /// its content is and how wide the part that shows is.
    fn scroller(ui: &mut Simulator<'_, Message>) -> (f32, f32, f32) {
        ui.find(|candidate: Candidate<'_>| match candidate {
            Candidate::Scrollable {
                bounds,
                content_bounds,
                translation,
                ..
            } => Some((translation.x, content_bounds.width, bounds.width)),
            _ => None,
        })
        .expect("the command should sit in a scroller")
    }

    fn move_pointer(ui: &mut Simulator<'_, Message>, position: Point) {
        ui.point_at(position);
        let _ = ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved {
            position,
        })]);
    }

    fn wheel(
        ui: &mut Simulator<'_, Message>,
        x: f32,
        y: f32,
    ) -> Vec<event::Status> {
        ui.simulate([iced::Event::Mouse(mouse::Event::WheelScrolled {
            delta: ScrollDelta::Lines { x, y },
        })])
    }

    #[test]
    fn given_command_box_then_the_command_is_shown() {
        let mut ui = simulator(command_box("sudo systemctl start docker"));

        assert!(ui.find("sudo systemctl start docker").is_ok());
    }

    #[test]
    fn given_command_box_when_copy_icon_pressed_then_its_message_is_sent() {
        let mut ui = box_in_window(SHORT, NARROW);

        move_pointer(&mut ui, copy_icon_at(NARROW.width));
        let _ = ui.simulate(simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Copy]);
    }

    #[test]
    fn given_command_box_then_copy_is_an_icon_without_a_text() {
        let mut ui = box_in_window(SHORT, NARROW);

        assert!(ui.find("Copy").is_err(), "the label is only a tooltip");
    }

    /// Draws the box with the pointer on the icon and checks it against the
    /// picture saved in `reference`. iced draws tooltips as overlays that
    /// widget queries do not reach, so the label is checked on the drawing.
    fn hovered_box_matches(label: &str, reference: &std::path::Path) -> bool {
        let palette = UiPalette::otty_dark();
        let window = Size::new(300.0, 120.0);
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            window,
            CommandBox::new(SHORT, glyph::plus(), label, &palette)
                .on_copy(Message::Copy),
        );

        move_pointer(&mut ui, copy_icon_at(window.width));

        ui.snapshot(&Theme::Dark)
            .expect("the box should draw")
            .matches_hash(reference)
            .expect("reference should be readable")
    }

    #[test]
    fn given_pointer_on_copy_icon_then_the_label_shows_as_a_tooltip() {
        let folder = TempDir::new("command-box");
        let reference = folder.path().join("tooltip");

        assert!(hovered_box_matches("Copy", &reference));
        assert!(!hovered_box_matches("Another text", &reference));
    }

    #[test]
    fn given_any_command_and_width_then_the_box_has_one_line_height() {
        for width in [460.0, 300.0, 200.0, 120.0] {
            for command in [SHORT, LONG] {
                let mut ui =
                    box_in_window(command, Size::new(width, NARROW.height));

                let under = ui.find(BELOW).expect("the text should show");
                let line = ui.find(command).expect("the command should show");

                assert_eq!(
                    under.bounds().y,
                    BOX_HEIGHT,
                    "{command} at {width}"
                );
                assert_eq!(
                    line.bounds().height,
                    LINE_HEIGHT,
                    "{command} at {width}"
                );
            }
        }
    }

    #[test]
    fn given_long_command_in_narrow_box_then_it_stays_whole_and_scrolls() {
        let mut ui = box_in_window(LONG, NARROW);

        let line = ui.find(LONG).expect("the exact text should show");
        let (offset, content, showing) = scroller(&mut ui);

        assert_eq!(offset, 0.0);
        assert_eq!(content, line.bounds().width);
        assert!(content > showing, "{content} should not fit {showing}");
    }

    #[test]
    fn given_long_command_then_only_the_text_scrolls_and_the_icon_stays_clear()
    {
        let mut ui = box_in_window(LONG, NARROW);
        let (_, _, showing) = scroller(&mut ui);

        // The icon, the gap and the paddings stay out of the scroller.
        assert_eq!(
            showing,
            NARROW.width
                - PADDING_LEFT
                - spacing::SPACE_2
                - size::CONTROL_SM
                - PADDING_OTHER
        );
    }

    #[test]
    fn given_short_command_then_it_has_nothing_to_scroll() {
        let mut ui = box_in_window(SHORT, NARROW);
        move_pointer(&mut ui, OVER_TEXT);

        let statuses = wheel(&mut ui, -3.0, 0.0);
        let (offset, content, showing) = scroller(&mut ui);

        assert!(content <= showing, "{content} should fit {showing}");
        assert_eq!(offset, 0.0);
        assert_eq!(statuses, vec![event::Status::Ignored]);
    }

    /// The box as a short command should look: the text, the icon and no bar,
    /// without any scroller.
    fn plain_box<'a>(command: &'a str) -> Element<'a, Message> {
        let palette = UiPalette::otty_dark();
        let background = palette.surface_chrome();

        container(
            row![
                text(command)
                    .size(COMMAND_SIZE)
                    .font(fonts::MONO)
                    .color(palette.text_strong())
                    .wrapping(Wrapping::None)
                    .width(Length::Fill),
                IconButton::new(glyph::plus(), "Copy", &palette)
                    .size(IconButtonSize::Small)
                    .on_press(Message::Copy),
            ]
            .spacing(spacing::SPACE_2)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding(Padding {
            top: PADDING_OTHER,
            right: PADDING_OTHER,
            bottom: PADDING_OTHER,
            left: PADDING_LEFT,
        })
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_SM.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
    }

    #[test]
    fn given_short_command_then_it_draws_without_a_bar_like_a_plain_box() {
        let folder = TempDir::new("command-box-plain");
        let reference = folder.path().join("short");
        let window = Size::new(300.0, 120.0);

        let plain_matches = Simulator::with_size(
            iced::Settings::default(),
            window,
            plain_box(SHORT),
        )
        .snapshot(&Theme::Dark)
        .expect("the box should draw")
        .matches_hash(&reference)
        .expect("reference should be readable");
        let box_matches = Simulator::with_size(
            iced::Settings::default(),
            window,
            command_box(SHORT),
        )
        .snapshot(&Theme::Dark)
        .expect("the box should draw")
        .matches_hash(&reference)
        .expect("reference should be readable");

        assert!(plain_matches);
        assert!(box_matches);
    }

    #[test]
    fn given_long_command_when_wheel_goes_sideways_then_the_text_scrolls() {
        let mut ui = box_in_window(LONG, NARROW);
        move_pointer(&mut ui, OVER_TEXT);

        let _ = wheel(&mut ui, -1.0, 0.0);

        let (offset, _, _) = scroller(&mut ui);
        assert!(offset > 0.0, "scrolled {offset}");
    }

    #[test]
    fn given_long_command_when_shift_wheel_then_the_text_scrolls() {
        let mut ui = box_in_window(LONG, NARROW);
        move_pointer(&mut ui, OVER_TEXT);
        let _ = ui.simulate([iced::Event::Keyboard(
            iced::keyboard::Event::ModifiersChanged(Modifiers::SHIFT),
        )]);

        let _ = wheel(&mut ui, 0.0, -1.0);

        let (offset, _, _) = scroller(&mut ui);
        assert!(offset > 0.0, "scrolled {offset}");
    }

    /// A plain vertical wheel has no meaning in a box of one line, so the
    /// text stays where it is, also once it has been scrolled.
    #[test]
    fn given_long_command_when_wheel_goes_up_or_down_then_the_text_stays() {
        let mut ui = box_in_window(LONG, NARROW);
        move_pointer(&mut ui, OVER_TEXT);

        let _ = wheel(&mut ui, 0.0, -3.0);
        let _ = wheel(&mut ui, 0.0, 3.0);
        let (at_start, _, _) = scroller(&mut ui);
        let _ = wheel(&mut ui, -1.0, 0.0);
        let (scrolled, _, _) = scroller(&mut ui);
        let _ = wheel(&mut ui, 0.0, -3.0);
        let (after_wheel, _, _) = scroller(&mut ui);

        assert_eq!(at_start, 0.0);
        assert!(scrolled > 0.0);
        assert_eq!(after_wheel, scrolled);
    }

    #[test]
    fn given_long_command_scrolled_to_its_end_then_copy_still_sends_its_message()
     {
        let mut ui = box_in_window(LONG, NARROW);
        move_pointer(&mut ui, OVER_TEXT);
        let _ = wheel(&mut ui, -100.0, 0.0);
        let (offset, content, showing) = scroller(&mut ui);
        assert!(
            (offset - (content - showing)).abs() < 1.0,
            "{offset} should be the end of {content} in {showing}"
        );

        move_pointer(&mut ui, copy_icon_at(NARROW.width));
        let _ = ui.simulate(simulator::click());

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::Copy]);
    }

    #[test]
    fn given_long_command_when_its_bar_is_dragged_then_the_text_scrolls() {
        let mut ui = box_in_window(LONG, NARROW);
        // The last pixels of the scroller, where the bar is.
        let on_bar_y = PADDING_OTHER + size::CONTROL_SM - 2.0;

        move_pointer(&mut ui, Point::new(20.0, on_bar_y));
        let _ = ui.simulate([iced::Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))]);
        move_pointer(&mut ui, Point::new(100.0, on_bar_y));
        let _ = ui.simulate([iced::Event::Mouse(
            mouse::Event::ButtonReleased(mouse::Button::Left),
        )]);

        let (offset, _, _) = scroller(&mut ui);
        assert!(offset > 0.0, "scrolled {offset}");
    }
}
