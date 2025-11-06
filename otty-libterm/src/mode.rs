use bitflags::bitflags;
use otty_escape::KeyboardMode;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TerminalMode: u32 {
        const NONE                    = 0;
        const SHOW_CURSOR             = 1;
        const APP_CURSOR              = 1 << 1;
        const APP_KEYPAD              = 1 << 2;
        const MOUSE_REPORT_CLICK      = 1 << 3;
        const BRACKETED_PASTE         = 1 << 4;
        const SGR_MOUSE               = 1 << 5;
        const MOUSE_MOTION            = 1 << 6;
        const LINE_WRAP               = 1 << 7;
        const LINE_FEED_NEW_LINE      = 1 << 8;
        const ORIGIN                  = 1 << 9;
        const INSERT                  = 1 << 10;
        const FOCUS_IN_OUT            = 1 << 11;
        const ALT_SCREEN              = 1 << 12;
        const MOUSE_DRAG              = 1 << 13;
        const UTF8_MOUSE              = 1 << 14;
        const ALTERNATE_SCROLL        = 1 << 15;
        const URGENCY_HINTS           = 1 << 16;
        const DISAMBIGUATE_ESC_CODES  = 1 << 17;
        const REPORT_EVENT_TYPES      = 1 << 18;
        const REPORT_ALTERNATE_KEYS   = 1 << 19;
        const REPORT_ALL_KEYS_AS_ESC  = 1 << 20;
        const REPORT_ASSOCIATED_TEXT  = 1 << 21;
        const MOUSE_MODE              = Self::MOUSE_REPORT_CLICK.bits() | Self::MOUSE_MOTION.bits() | Self::MOUSE_DRAG.bits();
        const KITTY_KEYBOARD_PROTOCOL = Self::DISAMBIGUATE_ESC_CODES.bits()
                                      | Self::REPORT_EVENT_TYPES.bits()
                                      | Self::REPORT_ALTERNATE_KEYS.bits()
                                      | Self::REPORT_ALL_KEYS_AS_ESC.bits()
                                      | Self::REPORT_ASSOCIATED_TEXT.bits();
         const ANY                    = u32::MAX;
    }
}

impl Default for TerminalMode {
    fn default() -> Self {
        Self::SHOW_CURSOR
            | Self::LINE_WRAP
            | Self::ALTERNATE_SCROLL
            | Self::URGENCY_HINTS
    }
}

impl From<KeyboardMode> for TerminalMode {
    fn from(value: KeyboardMode) -> Self {
        let mut mode = Self::empty();

        let disambiguate_esc_codes =
            value.contains(KeyboardMode::DISAMBIGUATE_ESC_CODES);
        mode.set(Self::DISAMBIGUATE_ESC_CODES, disambiguate_esc_codes);

        let report_event_types =
            value.contains(KeyboardMode::REPORT_EVENT_TYPES);
        mode.set(Self::REPORT_EVENT_TYPES, report_event_types);

        let report_alternate_keys =
            value.contains(KeyboardMode::REPORT_ALTERNATE_KEYS);
        mode.set(Self::REPORT_ALTERNATE_KEYS, report_alternate_keys);

        let report_all_keys_as_esc =
            value.contains(KeyboardMode::REPORT_ALL_KEYS_AS_ESC);
        mode.set(Self::REPORT_ALL_KEYS_AS_ESC, report_all_keys_as_esc);

        let report_associated_text =
            value.contains(KeyboardMode::REPORT_ASSOCIATED_TEXT);
        mode.set(Self::REPORT_ASSOCIATED_TEXT, report_associated_text);

        mode
    }
}
