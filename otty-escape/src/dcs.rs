use std::fmt::{self, Debug, Display, Formatter};

/// Represents a device control mode entry.
#[derive(Clone, PartialEq, Eq)]
pub struct EnterDeviceControl {
    pub byte: u8,
    pub params: Vec<i64>,
    pub intermediates: Vec<u8>,
    pub ignored_extra_intermediates: bool,
}

impl Debug for EnterDeviceControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EnterDeviceControl {{\n\
             \tparams: {:?},\n\
             \tintermediates: {},\n\
             \tbyte: {:?} (0x{:02X}),\n\
             \tignored_extra_intermediates: {}\n\
             }}",
            self.params,
            bytes_to_pretty_string(&self.intermediates),
            self.byte as char,
            self.byte,
            self.ignored_extra_intermediates,
        )
    }
}

impl Display for EnterDeviceControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EnterDeviceControl {{\n\
             \tparams: {:?},\n\
             \tintermediates: {},\n\
             \tbyte: {:?} (0x{:02X}),\n\
             \tignored_extra_intermediates: {}\n\
             }}",
            self.params,
            bytes_to_pretty_string(&self.intermediates),
            self.byte as char,
            self.byte,
            self.ignored_extra_intermediates,
        )
    }
}

/// Represents a short device control sequence whose payload is fully buffered.
#[derive(Clone, PartialEq, Eq)]
pub struct ShortDeviceControl {
    pub byte: u8,
    pub params: Vec<i64>,
    pub intermediates: Vec<u8>,
    pub data: Vec<u8>,
}

impl Debug for ShortDeviceControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ShortDeviceControl {{\n\
             \tparams: {:?},\n\
             \tintermediates: {},\n\
             \tbyte: {:?} (0x{:02X}),\n\
             \tdata: {}\n\
             }}",
            self.params,
            bytes_to_pretty_string(&self.intermediates),
            self.byte as char,
            self.byte,
            bytes_to_pretty_string(&self.data),
        )
    }
}

impl Display for ShortDeviceControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ShortDeviceControl {{\n\
             \tparams: {:?},\n\
             \tintermediates: {},\n\
             \tbyte: {:?} (0x{:02X}),\n\
             \tdata: {}\n\
             }}",
            self.params,
            bytes_to_pretty_string(&self.intermediates),
            self.byte as char,
            self.byte,
            bytes_to_pretty_string(&self.data),
        )
    }
}

#[inline]
pub(crate) fn is_short_dcs(intermediates: &[u8], byte: u8) -> bool {
    intermediates == [b'$'] && byte == b'q'
}

fn bytes_to_pretty_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:?} 0x{:x}", *byte as char, *byte))
        .collect::<Vec<_>>()
        .join(", ")
}
