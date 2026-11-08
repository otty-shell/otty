use iced::widget::{row, text};
use iced::{Alignment, Element};
use otty_ui_kit::components::progress_bar::ProgressBar;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

/// The progress line of a running pull.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PullLineViewModel {
    /// "Pulling nginx:latest".
    pub(crate) text: String,
    /// How far the bar is filled, 0 to 1.
    pub(crate) fraction: f32,
    /// "15.0 MB of 47.0 MB", once the engine sent sizes.
    pub(crate) amount: Option<String>,
}

/// "Pulling nginx:latest", the bar, and the bytes so far, going on to the next
/// line when they do not fit. The pull dialog and the footer of the Run form
/// show it.
pub(crate) fn pull_line<'a, Message: 'a>(
    progress: PullLineViewModel,
    palette: &UiPalette,
) -> Element<'a, Message> {
    let hint = typography::SMALL;
    let muted = |content: String| {
        text(content)
            .size(hint.size())
            .line_height(iced::Pixels(hint.line_height()))
            .font(fonts::UI)
            .color(palette.text_muted())
    };

    let mut line = row![
        muted(progress.text),
        ProgressBar::new(progress.fraction, palette),
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center);
    if let Some(amount) = progress.amount {
        line = line.push(muted(amount).font(fonts::MONO));
    }

    line.wrap().vertical_spacing(spacing::SPACE_1).into()
}
