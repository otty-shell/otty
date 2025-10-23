/// Terminal cursor shape.
#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Hash)]
pub enum CursorShape {
    /// Cursor is a block like `▒`.
    #[default]
    Block,
    /// Cursor is an underscore like `_`.
    Underline,
    /// Cursor is a vertical bar `⎸`.
    Beam,
    /// Cursor is a box like `☐`.
    HollowBlock,
    /// Invisible cursor.
    Hidden,
}

/// Terminal cursor configuration.
#[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub struct CursorStyle {
    pub shape: CursorShape,
    pub blinking: bool,
}
