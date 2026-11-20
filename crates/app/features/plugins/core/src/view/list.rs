//! The list of the installed plugins: Add plugin… and Add plugin from
//! folder… over it, each plugin with its icon, name, version, how far it
//! is trusted, its parts with a switch each, what the latest check said
//! and "⋯" with its actions; an empty state with Add plugin….

use iced::widget::{Column, Row, Space, column, container, row, svg, text};
use iced::{Alignment, Border, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::icon_button::IconButton;
use otty_ui_kit::components::letter_icon::letter_icon;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::components::{divider, glyph};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, radius, size, spacing};

use crate::event::PluginsIntent;
use crate::model::{PartRowViewModel, PluginRowViewModel};

/// Edge of the icon of a plugin.
const ICON_EDGE: f32 = 20.0;

/// Render the list in the page's dialog.
pub(super) fn view<'a>(
    rows: Vec<PluginRowViewModel>,
    error: Option<&'a str>,
    is_busy: bool,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let add = Button::new(i18n::t(Key::PluginsAddPlugin), palette)
        .kind(ButtonKind::Secondary)
        .size(ButtonSize::Small)
        .on_press(PluginsIntent::AddPlugin);
    let add_folder = Button::new(i18n::t(Key::PluginsAddFromFolder), palette)
        .kind(ButtonKind::Ghost)
        .size(ButtonSize::Small)
        .on_press(PluginsIntent::AddFromFolder);

    let mut body = Column::new().spacing(spacing::SPACE_4);
    body = body.push(row![add, add_folder].spacing(spacing::SPACE_2));
    if let Some(error) = error {
        body = body.push(small_text(error, palette.status_error()));
    }
    if rows.is_empty() {
        body = body.push(empty(palette));
    }
    for (index, plugin) in rows.into_iter().enumerate() {
        if index > 0 {
            body = body.push(divider::horizontal(palette));
        }
        body = body.push(plugin_row(plugin, is_busy, palette));
    }

    Dialog::new(i18n::t(Key::PluginsTitle), palette)
        .width(size::DIALOG_WIDE)
        .on_close(PluginsIntent::Close, i18n::t(Key::ButtonClose))
        .body(body)
        .into()
}

/// The empty state: what a plugin is, and Add plugin….
fn empty<'a>(palette: &UiPalette) -> Element<'a, PluginsIntent> {
    column![
        text(i18n::t(Key::PluginsEmpty))
            .font(fonts::UI_SEMIBOLD)
            .color(palette.text_strong()),
        small_text(i18n::t(Key::PluginsEmptyHint), palette.text_muted()),
        Button::new(i18n::t(Key::PluginsAddPlugin), palette)
            .kind(ButtonKind::Primary)
            .on_press(PluginsIntent::AddPlugin),
    ]
    .spacing(spacing::SPACE_2)
    .into()
}

/// One plugin: its title line, its parts, its news and its actions.
fn plugin_row<'a>(
    plugin: PluginRowViewModel,
    is_busy: bool,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let id = plugin.id.clone();

    let more =
        IconButton::new(glyph::dots(), i18n::t(Key::MoreActions), palette)
            .on_press(PluginsIntent::ToggleActions(id.clone()));
    let title = row![
        icon(plugin.icon.clone(), &plugin.name, palette),
        column![
            row![
                text(plugin.name.clone())
                    .font(fonts::UI_SEMIBOLD)
                    .color(palette.text_strong()),
                text(plugin.version.clone()).color(palette.text_muted()),
            ]
            .spacing(spacing::SPACE_2),
            small_text(plugin.trust, palette.text_muted()),
        ]
        .width(Length::Fill),
        more,
    ]
    .spacing(spacing::SPACE_3)
    .align_y(Alignment::Center);

    let mut lines = column![title].spacing(spacing::SPACE_2);
    let parts: Vec<Element<'a, PluginsIntent>> = plugin
        .parts
        .iter()
        .cloned()
        .map(|part| part_switch(&id, part, is_busy, palette))
        .collect();
    lines = lines.push(
        Row::with_children(parts)
            .spacing(spacing::SPACE_5)
            .wrap()
            .vertical_spacing(spacing::SPACE_2),
    );
    if let Some(news) = plugin.news.clone() {
        let color = if plugin.is_news_failure {
            palette.status_error()
        } else {
            palette.text_muted()
        };
        lines = lines.push(small_text(news, color));
    }
    if let Some(waiting) = plugin.waiting.clone() {
        lines = lines.push(
            row![
                small_text(waiting, palette.status_warn()),
                Button::new(i18n::t(Key::PluginsReview), palette)
                    .kind(ButtonKind::Secondary)
                    .size(ButtonSize::Small)
                    .on_press(PluginsIntent::Review(id.clone())),
            ]
            .spacing(spacing::SPACE_3)
            .align_y(Alignment::Center),
        );
    }
    if plugin.are_actions_open {
        lines = lines.push(actions(&plugin, palette));
    }

    let is_highlighted = plugin.is_highlighted;
    let background = palette.surface_selected();
    container(lines)
        .padding(spacing::SPACE_2)
        .width(Length::Fill)
        .style(move |_| {
            let mut style = container::Style::default();
            if is_highlighted {
                style.background = Some(background.into());
                style.border = Border::default().rounded(radius::RADIUS_SM);
            }
            style
        })
        .into()
}

/// One part with its switch, and Try again while it is paused.
fn part_switch<'a>(
    plugin: &str,
    part: PartRowViewModel,
    is_busy: bool,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut switch = Switch::new(part.is_on, palette).label(part.title);
    if !is_busy {
        let plugin = plugin.to_string();
        let reference = part.part.clone();
        switch = switch.on_toggle(move |on| PluginsIntent::SetPartOn {
            plugin: plugin.clone(),
            part: reference.clone(),
            on,
        });
    }
    if !part.is_paused {
        return switch.into();
    }

    row![
        switch,
        small_text(i18n::t(Key::PluginsPaused), palette.status_warn()),
        Button::new(i18n::t(Key::PluginsTryAgain), palette)
            .kind(ButtonKind::Secondary)
            .size(ButtonSize::Small)
            .on_press(PluginsIntent::TryAgain {
                plugin: plugin.to_string(),
                part: part.part,
            }),
    ]
    .spacing(spacing::SPACE_2)
    .align_y(Alignment::Center)
    .into()
}

/// The actions of "⋯": Check for updates, or Reload for a local plugin;
/// Open repository; Show permissions; Show log; Remove.
fn actions<'a>(
    plugin: &PluginRowViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let id = &plugin.id;
    let small = |label: &str, kind: ButtonKind, intent: PluginsIntent| {
        Button::new(label, palette)
            .kind(kind)
            .size(ButtonSize::Small)
            .on_press(intent)
    };

    let first = if plugin.is_local {
        small(
            i18n::t(Key::PluginsReload),
            ButtonKind::Ghost,
            PluginsIntent::Reload(id.clone()),
        )
    } else {
        small(
            i18n::t(Key::PluginsCheckForUpdates),
            ButtonKind::Ghost,
            PluginsIntent::CheckForUpdates(id.clone()),
        )
    };
    let mut list: Vec<Element<'a, PluginsIntent>> = vec![first.into()];
    if plugin.repository.is_some() {
        list.push(
            small(
                i18n::t(Key::PluginsOpenRepository),
                ButtonKind::Ghost,
                PluginsIntent::OpenRepository(id.clone()),
            )
            .into(),
        );
    }
    list.push(
        small(
            i18n::t(Key::PluginsShowPermissions),
            ButtonKind::Ghost,
            PluginsIntent::ShowPermissions(id.clone()),
        )
        .into(),
    );
    list.push(
        small(
            i18n::t(Key::PluginsShowLog),
            ButtonKind::Ghost,
            PluginsIntent::ShowLog(id.clone()),
        )
        .into(),
    );
    list.push(Space::new().width(spacing::SPACE_2).into());
    list.push(
        small(
            i18n::t(Key::PluginsRemove),
            ButtonKind::Danger,
            PluginsIntent::AskRemove(id.clone()),
        )
        .into(),
    );

    Row::with_children(list)
        .spacing(spacing::SPACE_1)
        .wrap()
        .vertical_spacing(spacing::SPACE_1)
        .into()
}

/// The icon of a plugin: the SVG of its first part, or the first letter
/// of its name.
fn icon<'a>(
    icon: Option<Vec<u8>>,
    name: &str,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let color = palette.text_strong();

    match icon {
        Some(bytes) => svg(svg::Handle::from_memory(bytes))
            .width(ICON_EDGE)
            .height(ICON_EDGE)
            .style(move |_, _| svg::Style { color: Some(color) })
            .into(),
        None => letter_icon(name, ICON_EDGE, color, palette),
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_plugin_manifest::PartRef;

    use super::*;

    fn example() -> PluginRowViewModel {
        PluginRowViewModel {
            id: String::from("example"),
            name: String::from("Example"),
            version: String::from("0.1.0"),
            trust: "Local",
            icon: None,
            parts: vec![PartRowViewModel {
                part: PartRef::agent("example-agent"),
                title: String::from("Example Agent (agent)"),
                is_on: true,
                is_paused: true,
            }],
            news: None,
            is_news_failure: false,
            waiting: None,
            is_local: true,
            repository: None,
            are_actions_open: true,
            is_highlighted: false,
        }
    }

    #[test]
    fn the_empty_list_offers_add_plugin() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(view(Vec::new(), None, false, &palette));

        assert!(ui.find(i18n::t(Key::PluginsEmpty)).is_ok());
    }

    /// The actions of a local plugin have Reload in place of Check for
    /// updates, and a paused part has Try again.
    #[test]
    fn a_local_plugin_reloads_and_a_paused_part_tries_again() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(view(vec![example()], None, false, &palette));

        assert!(ui.find(i18n::t(Key::PluginsCheckForUpdates)).is_err());
        let _ = ui.click(i18n::t(Key::PluginsReload));
        let _ = ui.click(i18n::t(Key::PluginsTryAgain));

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(
            sent.as_slice(),
            [PluginsIntent::Reload(reload), PluginsIntent::TryAgain { .. }]
                if reload == "example"
        ));
    }
}
