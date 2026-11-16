//! `Tag`: a short muted label, such as `20 lines hidden`, that stands in a
//! line next to other text.
//!
//! Not to be confused with the search filters of [`TagSearch`], which
//! [`TagRow`] draws as chips.
//!
//! [`TagSearch`]: super::tag_search::TagSearch
//! [`TagRow`]: super::tag_row::TagRow

use iced::widget::text::Wrapping;
use iced::widget::{container, text};
use iced::{Border, Element, Length, Pixels, alignment};

use super::ellipsis_text::text_width;
use crate::palette::UiPalette;
use crate::{fonts, radius, typography};

/// Space at the left and the right of the text (`padding: 0 6px`).
const PADDING_X: f32 = 6.0;
/// Free space the tag keeps above and below it in its line, so it does not
/// touch what is above or below the line.
const SPACE_Y: f32 = 2.0;
/// The highest the tag gets, in a high line such as the 22 px status line of
/// a terminal block, where 3 px stay free above and below it. In a lower
/// line it takes what the line leaves after the free space above and below.
const MAX_HEIGHT: f32 = 16.0;
/// The text size: the caption size, 11.5 px.
const TEXT_SIZE: f32 = typography::CAPTION.size();

/// Renders `label` as a tag in the middle of a line `line_height` high.
///
/// UI font at the caption size, `text-muted` on `surface-raised`,
/// `radius-xs` corners. The tag leaves 2 px free above and below it in the
/// line and is at most 16 px high: in the 22 px status line of a terminal
/// block it is 16 px with 3 px free above and below. Its text is in the middle both ways: the
/// space above the text is the space below it, and the space at its left is
/// the space at its right. The element is `line_height` high and
/// [`tag_width`] wide; the text stays on one line.
///
/// # Example
///
/// ```
/// use iced::Element;
/// use otty_ui_kit::components::tag::tag;
/// use otty_ui_kit::palette::UiPalette;
///
/// let palette = UiPalette::otty_dark();
/// let hidden: Element<'_, ()> = tag("20 lines hidden", 16.0, &palette);
/// ```
pub fn tag<'a, Message: 'a>(
    label: impl Into<String>,
    line_height: f32,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let height = tag_height(line_height);
    let background = palette.surface_raised();

    let label = text(label.into())
        .font(fonts::UI)
        .size(TEXT_SIZE)
        .line_height(Pixels(height))
        .height(Length::Fill)
        .align_y(alignment::Vertical::Center)
        .wrapping(Wrapping::None)
        .color(palette.text_muted());
    let tag = container(label)
        .height(height)
        .padding([0.0, PADDING_X])
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_XS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        });

    container(tag)
        .height(line_height)
        .align_y(alignment::Vertical::Center)
        .into()
}

/// Returns the width of the tag of `label`, measured with the font and the
/// size [`tag`] draws it with, so a caller can tell whether it fits.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::tag::tag_width;
///
/// assert!(tag_width("20 lines hidden") > tag_width("1 line hidden"));
/// ```
pub fn tag_width(label: &str) -> f32 {
    text_width(label, TEXT_SIZE, fonts::UI) + 2.0 * PADDING_X
}

/// The height of the tag in a line `line_height` high: the line less the
/// free space above and below, at most [`MAX_HEIGHT`]. In the 16 px line of
/// a 13 px terminal font it is 12 px, in the 22 px status line 16 px.
fn tag_height(line_height: f32) -> f32 {
    (line_height - 2.0 * SPACE_Y).clamp(0.0, MAX_HEIGHT)
}

#[cfg(test)]
mod tests {
    use iced::widget::column;
    use iced::{Color, Size, mouse};
    use iced_test::Simulator;

    use super::*;
    use crate::testing::{is_alone, load_kit_fonts};

    const LABEL: &str = "3 lines hidden";
    /// Where the line starts in the window of a picture of it: the padding
    /// of the window, in pixels.
    const WINDOW_PADDING: f32 = 10.0;

    /// A picture of what was drawn, at twice the size of the window.
    struct Picture {
        pixels: Vec<u8>,
        width: usize,
    }

    impl Picture {
        /// The RGBA pixel `x` pixels from the left and `y` from the top.
        fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
            let at = (y * self.width + x) * 4;

            [
                self.pixels[at],
                self.pixels[at + 1],
                self.pixels[at + 2],
                self.pixels[at + 3],
            ]
        }

        /// The pixels of the picture, as `(x, y)`, that are `color`.
        fn points_of(&self, color: [u8; 4]) -> Vec<(usize, usize)> {
            let height = self.pixels.len() / 4 / self.width;

            (0..height)
                .flat_map(|y| (0..self.width).map(move |x| (x, y)))
                .filter(|&(x, y)| self.pixel(x, y) == color)
                .collect()
        }
    }

    /// The space around a tag in a picture of its line, in pixels at 200 %.
    #[derive(Debug)]
    struct TagSpace {
        /// Between the top of the line and the top of the tag.
        above_tag: usize,
        /// Between the bottom of the tag and the bottom of the line.
        below_tag: usize,
        /// Between the edges of the tag and the letters in it.
        left: usize,
        right: usize,
        above_text: usize,
        below_text: usize,
        /// From the top of the highest letter to the bottom of the lowest.
        text_height: usize,
    }

    /// Draw `element` in a window of `size` filled with `background`, at
    /// twice the size, as a screen with a scale of 200 % shows it.
    fn picture(
        element: Element<'_, ()>,
        size: Size,
        background: Color,
    ) -> Picture {
        use iced::advanced::renderer::{Headless, Style};
        use iced_test::runtime::user_interface::{Cache, UserInterface};

        let mut renderer = iced::futures::executor::block_on(
            <iced::Renderer as Headless>::new(
                iced::Font::DEFAULT,
                Pixels(16.0),
                Some("tiny-skia"),
            ),
        )
        .expect("a headless renderer");
        let mut ui = UserInterface::build(
            element,
            size,
            Cache::default(),
            &mut renderer,
        );
        ui.draw(
            &mut renderer,
            &iced::Theme::Dark,
            &Style {
                text_color: Color::WHITE,
            },
            mouse::Cursor::Unavailable,
        );

        let width = 2 * size.width as usize;
        let height = 2 * size.height as usize;
        let pixels = renderer.screenshot(
            iced::Size::new(width as u32, height as u32),
            2.0,
            background,
        );

        Picture { pixels, width }
    }

    /// The RGBA bytes of `color`, as a picture has them.
    fn rgba(color: Color) -> [u8; 4] {
        color.into_rgba8()
    }

    /// Draw the tag [`LABEL`] in a line `line_height` high and measure the
    /// space around it and around its letters.
    fn tag_space(palette: &UiPalette, line_height: f32) -> TagSpace {
        let line = tag(LABEL, line_height, palette);
        let window = container(line).padding(WINDOW_PADDING);
        let canvas = palette.surface_canvas();
        let size = Size::new(240.0, line_height + 2.0 * WINDOW_PADDING);
        let picture = picture(window.into(), size, canvas);
        let line_top = 2 * WINDOW_PADDING as usize;
        let line_bottom = line_top + 2 * line_height as usize - 1;

        let fill = rgba(palette.surface_raised());
        let tag = picture.points_of(fill);
        let left = tag.iter().map(|point| point.0).min().expect("a tag");
        let right = tag.iter().map(|point| point.0).max().expect("a tag");
        let top = tag.iter().map(|point| point.1).min().expect("a tag");
        let bottom = tag.iter().map(|point| point.1).max().expect("a tag");
        // The round corners are left out: they are not letters.
        let corner = 2 * radius::RADIUS_XS as usize;
        let letters: Vec<(usize, usize)> = (top..=bottom)
            .flat_map(|y| (left + corner..=right - corner).map(move |x| (x, y)))
            .filter(|&(x, y)| picture.pixel(x, y) != fill)
            .collect();
        let letter_left = letters.iter().map(|p| p.0).min().expect("letters");
        let letter_right = letters.iter().map(|p| p.0).max().expect("letters");
        let letter_top = letters.iter().map(|p| p.1).min().expect("letters");
        let letter_bottom = letters.iter().map(|p| p.1).max().expect("letters");

        TagSpace {
            above_tag: top - line_top,
            below_tag: line_bottom - bottom,
            left: letter_left - left,
            right: right - letter_right,
            above_text: letter_top - top,
            below_text: bottom - letter_bottom,
            text_height: letter_bottom - letter_top + 1,
        }
    }

    /// Owner, issue 07: the status line takes the 6 px gap above its block
    /// and is 22 px high; the tag in it is 16 px, with 3 px free above and
    /// below.
    #[test]
    fn a_tag_is_the_line_less_2_px_above_and_below_and_at_most_16_px_high() {
        assert_eq!(tag_height(16.0), 12.0);
        assert_eq!(tag_height(20.0), 16.0);
        assert_eq!(tag_height(22.0), 16.0);
        assert_eq!(tag_height(40.0), 16.0);
        assert_eq!(tag_height(3.0), 0.0);
    }

    #[test]
    fn given_a_tag_in_a_line_then_what_follows_starts_below_the_line() {
        let palette = UiPalette::otty_dark();
        let content: Element<'_, ()> =
            column![tag(LABEL, 16.0, &palette), text("below")].into();
        let mut ui = Simulator::with_size(
            iced::Settings::default(),
            Size::new(240.0, 100.0),
            content,
        );

        let below = ui.find("below").expect("the text below").bounds();

        assert_eq!(below.y, 16.0);
    }

    /// Owner, issues 11, 12 and 13: the tag keeps 2 px free above and below
    /// it in its line, so it does not touch what is above the line (a line
    /// higher than the tag gets gives more); its text is 11.5 px, as in
    /// issue 04; and it has as much space left of its text as right of it,
    /// and as much above as below. Checked in both themes in the 16 px line
    /// of the app's 13 px terminal font, in the lines of a 12 px and an
    /// 18 px font, and in the 22 px status line of issue 07, where the tag
    /// is 16 px high with 3 px free above and below it. The letters of a
    /// glyph do not start at the edge of their box, so one pixel at 200 %
    /// may differ.
    #[test]
    fn a_tag_has_room_in_its_line_and_even_space_around_its_text() {
        if !is_alone(
            "a_tag_has_room_in_its_line_and_even_space_around_its_text",
        ) {
            return;
        }
        load_kit_fonts();

        for palette in [UiPalette::otty_dark(), UiPalette::otty_light()] {
            for line_height in [15.0, 16.0, 22.0, 23.0] {
                let space = tag_space(&palette, line_height);

                if line_height == 22.0 {
                    assert_eq!(space.above_tag, 6, "{line_height}: {space:?}");
                    assert_eq!(space.below_tag, 6, "{line_height}: {space:?}");
                }

                // 2 px at 200 %, exactly while the tag is below its
                // highest.
                assert!(space.above_tag >= 4, "{line_height}: {space:?}");
                assert!(space.below_tag >= 4, "{line_height}: {space:?}");
                if line_height < 2.0 * SPACE_Y + MAX_HEIGHT {
                    assert_eq!(space.above_tag, 4, "{line_height}: {space:?}");
                    assert_eq!(space.below_tag, 4, "{line_height}: {space:?}");
                }
                // `lines hidden` reaches from the top of `l`, `h` and `d`
                // to the line the letters stand on: 8 px and more for
                // 11.5 px text, under 7 px for the 9 px text before.
                assert!(space.text_height >= 16, "{line_height}: {space:?}");
                assert!(
                    space.left.abs_diff(space.right) <= 1,
                    "{line_height}: {space:?}"
                );
                assert!(
                    space.above_text.abs_diff(space.below_text) <= 1,
                    "{line_height}: {space:?}"
                );
            }
        }
    }
}
