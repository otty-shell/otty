use bitflags::bitflags;

/// XTMODKEYS modifyOtherKeys state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifyOtherKeysState {
    Reset,
    EnableExceptWellDefined,
    EnableAll,
}

bitflags! {
    /// [`kitty keyboard protocol'] modes.
    /// https://sw.kovidgoyal.net/kitty/keyboard-protocol
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct KeyboardMode : u8 {
        const NO_MODE                 = 0b0000_0000;
        const DISAMBIGUATE_ESC_CODES  = 0b0000_0001;
        const REPORT_EVENT_TYPES      = 0b0000_0010;
        const REPORT_ALTERNATE_KEYS   = 0b0000_0100;
        const REPORT_ALL_KEYS_AS_ESC  = 0b0000_1000;
        const REPORT_ASSOCIATED_TEXT  = 0b0001_0000;
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardModeApplyBehavior {
    #[default]
    Replace,
    Union,
    Difference,
}
