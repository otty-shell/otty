use gpui::{Keystroke, Modifiers};
use otty_libterm::surface::SurfaceMode;
use otty_ui_term_gpui::{BindingAction, TerminalBinding, TerminalBindings};

fn key(value: &str, modifiers: Modifiers) -> Keystroke {
    Keystroke {
        key: value.to_string(),
        key_char: None,
        modifiers,
    }
}

#[test]
fn resolves_default_navigation_for_normal_and_application_cursor_modes() {
    let bindings = TerminalBindings::default();

    assert_eq!(
        bindings
            .resolve(&key("up", Modifiers::default()), SurfaceMode::empty()),
        Some(&BindingAction::Bytes(b"\x1b[A".to_vec()))
    );
    assert_eq!(
        bindings.resolve(
            &key("up", Modifiers::default()),
            SurfaceMode::APP_CURSOR,
        ),
        Some(&BindingAction::Bytes(b"\x1bOA".to_vec()))
    );
}

#[test]
fn replacing_an_exact_binding_preserves_lookup_order() {
    let mut bindings = TerminalBindings::default();
    let custom = TerminalBinding::new(
        key("up", Modifiers::default()),
        SurfaceMode::empty(),
        SurfaceMode::APP_CURSOR,
        BindingAction::Bytes(b"custom".to_vec()),
    );

    bindings.add(custom);

    assert_eq!(
        bindings
            .resolve(&key("up", Modifiers::default()), SurfaceMode::empty()),
        Some(&BindingAction::Bytes(b"custom".to_vec()))
    );
}

#[test]
fn encodes_control_character_and_printable_text_once() {
    let bindings = TerminalBindings::default();
    let ctrl = Modifiers {
        control: true,
        ..Modifiers::default()
    };

    assert_eq!(
        bindings.resolve(&key("c", ctrl), SurfaceMode::empty()),
        Some(&BindingAction::Bytes(vec![0x03]))
    );
    assert_eq!(
        bindings.bytes_for_printable(&key("é", Modifiers::default())),
        Some("é".as_bytes())
    );
}
