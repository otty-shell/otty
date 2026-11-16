use otty_app_domain::shortcuts::AppShortcut;
use otty_ui_kit::components::target_dot::TargetKind;

/// What the status bar needs from the rest of the app.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StatusBarInput<'a> {
    /// Machine of the active session, or this computer with no tabs.
    pub(crate) machine_name: &'a str,
    pub(crate) machine_target: TargetKind,
    /// Short folder path of the focused pane, with `~` for home; `None`
    /// unless the active tab is a terminal tab whose shell reported one.
    pub(crate) folder: Option<&'a str>,
    /// Place of the selected block among the command blocks of the
    /// focused pane: its number, counted from 1 at the oldest command
    /// block, and how many command blocks the pane has. `None` with no
    /// block selected there.
    pub(crate) block_place: Option<(usize, usize)>,
    /// Number of panes in the active terminal tab.
    pub(crate) pane_count: Option<usize>,
    /// Number of open tabs.
    pub(crate) session_count: usize,
}

/// View model of the status bar at the bottom of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusBarViewModel {
    machine_name: String,
    machine_target: TargetKind,
    folder: Option<String>,
    block_counter: Option<BlockCounter>,
    pane_count: Option<usize>,
    session_count: usize,
}

impl StatusBarViewModel {
    /// Return the machine name.
    pub(crate) fn machine_name(&self) -> &str {
        &self.machine_name
    }

    /// Return the dot kind of the machine.
    pub(crate) fn machine_target(&self) -> TargetKind {
        self.machine_target
    }

    /// Return the folder path of the focused pane, with `~` for home and
    /// only the last folders of a long path.
    pub(crate) fn folder(&self) -> Option<&str> {
        self.folder.as_deref()
    }

    /// Return the counter of the selected block; `None` with no block
    /// selected in the focused pane.
    pub(crate) fn block_counter(&self) -> Option<&BlockCounter> {
        self.block_counter.as_ref()
    }

    /// Return the pane count of the active terminal tab.
    pub(crate) fn pane_count(&self) -> Option<usize> {
        self.pane_count
    }

    /// Return the number of open sessions.
    pub(crate) fn session_count(&self) -> usize {
        self.session_count
    }

    /// Report whether no session is open (the empty state).
    pub(crate) fn is_empty(&self) -> bool {
        self.session_count == 0
    }

    /// Build the status bar data from the active tab and focused pane.
    pub(crate) fn new(input: StatusBarInput<'_>) -> Self {
        Self {
            machine_name: input.machine_name.to_string(),
            machine_target: input.machine_target,
            folder: input.folder.map(str::to_string),
            block_counter: input.block_place.map(BlockCounter::new),
            pane_count: input.pane_count,
            session_count: input.session_count,
        }
    }
}

/// The counter of the selected block in the status bar: `Block 5 of 6`,
/// followed by the keys that move the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockCounter {
    number: usize,
    count: usize,
    keys: [String; 2],
}

impl BlockCounter {
    /// Return the number of the block, counted from 1 at the oldest
    /// command block.
    pub(crate) fn number(&self) -> usize {
        self.number
    }

    /// Return how many command blocks the pane has.
    pub(crate) fn count(&self) -> usize {
        self.count
    }

    /// Return the labels of the keys that select the block before and the
    /// block after, on the current platform.
    pub(crate) fn keys(&self) -> &[String; 2] {
        &self.keys
    }

    /// The counter of the block `number` of `count` command blocks.
    fn new((number, count): (usize, usize)) -> Self {
        Self {
            number,
            count,
            keys: [
                AppShortcut::PreviousBlock.label(),
                AppShortcut::NextBlock.label(),
            ],
        }
    }
}

/// The widths of the status bar, in pixels, with the block counter in full
/// and with the text of the counter only, measured with the font and the
/// size the bar is drawn with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CounterWidths {
    pub(crate) with_keys: f32,
    pub(crate) text_only: f32,
}

/// What of the block counter the status bar shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CounterParts {
    /// `Block 5 of 6` and the two keys that move the selection.
    WithKeys,
    /// `Block 5 of 6` alone.
    TextOnly,
    /// No counter.
    Hidden,
}

impl CounterParts {
    /// Choose what of the counter fits in a status bar `available` pixels
    /// wide, so the bar stays one line: the two keys are left out first,
    /// then `Block 5 of 6`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let widths = CounterWidths {
    ///     with_keys: 600.0,
    ///     text_only: 450.0,
    /// };
    ///
    /// assert_eq!(CounterParts::fit(500.0, widths), CounterParts::TextOnly);
    /// ```
    pub(crate) fn fit(available: f32, widths: CounterWidths) -> Self {
        if widths.with_keys <= available {
            return Self::WithKeys;
        }
        if widths.text_only <= available {
            return Self::TextOnly;
        }

        Self::Hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        folder: Option<&str>,
        pane_count: Option<usize>,
        session_count: usize,
    ) -> StatusBarInput<'_> {
        StatusBarInput {
            machine_name: "fedora",
            machine_target: TargetKind::Local,
            folder,
            block_place: None,
            pane_count,
            session_count,
        }
    }

    #[test]
    fn the_fifth_of_six_selected_blocks_is_counted_with_the_keys_that_move_the_selection()
     {
        let vm = StatusBarViewModel::new(StatusBarInput {
            block_place: Some((5, 6)),
            ..input(Some("~"), Some(1), 1)
        });

        let counter = vm.block_counter().expect("a block is selected");
        assert_eq!((counter.number(), counter.count()), (5, 6));
        assert_eq!(
            counter.keys(),
            &[
                AppShortcut::PreviousBlock.label(),
                AppShortcut::NextBlock.label()
            ]
        );
    }

    /// A bar 600 px wide with the counter in full, 450 px with its text
    /// only.
    fn counter_widths() -> CounterWidths {
        CounterWidths {
            with_keys: 600.0,
            text_only: 450.0,
        }
    }

    #[test]
    fn a_wide_status_bar_shows_the_counter_with_its_keys() {
        let parts = CounterParts::fit(600.0, counter_widths());

        assert_eq!(parts, CounterParts::WithKeys);
    }

    #[test]
    fn in_a_narrow_status_bar_the_keys_go_first_then_the_counter() {
        let without_keys = CounterParts::fit(599.0, counter_widths());
        let text_only = CounterParts::fit(450.0, counter_widths());
        let hidden = CounterParts::fit(449.0, counter_widths());

        assert_eq!(without_keys, CounterParts::TextOnly);
        assert_eq!(text_only, CounterParts::TextOnly);
        assert_eq!(hidden, CounterParts::Hidden);
    }

    #[test]
    fn with_no_selected_block_there_is_no_counter() {
        let vm = StatusBarViewModel::new(input(Some("~"), Some(1), 1));

        assert_eq!(vm.block_counter(), None);
    }

    #[test]
    fn given_terminal_tab_when_built_then_folder_path_and_counts_are_shown() {
        let vm = StatusBarViewModel::new(input(
            Some("~/…/ottyrun/otty"),
            Some(2),
            3,
        ));

        assert_eq!(vm.machine_name(), "fedora");
        assert_eq!(vm.machine_target(), TargetKind::Local);
        assert_eq!(vm.folder(), Some("~/…/ottyrun/otty"));
        assert_eq!(vm.pane_count(), Some(2));
        assert_eq!(vm.session_count(), 3);
        assert!(!vm.is_empty());
    }

    #[test]
    fn given_settings_tab_when_built_then_no_pane_data_is_shown() {
        let vm = StatusBarViewModel::new(input(None, None, 2));

        assert_eq!(vm.machine_name(), "fedora");
        assert_eq!(vm.folder(), None);
        assert_eq!(vm.pane_count(), None);
        assert_eq!(vm.session_count(), 2);
        assert!(!vm.is_empty());
    }

    #[test]
    fn given_no_tabs_when_built_then_bar_is_empty_with_machine() {
        let vm = StatusBarViewModel::new(input(None, None, 0));

        assert!(vm.is_empty());
        assert_eq!(vm.machine_name(), "fedora");
        assert_eq!(vm.pane_count(), None);
    }

    #[test]
    fn given_folder_change_when_rebuilt_then_new_folder_is_shown() {
        let first = StatusBarViewModel::new(input(Some("~"), Some(1), 1));
        let second =
            StatusBarViewModel::new(input(Some("/var/log"), Some(1), 1));

        assert_eq!(first.folder(), Some("~"));
        assert_eq!(second.folder(), Some("/var/log"));
    }

    #[test]
    fn given_split_then_close_when_rebuilt_then_pane_count_follows() {
        let one = StatusBarViewModel::new(input(Some("~"), Some(1), 1));
        let two = StatusBarViewModel::new(input(Some("~"), Some(2), 1));
        let back = StatusBarViewModel::new(input(Some("~"), Some(1), 1));

        assert_eq!(one.pane_count(), Some(1));
        assert_eq!(two.pane_count(), Some(2));
        assert_eq!(back.pane_count(), Some(1));
    }
}
