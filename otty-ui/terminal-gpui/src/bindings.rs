use gpui::{Keystroke, Modifiers};
use otty_libterm::surface::SurfaceMode;

/// Result of matching a GPUI keystroke against terminal-local bindings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingAction {
    /// Write the exact bytes to the active terminal backend.
    Bytes(Vec<u8>),
    /// Copy the current selection.
    Copy,
    /// Paste the native clipboard.
    Paste,
    /// Consume the keystroke without writing bytes.
    Ignore,
}

/// One terminal-local keystroke binding with mode constraints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalBinding {
    keystroke: Keystroke,
    mode_include: SurfaceMode,
    mode_exclude: SurfaceMode,
    action: BindingAction,
}

impl TerminalBinding {
    /// Bound keystroke.
    pub fn keystroke(&self) -> &Keystroke {
        &self.keystroke
    }

    /// Action performed by the binding.
    pub fn action(&self) -> &BindingAction {
        &self.action
    }

    /// Create a binding for an exact keystroke and surface mode predicate.
    pub fn new(
        keystroke: Keystroke,
        mode_include: SurfaceMode,
        mode_exclude: SurfaceMode,
        action: BindingAction,
    ) -> Self {
        Self {
            keystroke,
            mode_include,
            mode_exclude,
            action,
        }
    }

    fn matches(&self, input: &Keystroke, mode: SurfaceMode) -> bool {
        self.keystroke == *input
            && mode.contains(self.mode_include)
            && !mode.intersects(self.mode_exclude)
    }

    fn same_trigger(&self, other: &Self) -> bool {
        self.keystroke == other.keystroke
            && self.mode_include == other.mode_include
            && self.mode_exclude == other.mode_exclude
    }
}

/// Ordered terminal-local key bindings with replacement semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalBindings {
    entries: Vec<TerminalBinding>,
}

impl TerminalBindings {
    /// Borrow all configured bindings in lookup order.
    pub fn entries(&self) -> &[TerminalBinding] {
        &self.entries
    }

    /// Create an empty binding table.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a binding, replacing an existing entry with the same trigger.
    pub fn add(&mut self, binding: TerminalBinding) {
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.same_trigger(&binding))
        {
            self.entries[index] = binding;
        } else {
            self.entries.push(binding);
        }
    }

    /// Add or replace several bindings.
    pub fn add_many(
        &mut self,
        bindings: impl IntoIterator<Item = TerminalBinding>,
    ) {
        for binding in bindings {
            self.add(binding);
        }
    }

    /// Resolve the first binding matching the input and active surface mode.
    pub fn resolve(
        &self,
        input: &Keystroke,
        mode: SurfaceMode,
    ) -> Option<&BindingAction> {
        self.entries
            .iter()
            .find(|binding| binding.matches(input, mode))
            .map(TerminalBinding::action)
    }

    /// Resolve an unmodified printable keystroke to its UTF-8 bytes.
    pub fn bytes_for_printable<'a>(
        &self,
        input: &'a Keystroke,
    ) -> Option<&'a [u8]> {
        if input.modifiers.control
            || input.modifiers.alt
            || input.modifiers.platform
            || input.modifiers.function
        {
            return None;
        }

        input
            .key_char
            .as_deref()
            .or(Some(input.key.as_str()))
            .filter(|value| !value.is_empty())
            .map(str::as_bytes)
    }
}

impl Default for TerminalBindings {
    fn default() -> Self {
        let mut bindings = Self::new();
        for (key, bytes) in [
            ("enter", b"\r".as_slice()),
            ("backspace", b"\x7f".as_slice()),
            ("escape", b"\x1b".as_slice()),
            ("tab", b"\t".as_slice()),
            ("insert", b"\x1b[2~".as_slice()),
            ("delete", b"\x1b[3~".as_slice()),
            ("pageup", b"\x1b[5~".as_slice()),
            ("pagedown", b"\x1b[6~".as_slice()),
            ("home", b"\x1b[H".as_slice()),
            ("end", b"\x1b[F".as_slice()),
            ("up", b"\x1b[A".as_slice()),
            ("down", b"\x1b[B".as_slice()),
            ("left", b"\x1b[D".as_slice()),
            ("right", b"\x1b[C".as_slice()),
        ] {
            bindings.add(binding(
                key,
                Modifiers::default(),
                SurfaceMode::empty(),
                SurfaceMode::APP_CURSOR,
                bytes,
            ));
        }
        for (key, bytes) in [
            ("home", b"\x1bOH".as_slice()),
            ("end", b"\x1bOF".as_slice()),
            ("up", b"\x1bOA".as_slice()),
            ("down", b"\x1bOB".as_slice()),
            ("left", b"\x1bOD".as_slice()),
            ("right", b"\x1bOC".as_slice()),
        ] {
            bindings.add(binding(
                key,
                Modifiers::default(),
                SurfaceMode::APP_CURSOR,
                SurfaceMode::empty(),
                bytes,
            ));
        }
        for number in 1..=20 {
            let key = format!("f{number}");
            let bytes = function_key_bytes(number);
            bindings.add(binding(
                &key,
                Modifiers::default(),
                SurfaceMode::empty(),
                SurfaceMode::empty(),
                bytes,
            ));
        }
        for character in b'a'..=b'z' {
            let modifiers = Modifiers {
                control: true,
                ..Modifiers::default()
            };
            bindings.add(binding(
                &(character as char).to_string(),
                modifiers,
                SurfaceMode::empty(),
                SurfaceMode::empty(),
                &[character - b'a' + 1],
            ));
        }

        bindings
    }
}

fn binding(
    key: &str,
    modifiers: Modifiers,
    include: SurfaceMode,
    exclude: SurfaceMode,
    bytes: &[u8],
) -> TerminalBinding {
    TerminalBinding::new(
        Keystroke {
            modifiers,
            key: key.to_string(),
            key_char: None,
        },
        include,
        exclude,
        BindingAction::Bytes(bytes.to_vec()),
    )
}

fn function_key_bytes(number: usize) -> &'static [u8] {
    match number {
        1 => b"\x1bOP",
        2 => b"\x1bOQ",
        3 => b"\x1bOR",
        4 => b"\x1bOS",
        5 => b"\x1b[15~",
        6 => b"\x1b[17~",
        7 => b"\x1b[18~",
        8 => b"\x1b[19~",
        9 => b"\x1b[20~",
        10 => b"\x1b[21~",
        11 => b"\x1b[23~",
        12 => b"\x1b[24~",
        13 => b"\x1b[25~",
        14 => b"\x1b[26~",
        15 => b"\x1b[28~",
        16 => b"\x1b[29~",
        17 => b"\x1b[31~",
        18 => b"\x1b[32~",
        19 => b"\x1b[33~",
        _ => b"\x1b[34~",
    }
}
