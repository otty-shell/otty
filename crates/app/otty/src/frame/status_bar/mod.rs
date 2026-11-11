//! The status bar at the bottom of the window: the machine, the folder of
//! the focused pane, the pane and session counts and the version.
//!
//! It is only a view. It holds no state and takes no intents, so it is no
//! component: a screen builds its view model from the tabs and the terminal
//! and draws it.

mod model;
pub(crate) mod view;

pub(crate) use self::model::{StatusBarInput, StatusBarViewModel};
