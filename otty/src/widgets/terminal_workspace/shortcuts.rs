//! Application shortcuts registered inside every terminal widget.
//!
//! The terminal widget owns key matching and reports a match back as
//! [`otty_ui_term::Event::Action`]; this module defines the actions it
//! can report and builds the bindings that produce them.

#[cfg(target_os = "macos")]
use iced::keyboard::Modifiers;
use iced::widget::pane_grid;
#[cfg(target_os = "macos")]
use otty_ui_term::SurfaceMode;
#[cfg(target_os = "macos")]
use otty_ui_term::bindings::KeyboardBinding;
use otty_ui_term::bindings::{Binding, BindingAction, InputKind};
#[cfg(target_os = "macos")]
use otty_ui_term::generate_bindings;

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
/// The split shortcut is bound on macOS only. Elsewhere
/// [`Modifiers::COMMAND`] resolves to `CTRL`, and the terminal binds
/// Ctrl+D to EOF, so registering the same combination here would replace
/// that binding: equal bindings overwrite each other. Binding this on
/// other platforms needs a key that does not collide with a terminal
/// control character.
pub(crate) fn app_bindings()
-> Vec<(Binding<InputKind>, BindingAction<TerminalWorkspaceAction>)> {
    #[cfg(target_os = "macos")]
    {
        generate_bindings!(
            KeyboardBinding;
            "d", Modifiers::LOGO; BindingAction::Action(
                TerminalWorkspaceAction::SplitPane {
                    axis: pane_grid::Axis::Vertical,
                }
            );
        )
    }

    #[cfg(not(target_os = "macos"))]
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn given_macos_when_bindings_built_then_command_d_splits_vertically() {
        let bindings = app_bindings();

        let (binding, action) = bindings
            .first()
            .expect("macOS must register the split shortcut");
        assert_eq!(binding.target, InputKind::Char(String::from("d")));
        assert_eq!(binding.modifiers, Modifiers::LOGO);
        assert_eq!(
            action,
            &BindingAction::Action(TerminalWorkspaceAction::SplitPane {
                axis: pane_grid::Axis::Vertical,
            }),
            "Cmd+D must resolve to a vertical split of the reporting pane"
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn given_other_platforms_when_bindings_built_then_ctrl_d_is_left_alone() {
        // `Modifiers::COMMAND` is `CTRL` here, and the terminal binds
        // Ctrl+D to EOF; claiming it would replace the EOF binding.
        assert!(
            app_bindings().is_empty(),
            "no application binding may shadow a terminal control character"
        );
    }
}
