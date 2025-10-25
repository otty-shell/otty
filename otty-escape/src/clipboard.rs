#[derive(Debug)]
pub enum ClipboardType {
    Clipboard,
    Primary,
    Secondary,
    Select,
    Unexpected(u8),
}

impl From<u8> for ClipboardType {
    fn from(value: u8) -> Self {
        match value {
            b'c' => Self::Clipboard,
            b'p' => Self::Primary,
            b'q' => Self::Secondary,
            b's' => Self::Select,
            other => Self::Unexpected(other),
        }
    }
}
