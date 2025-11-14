use crate::generate_bindings;
use egui::{Key, Modifiers, PointerButton};
use otty_libterm::TerminalMode;

#[derive(Clone, Hash, Debug, PartialEq, Eq)]
pub enum BindingAction {
    Copy,
    Paste,
    Char(char),
    Esc(String),
    LinkOpen,
    Ignore,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputKind {
    KeyCode(Key),
    Mouse(PointerButton),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding<T> {
    pub target: T,
    pub modifiers: Modifiers,
    pub terminal_mode_include: TerminalMode,
    pub terminal_mode_exclude: TerminalMode,
}

pub type KeyboardBinding = Binding<InputKind>;
pub type MouseBinding = Binding<InputKind>;

#[derive(Clone, Debug)]
pub struct BindingsLayout {
    layout: Vec<(Binding<InputKind>, BindingAction)>,
}

impl Default for BindingsLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl BindingsLayout {
    pub fn new() -> Self {
        let mut layout = Self {
            layout: default_keyboard_bindings(),
        };
        layout.add_bindings(platform_keyboard_bindings());
        layout.add_bindings(mouse_default_bindings());
        layout
    }

    pub fn add_bindings(
        &mut self,
        bindings: Vec<(Binding<InputKind>, BindingAction)>,
    ) {
        for (binding, action) in bindings {
            match self
                .layout
                .iter()
                .position(|(layout_binding, _)| layout_binding == &binding)
            {
                Some(position) => self.layout[position] = (binding, action),
                None => self.layout.push((binding, action)),
            }
        }
    }

    pub fn get_action(
        &self,
        input: InputKind,
        modifiers: Modifiers,
        terminal_mode: TerminalMode,
    ) -> BindingAction {
        for (binding, action) in &self.layout {
            let is_triggered = binding.target == input
                && modifiers.matches_exact(binding.modifiers)
                && terminal_mode.contains(binding.terminal_mode_include)
                && !terminal_mode.intersects(binding.terminal_mode_exclude);

            if is_triggered {
                return action.clone();
            }
        }

        BindingAction::Ignore
    }
}

fn default_keyboard_bindings() -> Vec<(Binding<InputKind>, BindingAction)> {
    generate_bindings!(
        KeyboardBinding;
        // NONE MODIFIERS
        Enter;     BindingAction::Char('\x0d');
        Backspace; BindingAction::Char('\x7f');
        Escape;    BindingAction::Char('\x1b');
        Tab;       BindingAction::Char('\x09');
        Insert;    BindingAction::Esc("\x1b[2~".into());
        Delete;    BindingAction::Esc("\x1b[3~".into());
        PageUp;    BindingAction::Esc("\x1b[5~".into());
        PageDown;  BindingAction::Esc("\x1b[6~".into());
        F1;        BindingAction::Esc("\x1bOP".into());
        F2;        BindingAction::Esc("\x1bOQ".into());
        F3;        BindingAction::Esc("\x1bOR".into());
        F4;        BindingAction::Esc("\x1bOS".into());
        F5;        BindingAction::Esc("\x1b[15~".into());
        F6;        BindingAction::Esc("\x1b[17~".into());
        F7;        BindingAction::Esc("\x1b[18~".into());
        F8;        BindingAction::Esc("\x1b[19~".into());
        F9;        BindingAction::Esc("\x1b[20~".into());
        F10;       BindingAction::Esc("\x1b[21~".into());
        F11;       BindingAction::Esc("\x1b[23~".into());
        F12;       BindingAction::Esc("\x1b[24~".into());
        F13;       BindingAction::Esc("\x1b[25~".into());
        F14;       BindingAction::Esc("\x1b[26~".into());
        F15;       BindingAction::Esc("\x1b[28~".into());
        F16;       BindingAction::Esc("\x1b[29~".into());
        F17;       BindingAction::Esc("\x1b[31~".into());
        F18;       BindingAction::Esc("\x1b[32~".into());
        F19;       BindingAction::Esc("\x1b[33~".into());
        F20;       BindingAction::Esc("\x1b[34~".into());
        // APP_CURSOR Excluding
        End,        ~TerminalMode::APP_CURSOR; BindingAction::Esc("\x1b[F".into());
        Home,       ~TerminalMode::APP_CURSOR; BindingAction::Esc("\x1b[H".into());
        ArrowUp,    ~TerminalMode::APP_CURSOR; BindingAction::Esc("\x1b[A".into());
        ArrowDown,  ~TerminalMode::APP_CURSOR; BindingAction::Esc("\x1b[B".into());
        ArrowLeft,  ~TerminalMode::APP_CURSOR; BindingAction::Esc("\x1b[D".into());
        ArrowRight, ~TerminalMode::APP_CURSOR; BindingAction::Esc("\x1b[C".into());
        // APP_CURSOR Including
        End,        +TerminalMode::APP_CURSOR; BindingAction::Esc("\x1BOF".into());
        Home,       +TerminalMode::APP_CURSOR; BindingAction::Esc("\x1BOH".into());
        ArrowUp,    +TerminalMode::APP_CURSOR; BindingAction::Esc("\x1bOA".into());
        ArrowDown,  +TerminalMode::APP_CURSOR; BindingAction::Esc("\x1bOB".into());
        ArrowLeft,  +TerminalMode::APP_CURSOR; BindingAction::Esc("\x1bOD".into());
        ArrowRight, +TerminalMode::APP_CURSOR; BindingAction::Esc("\x1bOC".into());
        // CTRL (aka COMMAND on macOS)
        ArrowUp,    Modifiers::COMMAND; BindingAction::Esc("\x1b[1;5A".into());
        ArrowDown,  Modifiers::COMMAND; BindingAction::Esc("\x1b[1;5B".into());
        ArrowLeft,  Modifiers::COMMAND; BindingAction::Esc("\x1b[1;5D".into());
        ArrowRight, Modifiers::COMMAND; BindingAction::Esc("\x1b[1;5C".into());
        End,          Modifiers::CTRL; BindingAction::Esc("\x1b[1;5F".into());
        Home,         Modifiers::CTRL; BindingAction::Esc("\x1b[1;5H".into());
        Delete,       Modifiers::CTRL; BindingAction::Esc("\x1b[3;5~".into());
        PageUp,       Modifiers::CTRL; BindingAction::Esc("\x1b[5;5~".into());
        PageDown,     Modifiers::CTRL; BindingAction::Esc("\x1b[6;5~".into());
        F1,           Modifiers::CTRL; BindingAction::Esc("\x1bO;5P".into());
        F2,           Modifiers::CTRL; BindingAction::Esc("\x1bO;5Q".into());
        F3,           Modifiers::CTRL; BindingAction::Esc("\x1bO;5R".into());
        F4,           Modifiers::CTRL; BindingAction::Esc("\x1bO;5S".into());
        F5,           Modifiers::CTRL; BindingAction::Esc("\x1b[15;5~".into());
        F6,           Modifiers::CTRL; BindingAction::Esc("\x1b[17;5~".into());
        F7,           Modifiers::CTRL; BindingAction::Esc("\x1b[18;5~".into());
        F8,           Modifiers::CTRL; BindingAction::Esc("\x1b[19;5~".into());
        F9,           Modifiers::CTRL; BindingAction::Esc("\x1b[20;5~".into());
        F10,          Modifiers::CTRL; BindingAction::Esc("\x1b[21;5~".into());
        F11,          Modifiers::CTRL; BindingAction::Esc("\x1b[23;5~".into());
        F12,          Modifiers::CTRL; BindingAction::Esc("\x1b[24;5~".into());
        A,            Modifiers::CTRL; BindingAction::Char('\x01');
        B,            Modifiers::CTRL; BindingAction::Char('\x02');
        C,            Modifiers::CTRL; BindingAction::Char('\x03');
        D,            Modifiers::CTRL; BindingAction::Char('\x04');
        E,            Modifiers::CTRL; BindingAction::Char('\x05');
        F,            Modifiers::CTRL; BindingAction::Char('\x06');
        G,            Modifiers::CTRL; BindingAction::Char('\x07');
        H,            Modifiers::CTRL; BindingAction::Char('\x08');
        I,            Modifiers::CTRL; BindingAction::Char('\x09');
        J,            Modifiers::CTRL; BindingAction::Char('\x0a');
        K,            Modifiers::CTRL; BindingAction::Char('\x0b');
        L,            Modifiers::CTRL; BindingAction::Char('\x0c');
        M,            Modifiers::CTRL; BindingAction::Char('\x0d');
        N,            Modifiers::CTRL; BindingAction::Char('\x0e');
        O,            Modifiers::CTRL; BindingAction::Char('\x0f');
        P,            Modifiers::CTRL; BindingAction::Char('\x10');
        Q,            Modifiers::CTRL; BindingAction::Char('\x11');
        R,            Modifiers::CTRL; BindingAction::Char('\x12');
        S,            Modifiers::CTRL; BindingAction::Char('\x13');
        T,            Modifiers::CTRL; BindingAction::Char('\x14');
        U,            Modifiers::CTRL; BindingAction::Char('\x51');
        V,            Modifiers::CTRL; BindingAction::Char('\x16');
        W,            Modifiers::CTRL; BindingAction::Char('\x17');
        X,            Modifiers::CTRL; BindingAction::Char('\x18');
        Y,            Modifiers::CTRL; BindingAction::Char('\x19');
        Z,            Modifiers::CTRL; BindingAction::Char('\x1a');
        OpenBracket,  Modifiers::CTRL; BindingAction::Char('\x1b');
        CloseBracket, Modifiers::CTRL; BindingAction::Char('\x1d');
        Backslash,    Modifiers::CTRL; BindingAction::Char('\x1c');
        Minus,        Modifiers::CTRL; BindingAction::Char('\x1f');
        // SHIFT
        Enter,      Modifiers::SHIFT; BindingAction::Char('\x0d');
        Backspace,  Modifiers::SHIFT; BindingAction::Char('\x7f');
        Tab,        Modifiers::SHIFT; BindingAction::Esc("\x1b[Z".into());
        End,        Modifiers::SHIFT, +TerminalMode::ALT_SCREEN; BindingAction::Esc("\x1b[1;2F".into());
        Home,       Modifiers::SHIFT, +TerminalMode::ALT_SCREEN; BindingAction::Esc("\x1b[1;2H".into());
        PageUp,     Modifiers::SHIFT, +TerminalMode::ALT_SCREEN; BindingAction::Esc("\x1b[5;2~".into());
        PageDown,   Modifiers::SHIFT, +TerminalMode::ALT_SCREEN; BindingAction::Esc("\x1b[6;2~".into());
        ArrowUp,    Modifiers::SHIFT; BindingAction::Esc("\x1b[1;2A".into());
        ArrowDown,  Modifiers::SHIFT; BindingAction::Esc("\x1b[1;2B".into());
        ArrowLeft,  Modifiers::SHIFT; BindingAction::Esc("\x1b[1;2D".into());
        ArrowRight, Modifiers::SHIFT; BindingAction::Esc("\x1b[1;2C".into());
        // ALT
        Backspace,  Modifiers::ALT; BindingAction::Esc("\x1b\x7f".into());
        End,        Modifiers::ALT; BindingAction::Esc("\x1b[1;3F".into());
        Home,       Modifiers::ALT; BindingAction::Esc("\x1b[1;3H".into());
        Insert,     Modifiers::ALT; BindingAction::Esc("\x1b[3;2~".into());
        Delete,     Modifiers::ALT; BindingAction::Esc("\x1b[3;3~".into());
        PageUp,     Modifiers::ALT; BindingAction::Esc("\x1b[5;3~".into());
        PageDown,   Modifiers::ALT; BindingAction::Esc("\x1b[6;3~".into());
        ArrowUp,    Modifiers::ALT; BindingAction::Esc("\x1b[1;3A".into());
        ArrowDown,  Modifiers::ALT; BindingAction::Esc("\x1b[1;3B".into());
        ArrowLeft,  Modifiers::ALT; BindingAction::Esc("\x1b[1;3D".into());
        ArrowRight, Modifiers::ALT; BindingAction::Esc("\x1b[1;3C".into());
        // SHIFT + ALT
        End,        Modifiers::SHIFT | Modifiers::ALT; BindingAction::Esc("\x1b[1;4F".into());
        Home,       Modifiers::SHIFT | Modifiers::ALT; BindingAction::Esc("\x1b[1;4H".into());
        ArrowUp,    Modifiers::SHIFT | Modifiers::ALT; BindingAction::Esc("\x1b[1;4A".into());
        ArrowDown,  Modifiers::SHIFT | Modifiers::ALT; BindingAction::Esc("\x1b[1;4B".into());
        ArrowLeft,  Modifiers::SHIFT | Modifiers::ALT; BindingAction::Esc("\x1b[1;4D".into());
        ArrowRight, Modifiers::SHIFT | Modifiers::ALT; BindingAction::Esc("\x1b[1;4C".into());
        // SHIFT + CTRL
        End,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Esc("\x1b[1;6F".into());
        Home,       Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Esc("\x1b[1;6H".into());
        ArrowUp,    Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Esc("\x1b[1;6A".into());
        ArrowDown,  Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Esc("\x1b[1;6B".into());
        ArrowLeft,  Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Esc("\x1b[1;6D".into());
        ArrowRight, Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Esc("\x1b[1;6C".into());
        A,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x01');
        B,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x02');
        C,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x03');
        D,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x04');
        E,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x05');
        F,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x06');
        G,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x07');
        H,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x08');
        I,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x09');
        J,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x0a');
        K,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x0b');
        L,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x0c');
        M,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x0d');
        N,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x0e');
        O,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x0f');
        P,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x10');
        Q,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x11');
        R,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x12');
        S,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x13');
        T,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x14');
        U,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x51');
        V,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x16');
        W,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x17');
        X,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x18');
        Y,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x19');
        Z,        Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x1a');
        Num2,     Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x00');
        Num6,     Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x1e');
        Minus,    Modifiers::SHIFT | Modifiers::CTRL; BindingAction::Char('\x1f');
        // CTRL + ALT
        End,        Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;7F".into());
        Home,       Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;7H".into());
        PageUp,     Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[5;7~".into());
        PageDown,   Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[6;7~".into());
        ArrowUp,    Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;7A".into());
        ArrowDown,  Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;7B".into());
        ArrowLeft,  Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;7D".into());
        ArrowRight, Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;7C".into());
        // SHIFT + CTRL + ALT
        End,        Modifiers::SHIFT | Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;8F".into());
        Home,       Modifiers::SHIFT | Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;8H".into());
        ArrowUp,    Modifiers::SHIFT | Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;8A".into());
        ArrowDown,  Modifiers::SHIFT | Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;8B".into());
        ArrowLeft,  Modifiers::SHIFT | Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;8D".into());
        ArrowRight, Modifiers::SHIFT | Modifiers::CTRL | Modifiers::ALT; BindingAction::Esc("\x1b[1;8C".into());
    )
}

#[cfg(target_os = "macos")]
fn platform_keyboard_bindings() -> Vec<(Binding<InputKind>, BindingAction)> {
    generate_bindings!(
        KeyboardBinding;
        C, Modifiers::MAC_CMD; BindingAction::Copy;
        V, Modifiers::MAC_CMD; BindingAction::Paste;
    )
}

#[cfg(not(target_os = "macos"))]
fn platform_keyboard_bindings() -> Vec<(Binding<InputKind>, BindingAction)> {
    generate_bindings!(
        KeyboardBinding;
        C, Modifiers::SHIFT | Modifiers::COMMAND; BindingAction::Copy;
        V, Modifiers::SHIFT | Modifiers::COMMAND; BindingAction::Paste;
    )
}

fn mouse_default_bindings() -> Vec<(Binding<InputKind>, BindingAction)> {
    generate_bindings!(
        MouseBinding;
        Primary, Modifiers::COMMAND; BindingAction::LinkOpen;
    )
}
