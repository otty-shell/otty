//! The Add account dialog over the window (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 6, with the owner's
//! prototype): the Provider select, each provider with its logo and its
//! plugin, then under How the way OTTY adds it, "Log in with <agent>" and
//! "Paste a key", which is off and says why on hover; Cancel and Continue.
//! With no provider part on, the dialog says so and offers Plugins mode.

use iced::widget::{Space, column, container, row};
use iced::{Alignment, Border, Element, Length};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::button::{Button, ButtonKind};
use otty_ui_kit::components::dialog::Dialog;
use otty_ui_kit::components::note_select::{NoteSelect, NoteSelectOption};
use otty_ui_kit::components::small_text::small_text;
use otty_ui_kit::components::tooltip::tooltip;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{radius, spacing};

use super::super::add_account_dialog::AddAccountIntent;
use super::super::event::AgentsIntent;
use super::super::model::AddAccountDialogViewModel;

/// Edge of the round mark of a way to add an account.
const MARK_SIZE: f32 = 12.0;

/// Props for the Add account dialog.
pub(crate) struct AddAccountDialogProps<'a> {
    pub(crate) vm: AddAccountDialogViewModel,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the modal Add account dialog: Esc cancels.
pub(crate) fn view(
    props: AddAccountDialogProps<'_>,
) -> Element<'_, AgentsIntent> {
    let palette = props.theme.theme.ui();
    let vm = props.vm;
    let send = AgentsIntent::AddAccount;

    let mut body = column![small_text(vm.intro, palette.text_muted())]
        .spacing(spacing::SPACE_4);

    if let Some(text) = vm.no_providers {
        body = body.push(small_text(text, palette.text_body())).push(
            Button::new(i18n::t(Key::ButtonManagePlugins), palette)
                .kind(ButtonKind::Secondary)
                .on_press(send(AddAccountIntent::PluginsOpened)),
        );
    } else {
        body = body.push(provider_select(&vm, palette));
        body = body.push(how(&vm, palette));
    }

    if let Some(text) = vm.no_agent_login {
        body = body.push(small_text(text, palette.text_muted()));
    }
    if let Some(error) = vm.error {
        body = body.push(small_text(error, palette.status_error()));
    }

    let cancel = Button::new(i18n::t(Key::ButtonCancel), palette)
        .kind(ButtonKind::Ghost)
        .on_press(send(AddAccountIntent::Cancelled));
    let mut continue_button =
        Button::new(i18n::t(Key::AgentsContinue), palette)
            .kind(ButtonKind::Primary);
    if vm.can_continue {
        continue_button =
            continue_button.on_press(send(AddAccountIntent::Continued));
    }

    Dialog::new(vm.title, palette)
        .on_close(send(AddAccountIntent::Cancelled), i18n::t(Key::ButtonClose))
        .body(body)
        .action(cancel)
        .action(continue_button)
        .into()
}

/// "Provider" over the select of the provider parts that are on, each
/// with its logo and its plugin.
fn provider_select<'a>(
    vm: &AddAccountDialogViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let send = AgentsIntent::AddAccount;
    let options = vm
        .providers
        .iter()
        .map(|option| {
            NoteSelectOption::new(option.name.clone())
                .with_icon(option.icon.svg_or(icons::ALL_ACCOUNTS))
                .with_note(option.plugin.clone())
        })
        .collect();

    let select = NoteSelect::new(options, vm.picked, palette)
        .open(vm.is_list_open)
        .on_toggle(send(AddAccountIntent::ProviderListToggled))
        .on_pick(move |index| send(AddAccountIntent::ProviderPicked(index)));

    column![
        small_text(i18n::t(Key::AgentsColumnProvider), palette.text_muted()),
        select,
    ]
    .spacing(spacing::SPACE_2)
    .into()
}

/// "How": Log in with the part's agent, picked, when the part offers it,
/// and Paste a key, off, whose reason shows on hover.
fn how<'a>(
    vm: &AddAccountDialogViewModel,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let mut ways = column![small_text(
        i18n::t(Key::AgentsFieldHow),
        palette.text_strong()
    )]
    .spacing(spacing::SPACE_2);

    if let Some(log_in) = &vm.log_in {
        ways = ways.push(way(
            log_in.label.clone(),
            log_in.text.clone(),
            Way::Picked,
            palette,
        ));
    }

    let paste_key = way(
        vm.paste_key.label.to_string(),
        Some(vm.paste_key.text.clone()),
        Way::Off,
        palette,
    );
    ways = ways.push(tooltip(paste_key, vm.paste_key.reason, palette));

    ways.into()
}

/// Whether a way to add an account is the picked one or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    Picked,
    Off,
}

/// One way to add an account: a bordered row with a round mark, its
/// label and the line under it. The picked way is lit, a way that is off
/// is faded.
fn way<'a>(
    label: String,
    text: Option<String>,
    kind: Way,
    palette: &UiPalette,
) -> Element<'a, AgentsIntent> {
    let is_picked = kind == Way::Picked;
    let (label_color, text_color) = match kind {
        Way::Picked => (palette.text_strong(), palette.text_muted()),
        Way::Off => (palette.text_faint(), palette.text_faint()),
    };

    let mut words = column![small_text(label, label_color)]
        .spacing(spacing::SPACE_1)
        .width(Length::Fill);
    if let Some(text) = text {
        words = words.push(small_text(text, text_color));
    }

    let accent = palette.accent();
    let line = palette.line();
    let mark = container(Space::new())
        .width(MARK_SIZE)
        .height(MARK_SIZE)
        .style(move |_| container::Style {
            background: is_picked.then(|| accent.into()),
            border: Border {
                color: if is_picked { accent } else { line },
                width: 1.0,
                radius: (MARK_SIZE / 2.0).into(),
            },
            ..container::Style::default()
        });

    let border = if is_picked { accent } else { line };
    container(
        row![mark, words]
            .spacing(spacing::SPACE_3)
            .align_y(Alignment::Start),
    )
    .padding(spacing::SPACE_3)
    .width(Length::Fill)
    .style(move |_| container::Style {
        border: Border {
            color: border,
            width: 1.0,
            radius: radius::RADIUS_SM.into(),
        },
        ..container::Style::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;
    use crate::model::{
        AddAccountDialogViewModel, LogInChoiceViewModel,
        PasteKeyChoiceViewModel, ProviderOptionViewModel,
    };
    use crate::testing::provider_icon;

    fn option(name: &str) -> ProviderOptionViewModel {
        ProviderOptionViewModel {
            name: String::from(name),
            plugin: format!("{name} plugin"),
            icon: provider_icon("openai"),
        }
    }

    fn vm(can_continue: bool) -> AddAccountDialogViewModel {
        AddAccountDialogViewModel {
            title: "Add account",
            intro: "Pick the provider of the account.",
            providers: vec![option("OpenAI"), option("DeepSeek")],
            picked: Some(0),
            is_list_open: false,
            log_in: Some(LogInChoiceViewModel {
                label: String::from("Log in with Codex"),
                text: Some(String::from("Runs `codex login` there.")),
            }),
            paste_key: PasteKeyChoiceViewModel {
                label: "Paste a key",
                text: String::from("An API key of OpenAI"),
                reason: "Coming later",
            },
            no_agent_login: None,
            no_providers: None,
            error: None,
            can_continue,
        }
    }

    fn intents(
        vm: AddAccountDialogViewModel,
        press: &str,
    ) -> Vec<AgentsIntent> {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(AddAccountDialogProps {
            vm,
            theme: ThemeProps::new(themes.current()),
        }));

        ui.click(press).expect("the text should show");
        ui.into_messages().collect()
    }

    /// The dialog shows the picked provider with its plugin, Log in with
    /// its line, and Paste a key; Continue continues.
    #[test]
    fn the_dialog_shows_its_choices_and_continues() {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(AddAccountDialogProps {
            vm: vm(true),
            theme: ThemeProps::new(themes.current()),
        }));

        ui.find("Pick the provider of the account.")
            .expect("the intro should show");
        ui.find("OpenAI plugin").expect("the plugin should show");
        ui.find("Runs `codex login` there.")
            .expect("the line of Log in with should show");
        ui.find("An API key of OpenAI")
            .expect("Paste a key should show");

        assert_eq!(
            intents(vm(true), i18n::t(Key::AgentsContinue)),
            [AgentsIntent::AddAccount(AddAccountIntent::Continued)]
        );
    }

    /// Paste a key is off: a press on it sends nothing.
    #[test]
    fn paste_a_key_sends_nothing() {
        assert!(intents(vm(true), "Paste a key").is_empty());
    }

    #[test]
    fn continue_sends_nothing_while_it_is_off() {
        assert!(intents(vm(false), i18n::t(Key::AgentsContinue)).is_empty());
    }

    #[test]
    fn the_select_opens_and_cancel_cancels() {
        assert_eq!(
            intents(vm(true), "OpenAI"),
            [AgentsIntent::AddAccount(
                AddAccountIntent::ProviderListToggled
            )]
        );
        assert_eq!(
            intents(vm(true), i18n::t(Key::ButtonCancel)),
            [AgentsIntent::AddAccount(AddAccountIntent::Cancelled)]
        );
    }

    /// With no provider part on, the dialog offers Plugins mode.
    #[test]
    fn with_no_provider_part_the_dialog_opens_plugins_mode() {
        let none = AddAccountDialogViewModel {
            providers: Vec::new(),
            picked: None,
            log_in: None,
            no_providers: Some("No plugin with a provider is on."),
            ..vm(false)
        };

        assert_eq!(
            intents(none, i18n::t(Key::ButtonManagePlugins)),
            [AgentsIntent::AddAccount(AddAccountIntent::PluginsOpened)]
        );
    }
}
