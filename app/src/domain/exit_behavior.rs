use std::process::ExitStatus;

use serde::{Deserialize, Serialize};

/// What a command tab does once the process running in it exits.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExitBehavior {
    /// Close the pane as soon as the process exits.
    #[default]
    Close,
    /// Close the pane only when the process succeeds, so failures stay
    /// visible.
    CloseOnSuccess,
    /// Keep the pane open with the final output until the user closes it.
    KeepOpen,
}

impl ExitBehavior {
    /// Report whether a pane whose process ended with `exit_status`
    /// should close.
    pub(crate) fn closes_pane(self, exit_status: &ExitStatus) -> bool {
        match self {
            Self::Close => true,
            Self::CloseOnSuccess => exit_status.success(),
            Self::KeepOpen => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::ExitStatus;

    use super::ExitBehavior;

    fn exit_status(code: i32) -> ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            ExitStatus::from_raw(code << 8)
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::ExitStatusExt;
            ExitStatus::from_raw(code as u32)
        }
    }

    #[test]
    fn given_close_when_process_exits_then_pane_closes_for_any_status() {
        assert!(ExitBehavior::Close.closes_pane(&exit_status(0)));
        assert!(ExitBehavior::Close.closes_pane(&exit_status(1)));
    }

    #[test]
    fn given_close_on_success_when_process_succeeds_then_pane_closes() {
        assert!(ExitBehavior::CloseOnSuccess.closes_pane(&exit_status(0)));
    }

    #[test]
    fn given_close_on_success_when_process_fails_then_pane_stays_open() {
        assert!(!ExitBehavior::CloseOnSuccess.closes_pane(&exit_status(2)));
    }

    #[test]
    fn given_keep_open_when_process_exits_then_pane_stays_open() {
        assert!(!ExitBehavior::KeepOpen.closes_pane(&exit_status(0)));
        assert!(!ExitBehavior::KeepOpen.closes_pane(&exit_status(1)));
    }

    #[test]
    fn given_no_choice_when_defaulted_then_pane_closes_like_before() {
        assert_eq!(ExitBehavior::default(), ExitBehavior::Close);
    }

    #[test]
    fn given_exit_behaviors_when_serialized_then_snake_case_names_are_stored() {
        let names = [
            ExitBehavior::Close,
            ExitBehavior::CloseOnSuccess,
            ExitBehavior::KeepOpen,
        ]
        .map(|behavior| {
            serde_json::to_string(&behavior).expect("should serialize")
        });

        assert_eq!(
            names,
            [
                String::from("\"close\""),
                String::from("\"close_on_success\""),
                String::from("\"keep_open\""),
            ]
        );
    }
}
