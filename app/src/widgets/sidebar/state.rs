use std::time::{Duration, Instant};

use iced::Point;
use iced::widget::pane_grid;

use super::focus_note::FocusNote;
use super::side_panel::SidePanel;
use super::types::{HeaderMenu, OpenHeaderMenu, SidebarPane};

/// Smallest share of the window width the main area keeps when a side
/// panel opens or is dragged wider.
const MIN_MAIN_RATIO: f32 = 0.2;
const SIDEBAR_RESIZE_HOVER_SUPPRESS_MS: u64 = 200;

/// Internal runtime state for both side panels, focus mode and the open header menu.
///
/// The pane grid is `left | (main | right)`. Panel widths are kept as shares
/// of the window width; the two split ratios are derived from them after
/// every change.
#[derive(Debug)]
pub(super) struct SidebarState {
    left: SidePanel,
    right: SidePanel,
    focus_mode: bool,
    /// Note about focus mode, shown for a moment after entering it.
    focus_note: Option<FocusNote>,
    /// Sessions section of the left panel is open. View state, not saved.
    sessions_open: bool,
    /// Launches section of the left panel is open. View state, not saved.
    launches_open: bool,
    left_split: pane_grid::Split,
    right_split: pane_grid::Split,
    panes: pane_grid::State<SidebarPane>,
    header_menu: Option<OpenHeaderMenu>,
    cursor: Point,
    last_resize_at: Option<Instant>,
}

impl SidebarState {
    /// Whether the left panel is shown (open and not hidden by focus mode).
    pub(super) fn is_left_open(&self) -> bool {
        self.left.is_open() && !self.focus_mode
    }

    /// Whether the right panel is shown (open and not hidden by focus mode).
    pub(super) fn is_right_open(&self) -> bool {
        self.right.is_open() && !self.focus_mode
    }

    /// Whether focus mode hides both panels.
    pub(super) fn is_focus_mode(&self) -> bool {
        self.focus_mode
    }

    /// How visible the focus mode note is now; `None` once it has faded out
    /// or outside focus mode.
    pub(super) fn focus_note_opacity(&self) -> Option<f32> {
        self.focus_note.map(|note| note.opacity())
    }

    /// Whether the Sessions section of the left panel is open.
    pub(super) fn is_sessions_open(&self) -> bool {
        self.sessions_open
    }

    /// Whether the Launches section of the left panel is open.
    pub(super) fn is_launches_open(&self) -> bool {
        self.launches_open
    }

    /// Share of the window width the left panel takes now.
    pub(super) fn left_ratio(&self) -> f32 {
        if self.focus_mode {
            0.0
        } else {
            self.left.width_ratio()
        }
    }

    /// Share of the window width the right panel takes now.
    pub(super) fn right_ratio(&self) -> f32 {
        if self.focus_mode {
            0.0
        } else {
            self.right.width_ratio()
        }
    }

    /// The `left | main | right` pane grid the view draws.
    pub(super) fn panes(&self) -> &pane_grid::State<SidebarPane> {
        &self.panes
    }

    /// Last cursor position over the workspace; header menus open here.
    pub(super) fn cursor(&self) -> Point {
        self.cursor
    }

    /// The header menu that is open, if any, with where it opened.
    pub(super) fn header_menu(&self) -> Option<OpenHeaderMenu> {
        self.header_menu
    }

    /// Whether a panel edge was dragged a moment ago, so hover effects
    /// wait until the drag settles.
    pub(super) fn is_resizing(&self) -> bool {
        self.last_resize_at
            .map(|last| {
                last.elapsed()
                    <= Duration::from_millis(SIDEBAR_RESIZE_HOVER_SUPPRESS_MS)
            })
            .unwrap_or(false)
    }

    /// Show the left panel if it is hidden, hide it if it is shown. In focus
    /// mode the panel is hidden, so this leaves focus mode and shows it.
    /// The panel shows at the minimum width; a right panel too wide for it
    /// shrinks so the main area keeps its minimum share.
    pub(super) fn toggle_left(&mut self) {
        let show = !self.is_left_open();

        self.focus_mode = false;
        self.focus_note = None;
        if show {
            self.left.open();
            self.right.shrink_to(max_ratio_beside(self.left_ratio()));
        } else {
            self.left.close();
        }

        self.sync_panes();
    }

    /// Show the right panel if it is hidden, hide it if it is shown. In
    /// focus mode the panel is hidden, so this leaves focus mode and shows
    /// it. The panel shows at the minimum width; a left panel too wide for
    /// it shrinks so the main area keeps its minimum share.
    pub(super) fn toggle_right(&mut self) {
        let show = !self.is_right_open();

        self.focus_mode = false;
        self.focus_note = None;
        if show {
            self.right.open();
            self.left.shrink_to(max_ratio_beside(self.right_ratio()));
        } else {
            self.right.close();
        }

        self.sync_panes();
    }

    /// Enter or leave focus mode. Focus mode hides both panels without
    /// closing them, so leaving it shows what was open before. Entering it
    /// shows the focus mode note, which fades out on its own.
    pub(super) fn toggle_focus_mode(&mut self) {
        self.focus_mode = !self.focus_mode;
        self.focus_note = self.focus_mode.then(FocusNote::new);

        self.sync_panes();
    }

    /// Move the focus mode note to the frame at `now`, removing it once it
    /// has faded out.
    pub(super) fn advance_focus_note(&mut self, now: Instant) {
        let Some(note) = self.focus_note.as_mut() else {
            return;
        };

        if !note.advance(now) {
            self.focus_note = None;
        }
    }

    /// Open the Sessions section if it is closed, close it if it is open.
    pub(super) fn toggle_sessions(&mut self) {
        self.sessions_open = !self.sessions_open;
    }

    /// Open the Launches section if it is closed, close it if it is open.
    pub(super) fn toggle_launches(&mut self) {
        self.launches_open = !self.launches_open;
    }

    /// Make the Launches section visible: leave focus mode, show the left
    /// panel at the minimum width if it is closed (a right panel too wide
    /// for it shrinks, like with the panel button), and open the section.
    /// Parts already shown stay as they are. Returns whether the main area
    /// changed size.
    pub(super) fn reveal_launches(&mut self) -> bool {
        self.launches_open = true;

        if self.is_left_open() {
            return false;
        }

        self.focus_mode = false;
        self.focus_note = None;
        if !self.left.is_open() {
            self.left.open();
            self.right.shrink_to(max_ratio_beside(self.left_ratio()));
        }

        self.sync_panes();
        true
    }

    /// Remember the cursor position over the workspace.
    pub(super) fn update_cursor(&mut self, position: Point) {
        self.cursor = position;
    }

    /// Open a header menu at the cursor. Only one can be open, so this
    /// replaces any other.
    pub(super) fn open_header_menu(&mut self, menu: HeaderMenu) {
        self.header_menu = Some(OpenHeaderMenu::new(menu, self.cursor));
    }

    /// Close the open header menu, if any.
    pub(super) fn dismiss_header_menu(&mut self) {
        self.header_menu = None;
    }

    /// Note that a panel edge is being dragged now.
    pub(super) fn mark_resizing(&mut self) {
        self.last_resize_at = Some(Instant::now());
    }

    /// Apply a drag of one of the two panel edges. Returns whether the main
    /// area changed size. Drags in focus mode change nothing.
    pub(super) fn apply_resize(
        &mut self,
        event: pane_grid::ResizeEvent,
    ) -> bool {
        if self.focus_mode {
            return false;
        }

        let changed = if event.split == self.left_split {
            let was_open = self.left.is_open();
            let max_ratio = max_ratio_beside(self.right_ratio());
            let changed = self.left.drag_to(event.ratio, max_ratio);

            // A drag that opens the panel makes room for it, as the button does.
            if !was_open {
                self.right.shrink_to(max_ratio_beside(self.left_ratio()));
            }

            changed
        } else if event.split == self.right_split {
            // The pane grid gives the edge position inside the area right of
            // the left panel; the right panel is what lies past it.
            let was_open = self.right.is_open();
            let rest = 1.0 - self.left_ratio();
            let right_ratio = (1.0 - event.ratio) * rest;
            let max_ratio = max_ratio_beside(self.left_ratio());
            let changed = self.right.drag_to(right_ratio, max_ratio);

            // A drag that opens the panel makes room for it, as the button does.
            if !was_open {
                self.left.shrink_to(max_ratio_beside(self.right_ratio()));
            }

            changed
        } else {
            false
        };

        self.sync_panes();
        changed
    }

    /// Set both pane grid splits from the panel widths shown now.
    fn sync_panes(&mut self) {
        let left = self.left_ratio();
        let right = self.right_ratio();
        let rest = 1.0 - left;
        let main_share = if rest > 0.0 {
            ((rest - right) / rest).clamp(0.0, 1.0)
        } else {
            0.0
        };

        self.panes.resize(self.left_split, left);
        self.panes.resize(self.right_split, main_share);
    }
}

impl Default for SidebarState {
    fn default() -> Self {
        let (mut panes, left_pane) = pane_grid::State::new(SidebarPane::Left);
        let (main_pane, left_split) = panes
            .split(pane_grid::Axis::Vertical, left_pane, SidebarPane::Main)
            .expect("sidebar left split failed");
        let (_right_pane, right_split) = panes
            .split(pane_grid::Axis::Vertical, main_pane, SidebarPane::Right)
            .expect("sidebar right split failed");

        let mut state = Self {
            left: SidePanel::new(true),
            right: SidePanel::new(true),
            focus_mode: false,
            focus_note: None,
            sessions_open: true,
            launches_open: true,
            left_split,
            right_split,
            panes,
            header_menu: None,
            cursor: Point::ORIGIN,
            last_resize_at: None,
        };
        state.sync_panes();

        state
    }
}

/// Widest share a panel may take next to the other panel's `other_ratio`,
/// so the main area keeps its minimum share.
fn max_ratio_beside(other_ratio: f32) -> f32 {
    (1.0 - MIN_MAIN_RATIO - other_ratio).max(0.0)
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use iced::time::{Duration, Instant};
    use iced::widget::pane_grid;

    use super::{MIN_MAIN_RATIO, SidebarState};
    use crate::widgets::sidebar::side_panel::SIDE_PANEL_MIN_RATIO;
    use crate::widgets::sidebar::types::{HeaderMenu, OpenHeaderMenu};

    /// Ratios of the two splits: (left | rest) and, inside the rest,
    /// (main | right).
    fn split_ratios(state: &SidebarState) -> (f32, f32) {
        let pane_grid::Node::Split { ratio: left, b, .. } =
            state.panes().layout()
        else {
            panic!("sidebar should split the left panel from the rest");
        };
        let pane_grid::Node::Split { ratio: main, .. } = b.as_ref() else {
            panic!("sidebar should split the main area from the right panel");
        };

        (*left, *main)
    }

    /// Drag the edge of the left panel to `ratio` of the window width.
    fn drag_left_edge(state: &mut SidebarState, ratio: f32) {
        let split = state.left_split;
        state.apply_resize(pane_grid::ResizeEvent { split, ratio });
    }

    /// Drag the edge of the right panel to `ratio` of the area right of the
    /// left panel, as the pane grid reports it.
    fn drag_right_edge(state: &mut SidebarState, ratio: f32) {
        let split = state.right_split;
        state.apply_resize(pane_grid::ResizeEvent { split, ratio });
    }

    /// Share of the window width left for the main area.
    fn main_ratio(state: &SidebarState) -> f32 {
        1.0 - state.left_ratio() - state.right_ratio()
    }

    #[test]
    fn new_sidebar_opens_both_panels_at_the_minimum_width() {
        let state = SidebarState::default();

        // Main gets what the two panels leave, as a share of the area right
        // of the left panel.
        let rest = 1.0 - SIDE_PANEL_MIN_RATIO;
        let expected_main = (rest - SIDE_PANEL_MIN_RATIO) / rest;
        let (left, main) = split_ratios(&state);
        assert_eq!(left, SIDE_PANEL_MIN_RATIO);
        assert!((main - expected_main).abs() < 1e-6);
    }

    #[test]
    fn closed_right_panel_gives_main_the_area_right_of_the_left() {
        let mut state = SidebarState::default();

        state.toggle_right();

        assert_eq!(split_ratios(&state), (SIDE_PANEL_MIN_RATIO, 1.0));
    }

    #[test]
    fn right_panel_opened_beside_a_widest_left_panel_takes_the_minimum_width() {
        let mut state = SidebarState::default();
        state.toggle_right();
        drag_left_edge(&mut state, 1.0);
        assert!((state.left_ratio() - (1.0 - MIN_MAIN_RATIO)).abs() < 1e-6);

        state.toggle_right();

        assert!(state.is_left_open());
        assert!(state.is_right_open());
        assert_eq!(state.right_ratio(), SIDE_PANEL_MIN_RATIO);
        let shrunk_left = 1.0 - MIN_MAIN_RATIO - SIDE_PANEL_MIN_RATIO;
        assert!((state.left_ratio() - shrunk_left).abs() < 1e-6);
        assert!((main_ratio(&state) - MIN_MAIN_RATIO).abs() < 1e-6);
    }

    #[test]
    fn left_panel_opened_beside_a_widest_right_panel_takes_the_minimum_width() {
        let mut state = SidebarState::default();
        state.toggle_left();
        drag_right_edge(&mut state, 0.0);
        assert!((state.right_ratio() - (1.0 - MIN_MAIN_RATIO)).abs() < 1e-6);

        state.toggle_left();

        assert!(state.is_left_open());
        assert!(state.is_right_open());
        assert_eq!(state.left_ratio(), SIDE_PANEL_MIN_RATIO);
        let shrunk_right = 1.0 - MIN_MAIN_RATIO - SIDE_PANEL_MIN_RATIO;
        assert!((state.right_ratio() - shrunk_right).abs() < 1e-6);
        assert!((main_ratio(&state) - MIN_MAIN_RATIO).abs() < 1e-6);
    }

    #[test]
    fn launches_revealed_beside_a_widest_right_panel_open_the_left_at_the_minimum_width()
     {
        let mut state = SidebarState::default();
        state.toggle_left();
        drag_right_edge(&mut state, 0.0);

        let resized = state.reveal_launches();

        assert!(resized);
        assert!(state.is_left_open());
        assert_eq!(state.left_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!((main_ratio(&state) - MIN_MAIN_RATIO).abs() < 1e-6);
    }

    #[test]
    fn right_panel_dragged_open_beside_a_widest_left_panel_takes_the_minimum_width()
     {
        let mut state = SidebarState::default();
        state.toggle_right();
        drag_left_edge(&mut state, 1.0);

        drag_right_edge(&mut state, 0.0);

        assert!(state.is_right_open());
        assert_eq!(state.right_ratio(), SIDE_PANEL_MIN_RATIO);
        assert!((main_ratio(&state) - MIN_MAIN_RATIO).abs() < 1e-6);
    }

    #[test]
    fn right_panel_opened_beside_a_wide_left_panel_with_room_leaves_it_alone() {
        let mut state = SidebarState::default();
        state.toggle_right();
        drag_left_edge(&mut state, 0.5);

        state.toggle_right();

        assert_eq!(state.left_ratio(), 0.5);
        assert_eq!(state.right_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn focus_mode_gives_the_whole_width_to_main() {
        let mut state = SidebarState::default();

        state.toggle_focus_mode();

        assert_eq!(split_ratios(&state), (0.0, 1.0));
    }

    #[test]
    fn entering_focus_mode_shows_the_note_fully() {
        let mut state = SidebarState::default();

        state.toggle_focus_mode();

        assert_eq!(state.focus_note_opacity(), Some(1.0));
    }

    #[test]
    fn leaving_focus_mode_hides_the_note() {
        let mut state = SidebarState::default();
        state.toggle_focus_mode();

        state.toggle_focus_mode();

        assert_eq!(state.focus_note_opacity(), None);
    }

    #[test]
    fn showing_a_panel_from_focus_mode_hides_the_note() {
        let mut state = SidebarState::default();
        state.toggle_focus_mode();

        state.toggle_left();

        assert_eq!(state.focus_note_opacity(), None);
    }

    #[test]
    fn note_starts_counting_at_its_first_frame() {
        let first_frame = Instant::now() + Duration::from_secs(60);
        let mut state = SidebarState::default();
        state.toggle_focus_mode();

        state.advance_focus_note(first_frame);

        assert_eq!(state.focus_note_opacity(), Some(1.0));
    }

    #[test]
    fn note_is_removed_once_it_has_faded_out() {
        let first_frame = Instant::now();
        let mut state = SidebarState::default();
        state.toggle_focus_mode();
        state.advance_focus_note(first_frame);

        state.advance_focus_note(first_frame + Duration::from_secs(10));

        assert!(state.is_focus_mode());
        assert_eq!(state.focus_note_opacity(), None);
    }

    #[test]
    fn frame_outside_focus_mode_shows_no_note() {
        let mut state = SidebarState::default();

        state.advance_focus_note(Instant::now());

        assert_eq!(state.focus_note_opacity(), None);
    }

    #[test]
    fn resize_just_marked_counts_as_resizing() {
        let mut state = SidebarState::default();

        state.mark_resizing();

        assert!(state.is_resizing());
    }

    #[test]
    fn header_menu_opens_at_the_cursor() {
        let mut state = SidebarState::default();
        let cursor = Point::new(12.0, 34.0);

        state.update_cursor(cursor);
        state.open_header_menu(HeaderMenu::Launches);

        assert_eq!(
            state.header_menu(),
            Some(OpenHeaderMenu::new(HeaderMenu::Launches, cursor))
        );
    }
}
