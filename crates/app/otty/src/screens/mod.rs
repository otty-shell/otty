//! The screen of each mode: which feature fills the left panel, the main
//! area, the right panel, the status bar and the mode overlays. `view.rs`
//! picks one by the active mode.
//!
//! Each feature fills its places here, through the place traits of
//! `otty-app-slots`.

pub(crate) mod containers;
pub(crate) mod start_card;
pub(crate) mod terminal;

use iced::widget::container;
use iced::{Element, Length};

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
