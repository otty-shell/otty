//! The icon of an agent or a provider as the registry gives it (spec
//! 2026-11-19-agent-plugins, "Links in the interface"): the SVG of its
//! entry, else the first letter of its name.

use std::borrow::Cow;
use std::fmt;

use iced::widget::svg;

/// How the icon of an agent or a provider is drawn.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum EntryIcon {
    /// The SVG of its registry entry, tinted by the app.
    Svg(Cow<'static, [u8]>),
    /// The first letter of this name on a rounded square: there is no
    /// entry, or the entry has no icon.
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

#[cfg(test)]
mod tests {
    use super::*;

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
