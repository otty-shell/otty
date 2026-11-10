//! The I/O of Containers mode: the engine search, the engine calls, the
//! pulls, the folder checks of the Run form, the browser and the clipboard,
//! the events watch of the engines and the followed logs of the inspector.
//!
//! The containers feature says what to do and what to watch; this crate
//! does it with the client of `otty-containers`. Each piece of work is a
//! [`Task`](iced::Task) and each watch is a
//! [`Subscription`](iced::Subscription), and all of them end in a
//! [`ContainersIntent`](otty_app_containers::ContainersIntent). The binary
//! only calls them and wraps the intents into its message.
//!
//! # Examples
//!
//! ```
//! use std::path::PathBuf;
//!
//! // Ask the disk whether a folder of the Run form exists.
//! let check = otty_app_containers_io::check_folder(PathBuf::from("/srv"));
//!
//! assert_eq!(check.units(), 1);
//! ```

mod effects;
mod engine_watch;
mod followed_logs;

pub use effects::{
    cancel_pull, check_folder, check_log_run, copy_to_clipboard,
    discover_engines, discover_in_background, load_image_defaults,
    load_inspect, load_list, load_lists, open_url, pull_image, pull_run_image,
    run_action, run_container, wait_for_late_answer, wait_for_typing_pause,
};
pub use engine_watch::watch_engines;
pub use followed_logs::follow_logs;
