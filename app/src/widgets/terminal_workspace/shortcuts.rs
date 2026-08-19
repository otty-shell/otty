//! Application shortcuts registered inside every terminal widget.
//!
//! The terminal widget owns key matching and reports a match back as
//! [`otty_ui_term::Event::BindingActionDispatched`]; this module defines
//! the actions it can report and builds the bindings that produce them.

use iced::keyboard::Modifiers;
use iced::widget::pane_grid;
use otty_ui_term::bindings::{
    Binding, BindingAction, InputKind, KeyboardBinding,
};
use otty_ui_term::{SurfaceMode, generate_bindings};

/// An application action a terminal widget can report back.
///
/// This is the `T` of [`otty_ui_term::BindingAction`] and
/// [`otty_ui_term::Event`] for this application: the widget crate never
/// interprets it, it only carries it from the matched key to the event.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TerminalWorkspaceAction {
    /// Split the reporting pane along the given axis.
    SplitPane { axis: pane_grid::Axis },
}

/// Return the application bindings every terminal widget registers.
///
/// macOS uses Cmd+D. Other platforms use Logo+Shift+D so the application
/// binding does not replace the terminal's Ctrl+D EOF binding.
pub(crate) fn app_bindings()
-> Vec<(Binding<InputKind>, BindingAction<TerminalWorkspaceAction>)> {
    let modifiers = split_pane_modifiers(cfg!(target_os = "macos"));

    generate_bindings!(
        KeyboardBinding;
        "d", modifiers; BindingAction::Action(
            TerminalWorkspaceAction::SplitPane {
                axis: pane_grid::Axis::Vertical,
            }
        );
    )
}

fn split_pane_modifiers(is_macos: bool) -> Modifiers {
    if is_macos {
        Modifiers::LOGO
    } else {
        Modifiers::LOGO | Modifiers::SHIFT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_platform_when_bindings_built_then_shortcut_splits_vertically() {
        let bindings = app_bindings();

        let (binding, action) = bindings
            .first()
            .expect("every platform must register the split shortcut");
        assert_eq!(binding.target, InputKind::Char(String::from("d")));
        assert_eq!(
            binding.modifiers,
            split_pane_modifiers(cfg!(target_os = "macos"))
        );
        assert_eq!(
            action,
            &BindingAction::Action(TerminalWorkspaceAction::SplitPane {
                axis: pane_grid::Axis::Vertical,
            }),
            "the platform shortcut must split the reporting pane vertically"
        );
    }

    #[test]
    fn given_target_platform_when_modifiers_selected_then_ctrl_d_is_never_used()
    {
        assert_eq!(split_pane_modifiers(true), Modifiers::LOGO);
        assert_eq!(
            split_pane_modifiers(false),
            Modifiers::LOGO | Modifiers::SHIFT
        );
    }
}
