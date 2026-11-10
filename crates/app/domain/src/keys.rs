//! The keys that press a shortcut, without iced, and the text labels write
//! for them.
//!
//! The binary turns an iced key press into these types to find the shortcut
//! it presses, and the labels are built from the same values, so labels and
//! key matching share one source of keys.

/// The modifier keys that must be held with a shortcut key, exactly.
///
/// # Examples
///
/// ```
/// use otty_app_domain::keys::KeyModifiers;
///
/// let modifiers = KeyModifiers::NONE.with_ctrl().with_shift();
///
/// assert!(modifiers.ctrl() && modifiers.shift());
/// assert!(!modifiers.alt() && !modifiers.logo());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyModifiers {
    ctrl: bool,
    alt: bool,
    logo: bool,
    shift: bool,
}

impl KeyModifiers {
    /// Whether Ctrl is held.
    pub fn ctrl(&self) -> bool {
        self.ctrl
    }

    /// Whether Alt (Option on macOS) is held.
    pub fn alt(&self) -> bool {
        self.alt
    }

    /// Whether the Logo key is held: Super on Linux, Cmd on macOS.
    pub fn logo(&self) -> bool {
        self.logo
    }

    /// Whether Shift is held.
    pub fn shift(&self) -> bool {
        self.shift
    }

    /// Whether no modifier is held.
    pub fn is_empty(&self) -> bool {
        !(self.ctrl || self.alt || self.logo || self.shift)
    }

    /// No modifier held.
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        logo: false,
        shift: false,
    };

    /// These modifiers with Ctrl held too.
    pub const fn with_ctrl(self) -> Self {
        Self { ctrl: true, ..self }
    }

    /// These modifiers with Alt held too.
    pub const fn with_alt(self) -> Self {
        Self { alt: true, ..self }
    }

    /// These modifiers with the Logo key held too.
    pub const fn with_logo(self) -> Self {
        Self { logo: true, ..self }
    }

    /// These modifiers with Shift held too.
    pub const fn with_shift(self) -> Self {
        Self {
            shift: true,
            ..self
        }
    }
}

/// The key of a shortcut, apart from its modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutKey {
    /// A key matched by the character it types without modifiers, in lower
    /// case, such as `'k'` or `','`.
    Character(char),
    /// The 1 key of the digit row, matched by its place on the keyboard,
    /// whatever character the layout and Shift make it type.
    Digit1,
    /// The 2 key of the digit row, matched by its place.
    Digit2,
    /// The 3 key of the digit row, matched by its place.
    Digit3,
    /// The 4 key of the digit row, matched by its place.
    Digit4,
    /// The 5 key of the digit row, matched by its place.
    Digit5,
}

impl ShortcutKey {
    /// Return how labels write the key: a character in upper case, a digit
    /// as itself.
    fn text(self) -> String {
        match self {
            Self::Character(key) => key.to_uppercase().to_string(),
            Self::Digit1 => String::from("1"),
            Self::Digit2 => String::from("2"),
            Self::Digit3 => String::from("3"),
            Self::Digit4 => String::from("4"),
            Self::Digit5 => String::from("5"),
        }
    }
}

/// A key and the exact modifiers that must be held with it.
///
/// # Examples
///
/// ```
/// use otty_app_domain::keys::{
///     KeyModifiers, Platform, ShortcutKey, ShortcutKeys,
/// };
///
/// let keys = ShortcutKeys::new(
///     ShortcutKey::Character('t'),
///     KeyModifiers::NONE.with_ctrl().with_shift(),
/// );
///
/// assert_eq!(keys.label_on(Platform::Linux), "Ctrl Shift T");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShortcutKeys {
    key: ShortcutKey,
    modifiers: KeyModifiers,
}

impl ShortcutKeys {
    /// The key pressed with the modifiers.
    pub fn key(&self) -> ShortcutKey {
        self.key
    }

    /// The modifiers that must be held, exactly.
    pub fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    /// `key` pressed with exactly `modifiers` held.
    pub const fn new(key: ShortcutKey, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    /// Return the text menus and `Kbd` hints show for these keys on
    /// `platform`, for example "Ctrl Shift T".
    ///
    /// The modifiers come first, in the order Ctrl, Alt, the Logo key,
    /// Shift, then the key; the parts are separated by spaces. The Logo key
    /// reads "Super" on Linux and "Cmd" on macOS.
    pub fn label_on(&self, platform: Platform) -> String {
        let modifiers = self.modifiers;
        let mut parts: Vec<String> = Vec::new();
        if modifiers.ctrl() {
            parts.push(String::from("Ctrl"));
        }
        if modifiers.alt() {
            parts.push(String::from("Alt"));
        }
        if modifiers.logo() {
            parts.push(String::from(platform.logo_name()));
        }
        if modifiers.shift() {
            parts.push(String::from("Shift"));
        }

        parts.push(self.key.text());

        parts.join(" ")
    }
}

/// The platform a shortcut binding is written for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Linux and every other system that is not macOS: Ctrl Shift
    /// shortcuts.
    Linux,
    /// macOS: Cmd shortcuts.
    MacOs,
}

impl Platform {
    /// Return the platform this build runs on.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        }
    }

    /// Return how this platform's labels write the Logo key.
    fn logo_name(self) -> &'static str {
        match self {
            Platform::MacOs => "Cmd",
            _ => "Super",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTRL_SHIFT: KeyModifiers =
        KeyModifiers::NONE.with_ctrl().with_shift();

    #[test]
    fn label_orders_modifiers_and_uppercases_the_key() {
        let keys = ShortcutKeys::new(ShortcutKey::Character('c'), CTRL_SHIFT);

        assert_eq!(keys.label_on(Platform::Linux), "Ctrl Shift C");
    }

    #[test]
    fn label_keeps_digits_and_punctuation() {
        let alt_one = ShortcutKeys::new(
            ShortcutKey::Digit1,
            KeyModifiers::NONE.with_alt(),
        );
        let ctrl_comma = ShortcutKeys::new(
            ShortcutKey::Character(','),
            KeyModifiers::NONE.with_ctrl(),
        );

        assert_eq!(alt_one.label_on(Platform::Linux), "Alt 1");
        assert_eq!(ctrl_comma.label_on(Platform::Linux), "Ctrl ,");
    }

    #[test]
    fn label_puts_every_modifier_in_its_place() {
        let keys = ShortcutKeys::new(
            ShortcutKey::Character('t'),
            KeyModifiers::NONE
                .with_shift()
                .with_logo()
                .with_alt()
                .with_ctrl(),
        );

        assert_eq!(keys.label_on(Platform::Linux), "Ctrl Alt Super Shift T");
    }

    #[test]
    fn label_names_the_logo_key_after_the_platform() {
        let keys = ShortcutKeys::new(
            ShortcutKey::Character('e'),
            KeyModifiers::NONE.with_logo().with_shift(),
        );

        assert_eq!(keys.label_on(Platform::Linux), "Super Shift E");
        assert_eq!(keys.label_on(Platform::MacOs), "Cmd Shift E");
    }

    #[test]
    fn digit_keys_read_as_their_digit() {
        let digits = [
            (ShortcutKey::Digit1, "Cmd 1"),
            (ShortcutKey::Digit2, "Cmd 2"),
            (ShortcutKey::Digit3, "Cmd 3"),
            (ShortcutKey::Digit4, "Cmd 4"),
            (ShortcutKey::Digit5, "Cmd 5"),
        ];

        for (key, label) in digits {
            let keys = ShortcutKeys::new(key, KeyModifiers::NONE.with_logo());

            assert_eq!(keys.label_on(Platform::MacOs), label);
        }
    }

    #[test]
    fn no_modifier_is_empty_and_any_modifier_is_not() {
        assert!(KeyModifiers::NONE.is_empty());
        for modifiers in [
            KeyModifiers::NONE.with_ctrl(),
            KeyModifiers::NONE.with_alt(),
            KeyModifiers::NONE.with_logo(),
            KeyModifiers::NONE.with_shift(),
        ] {
            assert!(!modifiers.is_empty(), "{modifiers:?}");
        }
    }

    #[test]
    fn current_platform_is_the_build_target() {
        let current = if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        };

        assert_eq!(Platform::current(), current);
    }
}
