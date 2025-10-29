/// Terminal cursor shape.
#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Hash)]
pub enum CursorShape {
    /// `▒` Cursor.
    #[default]
    Block,
    /// `_` Cursor.
    Underline,
    /// `⎸` Cursor.
    Beam,
    /// Invisible cursor.
    Hidden,
}

/// Terminal cursor configuration.
#[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub struct CursorStyle {
    pub shape: CursorShape,
    pub blinking: bool,
}
