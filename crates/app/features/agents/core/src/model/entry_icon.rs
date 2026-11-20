//! The icon of an agent or a provider as the registry gives it (spec
//! 2026-11-19-agent-plugins, "Links in the interface"): the SVG of its
//! entry, else the first letter of its name, or of the registrable domain
//! of its host.

use std::borrow::Cow;
use std::fmt;

use iced::widget::svg;
use otty_agents::ProviderRef;

/// The second-level labels under which a country's domains are
/// registered, such as `co` of `example.co.uk`.
const SHARED_SECOND_LEVELS: [&str; 7] =
    ["ac", "co", "com", "edu", "gov", "net", "org"];

/// How the icon of an agent or a provider is drawn.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum EntryIcon {
    /// The SVG of its registry entry, tinted by the app.
    Svg(Cow<'static, [u8]>),
    /// The first letter of this name or host on a rounded square: there is
    /// no entry, or the entry has no icon.
    Letter(String),
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

    /// The letter icon of a provider no entry serves, known by
    /// `reference`: the name of the registrable domain of its host, such
    /// as `example` of `api.example.com`, else its id.
    pub(crate) fn of_unserved(reference: &ProviderRef) -> Self {
        let name = match reference {
            ProviderRef::Host(host) => registrable_name(host),
            ProviderRef::Id(id) => id.as_str(),
        };

        Self::Letter(name.to_string())
    }

    /// The SVG of the icon, or `fallback` for a letter icon, where only
    /// an SVG can be drawn, such as a cell of the sessions table.
    pub(crate) fn svg_or(&self, fallback: &'static [u8]) -> svg::Handle {
        match self {
            Self::Svg(icon) => svg::Handle::from_memory(icon.clone()),
            Self::Letter(_) => svg::Handle::from_memory(fallback),
        }
    }
}

/// The size of an SVG, not its bytes, so a failed test stays readable.
impl fmt::Debug for EntryIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Svg(icon) => write!(f, "Svg({} bytes)", icon.len()),
            Self::Letter(name) => write!(f, "Letter({name:?})"),
        }
    }
}

/// The name of the domain `host` is registered under: the label before
/// its last one, such as `example` of `api.example.com`, or the label
/// before a shared second level, such as `example` of `api.example.co.uk`.
/// A host of one label is itself.
fn registrable_name(host: &str) -> &str {
    let labels: Vec<&str> =
        host.split('.').filter(|label| !label.is_empty()).collect();

    match labels.as_slice() {
        [.., name, second, top]
            if top.len() == 2 && SHARED_SECOND_LEVELS.contains(second) =>
        {
            name
        },
        [.., name, _] => name,
        _ => host,
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::ProviderRef;

    use super::*;

    /// Issue 17: a host no entry serves is drawn with the first letter of
    /// its registrable domain, not of `api` or `www`.
    #[test]
    fn a_host_no_entry_serves_is_the_letter_of_its_domain() {
        let letter =
            |reference: ProviderRef| match EntryIcon::of_unserved(&reference) {
                EntryIcon::Letter(name) => name,
                other => panic!("a letter icon: {other:?}"),
            };
        let host = |host: &str| ProviderRef::Host(host.to_string());

        assert_eq!(letter(host("api.deepseek.com")), "deepseek");
        assert_eq!(letter(host("www.example.com")), "example");
        assert_eq!(letter(host("gateway.ai.example.org")), "example");
        assert_eq!(letter(host("api.example.co.uk")), "example");
        assert_eq!(letter(host("localhost")), "localhost");
        assert_eq!(letter(ProviderRef::of_id("mistral")), "mistral");
    }

    #[test]
    fn an_entry_with_no_icon_is_the_letter_of_its_name() {
        let logo: Cow<'static, [u8]> = Cow::Borrowed(b"<svg/>");

        assert_eq!(EntryIcon::of(Some(&logo), "Codex"), EntryIcon::Svg(logo));
        assert_eq!(
            EntryIcon::of(None, "api.deepseek.com"),
            EntryIcon::Letter(String::from("api.deepseek.com"))
        );
    }
}
