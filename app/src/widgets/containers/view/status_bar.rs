//! The Containers mode status bar: the engine and the counts of the open list on
//! the left, the sessions button and the OTTY version on the right.

use iced::Element;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
use otty_ui_kit::components::target_dot::TargetKind;

use crate::i18n::{self, Key};
use crate::theme::ThemeProps;
use crate::widgets::containers::status::StatusViewModel;

/// Render the Containers mode status bar. With an engine the left side shows its
/// name and version, then the counts; without one it names the mode and
/// gives the reason. The right side shows the sessions button (it sends
/// `on_sessions`) and `OTTY <version>`.
pub(crate) fn view<'a, Message: Clone + 'a>(
    vm: &StatusViewModel,
    session_count: usize,
    version: &str,
    on_sessions: Message,
    theme: ThemeProps<'a>,
) -> Element<'a, Message> {
    let mut bar = StatusBar::new(theme.theme.ui());

    bar = match &vm.engine {
        Some(engine) => {
            bar.left(StatusItem::plain_target(TargetKind::Local, engine))
        },
        None => bar.left(StatusItem::text(i18n::t(Key::ContainersStatusMode))),
    };
    bar = bar.left(StatusItem::text(&vm.info));

    bar.right(StatusItem::button(
        i18n::status_sessions(session_count),
        on_sessions,
    ))
    .right(StatusItem::text(format!("OTTY {version}")))
    .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;

    use super::*;
    use crate::theme::ThemeManager;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Message {
        ShowSessions,
    }

    fn vm(engine: Option<&str>) -> StatusViewModel {
        StatusViewModel {
            engine: engine.map(String::from),
            info: String::from("6 containers, 3 running"),
        }
    }

    #[test]
    fn with_an_engine_the_bar_shows_engine_counts_and_version() {
        let themes = ThemeManager::new();
        let vm = vm(Some("Docker 27.3"));
        let mut ui = simulator(view(
            &vm,
            4,
            "0.4.0",
            Message::ShowSessions,
            ThemeProps::new(themes.current()),
        ));

        for label in [
            "Docker 27.3",
            "6 containers, 3 running",
            "4 sessions",
            "OTTY 0.4.0",
        ] {
            ui.find(label).expect("status bar item should be visible");
        }
        assert!(ui.find(i18n::t(Key::ContainersStatusMode)).is_err());
    }

    #[test]
    fn without_an_engine_the_bar_names_the_mode() {
        let themes = ThemeManager::new();
        let vm = vm(None);
        let mut ui = simulator(view(
            &vm,
            0,
            "0.4.0",
            Message::ShowSessions,
            ThemeProps::new(themes.current()),
        ));

        ui.find(i18n::t(Key::ContainersStatusMode))
            .expect("mode name should be visible");
        ui.find("6 containers, 3 running")
            .expect("text should be visible");
    }

    #[test]
    fn the_sessions_button_sends_its_message() {
        let themes = ThemeManager::new();
        let vm = vm(Some("Docker 27.3"));
        let mut ui = simulator(view(
            &vm,
            4,
            "0.4.0",
            Message::ShowSessions,
            ThemeProps::new(themes.current()),
        ));

        ui.click("4 sessions").expect("button should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages, vec![Message::ShowSessions]);
    }
}
