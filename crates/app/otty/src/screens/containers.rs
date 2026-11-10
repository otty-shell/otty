//! The screen of Containers mode. The containers feature (issues 13 and 14)
//! fills all of it: the engine tree in the left panel, the lists and forms
//! in the main area, the inspector in the right panel, the status bar and
//! the row menu and dialogs.

use iced::Element;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::status_bar::StatusBar;

use super::empty_place;
use crate::message::AppEvent;

/// Left panel of Containers mode under the switcher: the engine tree.
pub(crate) fn left_panel<'a>(_theme: ThemeProps<'a>) -> Element<'a, AppEvent> {
    empty_place()
}

/// Main area of Containers mode: the lists, the forms and the no-engine
/// cards.
pub(crate) fn main<'a>(_theme: ThemeProps<'a>) -> Element<'a, AppEvent> {
    empty_place()
}

/// Right panel of Containers mode: the inspector.
pub(crate) fn right_panel<'a>(_theme: ThemeProps<'a>) -> Element<'a, AppEvent> {
    empty_place()
}

/// Status bar of Containers mode: the bar itself, so the main area keeps
/// its place; the containers feature adds its items.
pub(crate) fn status_bar<'a>(theme: ThemeProps<'a>) -> Element<'a, AppEvent> {
    StatusBar::new(theme.theme.ui()).into()
}

/// Overlays of Containers mode, bottom to top: the row menu, the
/// confirmation dialog and the create dialog. None of them is here yet.
pub(crate) fn overlays<'a>(
    _theme: ThemeProps<'a>,
) -> Vec<Element<'a, AppEvent>> {
    Vec::new()
}
