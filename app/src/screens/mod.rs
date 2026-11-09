//! The screen of each mode: what fills the panels, the main area, the status
//! bar and the mode overlays. `view.rs` picks one by the active mode.
//!
//! This module also holds what the root view and the mode screens share:
//! what fills the main area, and how an overlay lies in it.

pub(crate) mod containers;
pub(crate) mod terminal;

use iced::widget::container;
use iced::{Element, Length};
use otty_ui_kit::components::screen_area::ScreenArea;

use crate::layout;
use crate::router::AppEvent;
use crate::theme::ThemeProps;
use crate::widgets::containers::types::NoEngineCase;

/// Which screen fills the main area. Each value is one screen to the user: a
/// change of value starts at the top left, and the same value built again keeps
/// its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenId {
    /// The start card: no tab is open.
    StartCard,
    /// The content of a tab that is not a terminal: a launch editor.
    Tab(u64),
    /// The Run container form.
    RunForm,
    /// The card while the engine search runs.
    Searching,
    /// The no-engine card of one diagnosis.
    NoEngine(NoEngineCase),
}

/// What fills the main area. A function that chooses it has to say which of the
/// two it is, so a new screen can not miss the rule of [`ScreenArea`]: anything
/// that is not a terminal or a table is a [`MainContent::Screen`].
pub(crate) enum MainContent<'a> {
    /// A screen. It keeps a minimum width and scrolls when there is too little
    /// room.
    Screen {
        id: ScreenId,
        content: Element<'a, AppEvent>,
    },
    /// The panes of a terminal or the table of a Containers list. They scroll on
    /// their own and fill a height they are given: a table's sticky header and
    /// its rows can not work in an outer scroller, where the height has no end.
    ScrollsOnItsOwn(Element<'a, AppEvent>),
}

impl<'a> MainContent<'a> {
    /// The element that fills the main area: a screen in its [`ScreenArea`],
    /// or content that scrolls on its own as it is.
    pub(crate) fn into_element(
        self,
        theme_props: ThemeProps<'a>,
    ) -> Element<'a, AppEvent> {
        match self {
            Self::Screen { id, content } => {
                ScreenArea::new(id, content, theme_props.theme.ui()).into()
            },
            Self::ScrollsOnItsOwn(content) => content,
        }
    }
}

/// Lay `overlay` out in the main area of a layer over the whole window: under
/// the header and over the status bar. It gets the size of the main area, and
/// the pointer positions it reports are in the main area too.
pub(crate) fn over_main_area(
    overlay: Element<'_, AppEvent>,
) -> Element<'_, AppEvent> {
    container(overlay)
        .padding(layout::main_area_padding())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
