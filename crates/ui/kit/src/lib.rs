//! Design tokens of the OTTY design system for [`iced`] widgets.
//!
//! Every color, size, gap, corner radius and text style used by the app comes
//! from one of these modules:
//! - [`palette`] for colors and shadows of each theme;
//! - [`spacing`] for paddings and gaps;
//! - [`radius`] for corner radii;
//! - [`size`] for fixed heights and widths;
//! - [`typography`] for text sizes, line heights and weights;
//! - [`fonts`] for the bundled font files and `Font` values;
//! - [`components`] for ready-made controls and overlays;
//! - [`contrast`] for checking WCAG contrast between two colors.
//!
//! # Example
//!
//! ```
//! use iced::widget::{container, text};
//! use otty_ui_kit::{size, spacing, typography};
//!
//! let style = typography::BODY;
//! let row: container::Container<'_, ()> =
//!     container(text("main.rs").size(style.size()))
//!         .height(size::ROW)
//!         .padding(spacing::SPACE_3);
//! ```

pub mod components;
pub mod contrast;
pub mod fonts;
pub mod palette;
pub mod radius;
pub mod size;
pub mod spacing;
pub mod typography;
