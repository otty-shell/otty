mod sgr;
mod color;

use sgr::Sgr;
use otty_vte::CsiParam;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsiSequence {
    /// SGR: Set Graphics Rendition.
    /// These values affect how the character is rendered.
    Sgr(Sgr),

    /// CSI codes that relate to the cursor
    // Cursor(Cursor),

    // Edit(Edit),

    // Mode(Mode),

    // Device(Box<Device>),

    // Mouse(MouseReport),

    // Window(Box<Window>),

    // Keyboard(Keyboard),

    /// ECMA-48 SCP
    // SelectCharacterPath(CharacterPath, i64),

    /// Unknown or unspecified; should be rare and is rather
    /// large, so it is boxed and kept outside of the enum
    /// body to help reduce space usage in the common cases.
    Unspecified(Box<Unspecified>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unspecified {
    pub params: Vec<CsiParam>,
    /// if true, more than two intermediates arrived and the
    /// remaining data was ignored
    pub parameters_truncated: bool,
    /// The final character in the CSI sequence; this typically
    /// defines how to interpret the other parameters.
    pub byte: u8,
}
