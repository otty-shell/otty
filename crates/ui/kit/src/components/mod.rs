//! Shared widgets of the design system, built from [`UiPalette`] and the kit
//! tokens.
//!
//! Each component takes the palette of the active theme and returns an
//! [`iced::Element`]. Components never bundle icons: callers pass an
//! [`iced::widget::svg::Handle`].
//!
//! [`UiPalette`]: crate::palette::UiPalette

pub mod button;
pub mod icon_button;
pub mod kbd;
pub mod menu;
pub mod segmented;
pub mod select;
pub mod switch;

use iced::Color;

/// Opacity of a disabled control (`opacity: .45` in the design system).
const DISABLED_OPACITY: f32 = 0.45;

/// Returns `color` faded to the disabled opacity.
fn faded(color: Color) -> Color {
    Color {
        a: color.a * DISABLED_OPACITY,
        ..color
    }
}
