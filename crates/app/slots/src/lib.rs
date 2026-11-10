//! The socket every feature of the OTTY app plugs into.
//!
//! The binary `otty-app` is the window frame. A feature, such as the tabs or
//! the containers, fills places in it: the left panel, the main area, the
//! right panel, the status bar and the overlays. This crate writes down what
//! a feature gives the frame, so the compiler checks it:
//!
//! - [`Feature`]: the feature's input (`Intent`), what it reports back
//!   (`Outcome`) and the outside data it reads while it reduces (`Ctx`). The
//!   binary gives an intent to `reduce` and reads the outcomes it returns: news
//!   for another feature, I/O to start, or a window operation. A feature never
//!   sends a message to another feature.
//! - [`Overlay`]: a menu or a dialog a feature draws over the window. The frame
//!   owns the layer order and the rules of each kind.
//! - [`Overlays`]: the place trait of a feature that has such an overlay.
//! - [`RightPanel`]: the place trait of a feature that fills the right panel.
//! - [`MainContent`]: what fills the main area, a screen or content that
//!   scrolls on its own.
//! - [`focus`]: the ids the frame moves the keyboard focus to.
//!
//! A trait for each place (left panel, main area, ...) is added here when the
//! first feature fills that place.
//!
//! # Example
//!
//! ```
//! use otty_app_slots::Feature;
//!
//! /// A counter that reports when it reaches its limit.
//! struct Counter {
//!     value: u32,
//! }
//!
//! enum CounterOutcome {
//!     LimitReached,
//! }
//!
//! impl Feature for Counter {
//!     type Intent = u32;
//!     type Outcome = CounterOutcome;
//!     type Ctx<'a> = ();
//!
//!     fn reduce(&mut self, step: u32, _ctx: ()) -> Vec<CounterOutcome> {
//!         self.value += step;
//!
//!         if self.value >= 10 {
//!             return vec![CounterOutcome::LimitReached];
//!         }
//!         Vec::new()
//!     }
//! }
//!
//! let mut counter = Counter { value: 8 };
//!
//! assert!(counter.reduce(1, ()).is_empty());
//! assert!(matches!(
//!     counter.reduce(1, ()).as_slice(),
//!     [CounterOutcome::LimitReached]
//! ));
//! ```

mod feature;
pub mod focus;
mod main_content;
mod overlay;
mod overlays;
mod right_panel;

pub use feature::Feature;
pub use main_content::MainContent;
pub use overlay::{Overlay, OverlayKind};
pub use overlays::Overlays;
pub use right_panel::RightPanel;
