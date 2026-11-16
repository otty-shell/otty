//! `BlockOutcome`: how the command of a terminal block ended, as the
//! [`BlockStatusLine`] and the [`BlockHeader`] show it, and the parts both
//! draw alike: the outcome icon, the small labels and their widths.
//!
//! [`BlockStatusLine`]: super::block_status_line::BlockStatusLine
//! [`BlockHeader`]: super::block_header::BlockHeader

use iced::widget::text::Wrapping;
use iced::widget::{svg, text};
use iced::{Color, Element, Pixels};

use super::ellipsis_text::text_width;
use super::glyph;
use super::spinner::Spinner;
use crate::palette::UiPalette;
use crate::{fonts, typography};

/// Side of the outcome icon.
const ICON_SIZE: f32 = 12.0;

/// How the command of a block ended, which picks the icon of its status.
///
/// The icons are the kit's own: the [`glyph::check`], the
/// [`glyph::cross`] and the [`Spinner`].
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::block_outcome::BlockOutcome;
///
/// let exit_code = 101;
/// let outcome = match exit_code {
///     0 => BlockOutcome::Ok,
///     _ => BlockOutcome::Failed,
/// };
///
/// assert_eq!(outcome, BlockOutcome::Failed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOutcome {
    /// The command ended with exit code 0: the kit check, in `status-ok`.
    Ok,
    /// The command ended with another exit code: the kit cross, in
    /// `status-error`.
    Failed,
    /// The command still runs: the kit [`Spinner`].
    Running,
    /// The command ended, but the shell sent no exit code: no icon.
    Unknown,
}

/// The 12 px icon of `outcome`: a check, a cross or a spinner; `None` for
/// [`BlockOutcome::Unknown`].
pub(super) fn outcome_icon<'a, Message: 'a>(
    outcome: &BlockOutcome,
    palette: &UiPalette,
) -> Option<Element<'a, Message>> {
    match outcome {
        BlockOutcome::Ok => Some(icon(glyph::check(), palette.status_ok())),
        BlockOutcome::Failed => {
            Some(icon(glyph::cross(), palette.status_error()))
        },
        BlockOutcome::Running => {
            Some(Spinner::new(palette).size(ICON_SIZE).into())
        },
        _ => None,
    }
}

/// The width the icon of `outcome` takes; `None` when it has none.
pub(super) fn outcome_icon_width(outcome: &BlockOutcome) -> Option<f32> {
    match outcome {
        BlockOutcome::Unknown => None,
        _ => Some(ICON_SIZE),
    }
}

/// `content` in the small UI text, in `color`, on one line.
pub(super) fn label<'a, Message: 'a>(
    content: String,
    color: Color,
) -> Element<'a, Message> {
    let style = typography::SMALL;

    text(content)
        .size(style.size())
        .line_height(Pixels(style.line_height()))
        .font(fonts::UI)
        .wrapping(Wrapping::None)
        .color(color)
        .into()
}

/// The width of `content` drawn as a [`label`].
pub(super) fn label_width(content: &str) -> f32 {
    text_width(content, typography::SMALL.size(), fonts::UI)
}

fn icon<'a, Message: 'a>(
    handle: svg::Handle,
    color: Color,
) -> Element<'a, Message> {
    svg(handle)
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(move |_, _| svg::Style { color: Some(color) })
        .into()
}
