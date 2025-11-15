mod backend;
mod bindings;
mod font;
#[cfg(feature = "backend-otty")]
mod otty_backend;
#[cfg(feature = "backend-otty")]
mod otty_theme;
#[cfg(feature = "backend-otty")]
mod otty_view;
mod theme;
mod types;
mod view;

pub use backend::settings::BackendSettings;
pub use backend::{BackendCommand, PtyEvent, TerminalBackend, TerminalMode};
pub use bindings::{Binding, BindingAction, InputKind, KeyboardBinding};
pub use font::{FontSettings, TerminalFont};
pub use theme::{ColorPalette, TerminalTheme};
pub use view::TerminalView;

// OTTY-based API (opt-in)
#[cfg(feature = "backend-otty")]
pub mod otty {
    use crate::bindings::{
        Binding as SharedBinding, BindingsLayout as SharedBindingsLayout,
        KeyboardBinding as SharedKeyboardBinding,
        MouseBinding as SharedMouseBinding,
    };

    pub use crate::bindings::{BindingAction, InputKind};
    pub use crate::otty_backend::{
        BackendCommand, RenderableContent, TerminalBackend,
    };
    pub use crate::otty_theme::{ColorPalette, TerminalTheme};
    pub use crate::otty_view::TerminalView;
    pub use otty_libterm::surface::SurfaceMode as TerminalMode;

    pub type Binding<T> = SharedBinding<T, TerminalMode>;
    pub type KeyboardBinding = SharedKeyboardBinding<TerminalMode>;
    pub type MouseBinding = SharedMouseBinding<TerminalMode>;
    pub type BindingsLayout = SharedBindingsLayout<TerminalMode>;
}
