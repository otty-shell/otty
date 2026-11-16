//! Test helpers of the terminal feature: a terminal widget that shows a
//! snapshot a test builds, for the layers drawn over the terminal.

use otty_app_testing::terminal::ScriptedTerminals;
use otty_libterm::TerminalSize;
use otty_libterm::surface::{Surface, SurfaceConfig, SurfaceModel};
use otty_ui_term::settings::{EngineSettings, Settings};
use otty_ui_term::{BlockSnapshot, SnapshotOwned};

use crate::types::Terminal;

/// A terminal `terminal_id` of `grid` from `terminals` that shows
/// `snapshot`.
pub(crate) fn terminal_showing(
    terminals: &ScriptedTerminals,
    terminal_id: u64,
    grid: TerminalSize,
    snapshot: SnapshotOwned,
) -> Terminal {
    let settings = Settings {
        engine: EngineSettings::default().with_size(grid),
        ..Settings::default()
    };
    let mut terminal: Terminal = (terminals.starter())(terminal_id, settings)
        .expect("a scripted terminal starts");
    terminals.report(terminal_id, snapshot);
    terminal.handle(otty_ui_term::Event::ContentSync { id: terminal_id });

    terminal
}

/// An empty screen of `grid` that holds `blocks`: the widget lays its rows
/// out, with the gaps around the blocks.
pub(crate) fn screen_of(
    grid: TerminalSize,
    blocks: Vec<BlockSnapshot>,
) -> SnapshotOwned {
    let mut snapshot =
        Surface::new(SurfaceConfig::default(), &grid).snapshot_owned();
    snapshot.blocks = blocks;

    snapshot
}
