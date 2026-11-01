//! The Docker mode status bar: the engine and the counts of the open list on
//! the left, the sessions button and the OTTY version on the right.

use iced::Element;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
use otty_ui_kit::components::target_dot::TargetKind;

use crate::i18n::{self, Key};
use crate::theme::ThemeProps;
use crate::widgets::docker::status::StatusViewModel;

/// Render the Docker mode status bar. With an engine the left side shows its
/// name and version, then the counts; without one it names the mode and
/// gives the reason. The right side shows the other engines found, the
/// sessions button (it sends `on_sessions`) and `OTTY <version>`.
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
        None => bar.left(StatusItem::text(i18n::t(Key::DockerStatusMode))),
    };
    bar = bar.left(StatusItem::text(&vm.info));

    if let Some(also_found) = &vm.also_found {
        bar = bar.right(StatusItem::text(also_found));
    }

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

    fn vm(engine: Option<&str>, also_found: Option<&str>) -> StatusViewModel {
        StatusViewModel {
            engine: engine.map(String::from),
            info: String::from("6 containers, 3 running"),
            also_found: also_found.map(String::from),
        }
    }

    #[test]
    fn with_an_engine_the_bar_shows_engine_counts_and_version() {
        let themes = ThemeManager::new();
        let vm = vm(Some("Docker 27.3"), Some("Podman 5.2 also found"));
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
            "Podman 5.2 also found",
            "4 sessions",
            "OTTY 0.4.0",
        ] {
            ui.find(label).expect("status bar item should be visible");
        }
        assert!(ui.find(i18n::t(Key::DockerStatusMode)).is_err());
    }

    #[test]
    fn without_an_engine_the_bar_names_the_mode() {
        let themes = ThemeManager::new();
        let vm = vm(None, None);
        let mut ui = simulator(view(
            &vm,
            0,
            "0.4.0",
            Message::ShowSessions,
            ThemeProps::new(themes.current()),
        ));

        ui.find(i18n::t(Key::DockerStatusMode))
            .expect("mode name should be visible");
        ui.find("6 containers, 3 running")
            .expect("text should be visible");
        assert!(ui.find("Podman 5.2 also found").is_err());
    }

    #[test]
    fn the_sessions_button_sends_its_message() {
        let themes = ThemeManager::new();
        let vm = vm(Some("Docker 27.3"), None);
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
