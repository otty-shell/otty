/// Operating system command with raw arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperatingSystemCommand {
    SetWindowTitle,
    SetColorIndex,
    Hyperlink,
    SetTextBackgroundColor,
    SetTextForegroundColor,
    SetTextCursorColor,
    SetMouseCursorShape,
    SetCursorStyle,
    Clipboard,
    ResetIndexedColors,
    ResetForegroundColor,
    ResetBackgroundColor,
    ResetCursorColor,
    Unhandled
}

impl From<&[u8]> for OperatingSystemCommand {
    fn from(action: &[u8]) -> Self {
        match action {
            b"0" | b"2" => Self::SetWindowTitle,
            b"4" => Self::SetColorIndex,
            b"8" => Self::Hyperlink,
            b"10" => Self::SetTextBackgroundColor,
            b"11" => Self::SetTextForegroundColor,
            b"12" => Self::SetTextCursorColor,
            b"22" => Self::SetMouseCursorShape,
            b"50" => Self::SetCursorStyle,
            b"52" => Self::Clipboard,
            b"104" => Self::ResetIndexedColors,
            b"110" => Self::ResetForegroundColor,
            b"111" => Self::ResetBackgroundColor,
            b"112" => Self::ResetCursorColor,
            _ => Self::Unhandled 
        }
    }
}
