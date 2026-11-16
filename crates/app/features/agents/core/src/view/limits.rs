//! The limits of a profile in its details: each window with its reset,
//! the credits and the source. An older answer is drawn greyed out (spec
//! Decision 5).

use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Border, Color, Element, Length, border};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{LimitBarViewModel, LimitsViewModel};
use super::fact;

/// Width of a bar in the details.
const DETAILS_BAR_WIDTH: f32 = 200.0;
/// Width of the percent after a bar.
const PERCENT_WIDTH: f32 = 40.0;
/// Height of a bar.
const BAR_HEIGHT: f32 = 4.0;
/// From this percent on, a bar is drawn in the error color.
const NEARLY_FULL: u8 = 90;

/// The limits in the details: each window with its percent and reset, the
/// credits, the source, and why a read failed.
pub(crate) fn details_limits<'a>(
    vm: LimitsViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let color = text_color(vm.is_stale, palette);
    let heading = typography::BODY;
    let mut block = column![
        text(i18n::t(Key::AgentsLimits))
            .size(heading.size())
            .line_height(iced::Pixels(heading.line_height()))
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong())
    ]
    .spacing(spacing::SPACE_2);

    for window in vm.bars {
        block = block.push(details_bar(window, vm.is_stale, palette));
    }
    if let Some(credits) = vm.credits {
        block =
            block.push(fact(Key::AgentsFieldCredits, credits, color, palette));
    }
    if let Some(source) = vm.source {
        block = block.push(fact(
            Key::AgentsFieldLimitsSource,
            source,
            color,
            palette,
        ));
    }
    let notes = [vm.recorded, vm.earlier_failures, vm.last_read];
    for note in notes.into_iter().flatten() {
        block = block.push(small_text(note, palette.text_muted()));
    }
    if let Some(failure) = vm.failure {
        block = block.push(small_text(failure, palette.status_error()));
    }
    if let Some(hint) = vm.log_in_hint {
        block = block.push(small_text(hint, palette.text_muted()));
    }

    block.width(Length::Fill).into()
}

/// One window in the details: its name, its bar, the percent and the
/// reset time.
fn details_bar<'a>(
    window: LimitBarViewModel,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let color = text_color(is_stale, palette);
    let percent = window.percent;
    let body = typography::BODY;

    row![
        text(window.name)
            .size(body.size())
            .line_height(iced::Pixels(body.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted())
            .width(super::LABEL_WIDTH),
        bar(percent, DETAILS_BAR_WIDTH, is_stale, palette),
        small_text(format!("{percent}%"), color)
            .width(PERCENT_WIDTH)
            .align_x(Alignment::End),
        small_text(window.resets.unwrap_or_default(), palette.text_muted()),
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center)
    .into()
}

/// A thin bar filled to `percent`: muted when the answer is old, red when
/// the window is nearly used up.
fn bar<'a>(
    percent: u8,
    width: f32,
    is_stale: bool,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let track = palette.surface_raised();
    let fill = if is_stale {
        palette.text_muted()
    } else if percent >= NEARLY_FULL {
        palette.status_error()
    } else {
        palette.accent()
    };
    let shape = Border {
        radius: border::radius(BAR_HEIGHT / 2.0),
        ..Border::default()
    };

    let filled = container(Space::new())
        .width(width * f32::from(percent) / 100.0)
        .height(BAR_HEIGHT)
        .style(move |_| container::Style {
            background: Some(fill.into()),
            border: shape,
            ..container::Style::default()
        });

    container(filled)
        .width(width)
        .height(BAR_HEIGHT)
        .style(move |_| container::Style {
            background: Some(track.into()),
            border: shape,
            ..container::Style::default()
        })
        .into()
}

/// The color of the texts of the limits: muted while the answer is old.
fn text_color(is_stale: bool, palette: &UiPalette) -> Color {
    if is_stale {
        palette.text_muted()
    } else {
        palette.text_body()
    }
}

fn small_text<'a>(
    content: impl text::IntoFragment<'a>,
    color: Color,
) -> text::Text<'a> {
    let small = typography::SMALL;

    text(content)
        .size(small.size())
        .line_height(iced::Pixels(small.line_height()))
        .font(fonts::UI)
        .color(color)
}
