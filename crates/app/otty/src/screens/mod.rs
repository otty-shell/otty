//! The screen of each mode: which feature fills the left panel, the main
//! area, the right panel, the status bar and the mode overlays. `view.rs`
//! picks one by the active mode.
//!
//! Each feature fills its places here, through the place traits of
//! `otty-app-slots`.

pub(crate) mod agents;
pub(crate) mod containers;
pub(crate) mod start_card;
pub(crate) mod terminal;

use iced::widget::container;
use iced::{Element, Length};
use otty_app_domain::mode::Mode;
use otty_app_i18n as i18n;
use otty_app_modes::ModesIntent;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
use otty_ui_kit::palette::UiPalette;

use crate::layout;
use crate::message::AppEvent;

/// Lay `overlay` out in the main area of a layer over the whole window:
/// under the header and over the status bar. It gets the size of the main
/// area, and the pointer positions it reports are in the main area too.
pub(crate) fn over_main_area(
    overlay: Element<'_, AppEvent>,
) -> Element<'_, AppEvent> {
    container(overlay)
        .padding(layout::main_area_padding())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The status bar of Containers and Agents mode with its right side and
/// nothing on the left yet: the sessions button, which shows the count of
/// `sessions` and switches to Terminal mode, and the OTTY version. The mode
/// adds its own left side.
pub(crate) fn bar_with_sessions(
    palette: &UiPalette,
    sessions: usize,
) -> StatusBar<AppEvent> {
    StatusBar::new(palette)
        .right(StatusItem::button(
            i18n::status_sessions(sessions),
            AppEvent::Modes(ModesIntent::Select(Mode::Terminal)),
        ))
        .right(StatusItem::text(format!(
            "OTTY {}",
            env!("CARGO_PKG_VERSION")
        )))
}
