//! The last blocks of a profile's dashboard, the same for every billing:
//! the Profile facts, the top section and where the numbers come from.

use iced::widget::{Column, column, row, text};
use iced::{Alignment, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::key_value_list::KeyValueList;
use otty_ui_kit::components::status_badge::{StatusBadge, StatusTone};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};

use super::super::event::AgentsIntent;
use super::super::model::{SourceRowViewModel, TopViewModel};
use super::{blocks, top};

/// Gap between the blocks (`gap: 28px`).
const BLOCK_GAP: f32 = 28.0;
/// Gap between a block title and its content (`gap: 10px`).
const TITLE_GAP: f32 = 10.0;

/// Render the Profile facts, the top section and the sources.
pub(super) fn view<'a>(
    facts: Vec<(&'static str, String)>,
    top: Option<TopViewModel>,
    sources: Vec<SourceRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut list = KeyValueList::new(palette);
    for (label, value) in facts {
        list = list.entry(label, value);
    }

    let mut blocks = Column::new().spacing(BLOCK_GAP).push(
        column![
            blocks::title(i18n::t(Key::AgentsProfileTitle), palette),
            list
        ]
        .spacing(TITLE_GAP),
    );
    if let Some(top) = top {
        blocks = blocks.push(top::view(top, palette));
    }
    if !sources.is_empty() {
        blocks = blocks.push(sources_list(sources, palette));
    }

    blocks.width(Length::Fill).into()
}

/// Where the numbers come from: each source with its number, its name and
/// its state; why it failed is the tooltip of the state.
fn sources_list<'a>(
    sources: Vec<SourceRowViewModel>,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let body = typography::BODY;

    let mut list = Column::new()
        .spacing(spacing::SPACE_3)
        .push(blocks::title(i18n::t(Key::AgentsSourcesTitle), palette));
    for (index, source) in sources.into_iter().enumerate() {
        let line = row![
            text(format!("{}", index + 1))
                .size(body.size())
                .font(fonts::UI)
                .color(palette.text_muted()),
            text(source.name)
                .size(body.size())
                .font(fonts::UI)
                .color(palette.text_body())
                .width(Length::Fill),
            state_badge(source.state, source.tone, source.tip, palette),
        ]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center);

        list = list.push(line);
    }

    list.into()
}

/// The state of a source, with `tip` as its tooltip when it has one.
fn state_badge(
    state: &'static str,
    tone: StatusTone,
    tip: Option<String>,
    palette: &UiPalette,
) -> StatusBadge {
    let badge = StatusBadge::new(state, tone, palette);

    match tip {
        Some(tip) => badge.tooltip(tip),
        None => badge,
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::container;
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    fn sources() -> Vec<SourceRowViewModel> {
        vec![
            SourceRowViewModel {
                name: "ChatGPT usage API",
                state: i18n::t(Key::AgentsSourceLoginExpired),
                tone: StatusTone::Warn,
                tip: Some(String::from("Open the agent to log in again")),
            },
            SourceRowViewModel {
                name: "Codex app-server",
                state: i18n::t(Key::AgentsSourceNotAvailable),
                tone: StatusTone::Neutral,
                tip: Some(String::from("no program")),
            },
        ]
    }

    fn shown(top: Option<TopViewModel>, texts: &[&str], missing: &[&str]) {
        let themes = ThemeManager::new();
        let facts = vec![
            (
                i18n::t(Key::AgentsFieldAccount),
                String::from("user@example.com"),
            ),
            (i18n::t(Key::AgentsFieldConfigDir), String::from("~/.codex")),
        ];
        let column = view(facts, top, sources(), themes.current().ui());
        let mut ui = simulator(container(column).width(480.0));

        for text in texts {
            ui.find(*text).expect("the text should show");
        }
        for text in missing {
            assert!(ui.find(*text).is_err(), "{text} should not show");
        }
    }

    #[test]
    fn the_column_shows_the_facts_the_top_section_and_the_sources() {
        let top = TopViewModel {
            tab: crate::top_tab::TopTab::ByProject,
            rows: crate::model::TopRowsViewModel::Projects(vec![
                crate::model::TopProjectRowViewModel {
                    name: String::from("otty"),
                    sessions: String::from("9"),
                    tokens: String::from("18.4M"),
                },
            ]),
        };

        shown(
            Some(top),
            &[
                i18n::t(Key::AgentsProfileTitle),
                i18n::t(Key::AgentsFieldAccount),
                "user@example.com",
                "~/.codex",
                i18n::t(Key::AgentsTopTitle),
                "otty",
                "18.4M",
                i18n::t(Key::AgentsSourcesTitle),
                "ChatGPT usage API",
                i18n::t(Key::AgentsSourceLoginExpired),
                "Codex app-server",
                i18n::t(Key::AgentsSourceNotAvailable),
            ],
            &["no program", "Open the agent to log in again"],
        );
    }

    #[test]
    fn with_no_top_section_it_is_left_out() {
        shown(
            None,
            &[i18n::t(Key::AgentsProfileTitle)],
            &[i18n::t(Key::AgentsTopTitle)],
        );
    }
}
