//! One side panel: whether it is open and how wide it is.
//!
//! Widths are shares of the window width, as before the left and right
//! panels existed. Both panels follow the same rules, so the sidebar state
//! holds two of these.

/// Smallest share an open side panel takes; side panels start at it and a
/// drag below it stops here.
pub(super) const SIDE_PANEL_MIN_RATIO: f32 = 0.16;
/// Share below which a dragged side panel collapses.
pub(super) const SIDE_PANEL_COLLAPSE_RATIO: f32 = SIDE_PANEL_MIN_RATIO * 0.5;

/// Open flag and width of one side panel.
#[derive(Debug, Clone, Copy)]
pub(super) struct SidePanel {
    open: bool,
    ratio: f32,
}

impl SidePanel {
    /// Return whether the panel is open.
    pub(super) fn is_open(&self) -> bool {
        self.open
    }

    /// Return the share of the window width the panel takes now: its width
    /// when open, zero when closed.
    pub(super) fn width_ratio(&self) -> f32 {
        if self.open { self.ratio } else { 0.0 }
    }

    /// Create a panel at the minimum width.
    pub(super) fn new(open: bool) -> Self {
        Self {
            open,
            ratio: SIDE_PANEL_MIN_RATIO,
        }
    }

    /// Open the panel at the minimum width, at most `max_ratio`. The panel
    /// buttons use this; a drag sets any other width.
    pub(super) fn open(&mut self, max_ratio: f32) {
        self.ratio = SIDE_PANEL_MIN_RATIO.min(max_ratio);
        self.open = true;
    }

    /// Close the panel.
    pub(super) fn close(&mut self) {
        self.open = false;
    }

    /// Apply a drag of the panel edge to `ratio` of the window width.
    ///
    /// An open panel follows the drag between the minimum width and
    /// `max_ratio`, and closes below the collapse width. A closed panel
    /// opens at the minimum width once the drag passes the collapse width.
    /// Returns whether the panel width changed.
    pub(super) fn drag_to(&mut self, ratio: f32, max_ratio: f32) -> bool {
        if !self.open {
            if ratio <= SIDE_PANEL_COLLAPSE_RATIO {
                return false;
            }

            self.ratio = SIDE_PANEL_MIN_RATIO.min(max_ratio);
            self.open = true;
            return true;
        }

        if ratio <= SIDE_PANEL_COLLAPSE_RATIO {
            self.open = false;
            return true;
        }

        self.ratio = ratio.max(SIDE_PANEL_MIN_RATIO).min(max_ratio);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{SIDE_PANEL_COLLAPSE_RATIO, SIDE_PANEL_MIN_RATIO, SidePanel};

    /// Widest share a panel may take when the other panel is closed.
    const MAX_RATIO: f32 = 0.8;

    #[test]
    fn new_open_panel_takes_the_minimum_width() {
        let panel = SidePanel::new(true);

        assert!(panel.is_open());
        assert_eq!(panel.width_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn new_closed_panel_takes_no_width() {
        let panel = SidePanel::new(false);

        assert!(!panel.is_open());
        assert_eq!(panel.width_ratio(), 0.0);
    }

    #[test]
    fn wide_panel_opens_again_at_the_minimum_width() {
        let mut panel = SidePanel::new(true);
        assert!(panel.drag_to(0.5, MAX_RATIO));

        panel.close();
        assert_eq!(panel.width_ratio(), 0.0);
        panel.open(MAX_RATIO);

        assert_eq!(panel.width_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn new_closed_panel_opens_at_the_minimum_width() {
        let mut panel = SidePanel::new(false);

        panel.open(MAX_RATIO);

        assert_eq!(panel.width_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn panel_opens_no_wider_than_the_room_left() {
        let mut panel = SidePanel::new(false);

        panel.open(0.1);

        assert_eq!(panel.width_ratio(), 0.1);
    }

    #[test]
    fn open_panel_dragged_wider_than_the_room_left_is_capped() {
        let mut panel = SidePanel::new(true);

        assert!(panel.drag_to(0.95, MAX_RATIO));

        assert_eq!(panel.width_ratio(), MAX_RATIO);
    }

    #[test]
    fn open_panel_dragged_below_the_minimum_width_stops_at_it() {
        let mut panel = SidePanel::new(true);
        let between = (SIDE_PANEL_COLLAPSE_RATIO + SIDE_PANEL_MIN_RATIO) / 2.0;

        assert!(panel.drag_to(between, MAX_RATIO));

        assert!(panel.is_open());
        assert_eq!(panel.width_ratio(), SIDE_PANEL_MIN_RATIO);
    }

    #[test]
    fn open_panel_dragged_below_the_collapse_width_closes() {
        let mut panel = SidePanel::new(true);

        assert!(panel.drag_to(SIDE_PANEL_COLLAPSE_RATIO / 2.0, MAX_RATIO));

        assert!(!panel.is_open());
    }

    #[test]
    fn closed_panel_dragged_a_little_stays_closed() {
        let mut panel = SidePanel::new(false);

        assert!(!panel.drag_to(SIDE_PANEL_COLLAPSE_RATIO / 2.0, MAX_RATIO));

        assert!(!panel.is_open());
    }

    #[test]
    fn closed_panel_dragged_past_the_collapse_width_opens_at_the_minimum_width()
    {
        let mut panel = SidePanel::new(false);

        assert!(panel.drag_to(0.5, MAX_RATIO));

        assert!(panel.is_open());
        assert_eq!(panel.width_ratio(), SIDE_PANEL_MIN_RATIO);
    }
}
