//! The sidebar: the left and right panels around the main area, their
//! widths, focus mode with its note, and the section header menus.

mod event;
mod focus_note;
mod model;
mod reducer;
mod side_panel;
mod state;
pub(crate) mod types;
pub(crate) mod view;

use iced::Subscription;
use iced::widget::pane_grid;

pub(crate) use self::event::{SidebarIntent, SidebarOutcome};
pub(crate) use self::model::SidebarViewModel;
use self::state::SidebarState;

/// Sidebar widget owning the left and right panels around the main area,
/// focus mode and the header menus.
pub(crate) struct SidebarWidget {
    state: SidebarState,
}

impl SidebarWidget {
    /// Build a read-only view model for the presentation layer.
    pub(crate) fn vm(&self) -> SidebarViewModel {
        SidebarViewModel::from_state(&self.state)
    }

    /// Return read-only access to pane-grid slots for rendering.
    pub(crate) fn panes(&self) -> &pane_grid::State<types::SidebarPane> {
        self.state.panes()
    }

    /// Return whether a header menu is currently open.
    pub(crate) fn has_header_menu_open(&self) -> bool {
        self.state.header_menu().is_some()
    }

    /// Return whether the focus mode note is on screen, so frames must keep
    /// coming until it fades out.
    pub(crate) fn is_focus_note_shown(&self) -> bool {
        self.state.focus_note_opacity().is_some()
    }

    /// Share of the window width both shown side panels take together; the
    /// main area gets the rest, and the terminal panes their grid in it.
    pub(crate) fn side_panels_ratio(&self) -> f32 {
        self.state.left_ratio() + self.state.right_ratio()
    }

    /// Return where the pointer was last over the main area; a pane menu
    /// and the context menu of the launches open there.
    pub(crate) fn cursor(&self) -> iced::Point {
        self.state.cursor()
    }

    /// Return whether a panel edge was dragged a moment ago: the Launches
    /// tree keeps no hover until the drag settles.
    pub(crate) fn is_resizing(&self) -> bool {
        self.state.is_resizing()
    }

    /// Construct sidebar with default layout state.
    pub(crate) fn new() -> Self {
        Self {
            state: SidebarState::default(),
        }
    }
}

#[cfg(test)]
impl SidebarWidget {
    /// Share of the window width the left panel takes now.
    pub(crate) fn left_ratio(&self) -> f32 {
        self.state.left_ratio()
    }

    /// Share of the window width the right panel takes now.
    pub(crate) fn right_ratio(&self) -> f32 {
        self.state.right_ratio()
    }
}

impl otty_app_slots::Feature for SidebarWidget {
    type Intent = SidebarIntent;
    type Outcome = SidebarOutcome;
    type Ctx<'a> = ();

    fn reduce(
        &mut self,
        intent: SidebarIntent,
        _ctx: (),
    ) -> Vec<SidebarOutcome> {
        reducer::reduce(&mut self.state, intent)
    }

    /// Frames for the focus mode note while it stays and fades out.
    fn subscription(&self) -> Subscription<SidebarIntent> {
        if !self.is_focus_note_shown() {
            return Subscription::none();
        }

        iced::window::frames().map(SidebarIntent::FocusNoteFrame)
    }
}
