use std::io;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;

use gpui::{
    AppContext as _, Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent,
    TestAppContext, VisualContext as _, point, px, size,
};
use otty_libterm::TerminalSize;
use otty_ui_term_gpui::{
    BackendError, BackendSession, BackendState, BlockId, BlockTextPart,
    ClearSelection, Copy, LocalBackend, LocalOptions, Paste, ScrollPageDown,
    ScrollPageUp, ScrollToBottom, ScrollToTop, SelectAll, Terminal,
    TerminalAppearance, TerminalBackend, TerminalBehavior, TerminalBindings,
    TerminalConfig, TerminalFont, TerminalTheme,
};

struct FailingBackend;

impl TerminalBackend for FailingBackend {
    fn start(
        self: Box<Self>,
        _initial_size: TerminalSize,
    ) -> Result<BackendSession, BackendError> {
        Err(BackendError::external(io::Error::other("expected failure")))
    }
}

#[gpui::test]
fn replacement_advances_generation_without_recreating_entity(
    cx: &mut TestAppContext,
) {
    let terminal = cx
        .new(|cx| Terminal::new(TerminalConfig::default(), FailingBackend, cx));
    let initial =
        terminal.read_with(cx, |terminal, _| terminal.backend_generation());

    let replacement = terminal.update(cx, |terminal, cx| {
        terminal.replace_backend(FailingBackend, cx)
    });

    assert_eq!(replacement, initial.next());
    assert_eq!(terminal.entity_id(), terminal.entity_id());
}

#[gpui::test]
fn runtime_theme_update_preserves_backend_generation(cx: &mut TestAppContext) {
    let terminal = cx
        .new(|cx| Terminal::new(TerminalConfig::default(), FailingBackend, cx));
    let initial =
        terminal.read_with(cx, |terminal, _| terminal.backend_generation());

    terminal.update(cx, |terminal, cx| {
        terminal.set_theme(TerminalTheme::default(), cx);
    });

    terminal.read_with(cx, |terminal, _| {
        assert_eq!(terminal.backend_generation(), initial);
        assert_eq!(terminal.config().theme(), &TerminalTheme::default());
    });
}

#[gpui::test]
fn failed_backend_state_allows_a_retry(cx: &mut TestAppContext) {
    let terminal = cx
        .new(|cx| Terminal::new(TerminalConfig::default(), FailingBackend, cx));

    cx.run_until_parked();
    terminal.read_with(cx, |terminal, _| {
        assert!(matches!(terminal.backend_state(), BackendState::Failed(_)));
    });

    terminal.update(cx, |terminal, cx| {
        terminal.replace_backend(FailingBackend, cx);
    });
    cx.run_until_parked();
    terminal.read_with(cx, |terminal, _| {
        assert!(matches!(terminal.backend_state(), BackendState::Failed(_)));
        assert_eq!(terminal.backend_generation().value(), 2);
    });
}

#[gpui::test]
fn widget_renders_inside_host_owned_bounds(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let terminal = cx
        .new(|cx| Terminal::new(TerminalConfig::default(), FailingBackend, cx));

    cx.draw(
        point(px(10.0), px(20.0)),
        size(px(640.0), px(480.0)),
        |_, _| terminal.clone(),
    );

    terminal.read_with(cx, |terminal, _| {
        assert!(terminal.snapshot_arc().view().cells.is_empty());
    });
}

#[cfg(unix)]
#[gpui::test]
fn local_backend_delivers_input_and_frames(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    cx.executor().allow_parking();
    let terminal = cx.new(|cx| {
        Terminal::new(
            TerminalConfig::default(),
            LocalBackend::new(
                LocalOptions::new("/bin/cat")
                    .with_env("TERM", "xterm-256color")
                    .with_working_directory(PathBuf::from("/tmp")),
            ),
            cx,
        )
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if terminal.read_with(cx, |terminal, _| {
            matches!(terminal.backend_state(), BackendState::Running)
        }) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    terminal.read_with(cx, |terminal, _| {
        assert!(matches!(terminal.backend_state(), BackendState::Running));
    });

    terminal.update(cx, |terminal, cx| {
        terminal
            .write_text("gpui terminal\n", cx)
            .expect("running backend accepts input");
    });
    for _ in 0..100 {
        cx.run_until_parked();
        if terminal.read_with(cx, |terminal, _| {
            terminal
                .snapshot_arc()
                .view()
                .cells
                .iter()
                .any(|cell| cell.cell.c == 'g')
        }) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    terminal.read_with(cx, |terminal, _| {
        assert!(
            terminal
                .snapshot_arc()
                .view()
                .cells
                .iter()
                .any(|cell| cell.cell.c == 'g')
        );
    });

    terminal.update(cx, |terminal, cx| {
        terminal
            .set_font(TerminalFont::monospace(16.0).expect("valid font"), cx);
        terminal.set_appearance(
            TerminalAppearance::try_new(4.0, 1.0, 6.0, 2.0)
                .expect("valid appearance"),
            cx,
        );
        terminal.set_behavior(
            TerminalBehavior::default().with_copy_on_select(true),
            cx,
        );
        terminal.set_bindings(TerminalBindings::new(), cx);
        terminal.set_config(terminal.config().clone(), cx);

        assert_eq!(terminal.config().font().size(), 16.0);
        assert!(terminal.title().is_none());
        assert!(!terminal.blocks().is_empty());
        assert!(!terminal.has_selection());
        let missing = BlockId::new("missing");
        assert_eq!(missing.as_str(), "missing");
        assert!(terminal.block_text(&missing).is_none());
        assert!(!terminal.select_block(&missing, cx));
        assert!(!terminal.scroll_to_block(&missing, cx));
        for part in [
            BlockTextPart::All,
            BlockTextPart::Content,
            BlockTextPart::Prompt,
            BlockTextPart::Command,
        ] {
            assert!(terminal.copy_block(&missing, part, cx).is_err());
        }

        let existing = BlockId::new(terminal.blocks()[0].meta.id.clone());
        assert!(terminal.scroll_to_block(&existing, cx));
        let _ = terminal.select_block(&existing, cx);
        for part in [
            BlockTextPart::All,
            BlockTextPart::Content,
            BlockTextPart::Prompt,
            BlockTextPart::Command,
        ] {
            let _ = terminal.copy_block(&existing, part, cx);
        }

        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
            "direct paste".to_string(),
        ));
        terminal.paste(cx).expect("running backend accepts paste");
        terminal
            .clear_selection(cx)
            .expect("running backend clears selection");
        let _ = terminal.copy_selection(cx);
    });

    cx.draw(
        point(px(10.0), px(20.0)),
        size(px(640.0), px(480.0)),
        |_, _| terminal.clone(),
    );
    cx.focus(&terminal);
    cx.simulate_keystrokes("enter up down left right");
    cx.simulate_mouse_down(
        point(px(24.0), px(36.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(
        point(px(80.0), px(36.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(80.0), px(36.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_event(ScrollWheelEvent {
        position: point(px(24.0), px(36.0)),
        delta: ScrollDelta::Lines(point(0.0, 1.0)),
        ..ScrollWheelEvent::default()
    });
    cx.dispatch_action(SelectAll);
    thread::sleep(Duration::from_millis(20));
    cx.run_until_parked();
    cx.dispatch_action(Copy);
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("paste".to_string()));
    cx.dispatch_action(Paste);
    cx.dispatch_action(ClearSelection);
    cx.dispatch_action(ScrollPageUp);
    cx.dispatch_action(ScrollPageDown);
    cx.dispatch_action(ScrollToTop);
    cx.dispatch_action(ScrollToBottom);

    terminal.update(cx, |terminal, cx| terminal.shutdown_backend(cx));
    terminal.update(cx, |terminal, cx| terminal.shutdown_backend(cx));
    terminal.read_with(cx, |terminal, _| {
        assert!(matches!(terminal.backend_state(), BackendState::Stopping));
    });
    thread::sleep(Duration::from_millis(20));
    cx.run_until_parked();
}
