//! Draws the text of a terminal snapshot.
//!
//! The work goes through one pipeline: `runs` groups the cells of a snapshot
//! into text runs, `draw` shapes each run and draws it on the grid,
//! `shaped_run_cache` remembers shaped runs so unchanged text is not shaped
//! again, and `font_coverage` remembers which characters no font has, so
//! their font search can be skipped.

mod draw;
mod font_coverage;
mod runs;
mod shaped_run_cache;

pub(crate) use draw::{TextDrawConfig, TextDrawState, draw_text_runs};
pub(crate) use runs::build_text_runs;
