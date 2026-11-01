use iced::Element;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};

use super::super::model::StatusBarViewModel;
use crate::i18n;
use crate::theme::ThemeProps;

/// Render the status bar at the bottom of the window.
///
/// The left side names the machine and, with open sessions, the folder path
/// of the focused pane. With open sessions the right side counts panes and
/// sessions. The program is left out: the header above the focused pane
/// names it. `OTTY <version>` always ends the right side.
pub(crate) fn view<'a, Message: Clone + 'a>(
    vm: &StatusBarViewModel,
    version: &str,
    theme: ThemeProps<'a>,
) -> Element<'a, Message> {
    let mut bar = StatusBar::new(theme.theme.ui()).left(
        StatusItem::plain_target(vm.machine_target(), vm.machine_name()),
    );

    if !vm.is_empty() {
        if let Some(folder) = vm.folder() {
            bar = bar.left(StatusItem::mono(folder));
        }

        if let Some(count) = vm.pane_count() {
            bar = bar.right(StatusItem::text(i18n::status_panes(count)));
        }

        bar = bar
            .right(StatusItem::text(i18n::status_sessions(vm.session_count())));
    }

    bar.right(StatusItem::text(format!("OTTY {version}")))
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_ui_kit::components::target_dot::TargetKind;

    use super::*;
    use crate::theme::ThemeManager;
    use crate::widgets::chrome::model::StatusBarInput;

    fn vm(
        folder: Option<&str>,
        pane_count: Option<usize>,
        session_count: usize,
    ) -> StatusBarViewModel {
        StatusBarViewModel::new(StatusBarInput {
            machine_name: "fedora",
            machine_target: TargetKind::Local,
            folder,
            pane_count,
            session_count,
        })
    }

    #[test]
    fn given_terminal_tab_when_rendered_then_machine_folder_counts_and_version_show()
     {
        let themes = ThemeManager::new();
        let vm = vm(Some("~/otty"), Some(2), 3);
        let mut ui = simulator(view::<()>(
            &vm,
            "0.4.0",
            ThemeProps::new(themes.current()),
        ));

        for label in ["fedora", "~/otty", "2 panes", "3 sessions", "OTTY 0.4.0"]
        {
            ui.find(label).expect("status bar item should be visible");
        }
    }

    #[test]
    fn given_no_tabs_when_rendered_then_machine_and_version_show() {
        let themes = ThemeManager::new();
        let vm = vm(None, None, 0);
        let mut ui = simulator(view::<()>(
            &vm,
            "0.4.0",
            ThemeProps::new(themes.current()),
        ));

        ui.find("fedora").expect("machine should be visible");
        ui.find("OTTY 0.4.0").expect("version should be visible");
        assert!(ui.find("0 sessions").is_err());
    }
}
