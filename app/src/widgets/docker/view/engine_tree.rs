//! The Docker mode left panel under the switcher: one root per engine and
//! its four lists with their counts.

use iced::widget::{Column, container, scrollable, text};
use iced::{Element, Length, Padding};
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::session_row::SessionTone;
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::DockerIntent;
use super::super::model::{TreeNote, TreeRowViewModel};
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Props for the engine tree.
pub(crate) struct EngineTreeProps<'a> {
    pub(crate) rows: Vec<TreeRowViewModel>,
    /// A short text under the rows, such as why no engine is shown.
    pub(crate) note: Option<&'static str>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the engine tree. A press on a list row shows that list in the main
/// area; a press on an engine root only opens or closes that engine.
pub(crate) fn view(props: EngineTreeProps<'_>) -> Element<'_, DockerIntent> {
    let palette = props.theme.theme.ui();

    let rows = props.rows.into_iter().map(|row| tree_row(row, palette));

    let mut list = Column::with_children(rows).width(Length::Fill);
    if let Some(note) = props.note {
        let hint = typography::SMALL;
        list = list.push(
            container(
                text(note)
                    .size(hint.size())
                    .line_height(iced::Pixels(hint.line_height()))
                    .font(fonts::UI)
                    .color(palette.text_muted()),
            )
            .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_3])),
        );
    }

    let body = scrollable(list)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(palette));

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding {
            top: spacing::SPACE_1,
            right: spacing::SPACE_3,
            bottom: spacing::SPACE_3,
            left: spacing::SPACE_3,
        })
        .into()
}

fn tree_row<'a>(
    vm: TreeRowViewModel,
    palette: &UiPalette,
) -> Element<'a, DockerIntent> {
    let press = if vm.is_engine {
        DockerIntent::ToggleEngine(vm.place.engine().clone())
    } else {
        DockerIntent::Open(vm.place)
    };

    let mut row = TreeRow::new(vm.label, palette)
        .depth(vm.depth)
        .selected(vm.is_selected)
        .on_press(press);
    if vm.is_engine {
        row = row.caret(vm.is_open).dot(TargetKind::Local);
    }

    let row = match vm.note {
        TreeNote::Count(count) => row.count(count),
        TreeNote::Text(text) => row.state(text),
        TreeNote::Failed => row
            .state(i18n::t(Key::DockerListFailedShort))
            .count_tone(SessionTone::Failed),
        TreeNote::Nothing => row,
    };

    row.into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_docker::{EngineKind, fixtures};

    use super::*;
    use crate::theme::ThemeManager;
    use crate::widgets::docker::types::{Place, ResourceKind};

    fn row(label: &str, note: TreeNote, place: Place) -> TreeRowViewModel {
        TreeRowViewModel {
            label: label.to_string(),
            depth: 1,
            is_engine: false,
            is_open: false,
            note,
            is_selected: false,
            place,
        }
    }

    fn place(resource: ResourceKind) -> Place {
        Place::new(
            fixtures::engine(EngineKind::Docker).address().clone(),
            resource,
        )
    }

    #[test]
    fn a_press_on_a_list_row_opens_that_list() {
        let themes = ThemeManager::new();
        let rows = vec![
            TreeRowViewModel {
                depth: 0,
                is_engine: true,
                is_open: true,
                ..row(
                    "Docker 29.9.1",
                    TreeNote::Nothing,
                    place(ResourceKind::Containers),
                )
            },
            row(
                "Containers",
                TreeNote::Text(String::from("2 running")),
                place(ResourceKind::Containers),
            ),
            row("Images", TreeNote::Count(3), place(ResourceKind::Images)),
            row("Volumes", TreeNote::Failed, place(ResourceKind::Volumes)),
        ];
        let mut ui = simulator(view(EngineTreeProps {
            rows,
            note: None,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("2 running").expect("running count should show");
        ui.find("3").expect("image count should show");
        ui.find(i18n::t(Key::DockerListFailedShort))
            .expect("failed list should say so");
        ui.click("Images").expect("images row should be visible");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![DockerIntent::Open(place(ResourceKind::Images))]
        );
    }

    #[test]
    fn a_press_on_an_engine_root_only_opens_or_closes_it() {
        let themes = ThemeManager::new();
        let podman = fixtures::engine(EngineKind::Podman);
        let rows = vec![TreeRowViewModel {
            depth: 0,
            is_engine: true,
            is_open: true,
            ..row(
                "Podman 5.8.8",
                TreeNote::Nothing,
                Place::new(podman.address().clone(), ResourceKind::Containers),
            )
        }];
        let mut ui = simulator(view(EngineTreeProps {
            rows,
            note: None,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click("Podman 5.8.8")
            .expect("the root should be visible");

        let intents: Vec<DockerIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![DockerIntent::ToggleEngine(podman.address().clone())]
        );
    }

    #[test]
    fn a_note_shows_under_the_rows() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(EngineTreeProps {
            rows: Vec::new(),
            note: Some("Docker is not running."),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Docker is not running.")
            .expect("the note should show");
    }
}
