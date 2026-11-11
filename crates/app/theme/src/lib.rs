//! The look of the OTTY app: the two built-in themes with their terminal and
//! interface colors, the icons, and the font the terminal is drawn with.
//!
//! Colors, sizes and fonts of the design system come from `otty-ui-kit`; this
//! crate adds what only the app knows, such as which theme is chosen and which
//! icon each mode has.
//!
//! # Example
//!
//! ```
//! use otty_app_theme::theme::{AppTheme, BuiltinTheme, ThemeProps};
//!
//! let chosen = BuiltinTheme::from_name("otty-light").unwrap_or_default();
//! let theme = AppTheme::new(chosen);
//! let props = ThemeProps::new(&theme);
//!
//! assert_eq!(props.theme.terminal_palette().background, "#fafbfc");
//! ```

pub mod fonts;
pub mod icons;
pub mod theme;
