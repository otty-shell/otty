use otty_vte::CsiParam;

/// Encapsulates a CSI sequence with its parsed metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsiSequence {
    pub params: Vec<CsiParam>,
    pub intermediates: Vec<u8>,
    pub parameters_truncated: bool,
    pub final_byte: u8,
}
