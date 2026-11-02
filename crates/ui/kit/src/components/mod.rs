//! Shared widgets of the design system, built from [`UiPalette`] and the kit
//! tokens.
//!
//! Each component takes the palette of the active theme and returns an
//! [`iced::Element`]. Components never bundle icons: callers pass an
//! [`iced::widget::svg::Handle`]. The only exception is the small caret,
//! cross, "⋯" dots, pin and sort arrow shapes that components draw as part
//! of their own structure.
//!
//! [`UiPalette`]: crate::palette::UiPalette

pub mod action_tile;
pub mod button;
pub mod chip;
pub mod command_box;
pub mod command_palette;
pub mod data_table;
pub mod data_tree;
pub mod dialog;
pub mod divider;
mod ellipsis_text;
pub mod empty_card;
mod field_chrome;
mod floating_list;
pub mod floating_note;
pub mod form_page;
pub mod form_section;
mod glyph;
pub mod icon_button;
mod input_keys;
pub mod inspector_header;
pub mod inspector_tabs;
pub mod kbd;
pub mod key_value_field;
pub mod key_value_list;
pub mod log_view;
pub mod mapping_field;
pub mod menu;
pub mod mode_menu;
pub mod mode_switcher;
pub mod notice;
mod overflow_menu;
pub mod palette_list;
pub mod pane_header;
pub mod panel_height;
pub mod panel_section;
pub mod progress_bar;
pub mod scrollbar;
pub mod segmented;
pub mod select;
pub mod session_group;
pub mod session_row;
pub mod setting_row;
pub mod split;
pub mod split_button;
pub mod status_badge;
pub mod status_bar;
pub mod surface;
pub mod switch;
mod table_body;
mod table_sideways;
pub mod tag_row;
pub mod tag_search;
pub mod tag_search_field;
pub mod target_dot;
pub mod text_field;
pub mod title_search;
pub mod token_field;
pub mod toolbar;
pub mod tree_row;
mod wrapped_text;

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
