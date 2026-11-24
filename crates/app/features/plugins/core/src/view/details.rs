//! The main area of Plugins mode: the page of the selected plugin, from the
//! top down, one column at most as wide as a form and centred left and
//! right, as the pages of Agents mode, with the tag of who reviewed it after its version and each of its
//! parts a card with its description and Can access (issues 35, 36); or a
//! card with a short line that says what to do, and the two ways to add a
//! plugin.

use iced::widget::{Column, Row, Space, column, container, row, svg, text};
use iced::{Alignment, Border, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_ui_kit::components::button::{Button, ButtonKind, ButtonSize};
use otty_ui_kit::components::ellipsis_text::EllipsisText;
use otty_ui_kit::components::empty_card::EmptyCard;
use otty_ui_kit::components::key_value_list::{KeyValueList, ValuePart};
use otty_ui_kit::components::letter_icon::letter_icon;
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::status_badge::{StatusBadge, StatusTone};
use otty_ui_kit::components::switch::Switch;
use otty_ui_kit::components::tag::{tag, warn_tag};
use otty_ui_kit::components::tooltip::tooltip;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, radius, size, spacing, typography};

use super::can_access::kind_tag;
use crate::event::PluginsIntent;
use crate::model::{
    MainViewModel, PartRowViewModel, PluginPageViewModel, SourceViewModel,
};

/// Edge of the icon before the plugin name.
const ICON_EDGE: f32 = 26.0;
/// Side padding of the page (`padding: 16px 20px`), as in Agents mode.
const PAGE_PADDING_X: f32 = 20.0;
/// Gap between the blocks of the page.
const BLOCK_GAP: f32 = 24.0;
/// Size of the title of a block.
const BLOCK_TITLE_SIZE: f32 = 14.0;
/// Height of the line the trust tag is centred in: the tag is 16 px high.
const TAG_LINE: f32 = 20.0;

/// Render the main area.
pub(crate) fn view<'a>(
    vm: MainViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    match vm {
        MainViewModel::Hint { title, line } => hint(title, line, palette),
        MainViewModel::Plugin(page) => plugin_page(*page, palette),
    }
}

/// The card with the mode icon, what to do, Add plugin… and Add plugin
/// from folder….
fn hint<'a>(
    title: &'static str,
    line: &'static str,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let icon_color = palette.text_muted();
    let icon = svg(svg::Handle::from_memory(icons::MODE_PLUGINS))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_, _| svg::Style {
            color: Some(icon_color),
        });
    let add = Button::new(i18n::t(Key::PluginsAddPlugin), palette)
        .kind(ButtonKind::Primary)
        .on_press(PluginsIntent::AddPlugin);
    let add_folder = Button::new(i18n::t(Key::PluginsAddFromFolder), palette)
        .kind(ButtonKind::Secondary)
        .on_press(PluginsIntent::AddFromFolder);

    EmptyCard::new(title, line, icon, palette)
        .action(row![add, add_folder].spacing(spacing::SPACE_2))
        .into()
}

/// The page of a plugin: the header, what it is, its news and actions,
/// then its parts, from the top of the main area down.
fn plugin_page<'a>(
    page: PluginPageViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let header = header(&page, palette);

    let mut about = column![].spacing(spacing::SPACE_2);
    if let Some(description) = page.description.clone() {
        about = about.push(small_text(description, palette.text_body()));
    }
    about = about.push(facts(&page, palette));

    let mut blocks = Column::new().spacing(BLOCK_GAP);
    blocks = blocks.push(about);
    if let Some(error) = page.error.clone() {
        blocks = blocks.push(small_text(error, palette.status_error()));
    }
    if let Some(news) = news(&page, palette) {
        blocks = blocks.push(news);
    }
    blocks = blocks.push(actions(&page, palette));
    blocks = blocks.push(titled(
        i18n::t(Key::PluginsSectionParts),
        parts(&page, palette),
        palette,
    ));

    let content = column![header, blocks]
        .spacing(BLOCK_GAP)
        .width(Length::Fill)
        .max_width(size::FORM);

    // Filling the height keeps the page at the top: the main area centres
    // only a screen lower than the room, such as a card (issue 39).
    container(content)
        .padding([spacing::SPACE_5, PAGE_PADDING_X])
        .center_x(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The icon, the name and the version of the plugin, and the tag of who
/// reviewed it with its note in the tooltip, in the warn tone for a
/// community plugin.
fn header<'a>(
    page: &PluginPageViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let strong = palette.text_strong();
    let icon: Element<'a, PluginsIntent> = match page.icon.clone() {
        Some(bytes) => svg(svg::Handle::from_memory(bytes))
            .width(ICON_EDGE)
            .height(ICON_EDGE)
            .style(move |_, _| svg::Style {
                color: Some(strong),
            })
            .into(),
        None => letter_icon(&page.name, ICON_EDGE, strong, palette),
    };
    let name = EllipsisText::new(
        page.name.clone(),
        typography::TITLE.size(),
        fonts::UI_SEMIBOLD,
        strong,
    )
    .shrink(0.0);
    let version = text(page.version.clone()).color(palette.text_muted());
    let trust = if page.trust.is_warn {
        warn_tag(page.trust.label, TAG_LINE, palette)
    } else {
        tag(page.trust.label, TAG_LINE, palette)
    };
    let trust = tooltip(trust, page.trust.note, palette);

    row![icon, name, version, trust]
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .into()
}

/// Version, authors and source. A repository opens on a press.
fn facts<'a>(
    page: &PluginPageViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let open = || PluginsIntent::OpenRepository(page.id.clone());

    let mut facts = KeyValueList::new(palette)
        .entry(i18n::t(Key::PluginsFieldVersion), page.version.clone());
    if let Some(authors) = &page.authors {
        facts = facts.entry(i18n::t(Key::PluginsFieldAuthors), authors.clone());
    }
    let source = match &page.source {
        SourceViewModel::Folder(folder) => ValuePart::text(folder.clone()),
        SourceViewModel::Repository(url) => {
            ValuePart::link(url.clone(), open())
        },
    };
    facts = facts.entry_parts(i18n::t(Key::PluginsFieldSource), [source]);

    facts.into()
}

/// The update that waits with Review, and a newer version that needs a
/// newer OTTY.
fn news<'a>(
    page: &PluginPageViewModel,
    palette: &UiPalette,
) -> Option<Element<'a, PluginsIntent>> {
    let mut lines = column![].spacing(spacing::SPACE_2);
    let mut has_lines = false;

    if let Some(waiting) = page.waiting.clone() {
        lines = lines.push(
            row![
                small_text(waiting, palette.status_warn()),
                Button::new(i18n::t(Key::PluginsReview), palette)
                    .kind(ButtonKind::Secondary)
                    .size(ButtonSize::Small)
                    .on_press(PluginsIntent::Review(page.id.clone())),
            ]
            .spacing(spacing::SPACE_3)
            .align_y(Alignment::Center),
        );
        has_lines = true;
    }
    if let Some(too_new) = page.too_new.clone() {
        lines = lines.push(small_text(too_new, palette.text_muted()));
        has_lines = true;
    }

    has_lines.then(|| lines.into())
}

/// Check for updates, or Reload for a local plugin; Show log; Remove.
fn actions<'a>(
    page: &PluginPageViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let id = &page.id;
    let small = |label: &str, kind: ButtonKind, intent: PluginsIntent| {
        Button::new(label, palette)
            .kind(kind)
            .size(ButtonSize::Small)
            .on_press(intent)
    };

    let first = if page.is_local {
        small(
            i18n::t(Key::PluginsReload),
            ButtonKind::Secondary,
            PluginsIntent::Reload(id.clone()),
        )
    } else {
        small(
            i18n::t(Key::PluginsCheckForUpdates),
            ButtonKind::Secondary,
            PluginsIntent::CheckForUpdates(id.clone()),
        )
    };
    let log = small(
        i18n::t(Key::PluginsShowLog),
        ButtonKind::Ghost,
        PluginsIntent::ShowLog(id.clone()),
    );
    let remove = small(
        i18n::t(Key::PluginsRemove),
        ButtonKind::Danger,
        PluginsIntent::AskRemove(id.clone()),
    );

    Row::with_children([first.into(), log.into(), remove.into()])
        .spacing(spacing::SPACE_2)
        .wrap()
        .vertical_spacing(spacing::SPACE_2)
        .into()
}

/// A title over its block.
fn titled<'a>(
    title: &str,
    body: Element<'a, PluginsIntent>,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let title = text(title.to_string())
        .size(BLOCK_TITLE_SIZE)
        .font(fonts::UI_SEMIBOLD)
        .color(palette.text_strong());

    column![title, body].spacing(spacing::SPACE_3).into()
}

/// The parts of the plugin, one after the other.
fn parts<'a>(
    page: &PluginPageViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let parts: Vec<Element<'a, PluginsIntent>> = page
        .parts
        .iter()
        .cloned()
        .map(|part| part_card(&page.id, part, page.is_busy, palette))
        .collect();

    Column::with_children(parts)
        .spacing(spacing::SPACE_4)
        .into()
}

/// One part as a card: a header with its name, the tag of its kind, its
/// health badge, Try again while it is paused and its switch at the right
/// end; its description with Can access (5) at the right end, or "Asks for
/// no permissions"; its last failed call with Copy report.
fn part_card<'a>(
    plugin: &str,
    part: PartRowViewModel,
    is_busy: bool,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let mut switch = Switch::new(part.is_on, palette);
    if !is_busy {
        let plugin = plugin.to_string();
        let reference = part.part.clone();
        switch = switch.on_toggle(move |on| PluginsIntent::SetPartOn {
            plugin: plugin.clone(),
            part: reference.clone(),
            on,
        });
    }
    let tone = if !part.is_on {
        StatusTone::Neutral
    } else if part.is_paused {
        StatusTone::Warn
    } else {
        StatusTone::Ok
    };
    let mut badge = StatusBadge::new(part.health, tone, palette);
    if part.is_paused {
        badge = badge.tooltip(i18n::t(Key::PluginsPaused));
    }

    let foot = foot(plugin, &part, palette);

    let mut header = Row::new()
        .spacing(spacing::SPACE_3)
        .align_y(Alignment::Center)
        .height(size::CONTROL_MD)
        .push(
            text(part.name)
                .font(fonts::UI_SEMIBOLD)
                .color(palette.text_strong()),
        )
        .push(kind_tag(part.kind, palette))
        .push(badge);
    if part.is_paused {
        header = header.push(
            Button::new(i18n::t(Key::PluginsTryAgain), palette)
                .kind(ButtonKind::Secondary)
                .size(ButtonSize::Small)
                .on_press(PluginsIntent::TryAgain {
                    plugin: plugin.to_string(),
                    part: part.part.clone(),
                }),
        );
    }
    header = header.push(Space::new().width(Length::Fill)).push(switch);

    let mut card = column![header, foot].spacing(spacing::SPACE_2);
    if let Some(last) = part.last_failure {
        card = card.push(
            row![
                small_text(last.text, palette.text_muted()),
                Button::new(i18n::t(Key::AgentsCopyReport), palette)
                    .kind(ButtonKind::Ghost)
                    .size(ButtonSize::Small)
                    .on_press(PluginsIntent::CopyReport(last.report)),
            ]
            .spacing(spacing::SPACE_2)
            .align_y(Alignment::Center),
        );
    }

    let background = palette.surface_panel();
    let line = palette.line();
    container(card)
        .width(Length::Fill)
        .padding([spacing::SPACE_4, spacing::SPACE_5])
        .style(move |_| container::Style {
            background: Some(background.into()),
            border: Border {
                radius: radius::RADIUS_MD.into(),
                width: size::LINE,
                color: line,
            },
            ..container::Style::default()
        })
        .into()
}

/// The line under the header of a part card: its description, muted,
/// and Can access with the number of places at the right end, or "Asks
/// for no permissions" when there are none.
fn foot<'a>(
    plugin: &str,
    part: &PartRowViewModel,
    palette: &UiPalette,
) -> Element<'a, PluginsIntent> {
    let description: Element<'a, PluginsIntent> = match &part.description {
        Some(description) => {
            container(small_text(description.clone(), palette.text_muted()))
                .width(Length::Fill)
                .into()
        },
        None => Space::new().width(Length::Fill).into(),
    };
    let access: Element<'a, PluginsIntent> = if part.places == 0 {
        small_text(i18n::t(Key::PluginsNoPermissions), palette.text_muted())
            .into()
    } else {
        let label = i18n::filled(
            Key::TplPluginsCanAccessButton,
            &[("count", &part.places.to_string())],
        );
        Button::new(label, palette)
            .kind(ButtonKind::Secondary)
            .size(ButtonSize::Small)
            .on_press(PluginsIntent::ShowCanAccess {
                plugin: plugin.to_string(),
                part: part.part.clone(),
            })
            .into()
    };

    row![description, access]
        .spacing(spacing::SPACE_4)
        .align_y(Alignment::Center)
        .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_plugin_manifest::PartRef;

    use super::*;
    use crate::model::{LastFailureViewModel, TrustTagViewModel};

    fn example() -> PluginPageViewModel {
        PluginPageViewModel {
            id: String::from("example"),
            name: String::from("Example"),
            version: String::from("0.1.0"),
            description: None,
            trust: TrustTagViewModel {
                label: "Local",
                note: "Local plugin: installed from a folder",
                is_warn: false,
            },
            icon: None,
            authors: Some(String::from("OTTY team")),
            source: SourceViewModel::Folder(String::from("/work/example")),
            parts: vec![PartRowViewModel {
                part: PartRef::agent("example-agent"),
                name: String::from("Example Agent"),
                kind: "Agent",
                description: Some(String::from(
                    "Finds Example Agent sessions, tokens and their money.",
                )),
                is_on: true,
                health: "Paused",
                is_paused: true,
                places: 5,
                last_failure: None,
            }],
            waiting: None,
            too_new: None,
            is_local: true,
            error: None,
            is_busy: false,
        }
    }

    fn page_view<'a>(
        page: PluginPageViewModel,
        palette: &UiPalette,
    ) -> Element<'a, PluginsIntent> {
        view(MainViewModel::Plugin(Box::new(page)), palette)
    }

    /// Issue 30: with nothing to show, the card says what to do and offers
    /// both ways to add a plugin.
    #[test]
    fn the_hint_says_what_to_do_and_adds_a_plugin() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(view(
            MainViewModel::Hint {
                title: "No plugin selected",
                line: "Pick a plugin on the left.",
            },
            &palette,
        ));

        assert!(ui.find("Pick a plugin on the left.").is_ok());
        let _ = ui.click(i18n::t(Key::PluginsAddPlugin));
        let _ = ui.click(i18n::t(Key::PluginsAddFromFolder));

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(
            sent.as_slice(),
            [PluginsIntent::AddPlugin, PluginsIntent::AddFromFolder]
        ));
    }

    /// The page shows what the plugin is and the permissions of its
    /// parts; a local plugin reloads and a paused part tries again.
    #[test]
    fn a_local_plugin_reloads_and_a_paused_part_tries_again() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(page_view(example(), &palette));

        assert!(ui.find("OTTY team").is_ok());
        assert!(ui.find("/work/example").is_ok());
        assert!(ui.find(i18n::t(Key::PluginsCheckForUpdates)).is_err());
        let _ = ui.click(i18n::t(Key::PluginsReload));
        let _ = ui.click(i18n::t(Key::PluginsTryAgain));
        let _ = ui.click(i18n::t(Key::PluginsRemove));

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(
            sent.as_slice(),
            [
                PluginsIntent::Reload(reload),
                PluginsIntent::TryAgain { .. },
                PluginsIntent::AskRemove(remove),
            ] if reload == "example" && remove == "example"
        ));
    }

    /// Issue 19: the card of a part shows its last failed call, and Copy
    /// report copies its report.
    #[test]
    fn a_part_shows_its_last_failure_and_copies_its_report() {
        let palette = UiPalette::otty_dark();
        let mut page = example();
        page.parts[0].last_failure = Some(LastFailureViewModel {
            text: String::from("Last failure 5 min ago: numbers, failed"),
            report: String::from("Call: numbers"),
        });
        let mut ui = simulator(page_view(page, &palette));

        assert!(ui.find("Last failure 5 min ago: numbers, failed").is_ok());
        let _ = ui.click(i18n::t(Key::AgentsCopyReport));

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(
            sent.as_slice(),
            [PluginsIntent::CopyReport(report)] if report == "Call: numbers"
        ));
    }

    /// Issue 36: the tag of who reviewed the plugin follows its version;
    /// its note is in the tooltip, not a line of the page. The facts are
    /// Version, Authors and Source, no Repository.
    #[test]
    fn the_trust_tag_follows_the_version() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(page_view(example(), &palette));

        let version = ui.find("0.1.0").expect("the version").bounds();
        let trust = ui.find("Local").expect("the trust tag").bounds();

        assert!(version.x < trust.x);
        assert!((version.center_y() - trust.center_y()).abs() < 2.0);
        assert!(ui.find("Local plugin: installed from a folder").is_err());
        assert!(ui.find("Repository").is_err());
        assert!(
            ui.find("https://github.com/otty-shell/otty-plugin-example")
                .is_err()
        );
    }

    /// Issue 35, 36: the card of a part shows its name, the tag of its
    /// kind and its health badge, under them its description and Can
    /// access (5), which opens what it can access.
    #[test]
    fn a_part_card_shows_its_description_and_can_access() {
        let palette = UiPalette::otty_dark();
        let mut ui = simulator(page_view(example(), &palette));
        let button =
            i18n::filled(Key::TplPluginsCanAccessButton, &[("count", "5")]);

        let name = ui.find("Example Agent").expect("the name").bounds();
        let kind = ui.find("Agent").expect("the kind").bounds();
        let health = ui.find("Paused").expect("the health").bounds();
        let description = ui
            .find("Finds Example Agent sessions, tokens and their money.")
            .expect("the description")
            .bounds();
        let access = ui.find(button.as_str()).expect("Can access").bounds();

        assert!(name.x < kind.x && kind.x < health.x);
        assert!((name.center_y() - kind.center_y()).abs() < 2.0);
        assert!(description.y > name.y);
        assert!(description.x < access.x);
        assert!(ui.find("~/.x/**").is_err());
        let _ = ui.click(button.as_str());
        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(
            sent.as_slice(),
            [PluginsIntent::ShowCanAccess { plugin, part }]
                if plugin == "example" && *part == PartRef::agent("example-agent")
        ));
    }

    /// Issue 36: a part with no permission and no description shows only
    /// "Asks for no permissions", and no Can access.
    #[test]
    fn a_part_with_no_permission_has_no_can_access() {
        let palette = UiPalette::otty_dark();
        let mut page = example();
        page.parts[0].places = 0;
        page.parts[0].description = None;
        let mut ui = simulator(page_view(page, &palette));

        assert!(ui.find(i18n::t(Key::PluginsNoPermissions)).is_ok());
        assert!(
            ui.find(
                i18n::filled(Key::TplPluginsCanAccessButton, &[("count", "0")])
                    .as_str()
            )
            .is_err()
        );
    }

    /// Issue 35: a source from a repository is its link, and the page
    /// shows no line of what Reload or Check for updates did.
    #[test]
    fn a_repository_source_opens_it() {
        let palette = UiPalette::otty_dark();
        let mut page = example();
        page.is_local = false;
        page.source = SourceViewModel::Repository(String::from(
            "https://github.com/someone/example",
        ));
        let mut ui = simulator(page_view(page, &palette));

        let _ = ui.click("https://github.com/someone/example");

        let sent: Vec<PluginsIntent> = ui.into_messages().collect();
        assert!(matches!(
            sent.as_slice(),
            [PluginsIntent::OpenRepository(id)] if id == "example"
        ));
    }
}
