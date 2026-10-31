//! Shared widgets of the design system, built from [`UiPalette`] and the kit
//! tokens.
//!
//! Each component takes the palette of the active theme and returns an
//! [`iced::Element`]. Components never bundle icons: callers pass an
//! [`iced::widget::svg::Handle`]. The only exception is the small caret,
//! cross and plus shapes that rows, tabs and fields draw as part of their
//! own structure.
//!
//! [`UiPalette`]: crate::palette::UiPalette

mod backspace_guard;
pub mod button;
mod field_chrome;
pub mod form_section;
mod glyph;
pub mod icon_button;
pub mod kbd;
pub mod key_value_field;
pub mod menu;
pub mod rail_button;
pub mod segmented;
pub mod select;
pub mod setting_row;
pub mod switch;
pub mod tab_strip;
pub mod text_field;
pub mod token_field;
pub mod tree_row;

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
