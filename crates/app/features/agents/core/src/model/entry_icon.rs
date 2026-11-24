//! The icon of an agent or a provider as the registry gives it (spec
//! 2026-11-19-agent-plugins, "Links in the interface"): the SVG of its
//! entry, else the first letter of its name; the broken plugin icon when
//! the plugin that serves it is broken (issue 17 of spec
//! 2026-11-23-accounts-apart-from-profiles).

use std::borrow::Cow;
use std::fmt;

use iced::widget::svg;
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons;

/// How the icon of an agent or a provider is drawn.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum EntryIcon {
    /// The SVG of its registry entry, tinted by the app.
    Svg(Cow<'static, [u8]>),
    /// The first letter of this name on a rounded square: there is no
    /// entry, or the entry has no icon.
    Letter(String),
    /// The broken plugin icon in the warn colour: the plugin with this
    /// name, which would serve it, is broken, so no part serves it.
    Broken(String),
}

impl EntryIcon {
    /// The icon of an entry with the SVG `icon`, or of the name `name`
    /// when it has none.
    pub(crate) fn of(icon: Option<&Cow<'static, [u8]>>, name: &str) -> Self {
        match icon {
            Some(icon) => Self::Svg(icon.clone()),
            None => Self::Letter(name.to_string()),
        }
    }

    /// The SVG of the icon, or `fallback` for a letter icon, where only
    /// an SVG can be drawn, such as a cell of the sessions table.
    pub(crate) fn svg_or(&self, fallback: &'static [u8]) -> svg::Handle {
        match self {
            Self::Svg(icon) => svg::Handle::from_memory(icon.clone()),
            Self::Broken(_) => svg::Handle::from_memory(icons::PLUGIN_BROKEN),
            Self::Letter(_) => svg::Handle::from_memory(fallback),
        }
    }

    /// The tooltip of the broken plugin icon, "Plugin OpenAI failed, see
    /// Plugins"; `None` for another icon.
    pub(crate) fn note(&self) -> Option<String> {
        match self {
            Self::Broken(plugin) => Some(i18n::filled(
                Key::TplAgentsPluginBroken,
                &[("name", plugin)],
            )),
            _ => None,
        }
    }
}

/// The size of an SVG, not its bytes, so a failed test stays readable.
impl fmt::Debug for EntryIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Svg(icon) => write!(f, "Svg({} bytes)", icon.len()),
            Self::Letter(name) => write!(f, "Letter({name:?})"),
            Self::Broken(plugin) => write!(f, "Broken({plugin:?})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::AgentId;
    use otty_app_domain::plugins::{BrokenPart, PartSide, PluginNotes};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{found_key, with_registry_and_profiles};

    #[test]
    fn an_entry_with_no_icon_is_the_letter_of_its_name() {
        let logo: Cow<'static, [u8]> = Cow::Borrowed(b"<svg/>");

        assert_eq!(EntryIcon::of(Some(&logo), "Codex"), EntryIcon::Svg(logo));
        assert_eq!(
            EntryIcon::of(None, "api.deepseek.com"),
            EntryIcon::Letter(String::from("api.deepseek.com"))
        );
    }

    /// The broken parts of the plugin `plugin`, named `name`: `parts`,
    /// each its side and id.
    fn broken(
        plugin: &str,
        name: &str,
        parts: &[(PartSide, &str)],
    ) -> PluginNotes {
        PluginNotes {
            broken_parts: parts
                .iter()
                .map(|(side, part)| BrokenPart {
                    plugin: plugin.to_string(),
                    plugin_name: name.to_string(),
                    side: *side,
                    part: part.to_string(),
                })
                .collect(),
            ..PluginNotes::default()
        }
    }

    /// Issue 17 of spec 2026-11-23-accounts-apart-from-profiles: a profile
    /// whose agent's plugin is broken, so no part serves it, has the
    /// broken plugin icon with a tooltip that says so; an agent a healthy
    /// part serves keeps its own icon, and one no plugin knows keeps the
    /// letter icon.
    #[test]
    fn the_agent_of_a_broken_plugin_has_the_broken_icon() {
        let mut state = with_registry_and_profiles(
            crate::testing::registry_without_codex(),
            &[],
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::PluginNotesChanged(broken(
                "openai",
                "OpenAI",
                &[(PartSide::Agent, "codex"), (PartSide::Agent, "claude-code")],
            )),
        );

        let codex = state.agent_icon(&AgentId::new("codex"));
        let claude = state.agent_icon(&AgentId::new("claude-code"));
        let unknown = state.agent_icon(&AgentId::new("coder"));

        assert_eq!(codex, EntryIcon::Broken(String::from("OpenAI")));
        assert_eq!(
            codex.note(),
            Some(i18n::filled(
                Key::TplAgentsPluginBroken,
                &[("name", "OpenAI")]
            ))
        );
        assert_eq!(claude, crate::testing::agent_icon("claude-code"));
        assert_eq!(claude.note(), None);
        assert_eq!(unknown, EntryIcon::Letter(String::from("coder")));
    }

    /// Issue 17: an account whose provider part's plugin is broken has the
    /// broken plugin icon.
    #[test]
    fn the_account_of_a_broken_plugin_has_the_broken_icon() {
        let mut state = with_registry_and_profiles(
            crate::testing::registry_without_deepseek(),
            &[],
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::PluginNotesChanged(broken(
                "deepseek",
                "DeepSeek",
                &[(PartSide::Provider, "deepseek")],
            )),
        );

        let icon = state.account_icon(&found_key("deepseek"));

        assert_eq!(icon, EntryIcon::Broken(String::from("DeepSeek")));
        assert_eq!(
            icon.svg_or(icons::MODE_AGENTS),
            svg::Handle::from_memory(icons::PLUGIN_BROKEN)
        );
    }
}
