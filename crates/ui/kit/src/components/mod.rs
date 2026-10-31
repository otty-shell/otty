//! Shared widgets of the design system, built from [`UiPalette`] and the kit
//! tokens.
//!
//! Each component takes the palette of the active theme and returns an
//! [`iced::Element`]. Components never bundle icons: callers pass an
//! [`iced::widget::svg::Handle`]. The only exception is the small caret
//! and cross shapes that rows and fields draw as part of their own
//! structure.
//!
//! [`UiPalette`]: crate::palette::UiPalette

pub mod action_tile;
pub mod button;
pub mod chip;
pub mod command_palette;
pub mod dialog;
pub mod divider;
pub mod empty_card;
mod field_chrome;
pub mod floating_note;
pub mod form_page;
pub mod form_section;
mod glyph;
pub mod icon_button;
mod input_keys;
pub mod kbd;
pub mod key_value_field;
pub mod menu;
pub mod palette_list;
pub mod pane_header;
pub mod panel_section;
pub mod scrollbar;
pub mod segmented;
pub mod select;
pub mod session_group;
pub mod session_row;
pub mod setting_row;
pub mod split;
pub mod split_button;
pub mod status_bar;
pub mod surface;
pub mod switch;
pub mod target_dot;
pub mod text_field;
pub mod title_search;
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
